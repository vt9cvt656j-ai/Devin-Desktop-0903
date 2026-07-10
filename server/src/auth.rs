use axum::async_trait;
use axum::extract::{FromRequestParts, Path, State};
use axum::http::request::Parts;
use axum::Json;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::Config;
use crate::error::{ApiResult, AppError};
use crate::AppState;

// ---- models ----------------------------------------------------------------

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: uuid::Uuid,
    pub email: String,
    #[serde(skip)]
    pub password_hash: String,
    pub role: String,
    pub plan: String,
    pub plan_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub credits_cents: i64,
    pub quota_total_cents: i64,
    pub quota_window_cap_cents: i64,
    pub quota_window_cents: i64,
    pub quota_window_reset_at: Option<chrono::DateTime<chrono::Utc>>,
    pub quota_weekly_cap_cents: i64,
    pub quota_week_used_cents: i64,
    pub quota_week_reset_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user id
    pub email: String,
    pub role: String,
    pub exp: i64,
}

// ---- JWT extractor: any handler taking `claims: Claims` requires a valid token

#[async_trait]
impl FromRequestParts<AppState> for Claims {
    type Rejection = AppError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::unauthorized("缺少 Authorization 头"))?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::unauthorized("Authorization 需为 Bearer <token>"))?;
        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.cfg.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AppError::unauthorized("无效或已过期的令牌"))?;
        Ok(data.claims)
    }
}

fn issue_token(cfg: &Config, user: &User) -> ApiResult<String> {
    let claims = Claims {
        sub: user.id.to_string(),
        email: user.email.clone(),
        role: user.role.clone(),
        exp: chrono::Utc::now().timestamp() + cfg.jwt_ttl_secs,
    };
    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(cfg.jwt_secret.as_bytes()),
    )?)
}

/// Resolve a user id from a raw JWT (used by the model gateway so the IDE can
/// authenticate with the login token, not just an API key). None if invalid.
pub fn user_from_jwt(cfg: &Config, token: &str) -> Option<uuid::Uuid> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(cfg.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .ok()?;
    uuid::Uuid::parse_str(&data.claims.sub).ok()
}

/// Lightweight email format check ("合规").
pub fn valid_email(e: &str) -> bool {
    let e = e.trim();
    if e.len() < 6 || e.len() > 254 || e.contains(char::is_whitespace) {
        return false;
    }
    let mut parts = e.split('@');
    let (local, domain) = match (parts.next(), parts.next(), parts.next()) {
        (Some(l), Some(d), None) => (l, d),
        _ => return false,
    };
    !local.is_empty()
        && domain.len() >= 3
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && domain.rsplit('.').next().is_some_and(|tld| tld.len() >= 2)
}

// ---- verification codes (Redis, with TTL) ----------------------------------

fn code_key(email: &str) -> String {
    format!("code:{}", email.trim().to_lowercase())
}

/// Max wrong guesses against a SINGLE emailed code before it is BURNED.
const MAX_CODE_ATTEMPTS: i64 = 5;
/// Hard ceiling on TOTAL failed guesses per email per hour — NOT reset by requesting
/// fresh codes, so resends can't multiply the attempt budget (cleared on success).
const HOURLY_ATTEMPT_CAP: i64 = 20;

fn attempts_key(email: &str) -> String {
    format!("code_tries:{}", email.trim().to_lowercase())
}

async fn store_code(state: &AppState, email: &str, code: &str) -> ApiResult<()> {
    let mut conn = state.redis.clone();
    let _: () = redis::cmd("SET")
        .arg(code_key(email))
        .arg(code)
        .arg("EX")
        .arg(state.cfg.code_ttl_secs)
        .query_async(&mut conn)
        .await?;
    // Fresh code → fresh attempt window.
    let _: () = redis::cmd("DEL")
        .arg(attempts_key(email))
        .query_async(&mut conn)
        .await?;
    Ok(())
}

async fn take_code(state: &AppState, email: &str, code: &str) -> ApiResult<bool> {
    let mut conn = state.redis.clone();
    let key = code_key(email);
    // Brute-force guard: count guesses within the code's TTL window. Past the cap,
    // BURN the code so even a later correct guess can't pass — the user must request
    // a fresh one (which resets the counter via store_code).
    // (a) Per-code budget: reset when a fresh code is sent (store_code DELs it).
    let tries_key = attempts_key(email);
    let tries: i64 = redis::cmd("INCR")
        .arg(&tries_key)
        .query_async(&mut conn)
        .await?;
    // EXPIRE every call (idempotent): setting it only on tries==1 risks a crash
    // between INCR and EXPIRE leaving a TTL-less key that locks the user out forever.
    let _: () = redis::cmd("EXPIRE")
        .arg(&tries_key)
        .arg(state.cfg.code_ttl_secs)
        .query_async(&mut conn)
        .await?;
    // (b) Hourly budget across the whole email, NOT reset by resends — caps total
    // guesses so an attacker can't keep requesting fresh codes to refill (a)'s 5.
    let hour_key = format!("code_tries_h:{}", email.trim().to_lowercase());
    let hourly: i64 = redis::cmd("INCR")
        .arg(&hour_key)
        .query_async(&mut conn)
        .await?;
    let _: () = redis::cmd("EXPIRE")
        .arg(&hour_key)
        .arg(3600i64)
        .query_async(&mut conn)
        .await?;
    if tries > MAX_CODE_ATTEMPTS || hourly > HOURLY_ATTEMPT_CAP {
        let _: () = redis::cmd("DEL").arg(&key).query_async(&mut conn).await?;
        return Err(AppError::bad("验证码尝试次数过多，请稍后重新获取验证码"));
    }
    let stored: Option<String> = redis::cmd("GET").arg(&key).query_async(&mut conn).await?;
    match stored {
        Some(s) if s == code => {
            let _: () = redis::cmd("DEL").arg(&key).query_async(&mut conn).await?;
            // Success → clear BOTH budgets so a legit user is never carried over.
            let _: () = redis::cmd("DEL")
                .arg(&tries_key)
                .arg(&hour_key)
                .query_async(&mut conn)
                .await?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Google-style, light-themed verification email. Table layout + inline styles only, so it renders
/// consistently across mail clients (Gmail / QQ / 163 / Outlook all strip <style> and dislike flex).
fn code_email_html(code: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"></head>
<body style="margin:0;padding:0;background:#f1f3f4;">
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background:#f1f3f4;padding:32px 12px;"><tr><td align="center">
<table role="presentation" width="448" cellpadding="0" cellspacing="0" style="max-width:448px;width:100%;background:#ffffff;border-radius:16px;border:1px solid #e8eaed;font-family:'Segoe UI',Roboto,'Helvetica Neue',Arial,'PingFang SC','Microsoft YaHei',sans-serif;">
<tr><td style="padding:40px 44px 6px;text-align:center;"><img src="https://code.mrday.one/api/logo.png" width="52" height="52" alt="Michael IDE" style="display:inline-block;width:52px;height:52px;border-radius:14px;" /></td></tr>
<tr><td style="padding:16px 44px 0;text-align:center;"><div style="font-size:22px;font-weight:500;color:#202124;">验证您的邮箱</div><div style="font-size:14px;color:#5f6368;line-height:1.7;margin-top:10px;">您正在登录 / 注册 <b style="color:#202124;">Michael IDE</b>，请在登录页面输入下面的验证码：</div></td></tr>
<tr><td style="padding:26px 44px 6px;text-align:center;"><div style="display:inline-block;background:#f6f9fe;border:1px solid #d2e3fc;border-radius:12px;padding:16px 22px 16px 32px;font-size:34px;font-weight:700;letter-spacing:10px;color:#1a73e8;font-family:'SF Mono',Menlo,Consolas,monospace;">{code}</div></td></tr>
<tr><td style="padding:18px 44px 0;text-align:center;"><div style="font-size:13px;color:#80868b;line-height:1.7;">验证码 <b>10 分钟</b>内有效，请勿泄露给他人。如果这不是您本人的操作，请忽略此邮件。</div></td></tr>
<tr><td style="padding:28px 44px 0;"><div style="border-top:1px solid #e8eaed;font-size:0;line-height:0;">&nbsp;</div></td></tr>
<tr><td style="padding:16px 44px 34px;text-align:center;"><div style="font-size:12px;color:#9aa0a6;line-height:1.6;">此邮件由 Michael IDE 自动发送，请勿直接回复。</div></td></tr>
</table></td></tr></table></body></html>"#,
        code = code
    )
}

async fn send_code_email(cfg: &Config, to: &str, code: &str) -> ApiResult<bool> {
    if !cfg.mail_enabled() {
        tracing::warn!("[DEV] 邮件服务未配置. Code for {to}: {code}");
        return Ok(false);
    }
    let html = code_email_html(code);
    crate::email::send_mail(cfg, to, "Michael IDE 登录验证码", &html, true).await?;
    Ok(true)
}

fn gen_code() -> String {
    use rand::Rng;
    format!("{:06}", rand::thread_rng().gen_range(0..1_000_000))
}

async fn find_user(state: &AppState, email: &str) -> ApiResult<Option<User>> {
    Ok(
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&state.db)
            .await?,
    )
}

// ---- request/response payloads ---------------------------------------------

#[derive(Deserialize)]
pub struct EmailReq {
    pub email: String,
}
#[derive(Deserialize)]
pub struct LoginReq {
    pub email: String,
    pub password: String,
}
#[derive(Deserialize)]
pub struct RegisterReq {
    pub email: String,
    pub password: String,
    pub code: String,
}
#[derive(Deserialize)]
pub struct CodeReq {
    pub email: String,
    pub code: String,
}

// ---- handlers --------------------------------------------------------------

pub async fn check_email(
    State(state): State<AppState>,
    Json(req): Json<EmailReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if !valid_email(&req.email) {
        return Err(AppError::bad("邮箱格式不正确"));
    }
    Ok(Json(
        json!({ "exists": find_user(&state, &req.email).await?.is_some() }),
    ))
}

pub async fn send_code(
    State(state): State<AppState>,
    Json(req): Json<EmailReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if !valid_email(&req.email) {
        return Err(AppError::bad("邮箱格式不正确"));
    }
    // Cooldown: one code per 30s per email. Without it an attacker could spam fresh
    // codes to reset the attempt cap (and bomb the inbox / our email quota).
    {
        let mut conn = state.redis.clone();
        let ok: Option<String> = redis::cmd("SET")
            .arg(format!("code_cd:{}", req.email.trim().to_lowercase()))
            .arg("1")
            .arg("NX")
            .arg("EX")
            .arg(30i64)
            .query_async(&mut conn)
            .await?;
        if ok.is_none() {
            return Err(AppError::bad("验证码发送过于频繁，请 30 秒后再试"));
        }
    }
    let code = gen_code();
    store_code(&state, &req.email, &code).await?;
    let sent = send_code_email(&state.cfg, &req.email, &code).await?;
    Ok(Json(
        json!({ "sent": sent, "message": if sent { "验证码已发送到邮箱（10 分钟有效）" } else { "开发模式：验证码见服务端日志" } }),
    ))
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if !valid_email(&req.email) {
        return Err(AppError::bad("邮箱格式不正确"));
    }
    if req.password.len() < 6 {
        return Err(AppError::bad("密码至少 6 位"));
    }
    if !take_code(&state, &req.email, &req.code).await? {
        return Err(AppError::bad("验证码错误或已过期"));
    }
    if find_user(&state, &req.email).await?.is_some() {
        return Err(AppError::bad("该邮箱已注册，请直接登录"));
    }
    let hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST)?;
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING *",
    )
    .bind(&req.email)
    .bind(&hash)
    .fetch_one(&state.db)
    .await?;
    let token = issue_token(&state.cfg, &user)?;
    crate::realtime::record_event(
        &state,
        Some(user.id),
        "register",
        json!({ "email": user.email }),
    )
    .await;
    Ok(Json(json!({ "token": token, "user": user })))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if req.email.trim().is_empty() || req.password.is_empty() {
        return Err(AppError::bad("账号或密码不能为空"));
    }
    let user = find_user(&state, req.email.trim())
        .await?
        .ok_or_else(|| AppError::bad("账号不存在"))?;
    if !bcrypt::verify(&req.password, &user.password_hash)? {
        return Err(AppError::unauthorized("密码错误"));
    }
    sqlx::query("UPDATE users SET last_login_at = now() WHERE id = $1")
        .bind(user.id)
        .execute(&state.db)
        .await?;
    let token = issue_token(&state.cfg, &user)?;
    crate::realtime::record_event(
        &state,
        Some(user.id),
        "login",
        json!({ "email": user.email }),
    )
    .await;
    Ok(Json(json!({ "token": token, "user": user })))
}

pub async fn verify_code(
    State(state): State<AppState>,
    Json(req): Json<CodeReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if !take_code(&state, &req.email, &req.code).await? {
        return Err(AppError::bad("验证码错误或已过期"));
    }
    let user = find_user(&state, &req.email)
        .await?
        .ok_or_else(|| AppError::bad("该邮箱尚未注册，请先设置密码注册"))?;
    sqlx::query("UPDATE users SET last_login_at = now() WHERE id = $1")
        .bind(user.id)
        .execute(&state.db)
        .await?;
    let token = issue_token(&state.cfg, &user)?;
    crate::realtime::record_event(
        &state,
        Some(user.id),
        "login",
        json!({ "email": user.email, "via": "code" }),
    )
    .await;
    Ok(Json(json!({ "token": token, "user": user })))
}

pub async fn me(State(state): State<AppState>, claims: Claims) -> ApiResult<Json<User>> {
    let id = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    // Apply the 5h30m window refill + weekly reset so the profile shows current quota.
    let _ = sqlx::query(
        "UPDATE users SET \
         quota_window_cents = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN LEAST(quota_window_cap_cents, quota_total_cents) ELSE quota_window_cents END, \
         quota_window_reset_at = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN now() + interval '5 hours 30 minutes' ELSE quota_window_reset_at END, \
         quota_week_used_cents = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN 0 ELSE quota_week_used_cents END, \
         quota_week_reset_at = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN now() + interval '7 days' ELSE quota_week_reset_at END \
         WHERE id = $1",
    )
    .bind(id)
    .execute(&state.db)
    .await;
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::unauthorized("用户不存在"))?;
    Ok(Json(user))
}

pub async fn admin_users(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<Vec<User>>> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at DESC LIMIT 500")
        .fetch_all(&state.db)
        .await?;
    Ok(Json(users))
}

#[derive(Deserialize)]
pub struct SetRoleReq {
    pub role: String,
}

/// POST /api/admin/users/:id/role  { role: "admin"|"user" } — admin only.
pub async fn set_user_role(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<SetRoleReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    if req.role != "admin" && req.role != "user" {
        return Err(AppError::bad("角色只能是 admin 或 user"));
    }
    if claims.sub == id.to_string() {
        return Err(AppError::bad("不能修改自己的角色"));
    }
    let res = sqlx::query("UPDATE users SET role = $1, updated_at = now() WHERE id = $2")
        .bind(&req.role)
        .bind(id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::bad("用户不存在"));
    }
    crate::realtime::record_event(&state, Some(id), "role_change", json!({ "role": req.role }))
        .await;
    Ok(Json(json!({ "ok": true })))
}

/// DELETE /api/admin/users/:id — admin only.
pub async fn delete_user(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    if claims.sub == id.to_string() {
        return Err(AppError::bad("不能删除自己"));
    }
    let res = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::bad("用户不存在"));
    }
    Ok(Json(json!({ "ok": true })))
}
