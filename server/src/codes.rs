use axum::extract::{Path, State};
use axum::Json;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

/// Membership tiers sold on the platform.
pub const PLANS: [&str; 5] = ["trial", "basic", "pro", "power", "ultra"];

#[derive(Serialize, sqlx::FromRow)]
pub struct Code {
    pub id: uuid::Uuid,
    pub code: String,
    pub kind: String,
    pub plan: Option<String>,
    pub duration_days: Option<i32>,
    pub credits_cents: Option<i64>,
    pub note: String,
    pub status: String,
    pub used_by: Option<uuid::Uuid>,
    pub used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

fn admin_only(claims: &Claims) -> ApiResult<()> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    Ok(())
}

/// A readable random code like `AB3K-9F2M-7QX4-WPED`.
fn gen_code() -> String {
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // no easily-confused chars
    let mut rng = rand::thread_rng();
    let mut s = String::with_capacity(19);
    for g in 0..4 {
        if g > 0 {
            s.push('-');
        }
        for _ in 0..4 {
            s.push(CHARS[rng.gen_range(0..CHARS.len())] as char);
        }
    }
    s
}

// ---------- admin: generate ----------
#[derive(Deserialize)]
pub struct GenReq {
    pub kind: String,
    pub plan: Option<String>,
    pub duration_days: Option<i32>,
    pub credits_cents: Option<i64>,
    pub count: i32,
    pub note: Option<String>,
}

pub async fn admin_generate(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<GenReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let count = req.count.clamp(1, 500);
    match req.kind.as_str() {
        "plan" => {
            let plan = req.plan.as_deref().unwrap_or("");
            if !PLANS.contains(&plan) {
                return Err(AppError::bad("套餐无效"));
            }
            if req.duration_days.unwrap_or(0) <= 0 {
                return Err(AppError::bad("时长(天)需大于 0"));
            }
        }
        "credits" => {
            if req.credits_cents.unwrap_or(0) <= 0 {
                return Err(AppError::bad("额度需大于 0"));
            }
        }
        _ => return Err(AppError::bad("类型只能是 plan 或 credits")),
    }
    let note = req.note.unwrap_or_default();
    let mut codes = Vec::with_capacity(count as usize);
    for _ in 0..count {
        // Retry a few times on the (very unlikely) unique collision.
        let mut attempt = 0;
        loop {
            let code = gen_code();
            let res = sqlx::query(
                "INSERT INTO activation_codes (code, kind, plan, duration_days, credits_cents, note) VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(&code)
            .bind(&req.kind)
            .bind(&req.plan)
            .bind(req.duration_days)
            .bind(req.credits_cents)
            .bind(&note)
            .execute(&state.db)
            .await;
            match res {
                Ok(_) => {
                    codes.push(code);
                    break;
                }
                Err(e) => {
                    attempt += 1;
                    if attempt >= 5 {
                        return Err(e.into());
                    }
                }
            }
        }
    }
    Ok(Json(json!({ "codes": codes, "count": codes.len() })))
}

// ---------- admin: list ----------
pub async fn admin_list(State(state): State<AppState>, claims: Claims) -> ApiResult<Json<Vec<Code>>> {
    admin_only(&claims)?;
    let rows = sqlx::query_as::<_, Code>("SELECT * FROM activation_codes ORDER BY created_at DESC LIMIT 1000")
        .fetch_all(&state.db)
        .await?;
    Ok(Json(rows))
}

// ---------- admin: delete ----------
pub async fn admin_delete(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let res = sqlx::query("DELETE FROM activation_codes WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::bad("激活码不存在"));
    }
    Ok(Json(json!({ "ok": true })))
}

// ---------- apply a grant to a user (shared by redeem + admin grant) ----------
pub(crate) async fn apply_plan(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    uid: uuid::Uuid,
    plan: &str,
    days: i32,
) -> ApiResult<()> {
    // Extend from the later of (now, existing expiry) so stacking codes adds up.
    sqlx::query(
        "UPDATE users SET plan = $1, \
         plan_expires_at = GREATEST(COALESCE(plan_expires_at, now()), now()) + ($2 * interval '1 day'), \
         updated_at = now() WHERE id = $3",
    )
    .bind(plan)
    .bind(days)
    .bind(uid)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn apply_credits(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    uid: uuid::Uuid,
    cents: i64,
) -> ApiResult<()> {
    sqlx::query("UPDATE users SET credits_cents = credits_cents + $1, updated_at = now() WHERE id = $2")
        .bind(cents)
        .bind(uid)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn user_summary(state: &AppState, uid: uuid::Uuid) -> ApiResult<serde_json::Value> {
    let row = sqlx::query_as::<_, (String, Option<chrono::DateTime<chrono::Utc>>, i64)>(
        "SELECT plan, plan_expires_at, credits_cents FROM users WHERE id = $1",
    )
    .bind(uid)
    .fetch_one(&state.db)
    .await?;
    Ok(json!({ "plan": row.0, "plan_expires_at": row.1, "credits_cents": row.2 }))
}

// ---------- user: redeem (the IDE-facing endpoint) ----------
#[derive(Deserialize)]
pub struct RedeemReq {
    pub code: String,
}

pub async fn redeem(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<RedeemReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    let input = req.code.trim().to_uppercase();
    if input.is_empty() {
        return Err(AppError::bad("请输入激活码"));
    }

    let mut tx = state.db.begin().await?;
    let code = sqlx::query_as::<_, Code>("SELECT * FROM activation_codes WHERE code = $1 FOR UPDATE")
        .bind(&input)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::bad("激活码无效"))?;
    if code.status != "unused" {
        return Err(AppError::bad("激活码已被使用"));
    }
    let granted = if code.kind == "plan" {
        let plan = code.plan.clone().unwrap_or_else(|| "none".into());
        apply_plan(&mut tx, uid, &plan, code.duration_days.unwrap_or(0)).await?;
        json!({ "kind": "plan", "plan": plan, "duration_days": code.duration_days })
    } else {
        apply_credits(&mut tx, uid, code.credits_cents.unwrap_or(0)).await?;
        json!({ "kind": "credits", "credits_cents": code.credits_cents })
    };
    sqlx::query("UPDATE activation_codes SET status = 'used', used_by = $1, used_at = now() WHERE id = $2")
        .bind(uid)
        .bind(code.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    crate::realtime::record_event(&state, Some(uid), "redeem", json!({ "email": claims.email, "grant": granted })).await;
    Ok(Json(json!({ "ok": true, "granted": granted, "user": user_summary(&state, uid).await? })))
}

// ---------- admin: manually grant a plan / credits to a user ----------
#[derive(Deserialize)]
pub struct GrantReq {
    pub kind: String,
    pub plan: Option<String>,
    pub duration_days: Option<i32>,
    pub credits_cents: Option<i64>,
}

pub async fn admin_grant(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<GrantReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let mut tx = state.db.begin().await?;
    match req.kind.as_str() {
        "plan" => {
            let plan = req.plan.as_deref().unwrap_or("");
            if !PLANS.contains(&plan) {
                return Err(AppError::bad("套餐无效"));
            }
            if req.duration_days.unwrap_or(0) <= 0 {
                return Err(AppError::bad("时长(天)需大于 0"));
            }
            apply_plan(&mut tx, id, plan, req.duration_days.unwrap_or(0)).await?;
        }
        "credits" => {
            if req.credits_cents.unwrap_or(0) == 0 {
                return Err(AppError::bad("额度不能为 0"));
            }
            apply_credits(&mut tx, id, req.credits_cents.unwrap_or(0)).await?;
        }
        _ => return Err(AppError::bad("类型只能是 plan 或 credits")),
    }
    tx.commit().await?;
    crate::realtime::record_event(&state, Some(id), "grant", json!({ "by": claims.email, "kind": req.kind })).await;
    Ok(Json(json!({ "ok": true, "user": user_summary(&state, id).await? })))
}
