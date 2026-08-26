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
    /// 目录里的**人民币**标价，分。「主力」是 29500 = ¥295。
    pub amount_cents: i64,
    /// 同一件商品的美元标价，美分。可以为空 —— 控制台建的商品填不了它（PriceReq 没有这个
    /// 字段），于是这一栏是 NULL，而结账时给美元买家用的正是它。空着不会报错，只是那件
    /// 商品在美元线上没有自己的价，所以控制台必须把这个空**显示出来**，否则运营看不见。
    pub amount_usd_cents: Option<i64>,
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
    /// 下单时**打算**按哪个币种收（按 IP/语言/时区猜的，stripe.rs:426）。
    ///
    /// 它不是「真按这个币收了」：charge_ccy = usd 时根本不给 Stripe 传 currency 参数，
    /// Stripe 会按该价格的 base currency 结算。真实币种只有 charged_currency 说了算。
    /// 之所以还要下发它，是因为**没成交的订单**没有 charged_*，而控制台要说出「这个买家
    /// 当时看到的是多少钱」—— 美元买家看到的是 prices.amount_usd_cents，不是 amount_cents。
    pub resolved_currency: Option<String>,
    /// 买了几份。amount_cents 已经乘过它了，美元标价那一路要自己乘。
    pub quantity: Option<i32>,
    /// 退过款的时间。**注意库里没有「退了多少钱」**，只有这个时间戳，所以任何地方都
    /// 无法把退款金额从营收里减掉。控制台的做法是：照常计入，但把笔数明说出来 ——
    /// 减一个猜出来的数，比标注一个已知的缺口更糟。
    pub refunded_at: Option<chrono::DateTime<chrono::Utc>>,
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

// 人工「确认收款」已删除（2026-08-26）。
//
// 它曾是全系统**唯一由人**把订单写成 paid 的地方，而它能作用的集合是
// `status='pending' AND method<>'stripe'` —— 也就是只剩 `create_order`（POST /api/orders）
// 造出来的手工单，而前端三个 UI 没有任何一处调它。
//
// Stripe 那条线不需要它：webhook 漏掉的单会被 `stripe::spawn_reconciler` 每 10 分钟
// 补一次，走的是和 webhook 完全相同的 `fulfil_session`。所以删掉它不会让任何一笔真实
// 付款停在未支付。
//
// 删掉的理由不是它没用，而是它**能让账面上出现没收到的钱**：按一下，订单变成已支付、
// 权益立即发放，而没有任何一笔进账与之对应。控制台现在只如实显示付没付。
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
    /// 控制台不许再有人工把订单写成「已支付」的路径。
    ///
    /// 原来的 `admin_confirm_order` 是全系统唯一由人写 'paid' 的地方。它删掉之后，
    /// 唯一还会写 'paid' 的是 Stripe 那条线（webhook + 对账器，都走 `fulfil_session`）。
    /// 这条测试守的是「别再加回来」—— 加回来的症状不是报错，是账面上出现没收到的钱。
    #[test]
    fn nothing_lets_a_human_mark_an_order_paid() {
        // 只看**生产代码**那一段：这个断言的字面量就写在本文件的测试模块里，
        // 不切掉的话它永远匹配到自己，于是这条测试无论如何都是红的（第一版就是这么翻的）。
        let whole = include_str!("pay.rs");
        let src = whole
            .split_once("\n#[cfg(test)]")
            .map(|(head, _)| head)
            .expect("pay.rs 里应该有测试模块");
        assert!(
            !src.contains("pub async fn admin_confirm_order"),
            "人工确认收款被加回来了",
        );
        // 取消仍然保留：它只是把一笔没付的单关掉，不会凭空造出收入。
        assert!(src.contains("pub async fn admin_cancel_order"), "取消订单不该被一起删掉");
        let cancel = src
            .split("pub async fn admin_cancel_order")
            .nth(1)
            .expect("admin_cancel_order 必须存在");
        let body = &cancel[..cancel.len().min(1200)];
        assert!(
            body.contains("status = 'canceled'") && !body.contains("status = 'paid'"),
            "取消订单只能写 canceled",
        );

        // 控制台那一侧：按钮和它调的接口都得没有，否则界面上还留着一个必然 404 的按钮。
        let ui = include_str!("../admin-ui/src/pages/Billing.tsx");
        assert!(!ui.contains("/confirm`"), "控制台还在调确认收款接口");
        assert!(
            !ui.contains(">\n                              确认收款"),
            "确认收款按钮还在界面上",
        );
        // 金额不许再用那个写死 `$` 的 cents() —— 这一屏的钱有人民币也有美元，
        // 库里 42590 是人民币，用它渲染就写成 $425.90。
        // 口径只许有一份：收款页和总览页都得从 lib/money.ts 取。
        // 以前两屏各写一份，收款页先改好、总览页没跟上，而总览页注释里还写着
        // 「Billing.tsx 已经改过」—— 抄出来的东西就是这么烂在原地的。
        let overview = include_str!("../admin-ui/src/pages/Overview.tsx");
        for (name, src) in [("收款页", ui), ("总览页", overview)] {
            // 带上结尾引号：只写 "@/lib/money" 的话，`from "@/lib/moneyX"` 也会命中，
            // 这条断言就形同虚设（变异测试当场翻出来的）。
            assert!(
                src.contains("from \"@/lib/money\""),
                "{name}没用共享的金额口径，多半又自己写了一份",
            );
        }
        assert!(
            !ui.contains("cents(revenue)")
                && !ui.contains("cents(p.amount_cents)")
                && !overview.contains("cents(revenue)")
                && !overview.contains("cents(o.amount_cents)"),
            "还有地方拿写死美元符号的 cents() 渲染人民币金额",
        );
        // 「确认收款」这四个字不该再出现在控制台的**任何**可见文案里 —— 包括实时动态那张
        // 事件名映射表（order_paid 原来就写着「确认收款」，接口都 404 了它还挂在页面上）。
        for (name, src) in [("收款页", ui), ("总览页", overview)] {
            let visible: String = src
                .lines()
                .filter(|l| !l.trim_start().starts_with("//") && !l.trim_start().starts_with("*"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                !visible.contains("确认收款"),
                "{name}还有「确认收款」这个说法留在可见文案里",
            );
        }
        // 共享模块本身不许有默认币种：一个默认值就等于又把 `$` 写死了一次。
        let money = include_str!("../admin-ui/src/lib/money.ts");
        assert!(
            money.contains("export function formatMoney(minor: number, ccy: string)"),
            "formatMoney 的签名变了 —— 币种必须是必填参数",
        );
    }
}
