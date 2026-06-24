use axum::async_trait;
use axum::extract::{FromRequestParts, State};
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
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
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
    Ok(encode(&Header::default(), &claims, &EncodingKey::from_secret(cfg.jwt_secret.as_bytes()))?)
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

async fn store_code(state: &AppState, email: &str, code: &str) -> ApiResult<()> {
    let mut conn = state.redis.clone();
    let _: () = redis::cmd("SET")
        .arg(code_key(email))
        .arg(code)
        .arg("EX")
        .arg(state.cfg.code_ttl_secs)
        .query_async(&mut conn)
        .await?;
    Ok(())
}

async fn take_code(state: &AppState, email: &str, code: &str) -> ApiResult<bool> {
    let mut conn = state.redis.clone();
    let key = code_key(email);
    let stored: Option<String> = redis::cmd("GET").arg(&key).query_async(&mut conn).await?;
    match stored {
        Some(s) if s == code => {
            let _: () = redis::cmd("DEL").arg(&key).query_async(&mut conn).await?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

async fn send_code_email(cfg: &Config, to: &str, code: &str) -> ApiResult<bool> {
    if !cfg.smtp_enabled() {
        tracing::warn!("[DEV] SMTP not configured. Code for {to}: {code}");
        return Ok(false);
    }
    use lettre::message::header::ContentType;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

    let msg = Message::builder()
        .from(format!("Michael <{}>", cfg.smtp_user).parse()?)
        .to(to.parse()?)
        .subject("Michael 登录验证码")
        .header(ContentType::TEXT_PLAIN)
        .body(format!("你的验证码是：{code}\n\n10 分钟内有效。如非本人操作请忽略。"))?;
    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.smtp_host)?
            .credentials(Credentials::new(cfg.smtp_user.clone(), cfg.smtp_pass.clone()))
            .build();
    mailer.send(msg).await?;
    Ok(true)
}

fn gen_code() -> String {
    use rand::Rng;
    format!("{:06}", rand::thread_rng().gen_range(0..1_000_000))
}

async fn find_user(state: &AppState, email: &str) -> ApiResult<Option<User>> {
    Ok(sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(&state.db)
        .await?)
}

// ---- request/response payloads ---------------------------------------------

#[derive(Deserialize)]
pub struct EmailReq { pub email: String }
#[derive(Deserialize)]
pub struct LoginReq { pub email: String, pub password: String }
#[derive(Deserialize)]
pub struct RegisterReq { pub email: String, pub password: String, pub code: String }
#[derive(Deserialize)]
pub struct CodeReq { pub email: String, pub code: String }

// ---- handlers --------------------------------------------------------------

pub async fn check_email(State(state): State<AppState>, Json(req): Json<EmailReq>) -> ApiResult<Json<serde_json::Value>> {
    if !valid_email(&req.email) {
        return Err(AppError::bad("邮箱格式不正确"));
    }
    Ok(Json(json!({ "exists": find_user(&state, &req.email).await?.is_some() })))
}

pub async fn send_code(State(state): State<AppState>, Json(req): Json<EmailReq>) -> ApiResult<Json<serde_json::Value>> {
    if !valid_email(&req.email) {
        return Err(AppError::bad("邮箱格式不正确"));
    }
    let code = gen_code();
    store_code(&state, &req.email, &code).await?;
    let sent = send_code_email(&state.cfg, &req.email, &code).await?;
    Ok(Json(json!({ "sent": sent, "message": if sent { "验证码已发送到邮箱（10 分钟有效）" } else { "开发模式：验证码见服务端日志" } })))
}

pub async fn register(State(state): State<AppState>, Json(req): Json<RegisterReq>) -> ApiResult<Json<serde_json::Value>> {
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
    crate::realtime::record_event(&state, Some(user.id), "register", json!({ "email": user.email })).await;
    Ok(Json(json!({ "token": token, "user": user })))
}

pub async fn login(State(state): State<AppState>, Json(req): Json<LoginReq>) -> ApiResult<Json<serde_json::Value>> {
    if !valid_email(&req.email) {
        return Err(AppError::bad("邮箱格式不正确"));
    }
    let user = find_user(&state, &req.email).await?.ok_or_else(|| AppError::bad("该邮箱尚未注册"))?;
    if !bcrypt::verify(&req.password, &user.password_hash)? {
        return Err(AppError::unauthorized("密码错误"));
    }
    sqlx::query("UPDATE users SET last_login_at = now() WHERE id = $1").bind(user.id).execute(&state.db).await?;
    let token = issue_token(&state.cfg, &user)?;
    crate::realtime::record_event(&state, Some(user.id), "login", json!({ "email": user.email })).await;
    Ok(Json(json!({ "token": token, "user": user })))
}

pub async fn verify_code(State(state): State<AppState>, Json(req): Json<CodeReq>) -> ApiResult<Json<serde_json::Value>> {
    if !take_code(&state, &req.email, &req.code).await? {
        return Err(AppError::bad("验证码错误或已过期"));
    }
    let user = find_user(&state, &req.email).await?.ok_or_else(|| AppError::bad("该邮箱尚未注册，请先设置密码注册"))?;
    sqlx::query("UPDATE users SET last_login_at = now() WHERE id = $1").bind(user.id).execute(&state.db).await?;
    let token = issue_token(&state.cfg, &user)?;
    crate::realtime::record_event(&state, Some(user.id), "login", json!({ "email": user.email, "via": "code" })).await;
    Ok(Json(json!({ "token": token, "user": user })))
}

pub async fn me(State(state): State<AppState>, claims: Claims) -> ApiResult<Json<User>> {
    let id = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::unauthorized("用户不存在"))?;
    Ok(Json(user))
}

pub async fn admin_users(State(state): State<AppState>, claims: Claims) -> ApiResult<Json<Vec<User>>> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at DESC LIMIT 500")
        .fetch_all(&state.db)
        .await?;
    Ok(Json(users))
}
