use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

/// Shared, pooled HTTP client for upstream model calls. Building a fresh
/// `reqwest::Client` per request (the old behaviour) forced a brand-new TCP+TLS
/// handshake to the provider on every call — a large chunk of the "feels slow"
/// latency, and it compounds badly for an agent firing many sequential requests.
/// One pooled client keeps connections warm (keep-alive), so only the first call
/// to a host pays the handshake. No global timeout: streamed chat responses are
/// open-ended; only the connect phase is bounded (per-request timeouts are added
/// for the non-streaming calls that need them).
static GW_HTTP: std::sync::LazyLock<reqwest::Client> = std::sync::LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .pool_max_idle_per_host(16)
        .tcp_keepalive(std::time::Duration::from_secs(60))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});

fn admin_only(claims: &Claims) -> ApiResult<()> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    Ok(())
}

/// Normalize an OpenAI-compatible base URL: ensure it ends with a `/v1` segment
/// (so `https://gateway.example` becomes `https://gateway.example/v1`). If the
/// caller already included `/v1` (or any `/v1/...`), leave it untouched.
fn api_base(base: &str) -> String {
    let b = base.trim().trim_end_matches('/');
    if b.ends_with("/v1") || b.contains("/v1/") {
        b.to_string()
    } else {
        format!("{}/v1", b)
    }
}

#[derive(sqlx::FromRow)]
pub struct Model {
    pub id: uuid::Uuid,
    pub label: String,
    pub provider: String,
    pub base_url: String,
    pub model_id: Option<String>,
    pub api_key: String,
    pub price_cents: i64,
    pub rate: f64,
    pub active: bool,
    pub sort: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub enabled_models: Vec<String>,
}

fn allowed_ids(m: &Model) -> Vec<String> {
    if !m.enabled_models.is_empty() {
        return m.enabled_models.clone();
    }
    match &m.model_id {
        Some(s) if !s.is_empty() => vec![s.clone()],
        _ => vec![],
    }
}

/// Mask a secret for display: keep the last 4 chars.
fn mask(key: &str) -> String {
    if key.len() <= 4 {
        return "••••".into();
    }
    format!("••••{}", &key[key.len() - 4..])
}

// ---------- admin: list / create / delete ----------
/// GET /api/admin/models — full list for management (api_key masked).
pub async fn admin_list(State(state): State<AppState>, claims: Claims) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let rows = sqlx::query_as::<_, Model>("SELECT * FROM models ORDER BY sort, created_at")
        .fetch_all(&state.db)
        .await?;
    let list: Vec<serde_json::Value> = rows
        .iter()
        .map(|m| {
            json!({
                "id": m.id, "label": m.label, "provider": m.provider, "base_url": m.base_url,
                "model_id": m.model_id, "api_key_masked": mask(&m.api_key), "has_key": !m.api_key.is_empty(),
                "price_cents": m.price_cents, "rate": m.rate, "active": m.active, "sort": m.sort, "created_at": m.created_at,
                "enabled_models": m.enabled_models,
            })
        })
        .collect();
    Ok(Json(json!(list)))
}

#[derive(Deserialize)]
pub struct ModelReq {
    pub label: String,
    pub provider: Option<String>,
    pub base_url: String,
    pub model_id: Option<String>,
    pub api_key: String,
    pub rate: Option<f64>,
    pub sort: Option<i32>,
}

/// POST /api/admin/models — create a provider connection (admin). model_id is
/// optional; the exposed models are chosen later via the edit/enabled set.
pub async fn admin_create(State(state): State<AppState>, claims: Claims, Json(req): Json<ModelReq>) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    if req.label.trim().is_empty() || req.base_url.trim().is_empty() {
        return Err(AppError::bad("名称 / baseUrl 不能为空"));
    }
    let id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO models (label, provider, base_url, model_id, api_key, rate, sort) \
         VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
    )
    .bind(req.label.trim())
    .bind(req.provider.unwrap_or_default())
    .bind(req.base_url.trim().trim_end_matches('/'))
    .bind(req.model_id.unwrap_or_default().trim())
    .bind(req.api_key.trim())
    .bind(req.rate.unwrap_or(1.0).max(0.0))
    .bind(req.sort.unwrap_or(0))
    .fetch_one(&state.db)
    .await?;
    Ok(Json(json!({ "ok": true, "id": id })))
}

/// DELETE /api/admin/models/:id (admin).
pub async fn admin_delete(State(state): State<AppState>, claims: Claims, Path(id): Path<uuid::Uuid>) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let res = sqlx::query("DELETE FROM models WHERE id = $1").bind(id).execute(&state.db).await?;
    if res.rows_affected() == 0 {
        return Err(AppError::bad("模型不存在"));
    }
    Ok(Json(json!({ "ok": true })))
}

/// GET /api/admin/models/:id/available — proxy the provider's model catalogue
/// (OpenAI-compatible GET /models) using this connection's key.
pub async fn admin_available(State(state): State<AppState>, claims: Claims, Path(id): Path<uuid::Uuid>) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let m = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::bad("模型连接不存在"))?;
    if m.api_key.is_empty() {
        return Err(AppError::bad("该连接未配置 API Key"));
    }
    let url = format!("{}/models", api_base(&m.base_url));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::internal(e.to_string()))?;
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", m.api_key))
        .send()
        .await
        .map_err(|e| AppError::internal(format!("拉取模型列表失败: {e}")))?;
    let status = resp.status();
    let data: serde_json::Value = resp.json().await.unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        return Err(AppError { status: axum::http::StatusCode::BAD_GATEWAY, msg: format!("供应商错误 {}: {}", status.as_u16(), data) });
    }
    let ids: Vec<String> = data
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| arr.iter().filter_map(|x| x.get("id").and_then(|i| i.as_str()).map(String::from)).collect())
        .unwrap_or_default();
    Ok(Json(json!({ "models": ids, "enabled": m.enabled_models })))
}

#[derive(Deserialize)]
pub struct UpdateReq {
    pub label: Option<String>,
    pub provider: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>, // empty/missing = keep existing
    pub rate: Option<f64>,
    pub active: Option<bool>,
    pub sort: Option<i32>,
    pub enabled_models: Option<Vec<String>>,
}

/// POST /api/admin/models/:id — update a connection (incl. enabled model set). admin.
pub async fn admin_update(State(state): State<AppState>, claims: Claims, Path(id): Path<uuid::Uuid>, Json(req): Json<UpdateReq>) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let m = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::bad("模型连接不存在"))?;
    let label = req.label.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).unwrap_or(m.label);
    let provider = req.provider.unwrap_or(m.provider);
    let base_url = req.base_url.map(|s| s.trim().trim_end_matches('/').to_string()).filter(|s| !s.is_empty()).unwrap_or(m.base_url);
    let api_key = match req.api_key {
        Some(k) if !k.trim().is_empty() => k.trim().to_string(),
        _ => m.api_key,
    };
    let rate = req.rate.unwrap_or(m.rate).max(0.0);
    let active = req.active.unwrap_or(m.active);
    let sort = req.sort.unwrap_or(m.sort);
    let enabled = req.enabled_models.unwrap_or(m.enabled_models);
    sqlx::query("UPDATE models SET label=$1, provider=$2, base_url=$3, api_key=$4, rate=$5, active=$6, sort=$7, enabled_models=$8 WHERE id=$9")
        .bind(&label)
        .bind(&provider)
        .bind(&base_url)
        .bind(&api_key)
        .bind(rate)
        .bind(active)
        .bind(sort)
        .bind(&enabled)
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

// ---------- IDE-facing: list active models (safe fields, no secrets) ----------
/// GET /api/models — active models for the IDE (no api_key / base_url leaked).
pub async fn list_for_client(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE active = true ORDER BY sort, created_at")
        .fetch_all(&state.db)
        .await?;
    let mut list = Vec::new();
    for m in &rows {
        for mid in allowed_ids(m) {
            list.push(json!({
                "conn_id": m.id,
                "group": m.label,
                "provider": m.provider,
                "model_id": mid.clone(),
                "name": mid,
                "price_cents": m.price_cents,
            }));
        }
    }
    Ok(Json(json!(list)))
}

// ---------- IDE-facing: proxy a chat completion, billing credits ----------
/// POST /api/models/:id/chat — forwards an OpenAI-style chat request to the
/// model's provider, deducts the model's price from the caller's credits, and
/// returns the upstream JSON. Non-streaming.
pub async fn chat(State(state): State<AppState>, claims: Claims, Path(id): Path<uuid::Uuid>, Json(mut body): Json<serde_json::Value>) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    let model = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = $1 AND active = true")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::bad("模型不存在或已停用"))?;

    // pre-check: need a positive balance when the model isn't free
    if model.rate > 0.0 {
        let bal: i64 = sqlx::query_scalar("SELECT credits_cents FROM users WHERE id = $1")
            .bind(uid)
            .fetch_one(&state.db)
            .await?;
        if bal <= 0 {
            return Err(AppError { status: axum::http::StatusCode::PAYMENT_REQUIRED, msg: "额度不足，请充值".into() });
        }
    }

    // forward to the provider (OpenAI-compatible /chat/completions)
    if !body.is_object() {
        return Err(AppError::bad("请求体需为 JSON 对象"));
    }
    // honour the requested model when it's in this connection's enabled set
    let allowed = allowed_ids(&model);
    let requested = body.get("model").and_then(|v| v.as_str()).map(String::from);
    let chosen = match requested {
        Some(r) if allowed.contains(&r) => r,
        _ => allowed.first().cloned().unwrap_or_default(),
    };
    if chosen.is_empty() {
        return Err(AppError::bad("该连接未开放任何模型，请在后台编辑勾选"));
    }
    body["model"] = json!(chosen);

    // Weak-vision models (deepseek/minimax/glm/…) can't read images well. If the
    // request has images and the chosen model isn't vision-native, let gpt-5.5
    // describe the images first, then hand the text to the chosen model.
    if needs_vision_help(&chosen) {
        vision_preprocess(&state, &mut body).await;
    }

    let url = format!("{}/chat/completions", api_base(&model.base_url));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| AppError::internal(e.to_string()))?;
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", model.api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("模型调用失败: {e}")))?;
    let status = resp.status();
    let data: serde_json::Value = resp.json().await.unwrap_or_else(|_| json!({ "error": "上游返回非 JSON" }));
    if !status.is_success() {
        return Err(AppError { status: axum::http::StatusCode::BAD_GATEWAY, msg: format!("模型供应商错误 {}: {}", status.as_u16(), data) });
    }

    // bill on success: credits = total_tokens/1000 * rate (USD cents)
    let tokens = data.get("usage").and_then(|u| u.get("total_tokens")).and_then(|t| t.as_i64()).unwrap_or(0);
    let cost = ((tokens as f64) / 1000.0 * model.rate).round() as i64;
    if cost > 0 {
        let _ = sqlx::query("UPDATE users SET credits_cents = GREATEST(credits_cents - $1, 0) WHERE id = $2")
            .bind(cost)
            .bind(uid)
            .execute(&state.db)
            .await;
    }
    let _ = sqlx::query("INSERT INTO model_usage (user_id, model_id, cost_cents) VALUES ($1,$2,$3)")
        .bind(uid)
        .bind(model.id)
        .bind(cost)
        .execute(&state.db)
        .await;
    Ok(Json(data))
}

// ---------- admin: usage stats ----------
/// GET /api/admin/model-usage — recent usage + totals (admin).
pub async fn admin_usage(State(state): State<AppState>, claims: Claims) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM model_usage").fetch_one(&state.db).await?;
    let spent: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(cost_cents),0)::bigint FROM model_usage").fetch_one(&state.db).await?;
    Ok(Json(json!({ "calls": calls, "spent_cents": spent })))
}

// ================= Michael API keys + OpenAI-compatible gateway =================

#[derive(sqlx::FromRow)]
struct ApiKeyRow {
    id: uuid::Uuid,
    label: String,
    api_key: String,
    email: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

fn gen_api_key() -> String {
    let mut rng = rand::thread_rng();
    let hex: String = (0..40).map(|_| std::char::from_digit(rng.gen_range(0..16), 16).unwrap()).collect();
    format!("sk-michael-{hex}")
}

fn mask_key(k: &str) -> String {
    if k.len() <= 8 {
        return "••••".into();
    }
    format!("{}…{}", &k[..11.min(k.len())], &k[k.len() - 4..])
}

#[derive(Deserialize)]
pub struct ApiKeyReq {
    pub label: Option<String>,
    pub email: Option<String>,
}

/// POST /api/admin/apikeys — generate a gateway key for the admin (or a given user's email).
pub async fn admin_create_apikey(State(state): State<AppState>, claims: Claims, Json(req): Json<ApiKeyReq>) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let uid = match req.email.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        Some(email) => sqlx::query_scalar::<_, uuid::Uuid>("SELECT id FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::bad("用户不存在"))?,
        None => uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?,
    };
    let key = gen_api_key();
    sqlx::query("INSERT INTO api_keys (user_id, api_key, label) VALUES ($1,$2,$3)")
        .bind(uid)
        .bind(&key)
        .bind(req.label.unwrap_or_default())
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "ok": true, "api_key": key })))
}

/// GET /api/admin/apikeys — list keys (masked) with their owner email.
pub async fn admin_list_apikeys(State(state): State<AppState>, claims: Claims) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let rows = sqlx::query_as::<_, ApiKeyRow>(
        "SELECT k.id, k.label, k.api_key, u.email, k.created_at, k.last_used_at \
         FROM api_keys k LEFT JOIN users u ON u.id = k.user_id ORDER BY k.created_at DESC LIMIT 200",
    )
    .fetch_all(&state.db)
    .await?;
    let list: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| json!({ "id": r.id, "label": r.label, "email": r.email, "key_masked": mask_key(&r.api_key), "created_at": r.created_at, "last_used_at": r.last_used_at }))
        .collect();
    Ok(Json(json!(list)))
}

/// DELETE /api/admin/apikeys/:id
pub async fn admin_delete_apikey(State(state): State<AppState>, claims: Claims, Path(id): Path<uuid::Uuid>) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let res = sqlx::query("DELETE FROM api_keys WHERE id = $1").bind(id).execute(&state.db).await?;
    if res.rows_affected() == 0 {
        return Err(AppError::bad("密钥不存在"));
    }
    Ok(Json(json!({ "ok": true })))
}

/// GET /api/ide-key — convenience bootstrap: return a stable API key bound to
/// the owner (first admin), creating it once. Lets the IDE auto-configure with
/// no manual key. NOTE: public for single-tenant convenience; lock down (or
/// require login) before exposing to untrusted users.
pub async fn ide_key(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let uid: uuid::Uuid = sqlx::query_scalar("SELECT id FROM users WHERE role = 'admin' ORDER BY created_at LIMIT 1")
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::bad("尚无管理员账号"))?;
    let existing: Option<String> = sqlx::query_scalar("SELECT api_key FROM api_keys WHERE user_id = $1 AND label = 'ide-auto' ORDER BY created_at LIMIT 1")
        .bind(uid)
        .fetch_optional(&state.db)
        .await?;
    let key = match existing {
        Some(k) => k,
        None => {
            let k = gen_api_key();
            sqlx::query("INSERT INTO api_keys (user_id, api_key, label) VALUES ($1, $2, 'ide-auto')")
                .bind(uid)
                .bind(&k)
                .execute(&state.db)
                .await?;
            k
        }
    };
    Ok(Json(json!({ "api_key": key })))
}

/// A model id whose vision is weak/absent → route images through gpt-5.5 first.
fn needs_vision_help(model_id: &str) -> bool {
    let m = model_id.to_lowercase();
    let native = m.contains("gpt") || m.contains("gemini") || m.contains("claude")
        || m.contains("vision") || m.contains("-vl") || m.contains("image")
        || m.contains("o3") || m.contains("o4");
    !native
}

/// If the request carries images, ask gpt-5.5 to describe them, then rewrite the
/// messages to plain text (description injected) so a non-vision model can work
/// from it. No-op if there are no images or no gpt-5.5 connection is configured.
async fn vision_preprocess(state: &AppState, body: &mut serde_json::Value) {
    let mut images: Vec<serde_json::Value> = Vec::new();
    if let Some(msgs) = body.get("messages").and_then(|m| m.as_array()) {
        for m in msgs {
            if let Some(arr) = m.get("content").and_then(|c| c.as_array()) {
                for part in arr {
                    if part.get("type").and_then(|t| t.as_str()) == Some("image_url") {
                        images.push(part.clone());
                    }
                }
            }
        }
    }
    if images.is_empty() {
        return;
    }
    // best-effort: have gpt-5.5 describe the images (may fail → we still strip them)
    let mut desc: Option<String> = None;
    let conns = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE active = true ORDER BY sort, created_at")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
    if let Some(vconn) = conns.into_iter().find(|m| allowed_ids(m).iter().any(|id| id.eq_ignore_ascii_case("gpt-5.5"))) {
        let mut vcontent = vec![json!({
            "type": "text",
            "text": "请详细、客观地描述这些图片的全部内容（文字、数据、图表、代码、界面元素、布局、配色等），让一个无法读图的模型也能据此完成工作。只输出描述本身。"
        })];
        vcontent.extend(images.clone());
        let payload = json!({ "model": "gpt-5.5", "messages": [{ "role": "user", "content": vcontent }], "stream": false });
        if let Ok(client) = reqwest::Client::builder().timeout(std::time::Duration::from_secs(90)).build() {
            let url = format!("{}/chat/completions", api_base(&vconn.base_url));
            if let Ok(r) = client.post(&url).header("Authorization", format!("Bearer {}", vconn.api_key)).json(&payload).send().await {
                if let Ok(d) = r.json::<serde_json::Value>().await {
                    if let Some(s) = d["choices"][0]["message"]["content"].as_str() {
                        if !s.trim().is_empty() {
                            desc = Some(s.to_string());
                        }
                    }
                }
            }
        }
    }
    let note = match desc {
        Some(d) => format!("【图片内容（由 GPT-5.5 视觉识别）】：\n{}", d),
        None => "【图片】（视觉识别暂不可用，无法读取图片内容）".to_string(),
    };
    // ALWAYS strip images → plain text so a non-vision model never chokes on them.
    if let Some(msgs) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        let mut last_img: Option<usize> = None;
        for (i, m) in msgs.iter_mut().enumerate() {
            if let Some(arr) = m.get("content").and_then(|c| c.as_array()).cloned() {
                let mut text = String::new();
                let mut had = false;
                for part in &arr {
                    match part.get("type").and_then(|t| t.as_str()) {
                        Some("text") => {
                            if let Some(t) = part.get("text").and_then(|x| x.as_str()) {
                                if !text.is_empty() {
                                    text.push('\n');
                                }
                                text.push_str(t);
                            }
                        }
                        Some("image_url") => had = true,
                        _ => {}
                    }
                }
                m["content"] = json!(text);
                if had {
                    last_img = Some(i);
                }
            }
        }
        if let Some(idx) = last_img {
            if let Some(m) = msgs.get_mut(idx) {
                let cur = m.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                m["content"] = json!(format!("{}\n\n{}", cur, note));
            }
        }
    }
}

/// POST /v1/chat/completions — OpenAI-compatible gateway. Auth via a Michael API
/// key (Bearer). Resolves `model` to the connection that exposes it, forwards
/// the request (streaming passthrough), and bills the key owner's credits.
pub async fn chat_completions(State(state): State<AppState>, headers: HeaderMap, Json(mut body): Json<serde_json::Value>) -> Result<Response, AppError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::unauthorized("缺少 API Key"))?;
    let uid: uuid::Uuid = match sqlx::query_scalar::<_, uuid::Uuid>("SELECT user_id FROM api_keys WHERE api_key = $1")
        .bind(&token)
        .fetch_optional(&state.db)
        .await?
    {
        Some(u) => {
            let _ = sqlx::query("UPDATE api_keys SET last_used_at = now() WHERE api_key = $1").bind(&token).execute(&state.db).await;
            u
        }
        // Also accept the login JWT directly (the IDE authenticates with it).
        None => crate::auth::user_from_jwt(&state.cfg, &token).ok_or_else(|| AppError::unauthorized("登录已失效或密钥无效"))?,
    };

    if !body.is_object() {
        return Err(AppError::bad("请求体需为 JSON 对象"));
    }
    let model_id = body.get("model").and_then(|v| v.as_str()).map(String::from).ok_or_else(|| AppError::bad("缺少 model"))?;

    let conns = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE active = true ORDER BY sort, created_at")
        .fetch_all(&state.db)
        .await?;
    let conn = conns
        .into_iter()
        .find(|m| allowed_ids(m).contains(&model_id))
        .ok_or_else(|| AppError::bad(format!("模型 {model_id} 不可用")))?;

    // Refill the 5h30m window + reset the weekly counter when due.
    sqlx::query(
        "UPDATE users SET \
         quota_window_cents = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN LEAST(quota_window_cap_cents, quota_total_cents) ELSE quota_window_cents END, \
         quota_window_reset_at = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN now() + interval '5 hours 30 minutes' ELSE quota_window_reset_at END, \
         quota_week_used_cents = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN 0 ELSE quota_week_used_cents END, \
         quota_week_reset_at = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN now() + interval '7 days' ELSE quota_week_reset_at END \
         WHERE id = $1",
    )
    .bind(uid)
    .execute(&state.db)
    .await?;

    // Access gate: active-membership quota (window/total/weekly) OR pay-as-you-go credits.
    let (plan, plan_exp, q_total, q_window, q_weekly_cap, q_week_used, credits): (String, Option<chrono::DateTime<chrono::Utc>>, i64, i64, i64, i64, i64) =
        sqlx::query_as("SELECT plan, plan_expires_at, quota_total_cents, quota_window_cents, quota_weekly_cap_cents, quota_week_used_cents, credits_cents FROM users WHERE id = $1")
            .bind(uid)
            .fetch_one(&state.db)
            .await?;
    let plan_active = plan != "none" && plan_exp.map_or(true, |e| e > chrono::Utc::now());
    let quota_ok = plan_active && q_total > 0 && q_window > 0 && (q_weekly_cap == 0 || q_week_used < q_weekly_cap);
    let use_quota = quota_ok;
    if !quota_ok && credits <= 0 {
        let msg = if plan_active && q_total <= 0 {
            "总额度已用完"
        } else if plan_active && q_window <= 0 {
            "本时段额度已用完，请等待刷新（每 5.5 小时）"
        } else if plan_active && q_weekly_cap > 0 && q_week_used >= q_weekly_cap {
            "本周额度已用完"
        } else {
            "请先开通会员或充值额度"
        };
        return Err(AppError { status: StatusCode::PAYMENT_REQUIRED, msg: msg.into() });
    }

    let streaming = body.get("stream").and_then(|v| v.as_bool()).unwrap_or(false);
    let url = format!("{}/chat/completions", api_base(&conn.base_url));
    // Pooled client (warm keep-alive connections) instead of a fresh handshake
    // per request. Streaming stays open-ended; non-streaming gets a sane cap.
    //
    // Upstream providers (zyz et al.) intermittently return 502/503/504/429 or
    // drop a kept-alive connection mid-flight — the user just sees "网关又出问题".
    // Retry such *transient* failures up to 3 attempts with a short backoff so a
    // blip is absorbed instead of surfaced. We only retry BEFORE streaming the
    // body has started (a send error or a bad status line), so no half-streamed
    // response is ever double-sent, and billing still happens once, after success.
    let resp = {
        let mut got = None;
        let mut last = String::from("unknown");
        for attempt in 0u32..3 {
            let mut req = GW_HTTP
                .post(&url)
                .header("Authorization", format!("Bearer {}", conn.api_key))
                .json(&body);
            if !streaming {
                req = req.timeout(std::time::Duration::from_secs(120));
            }
            match req.send().await {
                Ok(r) => {
                    let s = r.status().as_u16();
                    if matches!(s, 502 | 503 | 504 | 429) && attempt < 2 {
                        last = format!("上游 {s}");
                        tokio::time::sleep(std::time::Duration::from_millis(400 * (attempt as u64 + 1))).await;
                        continue;
                    }
                    got = Some(r);
                    break;
                }
                // A send error means the request almost certainly never reached the
                // server (incl. a stale pooled connection) — safe to re-send.
                Err(e) => {
                    last = e.to_string();
                    if attempt < 2 {
                        tokio::time::sleep(std::time::Duration::from_millis(300 * (attempt as u64 + 1))).await;
                        continue;
                    }
                }
            }
        }
        got.ok_or_else(|| AppError {
            status: StatusCode::BAD_GATEWAY,
            msg: format!("模型上游连续失败（已重试3次）: {last}"),
        })?
    };
    let status = resp.status();

    // Translate raw upstream (zyz et al.) errors into a friendly, actionable
    // Chinese message instead of dumping a scary "502 {json}" at the user. These
    // are the provider's problem (forbidden key / no accounts / overloaded), not
    // ours — tell the user to switch model or contact the provider.
    if !status.is_success() {
        let model_name = body.get("model").and_then(|m| m.as_str()).unwrap_or("该模型").to_string();
        let raw = resp.text().await.unwrap_or_default();
        let low = raw.to_lowercase();
        let friendly = if low.contains("forbidden") {
            "上游暂不可用（供应商未授权 / 账户异常）。请换个模型，或联系模型供应商开通/续费。"
        } else if low.contains("no available account") || low.contains("no available") {
            "上游暂无可用账号。请换个模型，或稍后再试。"
        } else if status.as_u16() == 429 || low.contains("rate") || low.contains("frequent") || low.contains("过于频繁") {
            "请求过于频繁，请稍后再试。"
        } else if status.as_u16() == 401 || low.contains("unauthorized") || low.contains("invalid api key") {
            "上游密钥无效。请在后台「模型系统」更新该连接的 API Key。"
        } else {
            "上游暂时不可用，请换个模型或稍后再试。"
        };
        return Err(AppError {
            status: StatusCode::BAD_GATEWAY,
            msg: format!("【{model_name}】{friendly}"),
        });
    }

    async fn bill(state: &AppState, uid: uuid::Uuid, conn_id: uuid::Uuid, cost: i64, use_quota: bool) {
        if cost > 0 {
            if use_quota {
                let _ = sqlx::query(
                    "UPDATE users SET quota_total_cents = GREATEST(quota_total_cents - $1, 0), \
                     quota_window_cents = GREATEST(quota_window_cents - $1, 0), \
                     quota_week_used_cents = quota_week_used_cents + $1 WHERE id = $2",
                )
                .bind(cost)
                .bind(uid)
                .execute(&state.db)
                .await;
            } else {
                let _ = sqlx::query("UPDATE users SET credits_cents = GREATEST(credits_cents - $1, 0) WHERE id = $2").bind(cost).bind(uid).execute(&state.db).await;
            }
        }
        let _ = sqlx::query("INSERT INTO model_usage (user_id, model_id, cost_cents) VALUES ($1,$2,$3)").bind(uid).bind(conn_id).bind(cost).execute(&state.db).await;
    }

    if streaming {
        // Can't read token usage from a passed-through stream, so charge a flat
        // per-call fee (rounded rate). Stream the upstream SSE straight through.
        // Bill in the background so the first SSE byte isn't delayed by DB writes.
        {
            let st = state.clone();
            let cid = conn.id;
            let cost = conn.rate.round() as i64;
            tokio::spawn(async move { bill(&st, uid, cid, cost, use_quota).await });
        }
        let ct = resp
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/event-stream")
            .to_string();
        let out = Response::builder()
            .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
            .header(axum::http::header::CONTENT_TYPE, ct)
            .header("cache-control", "no-cache")
            .body(Body::from_stream(resp.bytes_stream()))
            .map_err(|e| AppError::internal(e.to_string()))?;
        Ok(out)
    } else {
        let data: serde_json::Value = resp.json().await.unwrap_or_else(|_| json!({ "error": "上游返回非 JSON" }));
        if !status.is_success() {
            return Err(AppError { status: StatusCode::BAD_GATEWAY, msg: format!("模型供应商错误 {}: {}", status.as_u16(), data) });
        }
        let tokens = data.get("usage").and_then(|u| u.get("total_tokens")).and_then(|t| t.as_i64()).unwrap_or(0);
        bill(&state, uid, conn.id, ((tokens as f64) / 1000.0 * conn.rate).round() as i64, use_quota).await;
        Ok(Json(data).into_response())
    }
}
