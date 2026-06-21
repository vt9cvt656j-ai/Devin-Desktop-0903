use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySqlPool, Row};
use std::sync::OnceLock;
use tauri::command;

static DB_POOL: OnceLock<MySqlPool> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResult {
    pub success: bool,
    pub email: String,
    pub user_id: String,
    pub message: String,
    pub is_new_user: bool,
}

pub async fn init_db() -> Result<(), String> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:@127.0.0.1:3306/michael_ide".to_string());

    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .map_err(|e| format!("MySQL connection failed: {e}"))?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id VARCHAR(36) PRIMARY KEY,
            email VARCHAR(255) NOT NULL UNIQUE,
            password_hash VARCHAR(255) NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
        "#,
    )
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to create users table: {e}"))?;

    DB_POOL
        .set(pool)
        .map_err(|_| "DB pool already initialized".to_string())?;

    tracing::info!("MySQL connected, users table ready");
    Ok(())
}

fn pool() -> Result<&'static MySqlPool, String> {
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
