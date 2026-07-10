//! Per-user Skills library: reusable AI prompts the user creates in the IDE, synced to the server so
//! they follow the account across devices / reinstalls. Stored as one JSON array per user.
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

fn uid(claims: &Claims) -> ApiResult<uuid::Uuid> {
    uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))
}

/// GET /api/skills — the current user's saved skills (array; `[]` if none yet).
pub async fn get_skills(State(state): State<AppState>, claims: Claims) -> ApiResult<Json<Value>> {
    let id = uid(&claims)?;
    let row: Option<(Value,)> = sqlx::query_as("SELECT skills FROM user_skills WHERE user_id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    Ok(Json(row.map(|r| r.0).unwrap_or_else(|| json!([]))))
}

/// PUT /api/skills — replace the user's whole skills list with the posted array (upsert). Bounded so
/// a single account can't store an unbounded blob.
pub async fn put_skills(
    State(state): State<AppState>,
    claims: Claims,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let id = uid(&claims)?;
    let arr = body
        .as_array()
        .ok_or_else(|| AppError::bad("skills 必须是数组"))?;
    if arr.len() > 200 {
        return Err(AppError::bad("技能数量过多（最多 200 个）"));
    }
    if body.to_string().len() > 512 * 1024 {
        return Err(AppError::bad("技能数据过大"));
    }
    sqlx::query(
        "INSERT INTO user_skills (user_id, skills, updated_at) VALUES ($1, $2, now()) \
         ON CONFLICT (user_id) DO UPDATE SET skills = EXCLUDED.skills, updated_at = now()",
    )
    .bind(id)
    .bind(&body)
    .execute(&state.db)
    .await?;
    Ok(Json(json!({ "ok": true, "count": arr.len() })))
}

/// GET /api/admin/skills — every user's skills (admin only), for the backend management view.
pub async fn admin_list_skills(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<Value>> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    let rows: Vec<(String, Value, String)> = sqlx::query_as(
        "SELECT u.email, s.skills, s.updated_at::text \
         FROM user_skills s JOIN users u ON u.id = s.user_id \
         ORDER BY s.updated_at DESC",
    )
    .fetch_all(&state.db)
    .await?;
    let users: Vec<Value> = rows
        .into_iter()
        .map(|(email, skills, updated_at)| {
            let count = skills.as_array().map(|a| a.len()).unwrap_or(0);
            json!({ "email": email, "count": count, "skills": skills, "updated_at": updated_at })
        })
        .collect();
    Ok(Json(json!({ "users": users })))
}
