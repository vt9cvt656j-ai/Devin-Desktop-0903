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
pub async fn admin_list(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<Vec<Code>>> {
    admin_only(&claims)?;
    let rows = sqlx::query_as::<_, Code>(
        "SELECT * FROM activation_codes ORDER BY created_at DESC LIMIT 1000",
    )
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
/// Per-plan quota spec: (total_cents, window_cap_cents, weekly_cap_cents, duration_days).
/// Amounts are USD cents; window_cap 0 = window disabled, weekly cap is primary (P0 quota fix).
pub(crate) fn plan_spec(plan: &str) -> Option<(i64, i64, i64, i32)> {
    match plan {
        "trial" => Some((5_000, 0, 500, 1)), // $50 total, $5/week cap, 1 day, ¥8.8
        "basic" => Some((33_000, 0, 5_000, 30)), // $330 total, $50/week cap, 30 days, ¥88
        "pro" => Some((65_000, 0, 10_000, 30)), // $650 total, $100/week cap, 30 days, ¥188
        "power" => Some((180_000, 0, 30_000, 30)), // $1800 total, $300/week cap, 30 days, ¥488
        "ultra" => Some((500_000, 0, 80_000, 30)), // $5000 total, $800/week cap, 30 days
        _ => None,
    }
}

/// Ordering of the built-in plans, so a grant never silently downgrades a user who
/// still holds a better plan. Unknown/absent plans rank 0.
pub(crate) fn plan_rank(plan: &str) -> i32 {
    match plan {
        "trial" => 1,
        "basic" => 2,
        "pro" => 3,
        "power" => 4,
        "ultra" => 5,
        _ => 0,
    }
}

pub(crate) async fn apply_plan(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    uid: uuid::Uuid,
    plan: &str,
    days: i32,
) -> ApiResult<()> {
    if let Some((total, window, weekly, spec_days)) = plan_spec(plan) {
        let dur = if days > 0 { days } else { spec_days };
        // Read the current state under a row lock: the new pools are derived from the
        // old ones, and two concurrent redeems must not both compute from the same base.
        let (cur_plan, cur_total, cur_window_cap, cur_window, cur_weekly): (
            String,
            i64,
            i64,
            i64,
            i64,
        ) = sqlx::query_as(
            "SELECT plan, quota_total_cents, quota_window_cap_cents, quota_window_cents, \
             quota_weekly_cap_cents FROM users WHERE id = $1 FOR UPDATE",
        )
        .bind(uid)
        .fetch_one(&mut **tx)
        .await?;

        // Quota is a balance, not a setting: grants ADD. Overwriting meant redeeming a
        // cheap code on top of an expensive plan destroyed the paid remainder (ultra
        // with $4000 left + a $50 trial code = $50), and stacking two of the same plan
        // only ever delivered one plan's worth.
        let new_total = cur_total.max(0).saturating_add(total);
        // Caps are entitlements: keep the better one rather than the newest one.
        let new_window_cap = cur_window_cap.max(window);
        // Don't shrink what's left in the live window; top it up toward the new cap.
        let new_window = cur_window.max(window.min(new_total)).min(new_window_cap);
        // 0 means "no weekly cap", so it wins over any finite cap.
        let new_weekly = if cur_weekly == 0 || weekly == 0 {
            0
        } else {
            cur_weekly.max(weekly)
        };
        let new_plan = if plan_rank(plan) >= plan_rank(&cur_plan) {
            plan.to_string()
        } else {
            cur_plan
        };

        // Note what is deliberately NOT reset here: quota_week_used_cents and the
        // window/week reset timestamps. Zeroing them on every grant let a user clear a
        // spent weekly cap (or refresh the 5.5h window) by redeeming any cheap code.
        // The access gate already rolls both windows over when their deadline passes.
        sqlx::query(
            "UPDATE users SET plan = $1, \
             plan_expires_at = GREATEST(COALESCE(plan_expires_at, now()), now()) + ($2 * interval '1 day'), \
             quota_total_cents = $3, quota_window_cap_cents = $4, quota_window_cents = $5, \
             quota_window_reset_at = COALESCE(quota_window_reset_at, now() + interval '5 hours 30 minutes'), \
             quota_weekly_cap_cents = $6, \
             quota_week_reset_at = COALESCE(quota_week_reset_at, now() + interval '7 days'), \
             updated_at = now() WHERE id = $7",
        )
        .bind(&new_plan)
        .bind(dur)
        .bind(new_total)
        .bind(new_window_cap)
        .bind(new_window)
        .bind(new_weekly)
        .bind(uid)
        .execute(&mut **tx)
        .await?;
    } else {
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
    }
    Ok(())
}

pub(crate) async fn apply_credits(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    uid: uuid::Uuid,
    cents: i64,
) -> ApiResult<()> {
    sqlx::query(
        "UPDATE users SET credits_cents = credits_cents + $1, updated_at = now() WHERE id = $2",
    )
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
    let code =
        sqlx::query_as::<_, Code>("SELECT * FROM activation_codes WHERE code = $1 FOR UPDATE")
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
    sqlx::query(
        "UPDATE activation_codes SET status = 'used', used_by = $1, used_at = now() WHERE id = $2",
    )
    .bind(uid)
    .bind(code.id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    crate::realtime::record_event(
        &state,
        Some(uid),
        "redeem",
        json!({ "email": claims.email, "grant": granted }),
    )
    .await;
    Ok(Json(
        json!({ "ok": true, "granted": granted, "user": user_summary(&state, uid).await? }),
    ))
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
    let summary = user_summary(&state, id).await?;
    crate::realtime::record_event(
        &state,
        Some(id),
        "user_updated",
        json!({ "by": claims.email, "action": "grant", "kind": req.kind, "user": summary.clone() }),
    )
    .await;
    Ok(Json(json!({ "ok": true, "user": summary })))
}

// ---------- admin: SET (not add) credits to an exact balance ----------
#[derive(Deserialize)]
pub struct SetCreditsReq {
    pub credits_cents: i64,
}

pub async fn admin_set_credits(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<SetCreditsReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    if req.credits_cents < 0 {
        return Err(AppError::bad("额度不能为负数"));
    }
    sqlx::query("UPDATE users SET credits_cents = $1, updated_at = now() WHERE id = $2")
        .bind(req.credits_cents)
        .bind(id)
        .execute(&state.db)
        .await?;
    let summary = user_summary(&state, id).await?;
    crate::realtime::record_event(&state, Some(id), "user_updated", json!({ "by": claims.email, "action": "set_credits", "credits_cents": req.credits_cents, "user": summary.clone() })).await;
    Ok(Json(json!({ "ok": true, "user": summary })))
}

// ---------- admin: SET plan + expiry to specific absolute values ----------
#[derive(Deserialize)]
pub struct SetPlanReq {
    pub plan: String,
    /// ISO-8601 timestamp for when the plan expires. If null/missing → plan stays
    /// active indefinitely (treated as far-future).
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When true, also reset the per-window / weekly quota counters to the new
    /// plan's caps (a fresh start). Defaults to true since admins usually want
    /// the user to feel the plan change immediately.
    #[serde(default = "default_true")]
    pub reset_quotas: bool,
}

fn default_true() -> bool {
    true
}

pub async fn admin_set_plan(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<SetPlanReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let plan = req.plan.trim();
    if plan != "none" && !PLANS.contains(&plan) {
        return Err(AppError::bad(
            "套餐无效（合法值: trial / basic / pro / power / ultra / none）",
        ));
    }
    if plan == "none" {
        // Treat plan="none" as a cancel — clear membership.
        sqlx::query(
            "UPDATE users SET plan = 'none', plan_expires_at = NULL, \
             quota_total_cents = 0, quota_window_cents = 0, quota_window_cap_cents = 0, \
             quota_weekly_cap_cents = 0, quota_week_used_cents = 0, \
             updated_at = now() WHERE id = $1",
        )
        .bind(id)
        .execute(&state.db)
        .await?;
    } else if req.reset_quotas {
        if let Some((total, window, weekly, _)) = plan_spec(plan) {
            sqlx::query(
                "UPDATE users SET plan = $1, plan_expires_at = $2, \
                 quota_total_cents = $3, quota_window_cap_cents = $4, quota_window_cents = LEAST($4, $3), \
                 quota_window_reset_at = now() + interval '5 hours 30 minutes', \
                 quota_weekly_cap_cents = $5, quota_week_used_cents = 0, quota_week_reset_at = now() + interval '7 days', \
                 updated_at = now() WHERE id = $6",
            )
            .bind(plan)
            .bind(req.expires_at)
            .bind(total)
            .bind(window)
            .bind(weekly)
            .bind(id)
            .execute(&state.db)
            .await?;
        } else {
            // Unknown plan_spec — at least set plan + expiry; quotas left as-is.
            sqlx::query("UPDATE users SET plan = $1, plan_expires_at = $2, updated_at = now() WHERE id = $3")
                .bind(plan)
                .bind(req.expires_at)
                .bind(id)
                .execute(&state.db)
                .await?;
        }
    } else {
        // Keep existing quotas, just retag the plan + expiry.
        sqlx::query(
            "UPDATE users SET plan = $1, plan_expires_at = $2, updated_at = now() WHERE id = $3",
        )
        .bind(plan)
        .bind(req.expires_at)
        .bind(id)
        .execute(&state.db)
        .await?;
    }
    let summary = user_summary(&state, id).await?;
    crate::realtime::record_event(&state, Some(id), "user_updated", json!({ "by": claims.email, "action": "set_plan", "plan": plan, "expires_at": req.expires_at, "user": summary.clone() })).await;
    Ok(Json(json!({ "ok": true, "user": summary })))
}

// ---------- admin: CANCEL a user's membership ----------
pub async fn admin_cancel_plan(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    sqlx::query(
        "UPDATE users SET plan = 'none', plan_expires_at = NULL, \
         quota_total_cents = 0, quota_window_cents = 0, quota_window_cap_cents = 0, \
         quota_weekly_cap_cents = 0, quota_week_used_cents = 0, \
         updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .execute(&state.db)
    .await?;
    let summary = user_summary(&state, id).await?;
    crate::realtime::record_event(
        &state,
        Some(id),
        "user_updated",
        json!({ "by": claims.email, "action": "cancel_plan", "user": summary.clone() }),
    )
    .await;
    Ok(Json(json!({ "ok": true, "user": summary })))
}
