use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

fn admin_only(claims: &Claims) -> ApiResult<()> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    Ok(())
}

// ---------------- prices (products) ----------------
#[derive(Serialize, sqlx::FromRow)]
pub struct Price {
    pub id: uuid::Uuid,
    pub label: String,
    pub kind: String,
    pub plan: Option<String>,
    pub duration_days: Option<i32>,
    pub credits_cents: Option<i64>,
    pub amount_cents: i64,
    pub active: bool,
    pub sort: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct PriceReq {
    pub label: String,
    pub kind: String,
    pub plan: Option<String>,
    pub duration_days: Option<i32>,
    pub credits_cents: Option<i64>,
    pub amount_cents: i64,
    pub sort: Option<i32>,
}

fn validate_product(
    kind: &str,
    plan: &Option<String>,
    duration_days: Option<i32>,
    credits_cents: Option<i64>,
) -> ApiResult<()> {
    match kind {
        "plan" => {
            let p = plan.as_deref().unwrap_or("");
            if !crate::codes::plan_is_grantable(p) {
                return Err(AppError::bad("套餐无效"));
            }
            if duration_days.unwrap_or(0) <= 0 {
                return Err(AppError::bad("时长(天)需大于 0"));
            }
        }
        "credits" => {
            if credits_cents.unwrap_or(0) <= 0 {
                return Err(AppError::bad("额度需大于 0"));
            }
        }
        _ => return Err(AppError::bad("类型只能是 plan 或 credits")),
    }
    Ok(())
}

/// GET /api/prices — public list of products for sale (active only).
pub async fn list_prices_public(State(state): State<AppState>) -> ApiResult<Json<Vec<Price>>> {
    let rows = sqlx::query_as::<_, Price>(
        "SELECT * FROM prices WHERE active = true ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

/// GET /api/admin/prices — all products (admin).
pub async fn admin_list_prices(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<Vec<Price>>> {
    admin_only(&claims)?;
    let rows = sqlx::query_as::<_, Price>("SELECT * FROM prices ORDER BY sort, created_at")
        .fetch_all(&state.db)
        .await?;
    Ok(Json(rows))
}

/// POST /api/admin/prices — create a product (admin).
pub async fn admin_create_price(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<PriceReq>,
) -> ApiResult<Json<Price>> {
    admin_only(&claims)?;
    if req.label.trim().is_empty() {
        return Err(AppError::bad("请填写名称"));
    }
    if req.amount_cents <= 0 {
        return Err(AppError::bad("价格需大于 0"));
    }
    validate_product(&req.kind, &req.plan, req.duration_days, req.credits_cents)?;
    let row = sqlx::query_as::<_, Price>(
        "INSERT INTO prices (label, kind, plan, duration_days, credits_cents, amount_cents, sort) \
         VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING *",
    )
    .bind(req.label.trim())
    .bind(&req.kind)
    .bind(&req.plan)
    .bind(req.duration_days)
    .bind(req.credits_cents)
    .bind(req.amount_cents)
    .bind(req.sort.unwrap_or(0))
    .fetch_one(&state.db)
    .await?;
    Ok(Json(row))
}

/// DELETE /api/admin/prices/:id (admin).
pub async fn admin_delete_price(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let res = sqlx::query("DELETE FROM prices WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::bad("商品不存在"));
    }
    Ok(Json(json!({ "ok": true })))
}

// ---------------- orders ----------------
#[derive(Serialize, sqlx::FromRow)]
pub struct Order {
    pub id: uuid::Uuid,
    pub user_id: Option<uuid::Uuid>,
    pub email: String,
    pub price_id: Option<uuid::Uuid>,
    pub kind: String,
    pub plan: Option<String>,
    pub duration_days: Option<i32>,
    pub credits_cents: Option<i64>,
    pub amount_cents: i64,
    pub status: String,
    pub method: String,
    pub note: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub paid_at: Option<chrono::DateTime<chrono::Utc>>,
    /// What Stripe actually took, and in what currency. NULL on manual grants and on every
    /// order placed before migration 20260827.
    ///
    /// `amount_cents` above is the catalogue's CNY shelf price in fen — 18800 for 「Power」 —
    /// so rendering it as USD reported a $34.99 sale as $188.00. Anything showing money
    /// should prefer this pair and fall back to `amount_cents` only when it is NULL.
    pub charged_cents: Option<i64>,
    pub charged_currency: Option<String>,
}

#[derive(Deserialize)]
pub struct BuyReq {
    pub price_id: uuid::Uuid,
}

/// POST /api/orders — a logged-in user creates an order for a product (the
/// IDE-facing buy endpoint). Stays 'pending' until a gateway callback or an
/// admin manual confirm grants it. Amount is taken from the server-side price.
pub async fn create_order(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<BuyReq>,
) -> ApiResult<Json<Order>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    let price = sqlx::query_as::<_, Price>("SELECT * FROM prices WHERE id = $1 AND active = true")
        .bind(req.price_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::bad("商品不存在或已下架"))?;
    let order = sqlx::query_as::<_, Order>(
        "INSERT INTO orders (user_id, email, price_id, kind, plan, duration_days, credits_cents, amount_cents, method) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'manual') RETURNING *",
    )
    .bind(uid)
    .bind(&claims.email)
    .bind(price.id)
    .bind(&price.kind)
    .bind(&price.plan)
    .bind(price.duration_days)
    .bind(price.credits_cents)
    .bind(price.amount_cents)
    .fetch_one(&state.db)
    .await?;
    crate::realtime::record_event(
        &state,
        Some(uid),
        "order_created",
        json!({ "email": claims.email, "amount_cents": price.amount_cents, "label": price.label }),
    )
    .await;
    Ok(Json(order))
}

/// GET /api/admin/orders — all orders (admin).
pub async fn admin_list_orders(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<Vec<Order>>> {
    admin_only(&claims)?;
    let rows =
        sqlx::query_as::<_, Order>("SELECT * FROM orders ORDER BY created_at DESC LIMIT 1000")
            .fetch_all(&state.db)
            .await?;
    Ok(Json(rows))
}

/// POST /api/admin/orders/:id/confirm — mark a pending order paid and grant it (admin).
pub async fn admin_confirm_order(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let mut tx = state.db.begin().await?;
    let order = sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::bad("订单不存在"))?;
    if order.status != "pending" {
        return Err(AppError::bad("订单状态不是待支付"));
    }
    // Manual confirmation is for orders settled outside Stripe. A Stripe order is a
    // checkout session, and Stripe decides whether it was paid — the webhook grants it.
    //
    // Without this guard the console showed a 确认收款 button beside every abandoned
    // checkout, and pressing it granted the plan for money that was never taken. Of the
    // five pending Stripe rows on the console right now, Stripe reports all five as
    // expired and unpaid.
    if order.method == "stripe" {
        return Err(AppError::bad(
            "这是 Stripe 订单，付款状态由 Stripe 决定，不能手动确认。已付款的订单会由 webhook 自动发放。",
        ));
    }
    let uid = order
        .user_id
        .ok_or_else(|| AppError::bad("订单无关联用户，无法发放"))?;
    if order.kind == "plan" {
        crate::codes::apply_plan(
            &mut tx,
            uid,
            order.plan.as_deref().unwrap_or("none"),
            order.duration_days.unwrap_or(0),
        )
        .await?;
    } else {
        crate::codes::apply_credits(&mut tx, uid, order.credits_cents.unwrap_or(0)).await?;
    }
    sqlx::query("UPDATE orders SET status = 'paid', paid_at = now() WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    crate::realtime::record_event(
        &state,
        Some(uid),
        "order_paid",
        json!({ "email": order.email, "amount_cents": order.amount_cents, "by": claims.email }),
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

/// POST /api/admin/orders/:id/cancel (admin).
pub async fn admin_cancel_order(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let res =
        sqlx::query("UPDATE orders SET status = 'canceled' WHERE id = $1 AND status = 'pending'")
            .bind(id)
            .execute(&state.db)
            .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::bad("订单不存在或状态不可取消"));
    }
    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    /// The guard is a string comparison on `orders.method`, so the test that matters is
    /// that the source still refuses "stripe" before reaching the grant. Asserting on the
    /// shipped text keeps a future edit from quietly dropping it: without this guard the
    /// console grants a plan for an abandoned checkout, which is free product.
    #[test]
    fn manual_confirmation_refuses_stripe_orders() {
        let src = include_str!("pay.rs");
        let body = src
            .split("pub async fn admin_confirm_order")
            .nth(1)
            .expect("admin_confirm_order must exist");
        let guard = body.find(r#"order.method == "stripe""#).expect("stripe orders must be refused");
        let grant = body.find("apply_plan").expect("the grant call must exist");
        assert!(guard < grant, "the method guard must come before anything is granted");
    }
}
