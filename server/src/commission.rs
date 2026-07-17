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

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Commission {
    pub id: uuid::Uuid,
    pub referrer_user_id: Option<uuid::Uuid>,
    pub referrer_email: String,
    pub customer_user_id: Option<uuid::Uuid>,
    pub customer_email: String,
    pub order_id: Option<uuid::Uuid>,
    pub source: String,
    pub amount_cents: i64,
    pub rate_bps: i32,
    pub commission_cents: i64,
    pub status: String,
    pub note: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub settled_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
pub struct CommissionSummary {
    pub total_cents: i64,
    pub pending_cents: i64,
    pub settled_cents: i64,
    pub rejected_cents: i64,
    pub total_count: i64,
    pub pending_count: i64,
    pub settled_count: i64,
    pub rejected_count: i64,
}

#[derive(Debug, Serialize)]
pub struct CommissionList {
    pub rows: Vec<Commission>,
    pub summary: CommissionSummary,
}

#[derive(Debug, Deserialize)]
pub struct CommissionCreateReq {
    pub referrer_email: String,
    pub customer_email: Option<String>,
    pub order_id: Option<uuid::Uuid>,
    pub source: Option<String>,
    pub amount_cents: i64,
    pub rate_bps: i32,
    pub commission_cents: Option<i64>,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CommissionStatusReq {
    pub status: String,
    pub note: Option<String>,
}

fn normalize_email(value: &str) -> String {
    value.trim().to_lowercase()
}

fn validate_status(status: &str) -> ApiResult<()> {
    match status {
        "pending" | "settled" | "rejected" => Ok(()),
        _ => Err(AppError::bad("佣金状态只能是 pending / settled / rejected")),
    }
}

fn calculated_commission(amount_cents: i64, rate_bps: i32) -> i64 {
    ((amount_cents as i128 * rate_bps as i128) / 10_000) as i64
}

async fn user_id_by_email(state: &AppState, email: &str) -> ApiResult<Option<uuid::Uuid>> {
    if email.is_empty() {
        return Ok(None);
    }
    Ok(
        sqlx::query_scalar::<_, uuid::Uuid>("SELECT id FROM users WHERE lower(email) = lower($1)")
            .bind(email)
            .fetch_optional(&state.db)
            .await?,
    )
}

/// GET /api/admin/commissions — list commission rows plus aggregate totals.
pub async fn admin_list_commissions(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<CommissionList>> {
    admin_only(&claims)?;
    let rows = sqlx::query_as::<_, Commission>(
        "SELECT * FROM commissions ORDER BY created_at DESC LIMIT 1000",
    )
    .fetch_all(&state.db)
    .await?;

    let mut summary = CommissionSummary {
        total_cents: 0,
        pending_cents: 0,
        settled_cents: 0,
        rejected_cents: 0,
        total_count: 0,
        pending_count: 0,
        settled_count: 0,
        rejected_count: 0,
    };
    for row in &rows {
        summary.total_cents += row.commission_cents;
        summary.total_count += 1;
        match row.status.as_str() {
            "pending" => {
                summary.pending_cents += row.commission_cents;
                summary.pending_count += 1;
            }
            "settled" => {
                summary.settled_cents += row.commission_cents;
                summary.settled_count += 1;
            }
            "rejected" => {
                summary.rejected_cents += row.commission_cents;
                summary.rejected_count += 1;
            }
            _ => {}
        }
    }

    Ok(Json(CommissionList { rows, summary }))
}

/// POST /api/admin/commissions — create a manual commission row.
pub async fn admin_create_commission(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<CommissionCreateReq>,
) -> ApiResult<Json<Commission>> {
    admin_only(&claims)?;

    let referrer_email = normalize_email(&req.referrer_email);
    let customer_email = normalize_email(req.customer_email.as_deref().unwrap_or(""));
    if !crate::auth::valid_email(&referrer_email) {
        return Err(AppError::bad("请填写有效的推广员邮箱"));
    }
    if !customer_email.is_empty() && !crate::auth::valid_email(&customer_email) {
        return Err(AppError::bad("客户邮箱格式无效"));
    }
    if req.amount_cents <= 0 {
        return Err(AppError::bad("成交金额需大于 0"));
    }
    if !(0..=10_000).contains(&req.rate_bps) {
        return Err(AppError::bad("佣金比例需在 0% 到 100% 之间"));
    }
    let commission_cents = req
        .commission_cents
        .filter(|v| *v >= 0)
        .unwrap_or_else(|| calculated_commission(req.amount_cents, req.rate_bps));
    if commission_cents > req.amount_cents {
        return Err(AppError::bad("佣金金额不能大于成交金额"));
    }

    let referrer_user_id = user_id_by_email(&state, &referrer_email).await?;
    let customer_user_id = user_id_by_email(&state, &customer_email).await?;
    let source = req
        .source
        .unwrap_or_else(|| "manual".to_string())
        .trim()
        .chars()
        .take(40)
        .collect::<String>();
    let note = req
        .note
        .unwrap_or_default()
        .trim()
        .chars()
        .take(500)
        .collect::<String>();

    let row = sqlx::query_as::<_, Commission>(
        "INSERT INTO commissions \
         (referrer_user_id, referrer_email, customer_user_id, customer_email, order_id, source, amount_cents, rate_bps, commission_cents, note) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING *",
    )
    .bind(referrer_user_id)
    .bind(&referrer_email)
    .bind(customer_user_id)
    .bind(&customer_email)
    .bind(req.order_id)
    .bind(if source.is_empty() { "manual" } else { &source })
    .bind(req.amount_cents)
    .bind(req.rate_bps)
    .bind(commission_cents)
    .bind(&note)
    .fetch_one(&state.db)
    .await?;

    crate::realtime::record_event(
        &state,
        referrer_user_id,
        "commission_created",
        json!({
            "referrer_email": referrer_email,
            "customer_email": customer_email,
            "amount_cents": req.amount_cents,
            "commission_cents": commission_cents,
            "by": claims.email,
        }),
    )
    .await;

    Ok(Json(row))
}

/// POST /api/admin/commissions/:id/status — pending / settled / rejected.
pub async fn admin_update_commission_status(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<CommissionStatusReq>,
) -> ApiResult<Json<Commission>> {
    admin_only(&claims)?;
    validate_status(req.status.as_str())?;
    let note = req.note.unwrap_or_default();
    let row = sqlx::query_as::<_, Commission>(
        "UPDATE commissions \
         SET status = $2, \
             note = CASE WHEN $3::text = '' THEN note ELSE $3 END, \
             settled_at = CASE WHEN $2 = 'settled' THEN now() ELSE NULL END, \
             updated_at = now() \
         WHERE id = $1 \
         RETURNING *",
    )
    .bind(id)
    .bind(&req.status)
    .bind(note.trim())
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::bad("佣金记录不存在"))?;

    crate::realtime::record_event(
        &state,
        row.referrer_user_id,
        "commission_status",
        json!({
            "referrer_email": row.referrer_email,
            "status": row.status,
            "commission_cents": row.commission_cents,
            "by": claims.email,
        }),
    )
    .await;

    Ok(Json(row))
}

/// DELETE /api/admin/commissions/:id — remove a commission record.
pub async fn admin_delete_commission(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let res = sqlx::query("DELETE FROM commissions WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::bad("佣金记录不存在"));
    }
    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commission_uses_basis_points_without_float_drift() {
        assert_eq!(calculated_commission(9_900, 2_000), 1_980); // $99 × 20%
        assert_eq!(calculated_commission(1_999, 1_250), 249); // floor cents
        assert_eq!(calculated_commission(10_000, 0), 0);
        assert_eq!(calculated_commission(10_000, 10_000), 10_000);
    }

    #[test]
    fn commission_status_is_strict() {
        assert!(validate_status("pending").is_ok());
        assert!(validate_status("settled").is_ok());
        assert!(validate_status("rejected").is_ok());
        assert!(validate_status("paid").is_err());
    }
}
