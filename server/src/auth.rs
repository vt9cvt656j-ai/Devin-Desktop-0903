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
    /// Daily free allowance, in 点. ¥0.5 = 10 点, so the ¥2 grant is 40 点.
    #[serde(default)]
    pub free_points: i64,
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
        let mut claims = data.claims;

        // The token is trusted for IDENTITY but never for PRIVILEGE.
        //
        // All 13 admin gates test `claims.role != "admin"`, and role was whatever was
        // baked into the JWT at login. With a 30-day TTL and no revocation that meant
        // demoting or even deleting an admin left their existing token fully
        // privileged for up to a month — and since the admin endpoints can re-grant
        // the role, one surviving token could take it back permanently. Re-reading
        // role from the row on every request makes demotion and deletion effective
        // immediately; a missing row (deleted user) now fails closed.
        let uid = uuid::Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::unauthorized("令牌主体无效"))?;
        let role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
            .bind(uid)
            .fetch_optional(&state.db)
            .await?;
        claims.role = role.ok_or_else(|| AppError::unauthorized("账号不存在或已注销"))?;
        Ok(claims)
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

/// Decode and validate a login JWT into its claims. For contexts that cannot use
/// the `Claims` extractor because there is no request to extract from — notably the
/// WebSocket feed, where the browser cannot send an Authorization header and the
/// token arrives in the first frame instead.
pub fn claims_from_jwt(cfg: &Config, token: &str) -> Option<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(cfg.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .ok()
    .map(|data| data.claims)
}

// ---- password-login brute-force guard --------------------------------------
// The emailed-code path is carefully budgeted (per-code attempts + an hourly ceiling that
// resends cannot reset). Password login had no budget at all — only bcrypt's cost factor,
// which slows an attacker but never stops one, and fail2ban on this host is configured for
// SSH, not for HTTP 401s from the backend.

/// Failed passwords tolerated per account per hour before login is paused.
const LOGIN_FAIL_PER_EMAIL: i64 = 10;
/// Failed passwords tolerated per source IP per hour — stops one host spraying many accounts.
const LOGIN_FAIL_PER_IP: i64 = 50;
const LOGIN_FAIL_WINDOW_SECS: i64 = 3600;

/// A throwaway hash so "no such account" spends the same CPU as a wrong password.
static DUMMY_HASH: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    bcrypt::hash("michael-timing-equalizer", bcrypt::DEFAULT_COST).unwrap_or_default()
});

/// Client IP as nginx sees it (`X-Real-IP`, else the first `X-Forwarded-For` hop).
fn client_ip(headers: &axum::http::HeaderMap) -> String {
    headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .or_else(|| {
            headers
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.split(',').next())
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Reject outright when either budget is spent; otherwise hand back the two counter keys.
async fn login_guard(state: &AppState, email: &str, ip: &str) -> ApiResult<(String, String)> {
    let mut conn = state.redis.clone();
    let ekey = format!("login_fail:{}", normalize_email(email));
    let ikey = format!("login_fail_ip:{ip}");
    let e_fails: Option<i64> = redis::cmd("GET").arg(&ekey).query_async(&mut conn).await?;
    let i_fails: Option<i64> = redis::cmd("GET").arg(&ikey).query_async(&mut conn).await?;
    if e_fails.unwrap_or(0) >= LOGIN_FAIL_PER_EMAIL || i_fails.unwrap_or(0) >= LOGIN_FAIL_PER_IP {
        return Err(AppError::bad(
            "登录失败次数过多，请稍后再试，或改用邮箱验证码登录",
        ));
    }
    Ok((ekey, ikey))
}

async fn login_fail(state: &AppState, ekey: &str, ikey: &str) {
    let mut conn = state.redis.clone();
    for k in [ekey, ikey] {
        let _: Result<i64, _> = redis::cmd("INCR").arg(k).query_async(&mut conn).await;
        // EXPIRE on every increment (idempotent): setting it only on the first failure
        // risks a crash in between leaving a TTL-less key that locks the account out forever
        // — the same reasoning as take_code() above.
        let _: Result<(), _> = redis::cmd("EXPIRE")
            .arg(k)
            .arg(LOGIN_FAIL_WINDOW_SECS)
            .query_async(&mut conn)
            .await;
    }
}

async fn login_ok(state: &AppState, ekey: &str, ikey: &str) {
    let mut conn = state.redis.clone();
    let _: Result<(), _> = redis::cmd("DEL")
        .arg(ekey)
        .arg(ikey)
        .query_async(&mut conn)
        .await;
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

/// Increment a counter inside a **fixed** window: the key is created together with
/// its TTL in one atomic `SET NX EX`, and later increments leave that TTL alone, so
/// the window expires a fixed time after the first hit no matter how many follow.
async fn bump_fixed_window(
    conn: &mut redis::aio::ConnectionManager,
    key: &str,
    ttl_secs: u64,
) -> ApiResult<i64> {
    let _: Option<String> = redis::cmd("SET")
        .arg(key)
        .arg(0i64)
        .arg("NX")
        .arg("EX")
        .arg(ttl_secs)
        .query_async(conn)
        .await?;
    Ok(redis::cmd("INCR").arg(key).query_async::<i64>(conn).await?)
}

async fn take_code(state: &AppState, email: &str, code: &str) -> ApiResult<bool> {
    let mut conn = state.redis.clone();
    let key = code_key(email);
    // Brute-force guard: count guesses within the code's TTL window. Past the cap,
    // BURN the code so even a later correct guess can't pass — the user must request
    // a fresh one (which resets the counter via store_code).
    // Both counters are created with their TTL in one atomic `SET NX EX` and then
    // only INCR'd, because INCR leaves an existing TTL alone.
    //
    // Calling EXPIRE on every attempt (the previous approach) made the window slide
    // forward with each guess, so the counter never actually expired while an attacker
    // kept poking: 20 wrong guesses put the hourly counter over the cap, and one
    // request per hour after that held it there — permanently blocking registration
    // and code-login for that address, from an anonymous caller. Creating the key with
    // its TTL up front also avoids the TTL-less-key risk that motivated EXPIRE-always.
    // (a) Per-code budget: reset when a fresh code is sent (store_code DELs it).
    let tries_key = attempts_key(email);
    let tries = bump_fixed_window(&mut conn, &tries_key, state.cfg.code_ttl_secs).await?;
    // (b) Hourly budget across the whole email, NOT reset by resends — caps total
    // guesses so an attacker can't keep requesting fresh codes to refill (a)'s 5.
    let hour_key = format!("code_tries_h:{}", normalize_email(email));
    let hourly = bump_fixed_window(&mut conn, &hour_key, 3600).await?;
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

/// The single canonical form of an email address. Everything that keys off an email
/// — Redis code/attempt/cooldown keys, user lookup, the stored row — must agree, or
/// the mismatch itself becomes the bug: codes were already stored lowercased while
/// lookup and INSERT used the raw string, so `Alice@x.com` missed the existing
/// `alice@x.com` row, passed the "already registered" check, and inserted a second
/// account for one mailbox (users.email's UNIQUE index is case-sensitive).
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

/// Look a user up case-insensitively. Matching on `lower(email)` rather than the
/// normalized string keeps mixed-case rows created before normalization loginable.
/// `LIMIT 1` (oldest first) because such rows may already collide pairwise, and a
/// bare `fetch_optional` would error out instead of resolving to the original account.
async fn find_user(state: &AppState, email: &str) -> ApiResult<Option<User>> {
    Ok(sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE lower(email) = $1 ORDER BY created_at LIMIT 1",
    )
    .bind(normalize_email(email))
    .fetch_optional(&state.db)
    .await?)
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

/// Per-address daily send ceiling. A 30s cooldown alone still allows 2880 mails a
/// day at one victim's inbox.
const CODE_SENDS_PER_EMAIL_PER_DAY: i64 = 12;
/// Ceiling on how many distinct sends one caller IP can trigger per hour. The
/// gateway has no rate-limit middleware at all, so without this a single host can
/// walk an address list and burn the whole email quota (which also gets the sending
/// domain blacklisted — damage that outlives the attack).
const CODE_SENDS_PER_IP_PER_HOUR: i64 = 20;

pub async fn send_code(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<EmailReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if !valid_email(&req.email) {
        return Err(AppError::bad("邮箱格式不正确"));
    }
    let email = normalize_email(&req.email);
    let mut conn = state.redis.clone();
    // Cooldown: one code per 30s per email. Without it an attacker could spam fresh
    // codes to reset the attempt cap (and bomb the inbox / our email quota).
    {
        let ok: Option<String> = redis::cmd("SET")
            .arg(format!("code_cd:{email}"))
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
    // Daily ceiling per address — protects the victim's inbox.
    let per_email =
        bump_fixed_window(&mut conn, &format!("code_send_d:{email}"), 24 * 3600).await?;
    if per_email > CODE_SENDS_PER_EMAIL_PER_DAY {
        return Err(AppError::bad("该邮箱今日验证码发送次数已达上限，请明天再试"));
    }
    // Hourly ceiling per caller — protects the email quota and sender reputation.
    // nginx restores the real client IP (conf.d/cloudflare-realip.conf), so
    // X-Forwarded-For's last hop is meaningful here; absent it we fall back to a
    // shared bucket rather than skipping the check.
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next_back())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    let per_ip = bump_fixed_window(&mut conn, &format!("code_send_ip:{ip}"), 3600).await?;
    if per_ip > CODE_SENDS_PER_IP_PER_HOUR {
        return Err(AppError::bad("请求过于频繁，请稍后再试"));
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
    // Duplicate check BEFORE consuming the code. take_code() deletes the code on success,
    // so the old order burned the code of anyone who retried a registration that had
    // already gone through — they then had to wait out the 30s resend cooldown.
    if find_user(&state, &req.email).await?.is_some() {
        return Err(AppError::bad("该邮箱已注册，请直接登录"));
    }
    if !take_code(&state, &req.email, &req.code).await? {
        return Err(AppError::bad("验证码错误或已过期"));
    }
    let hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST)?;
    // Store the canonical form so the UNIQUE index actually enforces one account per
    // mailbox from here on.
    // ON CONFLICT: two concurrent registrations for the same address both clear the check
    // above; without this the loser hits the UNIQUE index and surfaces as a raw 500.
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (email, password_hash) VALUES ($1, $2) \
         ON CONFLICT (email) DO NOTHING RETURNING *",
    )
    .bind(normalize_email(&req.email))
    .bind(&hash)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::bad("该邮箱已注册，请直接登录"))?;
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
    headers: axum::http::HeaderMap,
    Json(req): Json<LoginReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if req.email.trim().is_empty() || req.password.is_empty() {
        return Err(AppError::bad("账号或密码不能为空"));
    }
    let ip = client_ip(&headers);
    let (ekey, ikey) = login_guard(&state, &req.email, &ip).await?;
    // One message for "no such account" and "wrong password", and the same bcrypt cost on
    // both paths. Splitting them — and skipping bcrypt entirely when the account did not
    // exist — made both the response text AND the response time an enumeration oracle,
    // on an endpoint that had no rate limit at all.
    let user = match find_user(&state, req.email.trim()).await? {
        Some(u) => u,
        None => {
            let _ = bcrypt::verify(&req.password, &DUMMY_HASH);
            login_fail(&state, &ekey, &ikey).await;
            return Err(AppError::unauthorized("账号或密码错误"));
        }
    };
    if !bcrypt::verify(&req.password, &user.password_hash)? {
        login_fail(&state, &ekey, &ikey).await;
        return Err(AppError::unauthorized("账号或密码错误"));
    }
    login_ok(&state, &ekey, &ikey).await;
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

pub async fn me(State(state): State<AppState>, claims: Claims) -> ApiResult<Json<serde_json::Value>> {
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

    // 附带 michael-compression 的可用档位，客户端据此决定「本地要不要自己压」。
    // 这是**加字段**，老客户端会忽略它，不会破坏现有消费方（nginx 的 auth_request
    // 只看状态码）。
    let plan_active =
        user.plan != "none" && user.plan_expires_at.is_none_or(|e| e > chrono::Utc::now());
    // 关着的时候必须报 null，不能只是"不压缩"。客户端一旦从这里看到档位，就会
    // **关掉自己的本地压缩**（认为网关接管了）—— 只报档位却不真压，等于两边都不压，
    // 长对话直接撞穿模型原生窗口。报档位和真压缩必须由同一个开关控制。
    let tier = if state.cfg.compression_enabled {
        crate::compression::max_tier_for_plan(&user.plan, plan_active, user.credits_cents)
    } else {
        None
    };
    // Daily free-points grant, applied lazily on read: if the stored date is not today the
    // pool is overwritten with today's allowance, so yesterday's remainder is never carried.
    // Doing it here means the profile popup is always the freshest view of the pool.
    let free_points: i64 = sqlx::query_scalar(
        "UPDATE users SET \
           free_points = CASE WHEN free_points_date IS DISTINCT FROM CURRENT_DATE \
                              THEN $2 ELSE free_points END, \
           free_points_date = CURRENT_DATE \
         WHERE id = $1 RETURNING free_points",
    )
    .bind(id)
    .bind(crate::models::FREE_MILLI_POINTS_DAILY)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .unwrap_or(0);

    let mut body = serde_json::to_value(&user).unwrap_or_else(|_| json!({}));
    if let Some(obj) = body.as_object_mut() {
        // Stored in milli-点; exposed as fractional 点 so the client renders "39.94 点"
        // rather than a whole number that hides every sub-point call.
        obj.insert(
            "free_points".into(),
            json!(free_points as f64 / crate::models::MILLI as f64),
        );
        obj.insert(
            "free_points_daily".into(),
            json!(crate::models::FREE_POINTS_DAILY),
        );
        obj.insert(
            "michael_compression".into(),
            match tier {
                Some(t) => json!({ "tier": t.as_str(), "max_input_tokens": t.max_input_tokens() }),
                None => serde_json::Value::Null,
            },
        );
    }
    Ok(Json(body))
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

#[cfg(test)]
mod privilege_source_tests {
    /// Guard rail, not a behavioural test: the `Claims` extractor must keep reading
    /// `role` from the database instead of trusting the JWT claim.
    ///
    /// All 13 admin gates are `claims.role != "admin"`. When role came from the token,
    /// a 30-day JWT kept working after its owner was demoted or deleted, and because
    /// the admin endpoints can re-grant the role, one surviving token could take it
    /// back for good. There is no integration harness for the extractor here, so this
    /// asserts on the source: if someone "optimizes away" the lookup to save a query,
    /// this fails and explains why it exists.
    #[test]
    fn claims_extractor_resolves_role_from_the_database() {
        let src = include_str!("auth.rs");
        let extractor = src
            .split("impl FromRequestParts<AppState> for Claims")
            .nth(1)
            .expect("Claims extractor impl");
        let extractor = &extractor[..extractor.find("\nfn issue_token").unwrap_or(extractor.len())];
        assert!(
            extractor.contains("SELECT role FROM users WHERE id = $1"),
            "the extractor must re-read role from the users row, never trust the JWT claim"
        );
        assert!(
            extractor.contains("账号不存在或已注销"),
            "a deleted user's surviving token must fail closed"
        );
    }
}
