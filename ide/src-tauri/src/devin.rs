//! Integration with the hosted Devin API (https://docs.devin.ai/api-reference).
//!
//! The assistant panel can talk to a real Devin session instead of a raw
//! OpenAI-compatible model: the user types a prompt, we create (or continue) a
//! Devin session, and the frontend polls [`devin_get_session`] to stream the
//! session's messages back into the chat. Devin works against the user's local
//! project because the prompt carries the open file/selection as context.

use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "https://api.devin.ai";

/// Shared, pooled client: the frontend polls the session every few seconds, so a
/// single keep-alive client avoids a fresh TLS handshake on every poll.
static HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(60))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevinConfig {
    /// Personal or service API key (prefix `apk_`).
    pub api_key: String,
    /// API base URL. Defaults to the hosted API when empty.
    #[serde(default)]
    pub base_url: String,
}

impl DevinConfig {
    fn base(&self) -> &str {
        let b = self.base_url.trim().trim_end_matches('/');
        if b.is_empty() {
            DEFAULT_BASE_URL
        } else {
            b
        }
    }
}

/// Returned by `POST /v1/sessions`.
#[derive(Deserialize, Serialize, Clone)]
pub struct DevinSessionRef {
    pub session_id: String,
    pub url: String,
}

/// A single message in a Devin session.
#[derive(Deserialize, Serialize, Clone)]
pub struct DevinMessage {
    /// Message type, e.g. `devin_message` or `user_message`.
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub event_id: String,
    #[serde(default)]
    pub timestamp: String,
}

/// Subset of `GET /v1/session/{id}` we surface to the UI.
#[derive(Deserialize, Serialize, Clone)]
pub struct DevinSession {
    pub session_id: String,
    pub status: String,
    #[serde(default)]
    pub status_enum: Option<String>,
    #[serde(default)]
    pub messages: Vec<DevinMessage>,
}

async fn check(resp: reqwest::Response) -> Result<reqwest::Response, String> {
    if resp.status().is_success() {
        return Ok(resp);
    }
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    Err(format!("Devin API error ({status}): {body}"))
}

/// Create a new Devin session seeded with `prompt`.
#[tauri::command]
pub async fn devin_create_session(
    config: DevinConfig,
    prompt: String,
    title: Option<String>,
) -> Result<DevinSessionRef, String> {
    let url = format!("{}/v1/sessions", config.base());
    let mut payload = serde_json::json!({ "prompt": prompt });
    if let Some(title) = title.filter(|t| !t.trim().is_empty()) {
        payload["title"] = serde_json::Value::String(title);
    }
    let resp = HTTP
        .post(&url)
        .bearer_auth(&config.api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    check(resp)
        .await?
        .json::<DevinSessionRef>()
        .await
        .map_err(|e| e.to_string())
}

/// Send a follow-up message to an existing session.
#[tauri::command]
pub async fn devin_send_message(
    config: DevinConfig,
    session_id: String,
    message: String,
) -> Result<(), String> {
    let url = format!("{}/v1/session/{}/message", config.base(), session_id);
    let resp = HTTP
        .post(&url)
        .bearer_auth(&config.api_key)
        .json(&serde_json::json!({ "message": message }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    check(resp).await.map(|_| ())
}

/// Fetch the current status and messages of a session.
#[tauri::command]
pub async fn devin_get_session(
    config: DevinConfig,
    session_id: String,
) -> Result<DevinSession, String> {
    let url = format!("{}/v1/session/{}", config.base(), session_id);
    let resp = HTTP
        .get(&url)
        .bearer_auth(&config.api_key)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    check(resp)
        .await?
        .json::<DevinSession>()
        .await
        .map_err(|e| e.to_string())
}
