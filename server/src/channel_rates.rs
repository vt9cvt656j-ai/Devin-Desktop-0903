use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

const MAX_NAME_CHARS: usize = 80;
const MAX_NOTE_CHARS: usize = 500;
const MAX_RATE: f64 = 1_000_000.0;

fn admin_only(claims: &Claims) -> ApiResult<()> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    Ok(())
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ChannelRate {
    pub id: uuid::Uuid,
    pub name: String,
    pub usd_per_cny: f64,
    pub note: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ChannelRateReq {
    pub name: String,
    pub usd_per_cny: f64,
    pub note: Option<String>,
}

fn validate_request(req: ChannelRateReq) -> ApiResult<(String, f64, String)> {
    let name = req.name.trim().to_string();
    let note = req.note.unwrap_or_default().trim().to_string();

    if name.is_empty() {
        return Err(AppError::bad("请填写渠道名称"));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(AppError::bad("渠道名称不能超过 80 个字符"));
    }
    if !req.usd_per_cny.is_finite() || req.usd_per_cny <= 0.0 || req.usd_per_cny > MAX_RATE {
        return Err(AppError::bad("渠道汇率必须是有效的正数"));
    }
    if note.chars().count() > MAX_NOTE_CHARS {
        return Err(AppError::bad("备注不能超过 500 个字符"));
    }

    Ok((name, req.usd_per_cny, note))
}

fn map_write_error(error: sqlx::Error) -> AppError {
    if error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .as_deref()
        == Some("23505")
    {
        AppError::bad("渠道名称已存在")
    } else {
        error.into()
    }
}

/// GET /api/admin/channel-rates - list all saved channel exchange rates.
pub async fn admin_list(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<Vec<ChannelRate>>> {
    admin_only(&claims)?;
    let rows = sqlx::query_as::<_, ChannelRate>(
        "SELECT * FROM channel_rates ORDER BY created_at, lower(name)",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

/// POST /api/admin/channel-rates - create a channel exchange rate.
pub async fn admin_create(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<ChannelRateReq>,
) -> ApiResult<Json<ChannelRate>> {
    admin_only(&claims)?;
    let (name, usd_per_cny, note) = validate_request(req)?;
    let row = sqlx::query_as::<_, ChannelRate>(
        "INSERT INTO channel_rates (name, usd_per_cny, note) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(name)
    .bind(usd_per_cny)
    .bind(note)
    .fetch_one(&state.db)
    .await
    .map_err(map_write_error)?;
    Ok(Json(row))
}

/// POST /api/admin/channel-rates/:id - update a channel exchange rate.
pub async fn admin_update(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<ChannelRateReq>,
) -> ApiResult<Json<ChannelRate>> {
    admin_only(&claims)?;
    let (name, usd_per_cny, note) = validate_request(req)?;
    let row = sqlx::query_as::<_, ChannelRate>(
        "UPDATE channel_rates SET name = $2, usd_per_cny = $3, note = $4, updated_at = now() WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(name)
    .bind(usd_per_cny)
    .bind(note)
    .fetch_optional(&state.db)
    .await
    .map_err(map_write_error)?
    .ok_or_else(|| AppError::bad("渠道汇率不存在"))?;
    Ok(Json(row))
}

/// DELETE /api/admin/channel-rates/:id - delete a channel exchange rate.
pub async fn admin_delete(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let result = sqlx::query("DELETE FROM channel_rates WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::bad("渠道汇率不存在"));
    }
    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(name: &str, rate: f64) -> ChannelRateReq {
        ChannelRateReq {
            name: name.to_string(),
            usd_per_cny: rate,
            note: None,
        }
    }

    #[test]
    fn validates_and_trims_channel_rate() {
        let result = validate_request(ChannelRateReq {
            name: "  渠道 A  ".to_string(),
            usd_per_cny: 6.63,
            note: Some("  主渠道  ".to_string()),
        });
        assert!(result.is_ok());
        let (name, rate, note) = match result {
            Ok(value) => value,
            Err(_) => unreachable!("validated above"),
        };
        assert_eq!(name, "渠道 A");
        assert_eq!(rate, 6.63);
        assert_eq!(note, "主渠道");
    }

    #[test]
    fn rejects_invalid_channel_rates() {
        assert!(validate_request(request("", 6.63)).is_err());
        assert!(validate_request(request("渠道", 0.0)).is_err());
        assert!(validate_request(request("渠道", -1.0)).is_err());
        assert!(validate_request(request("渠道", f64::INFINITY)).is_err());
    }
}
