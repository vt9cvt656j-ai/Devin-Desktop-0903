use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};
use tauri::ipc::Channel;

const STANDARD_RESPONSE_HEADERS_TIMEOUT_SECS: u64 = 20;
const STANDARD_FIRST_STREAM_PROGRESS_TIMEOUT_SECS: u64 = 35;
const STANDARD_EMPTY_STREAM_PROGRESS_TIMEOUT_SECS: u64 = 18;
const STANDARD_STREAM_STALL_TIMEOUT_SECS: u64 = 45;
const HIGH_RESPONSE_HEADERS_TIMEOUT_SECS: u64 = 45;
const HIGH_FIRST_STREAM_PROGRESS_TIMEOUT_SECS: u64 = 90;
const HIGH_EMPTY_STREAM_PROGRESS_TIMEOUT_SECS: u64 = 18;
const HIGH_STREAM_STALL_TIMEOUT_SECS: u64 = 90;
const EXTENDED_RESPONSE_HEADERS_TIMEOUT_SECS: u64 = 90;
const EXTENDED_FIRST_STREAM_PROGRESS_TIMEOUT_SECS: u64 = 180;
const EXTENDED_EMPTY_STREAM_PROGRESS_TIMEOUT_SECS: u64 = 25;
const EXTENDED_STREAM_STALL_TIMEOUT_SECS: u64 = 120;

const RESPONSE_HEADERS_TIMEOUT_ENV: &str = "MICHAEL_AI_RESPONSE_HEADERS_TIMEOUT_SECS";
const FIRST_STREAM_PROGRESS_TIMEOUT_ENV: &str = "MICHAEL_AI_FIRST_PROGRESS_TIMEOUT_SECS";
const EMPTY_STREAM_PROGRESS_TIMEOUT_ENV: &str = "MICHAEL_AI_EMPTY_STREAM_TIMEOUT_SECS";
const STREAM_STALL_TIMEOUT_ENV: &str = "MICHAEL_AI_STREAM_STALL_TIMEOUT_SECS";

const RESPONSE_HEADERS_TIMEOUT_MIN_SECS: u64 = 5;
const RESPONSE_HEADERS_TIMEOUT_MAX_SECS: u64 = 300;
const FIRST_STREAM_PROGRESS_TIMEOUT_MIN_SECS: u64 = 10;
const FIRST_STREAM_PROGRESS_TIMEOUT_MAX_SECS: u64 = 300;
const EMPTY_STREAM_PROGRESS_TIMEOUT_MIN_SECS: u64 = 5;
const EMPTY_STREAM_PROGRESS_TIMEOUT_MAX_SECS: u64 = 120;
const STREAM_STALL_TIMEOUT_MIN_SECS: u64 = 15;
const STREAM_STALL_TIMEOUT_MAX_SECS: u64 = 300;
const STREAM_READ_POLL: Duration = Duration::from_secs(2);
const INCOMPLETE_SSE_STREAM_ERROR: &str = "AI stream closed before data: [DONE]（连接提前结束）；响应可能被截断，已拒绝本轮结果，请重试。";

/// Shared HTTP client. The agentic loop fires many sequential requests; a single
/// pooled client reuses TCP+TLS connections (keep-alive) instead of doing a fresh
/// handshake on every turn — the main source of "backend feels laggy" between
/// turns. No total `.timeout()` is set because chat responses stream open-ended;
/// connection setup is bounded here and each streaming request separately bounds
/// the wait for response headers.
static HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(8)
        .tcp_keepalive(Duration::from_secs(20))
        // The IDE↔LLM-gateway link must NEVER route through the macOS system proxy. Otherwise a
        // capture/MITM proxy the user (or the agent) set up — and left dangling on a dead port —
        // silently kills all AI requests ("无法连接服务器"). Talk to our gateway directly, always.
        .no_proxy()
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});

/// In-flight request cancellation. The JS side passes a unique `request_id` with
/// each streaming chat call and calls `cancel_ai(id)` when the user hits Stop —
/// the streaming loop polls this flag and ends the turn immediately, closing the
/// upstream connection and stopping token generation. (Stop used to only mute the
/// UI while the backend kept generating + billing.)
static CANCELS: LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn register_cancel(id: &str) -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    if let Ok(mut m) = CANCELS.lock() {
        m.insert(id.to_string(), flag.clone());
    }
    flag
}

/// RAII: drop removes the cancel flag from the registry on EVERY exit path of the
/// turn (normal end, error, stall, cancel) — so the map never leaks entries.
struct CancelGuard(String);
impl Drop for CancelGuard {
    fn drop(&mut self) {
        if let Ok(mut m) = CANCELS.lock() {
            m.remove(&self.0);
        }
    }
}

/// Flip the cancel flag for an in-flight request (called from JS on Stop). No-op
/// if the request already finished (its id is no longer registered).
#[tauri::command]
pub fn cancel_ai(request_id: String) {
    if let Ok(m) = CANCELS.lock() {
        if let Some(flag) = m.get(&request_id) {
            flag.store(true, Ordering::SeqCst);
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConfig {
    /// OpenAI-compatible base URL, e.g. `https://api.openai.com/v1`.
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    /// "max" thinking-budget hint (in tokens). Relays that support extended thinking
    /// budgets honor this; others ignore it. Set together with reasoning_effort="high"
    /// when the user picks the "极限/max" tier in the model hover card.
    #[serde(default)]
    pub thinking_budget: Option<u32>,
    /// Unique per-run id from the JS side. `cancel_ai(id)` flips a flag the stream
    /// loop polls, so the user's Stop actually aborts the in-flight upstream request
    /// (frees the connection + stops token burn) instead of only muting the UI.
    #[serde(default)]
    pub request_id: Option<String>,
    /// L0 server-side assembly (anti-reverse), default-off on the JS side. When set,
    /// the client ships the mode NAME + the static tool NAMES instead of the system
    /// prompt and the tool schemas; ai.rs relays them as `x-ide-mode` / `x-ide-tools`
    /// headers and the gateway injects the real prompt + schemas before proxying
    /// upstream — so neither the bundle nor the request carries that IP. Absent →
    /// byte-for-byte unchanged behavior.
    #[serde(default)]
    pub ide_mode: Option<String>,
    #[serde(default)]
    pub ide_tools: Option<String>,
    /// User-local wall-clock context. The IANA name is a label; the bounded
    /// offset is the source of truth for the current instant (including DST).
    #[serde(default)]
    pub ide_timezone: Option<String>,
    #[serde(default)]
    pub ide_utc_offset_minutes: Option<i16>,
}

/// Streamed back to the frontend over a Tauri channel as the model responds.
#[derive(Serialize, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AiEvent {
    Token {
        delta: String,
    },
    /// Streamed model "thinking" (reasoning_content) — shown as a collapsible card.
    Reasoning {
        delta: String,
    },
    /// `index` lets the frontend reassemble a tool call whose `id`/`name` arrive
    /// in the first delta while `arguments` stream across later deltas (the
    /// OpenAI streaming contract). Multiple parallel tool calls are told apart
    /// by their index.
    ToolCall {
        index: u32,
        id: String,
        name: String,
        arguments: String,
    },
    /// Internal timing breadcrumbs for diagnosing "not really streaming" reports.
    /// These are separate from real progress: response headers and raw chunks can
    /// prove the transport is alive without proving the model has emitted usable
    /// reasoning/content/tool arguments yet.
    StreamMetric {
        phase: String,
        elapsed_ms: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        bytes: Option<u64>,
    },
    Done,
    /// Token accounting from the final stream chunk — lets the UI show how much of
    /// the prompt was served from cache (the payoff of the prompt-cache work).
    Usage {
        prompt_tokens: u32,
        completion_tokens: u32,
        cached_tokens: u32,
    },
    Error {
        message: String,
    },
}

/// Call any OpenAI-compatible `/chat/completions` endpoint with streaming.
///
/// Tokens are pushed to `on_event` as they arrive so the UI can render the
/// answer incrementally. Works with OpenAI, Azure/OpenAI-compatible gateways,
/// and local servers such as Ollama (`http://localhost:11434/v1`).
#[tauri::command]
pub async fn ai_chat(
    config: AiConfig,
    messages: Vec<serde_json::Value>,
    on_event: Channel<AiEvent>,
) -> Result<(), String> {
    // `messages` is forwarded verbatim, so a user turn's `content` may be a plain
    // string or a multimodal array (`[{type:"text",...},{type:"image_url",...}]`).
    ai_chat_inner(config, messages, None, on_event).await
}

/// One-shot, non-streaming completion for in-editor AI (the Cmd+K inline editor).
/// Returns the assistant message content as a plain string. `max_tokens` is
/// bounded by the caller so the rewrite stays fast.
#[tauri::command]
pub async fn ai_complete(
    config: AiConfig,
    messages: Vec<serde_json::Value>,
    max_tokens: u32,
) -> Result<String, String> {
    let url = chat_completions_url(&config.base_url)?;
    let payload = serde_json::json!({
        "model": config.model,
        "stream": false,
        "max_tokens": max_tokens,
        "temperature": config.temperature.unwrap_or(0.1),
        "messages": messages,
    });

    let client = &*HTTP;
    let resp = client
        .post(&url)
        .bearer_auth(&config.api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format_ai_http_error(status, &text));
    }

    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let content = v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    Ok(content)
}

/// Tool-enabled chat. `messages` is forwarded verbatim to the provider, so the
/// frontend can send the full OpenAI shape — assistant turns carrying
/// `tool_calls` and `{"role":"tool","tool_call_id":...}` results — which is what
/// makes a real multi-turn agent loop possible.
#[tauri::command]
pub async fn ai_chat_with_tools(
    config: AiConfig,
    messages: Vec<serde_json::Value>,
    tools: Vec<serde_json::Value>,
    on_event: Channel<AiEvent>,
) -> Result<(), String> {
    ai_chat_inner(config, messages, Some(tools), on_event).await
}

/// Tag a message's plain-string `content` with an ephemeral `cache_control`
/// breakpoint (Anthropic prompt caching). No-op unless `content` is a plain
/// string, so already-structured (multimodal / tool_result) messages are left
/// untouched and can never be corrupted.
fn mark_cache_breakpoint(msg: &mut serde_json::Value) {
    if let Some(text) = msg
        .get("content")
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
    {
        msg["content"] = serde_json::json!([
            { "type": "text", "text": text, "cache_control": { "type": "ephemeral" } }
        ]);
    }
}

fn ai_error_detail_from_body(body: &str) -> String {
    let raw = body.trim();
    if raw.is_empty() {
        return "empty response body".into();
    }
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(s) = v.get("error").and_then(|e| e.as_str()) {
            return s.trim().to_string();
        }
        if let Some(s) = v
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
        {
            return s.trim().to_string();
        }
        if let Some(s) = v.get("message").and_then(|m| m.as_str()) {
            return s.trim().to_string();
        }
        if let Some(s) = v.get("detail").and_then(|m| m.as_str()) {
            return s.trim().to_string();
        }
        if let Some(code) = v
            .get("error")
            .and_then(|e| e.get("code"))
            .or_else(|| v.get("code"))
            .and_then(|c| {
                c.as_str()
                    .map(str::to_string)
                    .or_else(|| c.as_i64().map(|n| n.to_string()))
            })
        {
            return format!("error code: {code}");
        }
    }
    raw.to_string()
}

fn format_ai_http_error(status: reqwest::StatusCode, body: &str) -> String {
    format!(
        "AI request failed ({status}): {}",
        ai_error_detail_from_body(body)
    )
}

/// Normalize an OpenAI-compatible chat endpoint. Users usually paste a base URL
/// such as `https://api.openai.com/v1`, but many paste the provider root
/// (`https://api.openai.com`) or a full `/chat/completions` URL. Accept all three
/// shapes so BYOK does not fail for a harmless missing `/v1`.
fn chat_completions_url(base_url: &str) -> Result<String, String> {
    let base = base_url.trim().trim_end_matches('/');
    if !(base.starts_with("http://") || base.starts_with("https://")) {
        return Err("AI base URL must start with http:// or https://".into());
    }
    if base.ends_with("/chat/completions") {
        return Ok(base.to_string());
    }
    let api_base = if base.ends_with("/v1") || base.contains("/v1/") {
        base.to_string()
    } else {
        format!("{base}/v1")
    };
    Ok(format!("{api_base}/chat/completions"))
}

/// Relay the optional L0 server-side-assembly headers. When the JS side set
/// `ideMode`/`ideTools` (the default-off anti-reverse path), the gateway reads these
/// and injects the system prompt + tool schemas itself; when unset, the builder is
/// returned untouched so the request is byte-for-byte identical to before.
fn with_ide_headers(rb: reqwest::RequestBuilder, config: &AiConfig) -> reqwest::RequestBuilder {
    let mut rb = rb;
    if let Some(m) = config.ide_mode.as_deref().filter(|s| !s.is_empty()) {
        rb = rb.header("x-ide-mode", m);
    }
    if let Some(t) = config.ide_tools.as_deref().filter(|s| !s.is_empty()) {
        rb = rb.header("x-ide-tools", t);
    }
    if let Some(zone) = config.ide_timezone.as_deref().filter(|zone| {
        !zone.is_empty()
            && zone.len() <= 64
            && zone.bytes().all(|ch| {
                ch.is_ascii_alphanumeric() || matches!(ch, b'/' | b'_' | b'-' | b'+' | b'.')
            })
    }) {
        rb = rb.header("x-ide-timezone", zone);
    }
    if let Some(offset) = config
        .ide_utc_offset_minutes
        .filter(|offset| (-840..=840).contains(offset))
    {
        rb = rb.header("x-ide-utc-offset-minutes", offset.to_string());
    }
    rb
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StreamTimeouts {
    response_headers: Duration,
    first_progress: Duration,
    empty_stream: Duration,
    stall: Duration,
}

impl StreamTimeouts {
    fn for_config(config: &AiConfig) -> Self {
        Self::for_config_with_env(config, |name| std::env::var(name).ok())
    }

    fn for_config_with_env<F>(config: &AiConfig, read_env: F) -> Self
    where
        F: Fn(&str) -> Option<String>,
    {
        let effort = config
            .reasoning_effort
            .as_deref()
            .unwrap_or_default()
            .trim();
        let has_thinking_budget = config.thinking_budget.is_some_and(|budget| budget > 0);
        let defaults = if has_thinking_budget || effort.eq_ignore_ascii_case("max") {
            (
                EXTENDED_RESPONSE_HEADERS_TIMEOUT_SECS,
                EXTENDED_FIRST_STREAM_PROGRESS_TIMEOUT_SECS,
                EXTENDED_EMPTY_STREAM_PROGRESS_TIMEOUT_SECS,
                EXTENDED_STREAM_STALL_TIMEOUT_SECS,
            )
        } else if effort.eq_ignore_ascii_case("high") {
            (
                HIGH_RESPONSE_HEADERS_TIMEOUT_SECS,
                HIGH_FIRST_STREAM_PROGRESS_TIMEOUT_SECS,
                HIGH_EMPTY_STREAM_PROGRESS_TIMEOUT_SECS,
                HIGH_STREAM_STALL_TIMEOUT_SECS,
            )
        } else {
            (
                STANDARD_RESPONSE_HEADERS_TIMEOUT_SECS,
                STANDARD_FIRST_STREAM_PROGRESS_TIMEOUT_SECS,
                STANDARD_EMPTY_STREAM_PROGRESS_TIMEOUT_SECS,
                STANDARD_STREAM_STALL_TIMEOUT_SECS,
            )
        };

        Self {
            response_headers: bounded_timeout_from_env(
                read_env(RESPONSE_HEADERS_TIMEOUT_ENV),
                defaults.0,
                RESPONSE_HEADERS_TIMEOUT_MIN_SECS,
                RESPONSE_HEADERS_TIMEOUT_MAX_SECS,
            ),
            first_progress: bounded_timeout_from_env(
                read_env(FIRST_STREAM_PROGRESS_TIMEOUT_ENV),
                defaults.1,
                FIRST_STREAM_PROGRESS_TIMEOUT_MIN_SECS,
                FIRST_STREAM_PROGRESS_TIMEOUT_MAX_SECS,
            ),
            empty_stream: bounded_timeout_from_env(
                read_env(EMPTY_STREAM_PROGRESS_TIMEOUT_ENV),
                defaults.2,
                EMPTY_STREAM_PROGRESS_TIMEOUT_MIN_SECS,
                EMPTY_STREAM_PROGRESS_TIMEOUT_MAX_SECS,
            ),
            stall: bounded_timeout_from_env(
                read_env(STREAM_STALL_TIMEOUT_ENV),
                defaults.3,
                STREAM_STALL_TIMEOUT_MIN_SECS,
                STREAM_STALL_TIMEOUT_MAX_SECS,
            ),
        }
    }
}

fn bounded_timeout_from_env(
    raw: Option<String>,
    default_secs: u64,
    min_secs: u64,
    max_secs: u64,
) -> Duration {
    let seconds = raw
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|value| value.clamp(min_secs, max_secs))
        .unwrap_or(default_secs);
    Duration::from_secs(seconds)
}

fn duration_seconds_label(duration: Duration) -> String {
    if duration.subsec_nanos() == 0 {
        return duration.as_secs().to_string();
    }
    let seconds = format!("{:.3}", duration.as_secs_f64());
    seconds
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn response_headers_timeout_error(timeout: Duration) -> String {
    format!(
        "AI request timed out waiting for response headers after {} seconds",
        duration_seconds_label(timeout)
    )
}

async fn send_with_response_headers_timeout(
    request: reqwest::RequestBuilder,
    timeout: Duration,
) -> Result<reqwest::Response, String> {
    match tokio::time::timeout(timeout, request.send()).await {
        Ok(Ok(response)) => Ok(response),
        Ok(Err(error)) => Err(error.to_string()),
        Err(_) => Err(response_headers_timeout_error(timeout)),
    }
}

#[derive(Debug)]
struct StreamProgressDeadline {
    last_progress: Instant,
    first_activity: Option<Instant>,
    has_progress: bool,
    timeouts: StreamTimeouts,
}

impl StreamProgressDeadline {
    fn new(now: Instant, timeouts: StreamTimeouts) -> Self {
        Self {
            last_progress: now,
            first_activity: None,
            has_progress: false,
            timeouts,
        }
    }

    fn record_activity(&mut self, now: Instant) {
        if self.first_activity.is_none() {
            self.first_activity = Some(now);
        }
    }

    fn record(&mut self, now: Instant) {
        self.last_progress = now;
        self.has_progress = true;
    }

    fn record_delta(&mut self, delta: &serde_json::Value, now: Instant) -> bool {
        if !delta_has_real_progress(delta) {
            return false;
        }
        self.record(now);
        true
    }

    fn limit_and_anchor(&self) -> (Duration, Instant) {
        if self.has_progress {
            (self.timeouts.stall, self.last_progress)
        } else if let Some(first_activity) = self.first_activity {
            (self.timeouts.empty_stream, first_activity)
        } else {
            (self.timeouts.first_progress, self.last_progress)
        }
    }

    fn remaining(&self, now: Instant) -> Option<Duration> {
        let (limit, anchor) = self.limit_and_anchor();
        let elapsed = now.duration_since(anchor);
        (elapsed < limit).then(|| limit - elapsed)
    }

    fn error_message(&self) -> String {
        let (limit, _) = self.limit_and_anchor();
        let seconds = duration_seconds_label(limit);
        if self.has_progress {
            format!("模型连续 {seconds} 秒没有继续生成有效内容，已停止本轮，请重试。")
        } else if self.first_activity.is_some() {
            format!("上游已开始流式传输，但 {seconds} 秒内没有生成有效内容，已停止本轮，请重试。")
        } else {
            format!("模型在 {seconds} 秒内没有生成有效内容，已停止本轮，请重试。")
        }
    }
}

fn delta_has_real_progress(delta: &serde_json::Value) -> bool {
    delta["reasoning_content"]
        .as_str()
        .or_else(|| delta["reasoning"].as_str())
        .is_some_and(|text| !text.is_empty())
        || delta["content"]
            .as_str()
            .is_some_and(|text| !text.is_empty())
        || delta["tool_calls"].as_array().is_some_and(|calls| {
            calls.iter().any(|call| {
                call["id"].as_str().is_some_and(|value| !value.is_empty())
                    || call["function"]["name"]
                        .as_str()
                        .is_some_and(|value| !value.is_empty())
                    || call["function"]["arguments"]
                        .as_str()
                        .is_some_and(|value| !value.is_empty())
            })
        })
}

fn streamed_tool_call_index(call: &serde_json::Value) -> Result<u32, String> {
    let raw = call["index"]
        .as_u64()
        .ok_or_else(|| "streamed tool call is missing its numeric index".to_string())?;
    u32::try_from(raw).map_err(|_| "streamed tool call index is out of range".to_string())
}

fn elapsed_ms_since(started: Instant) -> u64 {
    let ms = started.elapsed().as_millis();
    u64::try_from(ms).unwrap_or(u64::MAX)
}

fn send_stream_metric(
    on_event: &Channel<AiEvent>,
    started: Instant,
    phase: &str,
    bytes: Option<u64>,
) {
    let _ = on_event.send(AiEvent::StreamMetric {
        phase: phase.to_string(),
        elapsed_ms: elapsed_ms_since(started),
        bytes,
    });
}

#[cfg(test)]
mod ide_header_tests {
    use super::*;

    fn config() -> AiConfig {
        AiConfig {
            base_url: "https://example.invalid/v1".into(),
            api_key: "test".into(),
            model: "test-model".into(),
            max_tokens: None,
            temperature: None,
            reasoning_effort: None,
            thinking_budget: None,
            request_id: None,
            ide_mode: Some("agent".into()),
            ide_tools: None,
            ide_timezone: Some("America/Los_Angeles".into()),
            ide_utc_offset_minutes: Some(-420),
        }
    }

    #[test]
    fn relays_bounded_user_timezone_headers() {
        let request = with_ide_headers(
            reqwest::Client::new().get("https://example.invalid"),
            &config(),
        )
        .build()
        .unwrap();
        assert_eq!(request.headers()["x-ide-timezone"], "America/Los_Angeles");
        assert_eq!(request.headers()["x-ide-utc-offset-minutes"], "-420");
    }

    #[test]
    fn drops_invalid_timezone_headers() {
        let mut cfg = config();
        cfg.ide_timezone = Some("bad\ntimezone".into());
        cfg.ide_utc_offset_minutes = Some(900);
        let request = with_ide_headers(reqwest::Client::new().get("https://example.invalid"), &cfg)
            .build()
            .unwrap();
        assert!(!request.headers().contains_key("x-ide-timezone"));
        assert!(!request.headers().contains_key("x-ide-utc-offset-minutes"));
    }

    #[test]
    fn normalizes_chat_completion_endpoint_shapes() {
        assert_eq!(
            chat_completions_url("https://api.openai.com").unwrap(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_url("https://api.openai.com/v1").unwrap(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_url("https://gateway.example/v1/chat/completions").unwrap(),
            "https://gateway.example/v1/chat/completions"
        );
        assert!(chat_completions_url("api.openai.com/v1").is_err());
    }

    #[test]
    fn ai_http_error_prefers_gateway_json_error_message() {
        let message = format_ai_http_error(
            reqwest::StatusCode::BAD_GATEWAY,
            r#"{"error":"【claude-opus-4-6】上游暂时不可用，请换个模型或稍后再试。"}"#,
        );
        assert_eq!(
            message,
            "AI request failed (502 Bad Gateway): 【claude-opus-4-6】上游暂时不可用，请换个模型或稍后再试。"
        );
    }

    #[test]
    fn ai_http_error_keeps_provider_code_when_no_message_exists() {
        let message = format_ai_http_error(reqwest::StatusCode::BAD_GATEWAY, r#"{"code":502}"#);
        assert_eq!(
            message,
            "AI request failed (502 Bad Gateway): error code: 502"
        );
    }
}

#[cfg(test)]
mod stream_timeout_tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{Read, Write};

    fn config(reasoning_effort: Option<&str>, thinking_budget: Option<u32>) -> AiConfig {
        AiConfig {
            base_url: "https://example.invalid/v1".into(),
            api_key: "test".into(),
            model: "test-model".into(),
            max_tokens: None,
            temperature: None,
            reasoning_effort: reasoning_effort.map(str::to_string),
            thinking_budget,
            request_id: None,
            ide_mode: None,
            ide_tools: None,
            ide_timezone: None,
            ide_utc_offset_minutes: None,
        }
    }

    fn timeouts(reasoning_effort: Option<&str>, thinking_budget: Option<u32>) -> StreamTimeouts {
        StreamTimeouts::for_config_with_env(&config(reasoning_effort, thinking_budget), |_| None)
    }

    async fn run_raw_sse_body(body: Vec<u8>) -> (Result<(), String>, Vec<serde_json::Value>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0_u8; 16 * 1024];
            let _ = socket.read(&mut request);
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            socket.write_all(headers.as_bytes()).unwrap();
            socket.write_all(&body).unwrap();
            socket.flush().unwrap();
        });

        let events = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
        let captured = events.clone();
        let channel: Channel<AiEvent> = Channel::new(move |body| {
            captured
                .lock()
                .unwrap()
                .push(body.deserialize::<serde_json::Value>().unwrap());
            Ok(())
        });
        let mut cfg = config(None, None);
        cfg.base_url = format!("http://{address}");
        let result = ai_chat_inner(
            cfg,
            vec![serde_json::json!({"role": "user", "content": "write it"})],
            Some(vec![]),
            channel,
        )
        .await;
        server.join().unwrap();
        let captured = events.lock().unwrap().clone();
        (result, captured)
    }

    fn non_metric_kinds(events: &[serde_json::Value]) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| event["kind"].as_str())
            .filter(|kind| *kind != "streamMetric")
            .map(str::to_string)
            .collect()
    }

    fn first_event_of_kind<'a>(
        events: &'a [serde_json::Value],
        kind: &str,
    ) -> &'a serde_json::Value {
        events
            .iter()
            .find(|event| event["kind"] == kind)
            .unwrap_or_else(|| panic!("missing event kind {kind}"))
    }

    #[test]
    fn default_medium_deadlines_stay_fast_and_bounded() {
        let standard = StreamTimeouts {
            response_headers: Duration::from_secs(20),
            first_progress: Duration::from_secs(35),
            empty_stream: Duration::from_secs(18),
            stall: Duration::from_secs(45),
        };
        assert_eq!(timeouts(Some("medium"), None), standard);
        assert_eq!(timeouts(Some("low"), None), standard);
        assert_eq!(timeouts(None, None), standard);
    }

    #[test]
    fn high_max_and_thinking_budget_get_longer_deadlines() {
        assert_eq!(
            timeouts(Some("high"), None),
            StreamTimeouts {
                response_headers: Duration::from_secs(45),
                first_progress: Duration::from_secs(90),
                empty_stream: Duration::from_secs(18),
                stall: Duration::from_secs(90),
            }
        );
        let extended = StreamTimeouts {
            response_headers: Duration::from_secs(90),
            first_progress: Duration::from_secs(180),
            empty_stream: Duration::from_secs(25),
            stall: Duration::from_secs(120),
        };
        assert_eq!(timeouts(Some("max"), None), extended);
        assert_eq!(timeouts(Some("high"), Some(32_000)), extended);
        assert_eq!(timeouts(Some("medium"), Some(1)), extended);
    }

    #[test]
    fn environment_overrides_are_independent_and_clamped() {
        let values = HashMap::from([
            (RESPONSE_HEADERS_TIMEOUT_ENV, "1".to_string()),
            (FIRST_STREAM_PROGRESS_TIMEOUT_ENV, "9999".to_string()),
            (EMPTY_STREAM_PROGRESS_TIMEOUT_ENV, "3".to_string()),
            (STREAM_STALL_TIMEOUT_ENV, "61".to_string()),
        ]);
        let overridden = StreamTimeouts::for_config_with_env(&config(Some("high"), None), |name| {
            values.get(name).cloned()
        });
        assert_eq!(
            overridden,
            StreamTimeouts {
                response_headers: Duration::from_secs(5),
                first_progress: Duration::from_secs(300),
                empty_stream: Duration::from_secs(5),
                stall: Duration::from_secs(61),
            }
        );

        let invalid = HashMap::from([
            (RESPONSE_HEADERS_TIMEOUT_ENV, "not-a-number".to_string()),
            (FIRST_STREAM_PROGRESS_TIMEOUT_ENV, "".to_string()),
            (
                EMPTY_STREAM_PROGRESS_TIMEOUT_ENV,
                "not-a-number".to_string(),
            ),
            (STREAM_STALL_TIMEOUT_ENV, "-10".to_string()),
        ]);
        assert_eq!(
            StreamTimeouts::for_config_with_env(&config(Some("medium"), None), |name| {
                invalid.get(name).cloned()
            }),
            timeouts(Some("medium"), None),
            "invalid overrides must preserve the selected profile defaults"
        );
    }

    #[test]
    fn uses_separate_first_progress_and_stall_deadlines() {
        let timeouts = timeouts(Some("medium"), None);

        let started = Instant::now();
        let mut progress = StreamProgressDeadline::new(started, timeouts);
        assert_eq!(
            progress.remaining(started + Duration::from_secs(34)),
            Some(Duration::from_secs(1))
        );
        assert_eq!(progress.remaining(started + timeouts.first_progress), None);

        progress.record(started + Duration::from_secs(20));
        assert_eq!(
            progress.remaining(started + Duration::from_secs(64)),
            Some(Duration::from_secs(1))
        );
        assert_eq!(progress.remaining(started + Duration::from_secs(65)), None);
    }

    #[test]
    fn raw_stream_activity_without_real_delta_uses_short_deadline() {
        let timeouts = timeouts(Some("high"), None);
        let started = Instant::now();
        let mut progress = StreamProgressDeadline::new(started, timeouts);

        assert_eq!(
            progress.remaining(started + Duration::from_secs(40)),
            Some(Duration::from_secs(50)),
            "before any bytes arrive, high reasoning may still wait for first progress"
        );

        let first_raw_chunk = started + Duration::from_secs(2);
        progress.record_activity(first_raw_chunk);
        assert_eq!(
            progress.remaining(first_raw_chunk + timeouts.empty_stream - Duration::from_secs(1)),
            Some(Duration::from_secs(1)),
            "once raw bytes arrive, empty streams must stop quickly instead of using the high first-progress window"
        );
        assert_eq!(
            progress.remaining(first_raw_chunk + timeouts.empty_stream),
            None
        );

        progress.record(first_raw_chunk + Duration::from_secs(4));
        assert_eq!(
            progress.remaining(first_raw_chunk + Duration::from_secs(4) + timeouts.stall),
            None,
            "real progress switches back to the normal stall deadline"
        );
    }

    #[test]
    fn only_non_empty_model_deltas_reset_progress() {
        let timeouts = timeouts(Some("medium"), None);
        let started = Instant::now();
        let mut progress = StreamProgressDeadline::new(started, timeouts);

        for non_progress in [
            serde_json::json!({}),
            serde_json::json!({"role": "assistant"}),
            serde_json::json!({"content": ""}),
            serde_json::json!({"reasoning_content": ""}),
            serde_json::json!({"tool_calls": [{"index": 0, "function": {"arguments": ""}}]}),
        ] {
            assert!(!progress.record_delta(&non_progress, started + Duration::from_secs(24)));
        }
        assert_eq!(
            progress.remaining(started + timeouts.first_progress),
            None,
            "heartbeats, role-only events, and empty deltas must not extend the deadline"
        );

        for real_progress in [
            serde_json::json!({"reasoning_content": "thinking"}),
            serde_json::json!({"content": "token"}),
            serde_json::json!({"tool_calls": [{"index": 0, "function": {"arguments": "{"}}]}),
        ] {
            let mut candidate = StreamProgressDeadline::new(started, timeouts);
            assert!(candidate.record_delta(&real_progress, started + Duration::from_secs(1)));
            assert!(candidate.has_progress);
        }
    }

    #[test]
    fn streamed_tool_calls_require_an_explicit_index() {
        assert_eq!(
            streamed_tool_call_index(&serde_json::json!({"index": 3})).unwrap(),
            3
        );
        assert!(streamed_tool_call_index(&serde_json::json!({})).is_err());
        assert!(streamed_tool_call_index(&serde_json::json!({"index": "0"})).is_err());
    }

    #[test]
    fn timeout_errors_report_the_configured_duration() {
        let timeouts = StreamTimeouts {
            response_headers: Duration::from_secs(73),
            first_progress: Duration::from_secs(81),
            empty_stream: Duration::from_secs(17),
            stall: Duration::from_secs(97),
        };
        assert_eq!(
            response_headers_timeout_error(timeouts.response_headers),
            "AI request timed out waiting for response headers after 73 seconds"
        );

        let started = Instant::now();
        let mut progress = StreamProgressDeadline::new(started, timeouts);
        assert_eq!(
            progress.error_message(),
            "模型在 81 秒内没有生成有效内容，已停止本轮，请重试。"
        );
        progress.record_activity(started + Duration::from_secs(1));
        assert_eq!(
            progress.error_message(),
            "上游已开始流式传输，但 17 秒内没有生成有效内容，已停止本轮，请重试。"
        );
        progress.record(started + Duration::from_secs(1));
        assert_eq!(
            progress.error_message(),
            "模型连续 97 秒没有继续生成有效内容，已停止本轮，请重试。"
        );
    }

    #[tokio::test]
    async fn response_header_wait_is_bounded() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (_socket, _) = listener.accept().unwrap();
            std::thread::sleep(Duration::from_millis(100));
        });

        let error = send_with_response_headers_timeout(
            reqwest::Client::new().get(format!("http://{address}/chat/completions")),
            Duration::from_millis(20),
        )
        .await
        .unwrap_err();
        assert_eq!(
            error,
            "AI request timed out waiting for response headers after 0.02 seconds"
        );
        server.join().unwrap();
    }

    #[tokio::test]
    async fn clean_eof_without_done_rejects_partial_tool_arguments() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let partial_args = r#"{"path":"src/main.js","content":"partial"#;
        let data = serde_json::json!({
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_partial",
                        "function": {
                            "name": "write_file",
                            "arguments": partial_args,
                        }
                    }]
                }
            }]
        });
        let body = format!("data: {data}\n\n");
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0_u8; 16 * 1024];
            let _ = socket.read(&mut request);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).unwrap();
            socket.flush().unwrap();
        });

        let events = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
        let captured = events.clone();
        let channel: Channel<AiEvent> = Channel::new(move |body| {
            captured
                .lock()
                .unwrap()
                .push(body.deserialize::<serde_json::Value>().unwrap());
            Ok(())
        });
        let mut cfg = config(None, None);
        cfg.base_url = format!("http://{address}");

        let error = ai_chat_inner(
            cfg,
            vec![serde_json::json!({"role": "user", "content": "write it"})],
            Some(vec![]),
            channel,
        )
        .await
        .unwrap_err();
        server.join().unwrap();

        assert_eq!(error, INCOMPLETE_SSE_STREAM_ERROR);
        let events = events.lock().unwrap();
        let kinds = non_metric_kinds(&events);
        assert_eq!(kinds, ["toolCall", "error"]);
        assert_eq!(
            first_event_of_kind(&events, "toolCall")["arguments"],
            partial_args
        );
        assert_eq!(
            first_event_of_kind(&events, "error")["message"],
            INCOMPLETE_SSE_STREAM_ERROR
        );
        assert!(!events.iter().any(|event| event["kind"] == "done"));
    }

    #[tokio::test]
    async fn malformed_json_before_done_rejects_complete_tool_argument_prefix() {
        let arguments = r#"{"path":"src/main.js","content":"prefix"}"#;
        let first = serde_json::json!({
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_prefix",
                        "function": {"name": "write_file", "arguments": arguments}
                    }]
                }
            }]
        });
        let body = format!("data: {first}\n\ndata: {{malformed\n\ndata: [DONE]\n\n").into_bytes();

        let (result, events) = run_raw_sse_body(body).await;

        let error = result.unwrap_err();
        assert!(error.contains("malformed SSE JSON"));
        let kinds = non_metric_kinds(&events);
        assert_eq!(kinds, ["toolCall", "error"]);
        assert_eq!(
            first_event_of_kind(&events, "toolCall")["arguments"],
            arguments
        );
        assert!(!events.iter().any(|event| event["kind"] == "done"));
    }

    #[tokio::test]
    async fn invalid_utf8_frame_before_done_rejects_the_stream() {
        let mut body = b"data: {\"choices\":[{\"delta\":{\"content\":\"".to_vec();
        body.push(0xff);
        body.extend_from_slice(b"\"}}]}\n\ndata: [DONE]\n\n");

        let (result, events) = run_raw_sse_body(body).await;

        let error = result.unwrap_err();
        assert!(error.contains("invalid UTF-8 SSE data"));
        let kinds = non_metric_kinds(&events);
        assert_eq!(kinds, ["error"]);
        assert!(!events.iter().any(|event| event["kind"] == "done"));
    }
}

async fn ai_chat_inner(
    config: AiConfig,
    messages: Vec<serde_json::Value>,
    tools: Option<Vec<serde_json::Value>>,
    on_event: Channel<AiEvent>,
) -> Result<(), String> {
    let timeouts = StreamTimeouts::for_config(&config);
    let url = chat_completions_url(&config.base_url)?;
    let stream_started = Instant::now();
    send_stream_metric(&on_event, stream_started, "requestStarted", None);
    let mut payload = serde_json::json!({
        "model": config.model,
        "stream": true,
        "messages": messages,
        // Ask the provider to emit a final `usage` chunk (incl. cached-prompt
        // tokens) during streaming — without this most OpenAI-compatible providers
        // send NO usage, so the cache meter has nothing to show ("缓存看不到").
        "stream_options": { "include_usage": true },
    });
    if let Some(max_tokens) = config.max_tokens {
        payload["max_tokens"] = serde_json::json!(max_tokens);
    }
    if let Some(temp) = config.temperature {
        payload["temperature"] = serde_json::json!(temp);
    }
    if let Some(ref t) = tools {
        if !t.is_empty() {
            payload["tools"] = serde_json::json!(t);
        }
    }
    if let Some(ref effort) = config.reasoning_effort {
        if !effort.is_empty() {
            payload["reasoning_effort"] = serde_json::json!(effort);
        }
    }
    if let Some(budget) = config.thinking_budget {
        if budget > 0 {
            payload["thinking_budget"] = serde_json::json!(budget);
            // Anthropic-native shape for relays that route via /v1/messages.
            payload["thinking"] = serde_json::json!({"type": "enabled", "budget_tokens": budget});
        }
    }

    // ── Prompt caching for Anthropic / Claude upstreams ─────────────────────
    // Anthropic reuses content marked with a `cache_control` breakpoint and serves
    // the LONGEST matching cached prefix (up to 4 breakpoints). We mark (1) the
    // system prompt — kept byte-stable in main.js so it caches across every turn —
    // and (2) a ROLLING TAIL breakpoint on the last up-to-2 plain-string messages
    // (ANY role — crucially incl. tool results, where a long agentic turn's history
    // piles up AFTER the last user message and would otherwise re-bill every step).
    //
    // Gated on the MODEL name (the gateway routes by model — base_url is always the
    // gateway, so the old base_url check never fired). Only PLAIN-STRING content is
    // reshaped, so tool/multimodal messages — and any non-Anthropic upstream — are
    // left byte-for-byte untouched and can't break. DeepSeek / OpenAI / Kimi cache
    // the stable prefix automatically, so they need no markup at all.
    // ⚠️ DISABLED on the current relay (zyz / "AWS渠道"): sending cache_control there
    // triggers cache CREATION (a 1.25× write premium) on the ~34K fixed prefix on
    // EVERY call but NEVER returns a cache READ — so each call cost ~25% MORE than no
    // caching at all (observed: one streaming call billed $0.65, ≈ 34.1K × $15/M × 1.25).
    // Re-enable ONLY against an upstream that actually SERVES cache reads (Anthropic /
    // Bedrock direct, or a caching-aware proxy like LiteLLM) — flip the flag to true.
    const ENABLE_PROMPT_CACHE: bool = false;
    let model_lc = config.model.to_ascii_lowercase();
    if ENABLE_PROMPT_CACHE
        && (config.base_url.contains("anthropic")
            || model_lc.contains("claude")
            || model_lc.contains("anthropic"))
    {
        if let Some(arr) = payload["messages"].as_array_mut() {
            // (1) System prompt — byte-stable across turns, caches once.
            if let Some(first) = arr
                .iter_mut()
                .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("system"))
            {
                mark_cache_breakpoint(first);
            }
            // (2) Rolling tail: walk from the end, mark the last up-to-2 messages
            // that carry plain-string, NON-EMPTY content (any role). Anthropic
            // serves the longest matching cached prefix, so marking the tail turns
            // the whole prior conversation — including the accumulated tool history
            // — into a cache hit next turn. `mark_cache_breakpoint` reshapes only
            // plain strings, so multimodal (screenshot) and empty-content messages
            // are skipped and can never be corrupted. ≤3 breakpoints total (budget 4).
            let mut marked = 0;
            for m in arr.iter_mut().rev() {
                if marked >= 2 {
                    break;
                }
                let has_text = m
                    .get("content")
                    .and_then(|c| c.as_str())
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(false);
                if has_text {
                    mark_cache_breakpoint(m);
                    marked += 1;
                }
            }
        }
    }

    let client = &*HTTP;
    let mut resp = send_with_response_headers_timeout(
        with_ide_headers(client.post(&url).bearer_auth(&config.api_key), &config).json(&payload),
        timeouts.response_headers,
    )
    .await?;
    // If a strict gateway rejects the request (4xx — most likely the optional
    // `stream_options` it doesn't recognize), drop that field and retry ONCE. So
    // asking for usage stats can never break chat.
    if resp.status().is_client_error() && payload.get("stream_options").is_some() {
        if let Some(o) = payload.as_object_mut() {
            o.remove("stream_options");
        }
        resp = send_with_response_headers_timeout(
            with_ide_headers(client.post(&url).bearer_auth(&config.api_key), &config)
                .json(&payload),
            timeouts.response_headers,
        )
        .await?;
    }

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let message = format_ai_http_error(status, &text);
        let _ = on_event.send(AiEvent::Error {
            message: message.clone(),
        });
        return Err(message);
    }
    send_stream_metric(&on_event, stream_started, "responseHeaders", None);

    let mut stream = resp.bytes_stream();
    // Accumulate RAW BYTES, not lossily-decoded strings: a multibyte UTF-8 char
    // (e.g. a Chinese character) can be split across two network chunks, and
    // decoding each chunk on its own would turn the split char into a `�`
    // replacement char — visible mojibake in the streamed answer. SSE lines are
    // delimited by '\n' (0x0A), a byte that never appears inside a multibyte
    // UTF-8 sequence, so splitting on the byte and decoding each *complete* line
    // is always valid.
    let mut buf: Vec<u8> = Vec::new();
    // A transport heartbeat is not model progress. The first non-empty
    // reasoning/token/tool-call delta must arrive promptly; after that, each real
    // delta gets a longer stall window. SSE comments, empty deltas, usage-only
    // chunks and arbitrary response bytes never extend either deadline.
    let mut progress = StreamProgressDeadline::new(Instant::now(), timeouts);
    let mut raw_stream_bytes: u64 = 0;
    let mut sent_first_chunk_metric = false;
    let mut sent_first_progress_metric = false;
    // Cancellation: register a flag keyed by the JS-supplied request_id (if any).
    // The guard removes it on every return path; the loop polls it so Stop aborts.
    let req_id = config.request_id.clone().filter(|s| !s.is_empty());
    let cancel_flag = req_id.as_deref().map(register_cancel);
    let _cancel_guard = req_id.map(CancelGuard);
    loop {
        // User hit Stop → cancel_ai flipped this flag: end the turn now so the
        // upstream connection closes and token generation stops
        // (at most STREAM_READ_POLL latency).
        if let Some(f) = &cancel_flag {
            if f.load(Ordering::SeqCst) {
                let _ = on_event.send(AiEvent::Done);
                return Ok(());
            }
        }
        let Some(remaining) = progress.remaining(Instant::now()) else {
            let _ = on_event.send(AiEvent::Error {
                message: progress.error_message(),
            });
            let _ = on_event.send(AiEvent::Done);
            return Ok(());
        };
        let chunk = match tokio::time::timeout(remaining.min(STREAM_READ_POLL), stream.next()).await
        {
            Ok(Some(Ok(c))) => c,
            // A mid-stream read error means the connection dropped partway (common
            // on cross-border / lossy links — "error decoding response body"). Keep
            // what we've already streamed and end the turn gracefully.
            Ok(Some(Err(_e))) => {
                let _ = on_event.send(AiEvent::Error {
                    message: "连接中断（网络波动），已保留生成的部分，正在自动恢复。".to_string(),
                });
                let _ = on_event.send(AiEvent::Done);
                return Ok(());
            }
            Ok(None) => {
                // OpenAI-compatible SSE is complete only after the explicit [DONE]
                // sentinel. A clean TCP EOF can still be a proxy/upstream truncation;
                // treating it as Done would authorize partially streamed tool arguments.
                let message = INCOMPLETE_SSE_STREAM_ERROR.to_string();
                let _ = on_event.send(AiEvent::Error {
                    message: message.clone(),
                });
                return Err(message);
            }
            Err(_elapsed) => {
                if progress.remaining(Instant::now()).is_none() {
                    let _ = on_event.send(AiEvent::Error {
                        message: progress.error_message(),
                    });
                    let _ = on_event.send(AiEvent::Done);
                    return Ok(());
                }
                continue;
            }
        };
        progress.record_activity(Instant::now());
        raw_stream_bytes = raw_stream_bytes.saturating_add(chunk.len() as u64);
        if !sent_first_chunk_metric {
            sent_first_chunk_metric = true;
            send_stream_metric(
                &on_event,
                stream_started,
                "firstChunk",
                Some(raw_stream_bytes),
            );
        }
        buf.extend_from_slice(&chunk);

        // Server-sent events are newline-delimited `data: {...}` lines.
        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            let line_bytes: Vec<u8> = buf.drain(..=pos).collect();
            let line = match std::str::from_utf8(&line_bytes) {
                Ok(line) => line.trim(),
                Err(error) => {
                    let message = format!("AI stream contains invalid UTF-8 SSE data: {error}");
                    let _ = on_event.send(AiEvent::Error {
                        message: message.clone(),
                    });
                    return Err(message);
                }
            };
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data == "[DONE]" {
                send_stream_metric(&on_event, stream_started, "done", Some(raw_stream_bytes));
                let _ = on_event.send(AiEvent::Done);
                return Ok(());
            }
            if data.is_empty() {
                continue;
            }
            let v = match serde_json::from_str::<serde_json::Value>(data) {
                Ok(value) => value,
                Err(error) => {
                    let message = format!("AI stream contains malformed SSE JSON: {error}");
                    let _ = on_event.send(AiEvent::Error {
                        message: message.clone(),
                    });
                    return Err(message);
                }
            };
            // Usage normally rides the FINAL chunk (choices may be empty there).
            // Cached-prompt tokens are reported differently per provider — take
            // whichever field is present: OpenAI/DeepSeek `prompt_tokens_details
            // .cached_tokens`, DeepSeek `prompt_cache_hit_tokens`, or Anthropic-
            // style `cache_read_input_tokens`.
            if let Some(usage) = v.get("usage").filter(|u| u.is_object()) {
                let completion = usage["completion_tokens"]
                    .as_u64()
                    .or_else(|| usage["output_tokens"].as_u64()) // Anthropic
                    .unwrap_or(0);
                let cache_read = usage["cache_read_input_tokens"].as_u64(); // Anthropic
                let cache_creation = usage["cache_creation_input_tokens"].as_u64().unwrap_or(0);
                let cached = usage["prompt_tokens_details"]["cached_tokens"]
                    .as_u64()
                    .or_else(|| usage["prompt_cache_hit_tokens"].as_u64()) // DeepSeek
                    .or_else(|| usage["cached_content_token_count"].as_u64()) // Gemini (OpenAI-compat)
                    .or_else(|| usage["cachedContentTokenCount"].as_u64()) // Gemini (native)
                    .or(cache_read)
                    .unwrap_or(0);
                let prompt_raw = usage["prompt_tokens"]
                    .as_u64()
                    .or_else(|| usage["input_tokens"].as_u64()) // Anthropic
                    .unwrap_or(0);
                // OpenAI/DeepSeek: `prompt_tokens` already INCLUDES cached tokens.
                // Anthropic: `input_tokens` EXCLUDES cached (reported separately),
                // so add them — otherwise `cached / prompt` reads as >100%.
                let prompt = if cache_read.is_some() {
                    prompt_raw + cached + cache_creation
                } else {
                    prompt_raw
                };
                if prompt > 0 || completion > 0 {
                    let _ = on_event.send(AiEvent::Usage {
                        prompt_tokens: prompt as u32,
                        completion_tokens: completion as u32,
                        cached_tokens: cached as u32,
                    });
                }
            }
            let delta = &v["choices"][0]["delta"];
            if progress.record_delta(delta, Instant::now()) && !sent_first_progress_metric {
                sent_first_progress_metric = true;
                send_stream_metric(
                    &on_event,
                    stream_started,
                    "firstProgress",
                    Some(raw_stream_bytes),
                );
            }
            // Thinking / reasoning stream (DeepSeek/MiniMax: reasoning_content; some: reasoning).
            if let Some(rt) = delta["reasoning_content"]
                .as_str()
                .or_else(|| delta["reasoning"].as_str())
            {
                if !rt.is_empty() {
                    let _ = on_event.send(AiEvent::Reasoning {
                        delta: rt.to_string(),
                    });
                }
            }
            if let Some(text) = delta["content"].as_str() {
                if !text.is_empty() {
                    let _ = on_event.send(AiEvent::Token {
                        delta: text.to_string(),
                    });
                }
            }
            if let Some(tcs) = delta["tool_calls"].as_array() {
                for tc in tcs {
                    let index = match streamed_tool_call_index(tc) {
                        Ok(index) => index,
                        Err(message) => {
                            let _ = on_event.send(AiEvent::Error { message });
                            let _ = on_event.send(AiEvent::Done);
                            return Ok(());
                        }
                    };
                    let id = tc["id"].as_str().unwrap_or("").to_string();
                    let name = tc["function"]["name"].as_str().unwrap_or("").to_string();
                    let args = tc["function"]["arguments"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    if !id.is_empty() || !name.is_empty() || !args.is_empty() {
                        let _ = on_event.send(AiEvent::ToolCall {
                            index,
                            id,
                            name,
                            arguments: args,
                        });
                    }
                }
            }
        }

        // Continuous heartbeat chunks may prevent the read timeout from firing, so
        // enforce the same real-progress deadline after every parsed network chunk.
        if progress.remaining(Instant::now()).is_none() {
            let _ = on_event.send(AiEvent::Error {
                message: progress.error_message(),
            });
            let _ = on_event.send(AiEvent::Done);
            return Ok(());
        }
    }
}

/// SSRF policy for this single-user, on-device dev IDE. The user explicitly wants
/// to fetch their own intranet / localhost dev servers, so loopback and private
/// LAN are ALLOWED (matches the `net.rs` http_request tool). What stays blocked is
/// what's never a legitimate fetch target and is the real danger: link-local
/// (169.254/16 and fe80::/10 — i.e. the cloud-metadata 169.254.169.254 endpoint),
/// plus unspecified / multicast / broadcast / documentation junk.
fn ip_fetch_allowed(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            let o = v4.octets();
            !(v4.is_link_local()        // 169.254/16 — blocks cloud metadata
                || v4.is_broadcast()
                || v4.is_unspecified()
                || v4.is_documentation()
                || v4.is_multicast()
                || o[0] == 0)
        }
        std::net::IpAddr::V6(v6) => {
            let s = v6.segments();
            !(v6.is_unspecified()
                || v6.is_multicast()
                // link-local fe80::/10
                || (s[0] & 0xffc0) == 0xfe80)
        }
    }
}

fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}

/// Best-effort HTML → readable text: drop `<script>`/`<style>` blocks and tags,
/// decode a few common entities, collapse whitespace.
fn html_to_text(html: &str) -> String {
    let lower = html.to_ascii_lowercase(); // ASCII-only fold preserves byte length & boundaries
    let bytes = html.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n / 2);
    let mut i = 0usize;
    while i < n {
        if bytes[i] == b'<' {
            if lower[i..].starts_with("<script") {
                match lower[i..].find("</script>") {
                    Some(rel) => {
                        i += rel + 9;
                    }
                    None => break,
                }
                out.push(' ');
                continue;
            }
            if lower[i..].starts_with("<style") {
                match lower[i..].find("</style>") {
                    Some(rel) => {
                        i += rel + 8;
                    }
                    None => break,
                }
                out.push(' ');
                continue;
            }
            match html[i..].find('>') {
                Some(rel) => {
                    i += rel + 1;
                }
                None => break,
            }
            out.push(' ');
            continue;
        }
        let l = utf8_len(bytes[i]).min(n - i);
        out.push_str(&html[i..i + l]);
        i += l;
    }
    let out = out
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Fetch a public web page for the agent. Guards against SSRF (public IPs only,
/// no redirects), bounds time/size, and returns readable text.
#[tauri::command]
pub async fn web_fetch(url: String) -> Result<String, String> {
    use std::net::ToSocketAddrs;

    let parsed = reqwest::Url::parse(url.trim()).map_err(|_| "无效的 URL".to_string())?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return Err("只允许 http/https 链接".into()),
    }
    let host = parsed.host_str().ok_or("URL 缺少主机名")?.to_string();
    let port = parsed.port_or_known_default().unwrap_or(80);

    // Resolve and require every resolved address to be public (SSRF guard). getaddrinfo is
    // BLOCKING — on Windows a degraded network makes it serially probe DNS→LLMNR→NetBIOS and
    // block 10-30s+, which (run directly on a Tokio worker) freezes that thread and can starve
    // the whole IPC/UI ("联网搜索卡死"). Run it on a blocking thread with a hard 5s cap so a
    // wedged resolver can never freeze the app.
    let host_c = host.clone();
    let addrs: Vec<std::net::SocketAddr> = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tokio::task::spawn_blocking(move || {
            (host_c.as_str(), port)
                .to_socket_addrs()
                .map(|it| it.collect::<Vec<_>>())
        }),
    )
    .await
    .map_err(|_| "DNS 解析超时（网络异常，请重试）".to_string())?
    .map_err(|_| "DNS 解析任务失败".to_string())?
    .map_err(|e| format!("DNS 解析失败: {e}"))?;
    if addrs.is_empty() {
        return Err("无法解析主机".into());
    }
    for a in &addrs {
        if !ip_fetch_allowed(a.ip()) {
            return Err("拒绝访问该地址（link-local / 元数据 / 多播等）".into());
        }
    }

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .connect_timeout(std::time::Duration::from_secs(6))
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(parsed)
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36")
        .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
        .header(reqwest::header::ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9,en;q=0.8")
        .header(reqwest::header::ACCEPT_ENCODING, "gzip, deflate, br")
        .header("Sec-Fetch-Dest", "document")
        .header("Sec-Fetch-Mode", "navigate")
        .header("Sec-Fetch-Site", "none")
        .header("Sec-Fetch-User", "?1")
        .header("Upgrade-Insecure-Requests", "1")
        .header("Cache-Control", "max-age=0")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    let code = status.as_u16();

    // Anti-bot wall (403/503/429): fall back to headless Chrome which has a real
    // TLS fingerprint and runs JS — bypasses Dianping/Meituan/etc. cookie walls.
    if code == 403 || code == 503 || code == 429 {
        if let Some(browser) = crate::capture::find_headless_browser() {
            let url_str = url.trim().to_string();
            let rendered = tokio::time::timeout(
                std::time::Duration::from_secs(16),
                tauri::async_runtime::spawn_blocking(move || render_dom(&browser, &url_str)),
            )
            .await;
            if let Ok(Ok(Some(html))) = rendered {
                let text = html_to_text(&html);
                if text.len() > 100 {
                    return Ok(text.chars().take(24_000).collect());
                }
            }
        }
        return Err(format!(
            "HTTP {} (反爬拦截，无头浏览器也未能获取内容)",
            code
        ));
    }

    if !status.is_success() {
        return Err(format!("HTTP {}", code));
    }

    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    let max = 800_000usize;
    let slice = &bytes[..bytes.len().min(max)];
    let raw = String::from_utf8_lossy(slice).to_string();

    let text = if ct.contains("html") || raw.trim_start().starts_with('<') {
        html_to_text(&raw)
    } else {
        raw
    };
    Ok(text.chars().take(24_000).collect())
}

fn percent_decode_str(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (
                (b[i + 1] as char).to_digit(16),
                (b[i + 2] as char).to_digit(16),
            ) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// DuckDuckGo wraps result links as `//duckduckgo.com/l/?uddg=<encoded>&...`.
fn ddg_unwrap(href: &str) -> String {
    if let Some(p) = href.find("uddg=") {
        let enc = href[p + 5..].split('&').next().unwrap_or("");
        percent_decode_str(enc)
    } else if href.starts_with("http") {
        href.to_string()
    } else {
        String::new()
    }
}

fn parse_ddg_results(html: &str) -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> = Vec::new();
    let mut rest = html;
    while let Some(pos) = rest.find("result__a") {
        rest = &rest[pos + 9..];
        let href = rest
            .find("href=\"")
            .and_then(|h| {
                let s = &rest[h + 6..];
                s.find('"').map(|e| s[..e].to_string())
            })
            .unwrap_or_default();
        let url = ddg_unwrap(&href);
        let title = rest
            .find('>')
            .and_then(|g| {
                let s = &rest[g + 1..];
                s.find("</a>").map(|e| html_to_text(&s[..e]))
            })
            .unwrap_or_default();
        let snippet = rest
            .find("result__snippet")
            .and_then(|sp| {
                let s = &rest[sp..];
                s.find('>').and_then(|g| {
                    let s2 = &s[g + 1..];
                    s2.find("</a>")
                        .or_else(|| s2.find("</div>"))
                        .map(|e| html_to_text(&s2[..e]))
                })
            })
            .unwrap_or_default();
        if !url.is_empty() && !title.is_empty() && !out.iter().any(|(_, u, _)| u == &url) {
            out.push((title, url, snippet));
        }
        if out.len() >= 10 {
            break;
        }
    }
    out
}

/// Web search (Google, Bing, and DuckDuckGo scraping, no API key) so the agent can FIND docs/articles,
/// then `web_fetch` the ones it wants. Returns title + real URL + snippet.
#[tauri::command]
pub async fn web_search(query: String) -> Result<String, String> {
    let q = query.trim();
    if q.is_empty() {
        return Err("空搜索词".into());
    }
    // The old code hit ONE DuckDuckGo endpoint with no fallback — it gets rate-
    // limited / blocked a lot, which is exactly why search "搜不到". Now we try the
    // html endpoint then fall back to the lite endpoint (lighter, far less blocked),
    // each with a rotated UA, returning the first non-empty result set.
    // Hard OVERALL deadline: a wedged network or a run of blocked engines must never hang the
    // agent turn. The fast scrape race (~8s cap) → headless-browser fallback all live inside 30s;
    // if even that blows through, return "no results" so the agent gets a graceful message, not a
    // frozen tool call.
    let results = tokio::time::timeout(Duration::from_secs(30), async {
        let mut r = ddg_search_multi(q).await;
        if r.is_empty() {
            // The bare HTTP scrape got blocked (anti-bot walls fingerprint reqwest's TLS / reject
            // its UA). Fall back to a REAL headless-browser render — the agent's own Chrome does the
            // search (real TLS fingerprint + JS + UA), getting through where the scrape can't.
            r = browser_render_search(q).await;
        }
        r
    })
    .await
    .unwrap_or_default();
    if results.is_empty() {
        return Ok(format!(
            "「{q}」这次没搜到结果（搜索引擎可能限流、反爬，或当前关键词没有索引结果）。不要原样重发或只换近义词反复搜索：已有明确官方 URL 就直接 web_fetch；只有出现新的具体假设时才换一次真正不同的来源或检索方式。仍无新增证据就停止，并如实说明这次没有检索到可验证结果。"
        ));
    }
    let mut out = format!("搜索「{q}」的结果（Google+Bing+DuckDuckGo 实际响应合并去重）：\n");
    for (i, (title, url, snippet)) in results.iter().take(12).enumerate() {
        out.push_str(&format!(
            "\n{}. {}\n   {}\n   {}\n",
            i + 1,
            title,
            url,
            snippet.chars().take(240).collect::<String>()
        ));
    }
    out.push_str("\n（用 web_fetch 打开上面任意 URL 读全文）");
    Ok(out)
}

/// Run ALL search engines concurrently, MERGE and deduplicate results.
/// Google goes first in merge order, followed by Bing, DuckDuckGo HTML, then DDG Lite.
/// Each engine has its own 8s timeout so the
/// total wall-clock is max ~8s (all run in parallel).
async fn ddg_search_multi(q: &str) -> Vec<(String, String, String)> {
    let (google, bing, ddg, lite) = tokio::join!(
        scrape_google(q.to_string()),
        scrape_bing(q.to_string()),
        scrape_ddg_html(q.to_string()),
        scrape_ddg_lite(q.to_string()),
    );
    let mut seen = std::collections::HashSet::new();
    let mut merged = Vec::new();
    for r in google.into_iter().chain(bing).chain(ddg).chain(lite) {
        if seen.insert(r.1.clone()) {
            merged.push(r);
        }
    }
    merged
}

const SEARCH_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

/// A search should be fast; bound it much tighter than a page fetch. A source that hasn't
/// answered in 8s is treated as blocked/dead so the race can settle on a working one.
fn search_client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(8))
        .build()
        .ok()
}

async fn scrape_ddg_html(q: String) -> Vec<(String, String, String)> {
    let client = match search_client() {
        Some(c) => c,
        None => return Vec::new(),
    };
    let resp = client
        .post("https://html.duckduckgo.com/html/")
        .header(reqwest::header::USER_AGENT, SEARCH_UA)
        .form(&[("q", q.as_str()), ("kl", "wt-wt")])
        .send()
        .await;
    match resp {
        Ok(r) => r
            .text()
            .await
            .map(|h| parse_ddg_results(&h))
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

async fn scrape_bing(q: String) -> Vec<(String, String, String)> {
    let client = match search_client() {
        Some(c) => c,
        None => return Vec::new(),
    };
    let has_cjk = q.chars().any(|c| {
        ('\u{4e00}'..='\u{9fff}').contains(&c)
            || ('\u{3040}'..='\u{30ff}').contains(&c)
            || ('\u{ac00}'..='\u{d7af}').contains(&c)
    });
    let (domain, lang) = if has_cjk {
        ("cn.bing.com", "zh-CN,zh;q=0.9,en;q=0.8")
    } else {
        ("www.bing.com", "en-US,en;q=0.9")
    };
    let url = format!("https://{domain}/search?q={}", urlencoding(&q));
    let resp = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, SEARCH_UA)
        .header(reqwest::header::ACCEPT_LANGUAGE, lang)
        .send()
        .await;
    match resp {
        Ok(r) => r.text().await.map(|h| parse_bing(&h)).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

async fn scrape_google(q: String) -> Vec<(String, String, String)> {
    let client = match search_client() {
        Some(c) => c,
        None => return Vec::new(),
    };
    let url = format!(
        "https://www.google.com/search?q={}&num=10&hl=zh-CN",
        urlencoding(&q)
    );
    let resp = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, SEARCH_UA)
        .header(reqwest::header::ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9,en;q=0.8")
        .header(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .send()
        .await;
    match resp {
        Ok(r) => r.text().await.map(|h| parse_google(&h)).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn parse_google(html: &str) -> Vec<(String, String, String)> {
    let mut results = Vec::new();
    let mut pos = 0;
    while let Some(div_start) = html[pos..].find("<div class=\"g\"") {
        let abs = pos + div_start;
        let chunk_end = html[abs..]
            .find("</div>")
            .map(|e| abs + e + 6)
            .unwrap_or(html.len().min(abs + 3000));
        let chunk = &html[abs..chunk_end];

        let title = extract_between(chunk, "<h3", "</h3>")
            .map(strip_tags)
            .unwrap_or_default();
        let url = extract_between(chunk, "<a href=\"/url?q=", "&")
            .map(|s| s.to_string())
            .or_else(|| extract_between(chunk, "<a href=\"http", "\"").map(|u| format!("http{u}")))
            .unwrap_or_default()
            .replace("&amp;", "&");
        let snippet = extract_between(chunk, "<span class=\"", "</span>")
            .map(|s| {
                let inner = s.find('>').map(|i| &s[i + 1..]).unwrap_or(s);
                strip_tags(inner)
            })
            .unwrap_or_default();

        if !title.is_empty() && (url.starts_with("http://") || url.starts_with("https://")) {
            results.push((title, url, snippet));
        }
        pos = chunk_end;
        if results.len() >= 10 {
            break;
        }
    }
    if results.is_empty() {
        let mut pos2 = 0;
        while let Some(a_start) = html[pos2..].find("<a href=\"/url?q=") {
            let abs2 = pos2 + a_start + 16;
            if let Some(end) = html[abs2..].find('&') {
                let raw = &html[abs2..abs2 + end];
                let url = percent_decode_str(raw);
                if url.starts_with("http")
                    && !url.contains("google.com")
                    && !url.contains("accounts.google")
                {
                    let title = extract_between(&html[abs2..], ">", "</a>")
                        .map(strip_tags)
                        .unwrap_or_else(|| url.clone());
                    if !title.is_empty() {
                        results.push((title, url, String::new()));
                    }
                }
            }
            pos2 = abs2 + 10;
            if results.len() >= 10 {
                break;
            }
        }
    }
    results
}

fn extract_between<'a>(hay: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = hay.find(open)? + open.len();
    let end = hay[start..].find(close)? + start;
    Some(&hay[start..end])
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(c);
        }
    }
    out.trim().to_string()
}

async fn scrape_ddg_lite(q: String) -> Vec<(String, String, String)> {
    let client = match search_client() {
        Some(c) => c,
        None => return Vec::new(),
    };
    let resp = client
        .post("https://lite.duckduckgo.com/lite/")
        .header(reqwest::header::USER_AGENT, SEARCH_UA)
        .form(&[("q", q.as_str())])
        .send()
        .await;
    match resp {
        Ok(r) => r
            .text()
            .await
            .map(|h| parse_ddg_lite(&h))
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

/// Last-resort search via a REAL headless-browser render (`chrome --dump-dom`).
/// A real Chrome (genuine TLS fingerprint, runs JS, real UA) gets through the
/// anti-bot walls that block the bare reqwest scrape — i.e. the agent's own
/// browser performs the search. Bounded by a hard timeout so a wedged Chrome
/// can't hang the turn; on timeout we just return what we have (often nothing).
async fn browser_render_search(q: &str) -> Vec<(String, String, String)> {
    let browser = match crate::capture::find_headless_browser() {
        Some(b) => b,
        None => return Vec::new(),
    };
    // (url, parser-kind): 0 = Bing b_algo, 1 = DDG html result__a.
    let targets = [
        (
            format!(
                "https://www.bing.com/search?q={}&setlang=en",
                urlencoding(q)
            ),
            0u8,
        ),
        (
            format!("https://html.duckduckgo.com/html/?q={}", urlencoding(q)),
            1u8,
        ),
    ];
    for (url, kind) in targets {
        let browser2 = browser.clone();
        let url2 = url.clone();
        let rendered = tokio::time::timeout(
            std::time::Duration::from_secs(14), // was 28s ×2 targets = up to 56s; keep the fallback snappy
            tauri::async_runtime::spawn_blocking(move || render_dom(&browser2, &url2)),
        )
        .await;
        if let Ok(Ok(Some(html))) = rendered {
            let r = if kind == 0 {
                parse_bing(&html)
            } else {
                parse_ddg_results(&html)
            };
            if !r.is_empty() {
                return r;
            }
        }
    }
    Vec::new()
}

/// Render `url` in headless Chrome and return its fully-rendered DOM (stdout of
/// `--dump-dom`). Chrome exits on its own after `--virtual-time-budget`, so this
/// doesn't hang; the caller still wraps it in a timeout as a backstop.
fn render_dom(browser: &str, url: &str) -> Option<String> {
    let out = crate::process_util::command(browser)
        .args([
            "--headless=new",
            "--disable-gpu",
            "--no-sandbox",
            "--no-first-run",
            "--disable-extensions",
            "--disable-background-networking",
            "--user-agent=Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
            "--virtual-time-budget=9000",
            "--dump-dom",
            url,
        ])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if out.stdout.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Minimal percent-encoding for a query string (no extra crate).
fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// Parse Bing's organic results (`<li class="b_algo"> … <h2><a href>title</a> … <p>snippet</p>`).
fn parse_bing(html: &str) -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> = Vec::new();
    let mut from = 0;
    while let Some(rel) = html[from..].find("b_algo") {
        let abs = from + rel;
        from = abs + 6;
        let end = (abs + 4000).min(html.len());
        let region = &html[abs..end]; // bound the per-result scan
        let (href, title) = region
            .find("<a ")
            .and_then(|a| {
                let s = &region[a..];
                let href = s.find("href=\"").and_then(|h| {
                    let s2 = &s[h + 6..];
                    s2.find('"').map(|e| s2[..e].to_string())
                })?;
                let title = s.find('>').and_then(|g| {
                    let s2 = &s[g + 1..];
                    s2.find("</a>").map(|e| html_to_text(&s2[..e]))
                })?;
                Some((href, title))
            })
            .unwrap_or_default();
        let snippet = region
            .find("<p")
            .and_then(|p| {
                let s = &region[p..];
                s.find('>').and_then(|g| {
                    let s2 = &s[g + 1..];
                    s2.find("</p>").map(|e| html_to_text(&s2[..e]))
                })
            })
            .unwrap_or_default();
        if href.starts_with("http") && !title.is_empty() && !out.iter().any(|(_, u, _)| u == &href)
        {
            out.push((title, href, snippet));
        }
        if out.len() >= 10 {
            break;
        }
    }
    out
}

/// Parse the simple `lite.duckduckgo.com/lite/` results table.
fn parse_ddg_lite(html: &str) -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> = Vec::new();
    let mut from = 0;
    while let Some(rel) = html[from..].find("result-link") {
        let abs = from + rel;
        from = abs + 11;
        // The enclosing <a ...> starts before "result-link"; grab its href + text.
        let a_start = html[..abs].rfind("<a").unwrap_or(abs);
        let tag = &html[a_start..];
        let href = tag
            .find("href=\"")
            .and_then(|h| {
                let s = &tag[h + 6..];
                s.find('"').map(|e| s[..e].to_string())
            })
            .unwrap_or_default();
        let url = ddg_unwrap(&href);
        let title = tag
            .find('>')
            .and_then(|g| {
                let s = &tag[g + 1..];
                s.find("</a>").map(|e| html_to_text(&s[..e]))
            })
            .unwrap_or_default();
        // Snippet: the next `result-snippet` cell after this link.
        let snippet = html[abs..]
            .find("result-snippet")
            .and_then(|sp| {
                let s = &html[abs + sp..];
                s.find('>').and_then(|g| {
                    let s2 = &s[g + 1..];
                    s2.find("</td>")
                        .or_else(|| s2.find("</a>"))
                        .map(|e| html_to_text(&s2[..e]))
                })
            })
            .unwrap_or_default();
        if !url.is_empty() && !title.is_empty() && !out.iter().any(|(_, u, _)| u == &url) {
            out.push((title, url, snippet));
        }
        if out.len() >= 10 {
            break;
        }
    }
    out
}
