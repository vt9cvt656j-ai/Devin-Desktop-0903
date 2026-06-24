use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::Claims;
use crate::config::Config;
use crate::error::{ApiResult, AppError};
use crate::AppState;

fn admin_only(claims: &Claims) -> ApiResult<()> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    Ok(())
}

/// Send a single email via the Brevo transactional HTTP API (over HTTPS/443, so
/// it works even when the host's outbound SMTP ports are blocked).
pub async fn send_mail(cfg: &Config, to: &str, subject: &str, body: &str, html: bool) -> ApiResult<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| AppError::internal(e.to_string()))?;
    let mut payload = json!({
        "sender": { "name": cfg.mail_from_name, "email": cfg.mail_from },
        "to": [ { "email": to } ],
        "subject": subject,
    });
    payload[if html { "htmlContent" } else { "textContent" }] = json!(body);
    let resp = client
        .post("https://api.brevo.com/v3/smtp/email")
        .header("api-key", &cfg.brevo_api_key)
        .header("accept", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("邮件发送失败: {e}")))?;
    if !resp.status().is_success() {
        let code = resp.status().as_u16();
        let txt = resp.text().await.unwrap_or_default();
        return Err(AppError::internal(format!("邮件服务返回 {code}: {txt}")));
    }
    Ok(())
}

async fn log_email(state: &AppState, to: &str, subject: &str, status: &str, error: Option<&str>, by: &str) {
    let _ = sqlx::query("INSERT INTO email_logs (to_email, subject, status, error, sent_by) VALUES ($1,$2,$3,$4,$5)")
        .bind(to)
        .bind(subject)
        .bind(status)
        .bind(error)
        .bind(by)
        .execute(&state.db)
        .await;
}

#[derive(Deserialize)]
pub struct NotifyReq {
    pub target: String, // all | plan | one
    pub plan: Option<String>,
    pub email: Option<String>,
    pub subject: String,
    pub body: String,
    pub html: Option<bool>,
}

/// POST /api/admin/notify — send a notification to all users / a plan tier / one address.
pub async fn notify(State(state): State<AppState>, claims: Claims, Json(req): Json<NotifyReq>) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    if req.subject.trim().is_empty() || req.body.trim().is_empty() {
        return Err(AppError::bad("主题和内容不能为空"));
    }
    let recipients: Vec<String> = match req.target.as_str() {
        "all" => {
            sqlx::query_scalar("SELECT email FROM users ORDER BY created_at DESC LIMIT 2000")
                .fetch_all(&state.db)
                .await?
        }
        "plan" => {
            let plan = req.plan.as_deref().unwrap_or("").to_string();
            if plan.is_empty() {
                return Err(AppError::bad("请选择套餐"));
            }
            sqlx::query_scalar("SELECT email FROM users WHERE plan = $1 ORDER BY created_at DESC LIMIT 2000")
                .bind(plan)
                .fetch_all(&state.db)
                .await?
        }
        "one" => {
            let e = req.email.as_deref().unwrap_or("").trim().to_string();
            if e.is_empty() {
                return Err(AppError::bad("请填写收件邮箱"));
            }
            vec![e]
        }
        _ => return Err(AppError::bad("target 只能是 all/plan/one")),
    };
    if recipients.is_empty() {
        return Err(AppError::bad("没有匹配的收件人"));
    }

    let html = req.html.unwrap_or(false);
    let dev = !state.cfg.mail_enabled();
    let mut sent = 0u32;
    let mut failed = 0u32;
    for to in &recipients {
        if dev {
            log_email(&state, to, &req.subject, "dev", Some("SMTP 未配置"), &claims.email).await;
            continue;
        }
        match send_mail(&state.cfg, to, &req.subject, &req.body, html).await {
            Ok(()) => {
                sent += 1;
                log_email(&state, to, &req.subject, "sent", None, &claims.email).await;
            }
            Err(e) => {
                failed += 1;
                log_email(&state, to, &req.subject, "failed", Some(e.msg.as_str()), &claims.email).await;
            }
        }
    }
    crate::realtime::record_event(&state, None, "notify", json!({ "by": claims.email, "total": recipients.len(), "sent": sent })).await;
    Ok(Json(json!({ "total": recipients.len(), "sent": sent, "failed": failed, "dev": dev })))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct EmailLog {
    pub id: i64,
    pub to_email: String,
    pub subject: String,
    pub status: String,
    pub error: Option<String>,
    pub sent_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// GET /api/admin/email-logs — recent outbound mail (admin only).
pub async fn logs(State(state): State<AppState>, claims: Claims) -> ApiResult<Json<Vec<EmailLog>>> {
    admin_only(&claims)?;
    let rows = sqlx::query_as::<_, EmailLog>("SELECT * FROM email_logs ORDER BY id DESC LIMIT 300")
        .fetch_all(&state.db)
        .await?;
    Ok(Json(rows))
}
