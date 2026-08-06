use axum::async_trait;
use axum::extract::{FromRequestParts, Path, State};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
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
    /// US order: given name first, family name second. Empty when never set.
    /// `avatar` is deliberately NOT a field here — this struct is also what
    /// `/api/admin/users` returns for up to 500 rows, and an inline image on each
    /// would turn that response into megabytes. `me` fetches it separately.
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    /// Interface language as a BCP-47 tag, or empty when never chosen. Clients treat an
    /// unrecognised value as "not set" and fall back to English.
    #[serde(default)]
    pub language: String,
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
    /// The `sessions` row this token was issued for, so a single device can be signed
    /// out without rotating the signing secret and ending everyone's session.
    ///
    /// Optional because tokens issued before sessions existed carry no `sid`, and those
    /// people should not be logged out by a deploy. Such a token still authenticates but
    /// cannot be listed or revoked individually; it ages out with the 30-day expiry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sid: Option<String>,
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
        let sid = claims.sid.as_deref().and_then(|s| uuid::Uuid::parse_str(s).ok());

        // One statement, because this runs on every authenticated request:
        //   * read the role from the row (see above),
        //   * confirm the session behind this token has not been revoked,
        //   * and bump last_seen_at, but only when it is already stale.
        //
        // The `$2 IS NULL` arm is what keeps pre-sessions tokens working: they carry no
        // sid, so there is no session to check and the request is judged on the user row
        // alone — exactly as it was before.
        let row: Option<(String, bool)> = sqlx::query_as(
            "WITH live AS ( \
                 SELECT id FROM sessions \
                 WHERE id = $2 AND user_id = $1 AND revoked_at IS NULL \
             ), touched AS ( \
                 UPDATE sessions SET last_seen_at = now() \
                 WHERE id IN (SELECT id FROM live) \
                   AND last_seen_at < now() - interval '5 minutes' \
                 RETURNING 1 \
             ) \
             SELECT u.role, ($2::uuid IS NULL OR EXISTS (SELECT 1 FROM live)) AS session_ok \
             FROM users u WHERE u.id = $1",
        )
        .bind(uid)
        .bind(sid)
        .fetch_optional(&state.db)
        .await?;

        let (role, session_ok) = row.ok_or_else(|| AppError::unauthorized("账号不存在或已注销"))?;
        if !session_ok {
            return Err(AppError::unauthorized("该设备的登录已被移除，请重新登录"));
        }
        claims.role = role;
        Ok(claims)
    }
}

/// Which kind of client signed in.
///
/// The hint the client sends wins because a Tauri window reports the system webview's
/// User-Agent — indistinguishable from Safari — so sniffing alone would file every
/// desktop sign-in as a browser. Anything unrecognised is called "web": guessing wrong
/// in the direction of the least specific answer is better than inventing a device.
pub(crate) fn device_kind(hint: Option<&str>, user_agent: &str) -> &'static str {
    match hint.map(str::trim).unwrap_or("").to_ascii_lowercase().as_str() {
        "desktop" => return "desktop",
        "mobile" => return "mobile",
        "web" => return "web",
        _ => {}
    }
    let ua = user_agent.to_ascii_lowercase();
    if ua.contains("tauri") || ua.contains("mrday") || ua.contains("electron") {
        "desktop"
    } else if ua.contains("iphone") || ua.contains("ipad") || ua.contains("android") || ua.contains("mobile") {
        "mobile"
    } else {
        "web"
    }
}

/// Open a `sessions` row for a sign-in and hand back a token that names it.
///
/// Every path that mints a token goes through here, so there is no way to end up with a
/// token that has no session behind it and therefore cannot be revoked.
async fn start_session(
    state: &AppState,
    user: &User,
    headers: &axum::http::HeaderMap,
    hint: Option<&str>,
) -> ApiResult<String> {
    let user_agent: String = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .chars()
        .take(200)
        .collect();
    let kind = device_kind(hint, &user_agent);

    let sid: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO sessions (user_id, kind, user_agent, ip) VALUES ($1,$2,$3,$4) RETURNING id",
    )
    .bind(user.id)
    .bind(kind)
    .bind(&user_agent)
    .bind(client_ip(headers))
    .fetch_one(&state.db)
    .await?;

    issue_token(&state.cfg, user, sid)
}

fn issue_token(cfg: &Config, user: &User, sid: uuid::Uuid) -> ApiResult<String> {
    let claims = Claims {
        sub: user.id.to_string(),
        email: user.email.clone(),
        role: user.role.clone(),
        exp: chrono::Utc::now().timestamp() + cfg.jwt_ttl_secs,
        sid: Some(sid.to_string()),
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
async fn find_user(state: &AppState, identity: &str) -> ApiResult<Option<User>> {
    // Matches on `email`, which in this deployment is really an IDENTITY column — the admin
    // account's stored email is literally "fendoushaonian", not an address. A username therefore
    // already works and no username column is needed; what blocked login was a `type="email"` on
    // the client input, which the browser rejected before the request was ever sent. Deliberately
    // ONE query: an email-or-username OR would let a username shadow someone else's real address.
    Ok(sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE lower(email) = $1 ORDER BY created_at LIMIT 1",
    )
    .bind(normalize_email(identity))
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
    /// What kind of client this is: "web", "desktop" or "mobile". Display only — it
    /// grants nothing, so a client that lies about it gains nothing. Absent from older
    /// clients, in which case the User-Agent decides.
    #[serde(default)]
    pub device: Option<String>,
}
#[derive(Deserialize)]
pub struct RegisterReq {
    pub email: String,
    pub password: String,
    pub code: String,
    /// What kind of client this is: "web", "desktop" or "mobile". Display only — it
    /// grants nothing, so a client that lies about it gains nothing. Absent from older
    /// clients, in which case the User-Agent decides.
    #[serde(default)]
    pub device: Option<String>,
}
#[derive(Deserialize)]
pub struct CodeReq {
    pub email: String,
    pub code: String,
    /// What kind of client this is: "web", "desktop" or "mobile". Display only — it
    /// grants nothing, so a client that lies about it gains nothing. Absent from older
    /// clients, in which case the User-Agent decides.
    #[serde(default)]
    pub device: Option<String>,
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
    headers: axum::http::HeaderMap,
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
    let token = start_session(&state, &user, &headers, req.device.as_deref()).await?;
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
    let token = start_session(&state, &user, &headers, req.device.as_deref()).await?;
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
    headers: axum::http::HeaderMap,
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
    let token = start_session(&state, &user, &headers, req.device.as_deref()).await?;
    crate::realtime::record_event(
        &state,
        Some(user.id),
        "login",
        json!({ "email": user.email, "via": "code" }),
    )
    .await;
    Ok(Json(json!({ "token": token, "user": user })))
}

/// `GET /api/authz` —— nginx `auth_request` 的目标。204 = 已登录，401 = 没登录。
///
/// ## 为什么必须和 /api/me 分开
///
/// `/_app_authz` 原来打的是 `/api/me`，而 `/api/me` 每次要跑 **两条 UPDATE + 一条
/// SELECT \***（配额窗口刷新、每日免费点发放、整行读取）。nginx 的 auth_request 是
/// **每个受门禁的请求**都触发一次 —— 包括 `/app/` 和 `/account/` 下的每一个 JS、CSS、
/// 字体文件。打开一次网页版 IDE 有几十个静态资源，就是几十次 auth 子请求、上百次
/// 对同一行 users 的写。
///
/// 线上实测（2026-08-05）：users 表 120 行，累计 **362,059 次 UPDATE**，而
/// model_usage 只有 78,086 行 —— 也就是说绝大多数写入根本不来自计费，来自这个门禁。
/// HOT update 让它没有膨胀（dead tuple 只有 26），但每条 UPDATE 都要拿行锁，同一个
/// 用户的并发请求因此串行化，日志里那条 `time to acquire exceeded slow threshold
/// aquired_after_secs=2.29` 就是连接卡在行锁上。
///
/// 这个问题 console_session.rs 早就写明白了（"/api/me … 还会跑两条 UPDATE，不适合
/// 每个请求都触发一次"），并且为 `/console/` 单独做了只读的 `/api/admin/authz`。
/// 这里只是把同一套做法补给 `/app/` 和 `/account/`。
///
/// 本函数**一次写都没有**：`Claims` 提取器已经按主键读过一次 users 校验身份和 role，
/// 到这里只需要回一个状态码。配额刷新和每日发放仍然在 `/api/me` 里做 —— 那才是真正
/// 要展示余额的地方，而且它由用户打开资料页触发，不是每个静态资源都触发。
pub async fn authz(_claims: Claims) -> Response {
    // 能走到这里说明 Claims 提取器已经验过令牌、且用户还在库里（删号立刻失效）。
    StatusCode::NO_CONTENT.into_response()
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
    .bind(crate::models::free_milli_points_daily())
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
            json!(crate::models::free_points_daily()),
        );
        // 面值分母随资料一起下发。客户端与两个管理页原先各自硬编码 663，其中三处还在
        // 写路径上（管理员输入的美元由前端乘 663 变成存库的真实分），改一处不改其余就会
        // "发出去多少"和"显示多少"对不上。现在只有服务端有这个数。
        obj.insert(
            "raw_cents_per_credit_usd".into(),
            json!(crate::settings::raw_cents_per_credit_usd()),
        );
        // Fetched on its own rather than as a `User` field: see the note on the struct.
        let avatar: Option<String> = sqlx::query_scalar("SELECT avatar FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
            .flatten();
        obj.insert("avatar".into(), json!(avatar));
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

/// Longest name half accepted. Generous for a legal name, short enough that the column
/// can never be used as free storage.
const NAME_MAX_CHARS: usize = 64;
/// Longest `data:` URL accepted, in characters. The browser resizes to a 256px square
/// before upload (~30 KB), so this is roughly seven times what an honest client sends —
/// enough headroom for an odd encoder, far short of letting the column hold a file.
const AVATAR_MAX_CHARS: usize = 300_000;

/// Names are free text, but not *anything*: control characters would let a display name
/// break out of the line it is rendered on, and leading or trailing space is invisible
/// yet compares unequal.
fn clean_name(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(AppError::bad("姓名不能包含控制字符"));
    }
    if trimmed.chars().count() > NAME_MAX_CHARS {
        return Err(AppError::bad(format!("姓名最长 {NAME_MAX_CHARS} 个字符")));
    }
    Ok(trimmed.to_owned())
}

/// Only these three encodings, and only as a self-contained `data:` URL. An `http(s)`
/// URL would turn every profile render into a request to a third party chosen by the
/// account holder — an SSRF-shaped hole on the server and a tracking pixel on the client.
fn clean_avatar(raw: &str) -> Result<Option<String>, AppError> {
    let value = raw.trim();
    if value.is_empty() {
        return Ok(None); // an explicit clear
    }
    if value.len() > AVATAR_MAX_CHARS {
        return Err(AppError::bad("头像过大，请换一张图片"));
    }
    const ALLOWED: [&str; 3] = [
        "data:image/png;base64,",
        "data:image/jpeg;base64,",
        "data:image/webp;base64,",
    ];
    let Some(prefix) = ALLOWED.iter().find(|p| value.starts_with(**p)) else {
        return Err(AppError::bad("头像格式不支持"));
    };
    // Reject anything that is not actually base64 after the comma, so the column cannot
    // be loaded with a payload that only *looks* like an image to this check.
    let payload = &value[prefix.len()..];
    if payload.is_empty()
        || !payload
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')
    {
        return Err(AppError::bad("头像数据无效"));
    }
    Ok(Some(value.to_owned()))
}

#[derive(Deserialize)]
pub struct ProfileReq {
    /// Absent leaves the stored value alone; present replaces it. That distinction is
    /// what lets the picture be saved without resending the names and vice versa.
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    /// A `data:` URL, or "" to remove the current picture.
    pub avatar: Option<String>,
    /// BCP-47 tag. Length-capped and character-restricted below rather than checked
    /// against a list: the list of offered languages belongs to the client.
    #[serde(default)]
    pub language: Option<String>,
}

/// `POST /api/me/profile` — the account holder edits their own display name and picture.
///
/// Scoped to the caller's own row by the token: there is no id in the path and none is
/// accepted from the body, so this endpoint cannot be aimed at another account.
pub async fn update_profile(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<ProfileReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let id = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    let first = req.first_name.as_deref().map(clean_name).transpose()?;
    let last = req.last_name.as_deref().map(clean_name).transpose()?;
    // Flattened deliberately: "not sent" and "sent an explicit clear" both collapse to
    // None here, and the boolean bound to $4 below is what tells them apart.
    let avatar: Option<String> = req.avatar.as_deref().map(clean_avatar).transpose()?.flatten();
    // A tag, not free text: letters and hyphens only, and short enough that no amount of
    // it can bloat the row. Nothing here decides behaviour on the server — it is handed
    // straight back to clients — but it is still stored input, so it is bounded.
    let language: Option<String> = req.language.as_deref().map(|v| {
        v.trim()
            .chars()
            .filter(|c| c.is_ascii_alphabetic() || *c == '-')
            .take(16)
            .collect::<String>()
    });

    // COALESCE keeps the stored value when the client sent no opinion. The avatar needs
    // the extra flag because "sent nothing" and "sent an explicit clear" both arrive as
    // a SQL NULL and mean opposite things.
    sqlx::query(
        "UPDATE users SET \
           first_name = COALESCE($2, first_name), \
           last_name  = COALESCE($3, last_name), \
           avatar     = CASE WHEN $4 THEN $5 ELSE avatar END, \
           language   = COALESCE($6, language), \
           updated_at = now() \
         WHERE id = $1",
    )
    .bind(id)
    .bind(first.as_deref())
    .bind(last.as_deref())
    .bind(req.avatar.is_some())
    .bind(avatar.as_deref())
    .bind(language.as_deref())
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "ok": true })))
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
            extractor.contains("SELECT u.role")
                && extractor.contains("FROM users u WHERE u.id = $1")
                && extractor.contains("claims.role = role"),
            "the extractor must re-read role from the users row, never trust the JWT claim"
        );
        assert!(
            extractor.contains("账号不存在或已注销"),
            "a deleted user's surviving token must fail closed"
        );
    }
}


#[cfg(test)]
mod authz_gate_tests {
    /// 门禁端点必须是**只读**的。
    ///
    /// nginx 的 `auth_request` 对每个受保护请求触发一次 —— 包括 `/app/` 和 `/account/`
    /// 下的每个静态资源。它原来打的是 `/api/me`，而 `/api/me` 每次跑两条 UPDATE
    /// （配额窗口刷新 + 每日免费点发放）。线上 users 表 120 行累计 362,059 次 UPDATE，
    /// 远超 model_usage 的 78,086 行 —— 写入的主体不是计费，是这道门禁。
    ///
    /// 这条测试盯住两件事：`authz` 自己不能写，且 nginx 不能再指回 /api/me。
    #[test]
    fn authz_endpoint_performs_no_writes() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/auth.rs"))
            .expect("read auth.rs");
        let start = src.find("pub async fn authz(").expect("authz 必须存在");
        let end = src[start..].find("\npub async fn me(").expect("me 紧随其后") + start;
        let body = &src[start..end];
        for write in ["UPDATE ", "INSERT ", "DELETE ", "sqlx::query"] {
            assert!(
                !body.contains(write),
                "authz 里出现了 `{write}` —— 它每个静态资源都会跑一次，必须保持零写入",
            );
        }
    }

    #[test]
    fn nginx_app_gate_points_at_the_readonly_endpoint() {
        let conf = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/nginx/michael-backend.conf"
        ))
        .expect("read nginx conf");
        let start = conf.find("location = /_app_authz").expect("门禁 location 必须在");
        // 按**字符**截取，不按字节：这个文件里有中文注释，`&conf[a..a+400]` 一旦落在
        // 多字节字符中间就会 panic（第一版就是这么挂的，而配置本身是对的）。
        let block: String = conf[start..].chars().take(400).collect();
        let block = block.as_str();
        assert!(
            block.contains("/api/authz"),
            "/_app_authz 必须打只读的 /api/authz",
        );
        assert!(
            !block.contains("proxy_pass http://127.0.0.1:8080/api/me"),
            "/_app_authz 不能再打 /api/me —— 那会让每个静态资源触发两条 UPDATE",
        );
    }
}
