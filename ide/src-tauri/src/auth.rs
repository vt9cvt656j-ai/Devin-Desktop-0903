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

pub async fn init_db() -> Result<(), String> {
    // Default to an on-disk SQLite file under ~/.michael_ide (created if needed);
    // `mode=rwc` lets SQLite create the database file itself.
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let dir = format!("{home}/.michael_ide");
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
    DB_POOL.get().ok_or_else(|| "Database not connected".to_string())
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

    let existing: Option<(String, String)> = sqlx::query_as(
        "SELECT id, password_hash FROM users WHERE email = ?"
    )
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
            let hash = bcrypt::hash(&password, 10)
                .map_err(|e| format!("Password hashing failed: {e}"))?;

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

use std::sync::Mutex;
use std::collections::HashMap;

static VERIFY_CODES: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn codes() -> &'static Mutex<HashMap<String, String>> {
    VERIFY_CODES.get_or_init(|| Mutex::new(HashMap::new()))
}

#[command]
pub async fn auth_send_code(email: String) -> Result<String, String> {
    if email.is_empty() || !email.contains('@') {
        return Err("Invalid email".to_string());
    }
    let code: String = (0..6).map(|_| (b'0' + (rand::random::<u8>() % 10)) as char).collect();
    tracing::info!("[DEV] Verification code for {email}: {code}");
    codes().lock().unwrap().insert(email.clone(), code.clone());
    Ok("验证码已发送（开发模式：查看终端日志）".to_string())
}

#[command]
pub async fn auth_verify_code(email: String, code: String) -> Result<AuthResult, String> {
    let stored = codes().lock().unwrap().get(&email).cloned();
    match stored {
        Some(expected) if expected == code => {
            codes().lock().unwrap().remove(&email);
            let pool = pool()?;
            let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE email = ?")
                .bind(&email).fetch_optional(pool).await.map_err(|e| format!("{e}"))?;
            let (user_id, is_new) = match existing {
                Some((id,)) => (id, false),
                None => {
                    let id = uuid::Uuid::new_v4().to_string();
                    let hash = bcrypt::hash("code-login", 10).unwrap_or_default();
                    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES (?, ?, ?)")
                        .bind(&id).bind(&email).bind(&hash)
                        .execute(pool).await.map_err(|e| format!("{e}"))?;
                    (id, true)
                }
            };
            Ok(AuthResult { success: true, email, user_id, message: "验证码登录成功".into(), is_new_user: is_new })
        }
        _ => Ok(AuthResult { success: false, email, user_id: String::new(), message: "验证码错误或已过期".into(), is_new_user: false }),
    }
}
