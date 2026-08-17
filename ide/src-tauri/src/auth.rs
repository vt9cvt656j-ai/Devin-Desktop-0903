use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool};
use std::sync::OnceLock;
use tauri::command;

static DB_POOL: OnceLock<SqlitePool> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResult {
    pub success: bool,
    pub email: String,
    pub user_id: String,
    pub message: String,
    pub is_new_user: bool,
}

/// 本地账号库所在的目录。
///
/// Windows 上**没有 HOME，只有 USERPROFILE**。这里以前只读 HOME，读不到就退到 "."——
/// 也就是把 SQLite 库建在**安装目录**下（Program Files 之类，通常不可写）。结果是
/// 账号库初始化失败，登录整条路在 Windows 上不可用。全项目别处都写了这个兜底，
/// 独独漏了这一处，所以单独抽出来并钉一条守卫。
fn auth_db_dir() -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    format!("{home}/.michael_ide")
}

pub async fn init_db() -> Result<(), String> {
    // Default to an on-disk SQLite file under ~/.michael_ide (created if needed);
    // `mode=rwc` lets SQLite create the database file itself.
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        let dir = auth_db_dir();
        let _ = std::fs::create_dir_all(&dir);
        format!("sqlite://{dir}/auth.db?mode=rwc")
    });

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .map_err(|e| format!("SQLite connection failed: {e}"))?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to create users table: {e}"))?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS marketplace_extensions (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            author TEXT NOT NULL,
            version TEXT NOT NULL DEFAULT '1.0.0',
            description TEXT,
            category TEXT DEFAULT 'Other',
            tags TEXT,
            featured INTEGER DEFAULT 0,
            downloads INTEGER DEFAULT 0,
            rating REAL DEFAULT 0.0,
            icon TEXT DEFAULT 'default',
            icon_svg TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to create marketplace_extensions table: {e}"))?;

    DB_POOL
        .set(pool)
        .map_err(|_| "DB pool already initialized".to_string())?;

    tracing::info!("SQLite connected, tables ready");
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct DbExtension {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Option<serde_json::Value>,
    pub featured: bool,
    pub downloads: i64,
    pub rating: f32,
    pub icon: Option<String>,
    pub icon_svg: Option<String>,
}

#[command]
pub async fn db_marketplace_list() -> Result<Vec<DbExtension>, String> {
    let p = pool()?;
    let rows: Vec<DbExtension> = sqlx::query_as::<_, DbExtension>(
        "SELECT id, name, author, version, description, category, tags, featured, downloads, rating, icon, icon_svg FROM marketplace_extensions ORDER BY downloads DESC"
    )
    .fetch_all(p)
    .await
    .map_err(|e| format!("Query failed: {e}"))?;
    Ok(rows)
}

#[command]
pub async fn db_marketplace_upsert(ext: DbExtension) -> Result<String, String> {
    let p = pool()?;
    let tags_json = ext.tags.unwrap_or(serde_json::json!([]));
    sqlx::query(
        r#"INSERT INTO marketplace_extensions (id, name, author, version, description, category, tags, featured, downloads, rating, icon, icon_svg)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET name=excluded.name, author=excluded.author, version=excluded.version, description=excluded.description,
        category=excluded.category, tags=excluded.tags, featured=excluded.featured, downloads=excluded.downloads, rating=excluded.rating, icon=excluded.icon, icon_svg=excluded.icon_svg"#
    )
    .bind(&ext.id).bind(&ext.name).bind(&ext.author).bind(&ext.version)
    .bind(&ext.description).bind(&ext.category).bind(&tags_json)
    .bind(ext.featured).bind(ext.downloads).bind(ext.rating)
    .bind(&ext.icon).bind(&ext.icon_svg)
    .execute(p)
    .await
    .map_err(|e| format!("Upsert failed: {e}"))?;
    Ok(format!("Extension {} saved", ext.id))
}

fn pool() -> Result<&'static SqlitePool, String> {
    DB_POOL
        .get()
        .ok_or_else(|| "Database not connected".to_string())
}

#[command]
pub async fn auth_login_or_register(email: String, password: String) -> Result<AuthResult, String> {
    if email.is_empty() || !email.contains('@') {
        return Err("Invalid email".to_string());
    }
    if password.len() < 6 {
        return Err("Password must be at least 6 characters".to_string());
    }

    let pool = pool()?;

    let existing: Option<(String, String)> =
        sqlx::query_as("SELECT id, password_hash FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("DB query failed: {e}"))?;

    match existing {
        Some((user_id, hash)) => {
            let valid = bcrypt::verify(&password, &hash)
                .map_err(|e| format!("Password verification failed: {e}"))?;
            if !valid {
                return Ok(AuthResult {
                    success: false,
                    email,
                    user_id: String::new(),
                    message: "密码错误".to_string(),
                    is_new_user: false,
                });
            }
            Ok(AuthResult {
                success: true,
                email,
                user_id,
                message: "登录成功".to_string(),
                is_new_user: false,
            })
        }
        None => {
            let user_id = uuid::Uuid::new_v4().to_string();
            let hash =
                bcrypt::hash(&password, 10).map_err(|e| format!("Password hashing failed: {e}"))?;

            sqlx::query("INSERT INTO users (id, email, password_hash) VALUES (?, ?, ?)")
                .bind(&user_id)
                .bind(&email)
                .bind(&hash)
                .execute(pool)
                .await
                .map_err(|e| format!("Failed to create user: {e}"))?;

            Ok(AuthResult {
                success: true,
                email,
                user_id,
                message: "注册成功，已自动登录".to_string(),
                is_new_user: true,
            })
        }
    }
}

#[command]
pub async fn auth_check_email(email: String) -> Result<bool, String> {
    let pool = pool()?;
    let row = sqlx::query("SELECT COUNT(*) as cnt FROM users WHERE email = ?")
        .bind(&email)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("DB query failed: {e}"))?;

    let count: i64 = row.try_get("cnt").unwrap_or(0);
    Ok(count > 0)
}

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// email -> (code, expires_at). In-memory by design — codes are short-lived.
static VERIFY_CODES: OnceLock<Mutex<HashMap<String, (String, Instant)>>> = OnceLock::new();
const CODE_TTL: Duration = Duration::from_secs(600); // 10 minutes

fn codes() -> &'static Mutex<HashMap<String, (String, Instant)>> {
    VERIFY_CODES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Lightweight email-format check ("合规"): exactly one '@', a non-empty local
/// part, a dotted domain, a 2+ char TLD, no whitespace, sane length.
pub fn valid_email(email: &str) -> bool {
    let e = email.trim();
    if e.len() < 6 || e.len() > 254 || e.contains(char::is_whitespace) {
        return false;
    }
    let mut parts = e.split('@');
    let (local, domain) = match (parts.next(), parts.next(), parts.next()) {
        (Some(l), Some(d), None) => (l, d),
        _ => return false, // zero, or more than one, '@'
    };
    if local.is_empty() || domain.len() < 3 || domain.starts_with('.') || domain.ends_with('.') {
        return false;
    }
    domain.contains('.') && domain.rsplit('.').next().is_some_and(|tld| tld.len() >= 2)
}

/// Send the 6-digit code to `to` via QQ SMTP. Credentials come from the
/// environment: `QQ_SMTP_USER` = the full qq email, `QQ_SMTP_PASS` = the SMTP
/// *authorization code* (NOT the QQ login password — generate it in QQ Mail →
/// 设置 → 账户 → POP3/IMAP/SMTP). If they're unset we fall back to dev mode (the
/// code is only logged) so the flow still works locally without secrets.
/// Returns Ok(true) when a real email was sent, Ok(false) in dev mode.
/// Load SMTP credentials: environment first, then a local, never-committed file
/// at ~/.michael_ide/smtp.env (KEY=VALUE per line). This keeps the secret out of
/// the source tree entirely — set it once and forget it.
fn smtp_creds() -> (String, String) {
    let mut user = std::env::var("QQ_SMTP_USER").unwrap_or_default();
    let mut pass = std::env::var("QQ_SMTP_PASS").unwrap_or_default();
    if user.is_empty() || pass.is_empty() {
        let home = std::env::var("HOME").unwrap_or_default();
        if let Ok(txt) = std::fs::read_to_string(format!("{home}/.michael_ide/smtp.env")) {
            for line in txt.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    let v = v.trim().trim_matches('"').to_string();
                    match k.trim() {
                        "QQ_SMTP_USER" if user.is_empty() => user = v,
                        "QQ_SMTP_PASS" if pass.is_empty() => pass = v,
                        _ => {}
                    }
                }
            }
        }
    }
    (user, pass)
}

async fn send_email_code(to: &str, code: &str) -> Result<bool, String> {
    let (user, pass) = smtp_creds();
    if user.is_empty() || pass.is_empty() {
        tracing::warn!("[DEV] SMTP not configured (set QQ_SMTP_USER/QQ_SMTP_PASS or ~/.michael_ide/smtp.env). Code for {to}: {code}");
        return Ok(false);
    }

    use lettre::message::header::ContentType;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

    let msg = Message::builder()
        .from(
            format!("Mr. Day One <{user}>")
                .parse()
                .map_err(|e| format!("bad sender address: {e}"))?,
        )
        .to(to
            .parse()
            .map_err(|e| format!("bad recipient address: {e}"))?)
        .subject("Mr. Day One 登录验证码")
        .header(ContentType::TEXT_PLAIN)
        .body(format!(
            "你的 Mr. Day One 验证码是：{code}\n\n10 分钟内有效。如非本人操作，请忽略本邮件。"
        ))
        .map_err(|e| format!("build email failed: {e}"))?;

    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.qq.com")
            .map_err(|e| format!("SMTP setup failed: {e}"))?
            .credentials(Credentials::new(user, pass))
            .build();

    mailer
        .send(msg)
        .await
        .map_err(|e| format!("发送邮件失败: {e}"))?;
    Ok(true)
}

#[command]
pub async fn auth_send_code(email: String) -> Result<String, String> {
    if !valid_email(&email) {
        return Err("邮箱格式不正确".to_string());
    }
    let code: String = (0..6)
        .map(|_| (b'0' + (rand::random::<u8>() % 10)) as char)
        .collect();
    codes()
        .lock()
        .unwrap()
        .insert(email.clone(), (code.clone(), Instant::now() + CODE_TTL));
    if send_email_code(&email, &code).await? {
        Ok("验证码已发送到你的邮箱（10 分钟内有效）".to_string())
    } else {
        Ok("验证码已生成（开发模式：未配置 SMTP，请查看终端日志）".to_string())
    }
}

/// Check a stored code matches and hasn't expired; consume it on success.
fn take_valid_code(email: &str, code: &str) -> bool {
    let mut map = codes().lock().unwrap();
    match map.get(email) {
        Some((expected, expires)) if expected == code && Instant::now() < *expires => {
            map.remove(email);
            true
        }
        _ => false,
    }
}

/// Existing-account login: the email must already exist and the password match.
#[command]
pub async fn auth_login(email: String, password: String) -> Result<AuthResult, String> {
    if !valid_email(&email) {
        return Err("邮箱格式不正确".to_string());
    }
    let pool = pool()?;
    let existing: Option<(String, String)> =
        sqlx::query_as("SELECT id, password_hash FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("DB query failed: {e}"))?;
    match existing {
        None => Ok(AuthResult {
            success: false,
            email,
            user_id: String::new(),
            message: "该邮箱尚未注册".into(),
            is_new_user: true,
        }),
        Some((user_id, hash)) => {
            let valid = bcrypt::verify(&password, &hash)
                .map_err(|e| format!("Password verification failed: {e}"))?;
            if valid {
                Ok(AuthResult {
                    success: true,
                    email,
                    user_id,
                    message: "登录成功".into(),
                    is_new_user: false,
                })
            } else {
                Ok(AuthResult {
                    success: false,
                    email,
                    user_id: String::new(),
                    message: "密码错误".into(),
                    is_new_user: false,
                })
            }
        }
    }
}

/// New-account registration: verify the email code, then create the account with
/// the password the user chose (the proper signup completion step).
#[command]
pub async fn auth_register(
    email: String,
    password: String,
    code: String,
) -> Result<AuthResult, String> {
    if !valid_email(&email) {
        return Err("邮箱格式不正确".to_string());
    }
    if password.len() < 6 {
        return Err("密码至少 6 位".to_string());
    }
    if !take_valid_code(&email, &code) {
        return Ok(AuthResult {
            success: false,
            email,
            user_id: String::new(),
            message: "验证码错误或已过期".into(),
            is_new_user: true,
        });
    }
    let pool = pool()?;
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("DB query failed: {e}"))?;
    if exists.is_some() {
        return Ok(AuthResult {
            success: false,
            email,
            user_id: String::new(),
            message: "该邮箱已注册，请直接登录".into(),
            is_new_user: false,
        });
    }
    let user_id = uuid::Uuid::new_v4().to_string();
    let hash = bcrypt::hash(&password, 10).map_err(|e| format!("Password hashing failed: {e}"))?;
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES (?, ?, ?)")
        .bind(&user_id)
        .bind(&email)
        .bind(&hash)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to create user: {e}"))?;
    Ok(AuthResult {
        success: true,
        email,
        user_id,
        message: "注册成功，已登录".into(),
        is_new_user: true,
    })
}

/// Passwordless code login for an EXISTING account. (New accounts must go through
/// auth_register so the user's chosen password gets stored.)
#[command]
pub async fn auth_verify_code(email: String, code: String) -> Result<AuthResult, String> {
    if !take_valid_code(&email, &code) {
        return Ok(AuthResult {
            success: false,
            email,
            user_id: String::new(),
            message: "验证码错误或已过期".into(),
            is_new_user: false,
        });
    }
    let pool = pool()?;
    let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("DB query failed: {e}"))?;
    match existing {
        Some((user_id,)) => Ok(AuthResult {
            success: true,
            email,
            user_id,
            message: "验证码登录成功".into(),
            is_new_user: false,
        }),
        None => Ok(AuthResult {
            success: false,
            email,
            user_id: String::new(),
            message: "该邮箱尚未注册，请先设置密码注册".into(),
            is_new_user: true,
        }),
    }
}

#[cfg(test)]
mod auth_dir_tests {
    /// HOME 和 USERPROFILE 都是进程级的，改它们的用例必须排队。
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct Restore(Vec<(&'static str, Option<String>)>);
    impl Drop for Restore {
        fn drop(&mut self) {
            for (key, old) in self.0.drain(..) {
                unsafe {
                    match old {
                        Some(v) => std::env::set_var(key, v),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
    }

    #[test]
    fn windows_has_no_home_so_userprofile_must_be_honoured() {
        let _serial = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let _restore = Restore(vec![
            ("HOME", std::env::var("HOME").ok()),
            ("USERPROFILE", std::env::var("USERPROFILE").ok()),
        ]);
        unsafe {
            std::env::remove_var("HOME");
            std::env::set_var("USERPROFILE", "C:\\Users\\me");
        }
        assert_eq!(super::auth_db_dir(), "C:\\Users\\me/.michael_ide");
        assert!(
            !super::auth_db_dir().starts_with('.'),
            "退到 \".\" 会把账号库建在安装目录里（Program Files 通常不可写），登录整条路会废掉",
        );

        // HOME 在的时候仍然优先用它——别把 macOS/Linux 的行为改掉。
        unsafe { std::env::set_var("HOME", "/Users/me") };
        assert_eq!(super::auth_db_dir(), "/Users/me/.michael_ide");
    }
}
