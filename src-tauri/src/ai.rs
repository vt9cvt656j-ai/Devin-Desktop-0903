use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::ipc::Channel;

const RESPONSE_HEADERS_TIMEOUT_SECS: u64 = 60;
const STANDARD_FIRST_STREAM_PROGRESS_TIMEOUT_SECS: u64 = 60;
const STANDARD_EMPTY_STREAM_PROGRESS_TIMEOUT_SECS: u64 = 60;
const STANDARD_STREAM_STALL_TIMEOUT_SECS: u64 = 60;
const HIGH_FIRST_STREAM_PROGRESS_TIMEOUT_SECS: u64 = 60;
const HIGH_EMPTY_STREAM_PROGRESS_TIMEOUT_SECS: u64 = 60;
const HIGH_STREAM_STALL_TIMEOUT_SECS: u64 = 90;
const EXTENDED_FIRST_STREAM_PROGRESS_TIMEOUT_SECS: u64 = 60;
const EXTENDED_EMPTY_STREAM_PROGRESS_TIMEOUT_SECS: u64 = 60;
const EXTENDED_STREAM_STALL_TIMEOUT_SECS: u64 = 120;

const FIRST_STREAM_PROGRESS_TIMEOUT_ENV: &str = "MICHAEL_AI_FIRST_PROGRESS_TIMEOUT_SECS";
const EMPTY_STREAM_PROGRESS_TIMEOUT_ENV: &str = "MICHAEL_AI_EMPTY_STREAM_TIMEOUT_SECS";
const STREAM_STALL_TIMEOUT_ENV: &str = "MICHAEL_AI_STREAM_STALL_TIMEOUT_SECS";

const FIRST_STREAM_PROGRESS_TIMEOUT_MIN_SECS: u64 = 60;
const FIRST_STREAM_PROGRESS_TIMEOUT_MAX_SECS: u64 = 60;
const EMPTY_STREAM_PROGRESS_TIMEOUT_MIN_SECS: u64 = 60;
const EMPTY_STREAM_PROGRESS_TIMEOUT_MAX_SECS: u64 = 60;
const STREAM_STALL_TIMEOUT_MIN_SECS: u64 = 15;
const STREAM_STALL_TIMEOUT_MAX_SECS: u64 = 300;
const STREAM_READ_POLL: Duration = Duration::from_secs(2);
const INCOMPLETE_SSE_STREAM_ERROR: &str = "AI stream closed before data: [DONE]（连接提前结束）；响应可能被截断，已拒绝本轮结果，请重试。";
const AI_ERROR_BODY_READ_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_AI_ERROR_BODY_BYTES: usize = 64 * 1024;
const CANCEL_POLL: Duration = Duration::from_millis(100);
const CANCELLED_AI_REQUEST: &str = "AI request cancelled";
const RESPONSE_DEADLINE_HEADER: &str = "x-ide-response-deadline-ms";
const MICHAEL_GATEWAY_HEALTH_URL: &str = "https://code.mrday.one/health";
const GATEWAY_TRANSPORT_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(50);
const GATEWAY_TRANSPORT_WARMUP_TIMEOUT: Duration = Duration::from_secs(5);

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
        // Prefer one multiplexed gateway connection and keep it alive while the
        // user is thinking between Agent turns. The startup HEAD below covers a
        // cold connection; HTTP/2 PINGs avoid paying that handshake again later.
        .http2_keep_alive_interval(Some(GATEWAY_TRANSPORT_KEEPALIVE_INTERVAL))
        .http2_keep_alive_timeout(Duration::from_secs(10))
        .http2_keep_alive_while_idle(true)
        // The IDE↔LLM-gateway link must NEVER route through the macOS system proxy. Otherwise a
        // capture/MITM proxy the user (or the agent) set up — and left dangling on a dead port —
        // silently kills all AI requests ("无法连接服务器"). Talk to our gateway directly, always.
        .no_proxy()
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});

async fn warm_gateway_transport_once(client: &reqwest::Client, url: &str) {
    let _ = client
        .head(url)
        .timeout(GATEWAY_TRANSPORT_WARMUP_TIMEOUT)
        .send()
        .await;
}

/// Warm and retain the desktop AI client's own gateway connection. Frontend
/// fetches use WebKit's separate pool, so they cannot prepay this TCP+TLS setup.
pub(crate) fn start_gateway_transport_warmup() {
    tauri::async_runtime::spawn(async {
        loop {
            warm_gateway_transport_once(&HTTP, MICHAEL_GATEWAY_HEALTH_URL).await;
            tokio::time::sleep(GATEWAY_TRANSPORT_KEEPALIVE_INTERVAL).await;
        }
    });
}

/// In-flight request cancellation. The JS side passes a unique physical `cancel_id`
/// (falling back to `request_id` for older callers) and calls `cancel_ai(id)` when the user hits Stop —
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

fn cancellation_id(config: &AiConfig) -> Option<String> {
    config
        .cancel_id
        .as_deref()
        .filter(|id| !id.is_empty())
        .or_else(|| config.request_id.as_deref().filter(|id| !id.is_empty()))
        .map(str::to_string)
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
    /// UI 档位原值（off/low/medium/high/xhigh/max）。前端对每个模型都会带上它（包括那些
    /// 只发 thinking/thinkingConfig 而不发 reasoning_effort 的家族），本字段只用于本地
    /// 看门狗分档，绝不写进上游请求体。
    #[serde(default)]
    pub thinking_effort: Option<String>,
    #[serde(default)]
    pub thinking: Option<serde_json::Value>,
    #[serde(default)]
    pub thinking_config: Option<serde_json::Value>,
    /// Gateway settlement identity. It is relayed as `x-ide-request-id` and may cover
    /// multiple physical model calls belonging to one user-visible run.
    #[serde(default)]
    pub request_id: Option<String>,
    /// Physical request identity used only by the local cancellation registry. This
    /// keeps Stop from cancelling another request that shares the settlement ID.
    #[serde(default)]
    pub cancel_id: Option<String>,
    /// Correlates the individual model request with the user-visible Agent run.
    /// These values are diagnostic metadata only and are strictly filtered before
    /// becoming HTTP headers.
    #[serde(default)]
    pub ide_run_id: Option<String>,
    #[serde(default)]
    pub ide_step_index: Option<i64>,
    #[serde(default)]
    pub ide_step_kind: Option<String>,
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
    /// Versioned semantic routing decisions produced by the IDE's model-backed engineering
    /// classifier. The gateway uses these flags to select Prompt Graph modules without parsing
    /// the user's prose a second time.
    #[serde(default)]
    pub ide_semantic_profile: Option<String>,
    /// ISO 3166-1 alpha-2 region code (lowercase) resolved by the IDE from the user's real
    /// IP egress (Cloudflare trace) with timezone fallback. The gateway uses it to inject
    /// region-appropriate package mirror guidance (e.g. npmmirror for mainland China).
    #[serde(default)]
    pub ide_region: Option<String>,
    /// michael-compression 的档位（"1m"/"2m"/"5m"），由 /api/me 下发、客户端原样回传。
    ///
    /// 这个字段此前**不存在**：JS 侧设了 `config.michaelCompression`，serde 默认忽略
    /// 未知字段，于是被静默丢弃 —— 桌面版永远发不出档位头。而客户端因为从 /api/me
    /// 看到了档位，已经关掉了自己的本地压缩和棘轮裁剪。净效果是三处压缩全不生效，
    /// 原始历史每轮整份上传，长会话必然撞穿模型窗口。
    #[serde(default)]
    pub michael_compression: Option<String>,
    /// 上一轮网关签发的前缀引用，本轮原样回发。
    ///
    /// 这条腿决定 2m/5m 能不能真的达到：有它，线路体积正比于**新增内容**；没有它，
    /// 客户端每轮都得上传完整历史，被 `_MODEL_REQUEST_BODY_BYTE_CAP`（3.5MB，约
    /// 875k token）卡死，5M 档在物理上不可达。
    #[serde(default)]
    pub mc_prefix: Option<String>,
    /// 与 mc_prefix 一起回发的覆盖条数，服务端用它校验客户端裁剪口径。
    #[serde(default)]
    pub mc_prefix_covered: Option<usize>,
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
    /// 网关本轮签发的 michael-compression 前缀引用。前端存下来，下一轮通过
    /// `config.mcPrefix` 回发，从而只上传新增消息。
    CompressionPrefix {
        token: String,
        /// 开头 system 块**之后**已被摘要覆盖的消息条数。
        ///
        /// 少了这个数客户端就不知道该省略前几条，只能整份重传 —— 既撞 3.5MB 字节上限
        /// （5M 档因此不可达），又会让早期内容同时以摘要和原文出现、上下文重复膨胀。
        covered: usize,
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
        #[serde(skip_serializing_if = "Option::is_none")]
        attempt: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<u16>,
    },
    /// Why the model stopped generating, normalized onto the OpenAI vocabulary
    /// (`stop` | `length` | `tool_calls` | `content_filter` | …).
    ///
    /// This is the ONLY authoritative signal that a response was cut off by the
    /// output-token limit. It was previously never parsed at all, which left the
    /// client's truncation guard with nothing but a JSON-shape heuristic — and that
    /// heuristic cannot tell a tool-call argument object that was cut mid-stream from
    /// one that is genuinely complete, because a truncated object can still be a valid
    /// JSON *prefix*. A `write_file` whose `content` was severed by `max_tokens` is
    /// exactly the case that must never reach execution.
    FinishReason {
        reason: String,
    },
    Done,
    /// Token accounting from the final stream chunk — lets the UI show how much of
    /// the prompt was served from cache (the payoff of the prompt-cache work).
    ///
    /// `prompt_tokens` is normalized to the tokens the model actually read this turn,
    /// cache included, so the frontend can use it as the context reading without
    /// knowing which provider shape produced it. `cached_tokens`/`cache_creation_tokens`
    /// are the breakdown of that total, not extras to be added to it.
    Usage {
        prompt_tokens: u32,
        completion_tokens: u32,
        cached_tokens: u32,
        cache_creation_tokens: u32,
        /// 这一轮模型**真的**花在思考上的 token 数。
        ///
        /// 前端一直有消费方（_recordUsage 读 completion_tokens_details.reasoning_tokens，
        /// 上下文环里显示「思考 高 · 推理 1.2k」），但这个字段从来没被传上去过——于是
        /// 那半句永远不显示，用户拨了深度也看不到任何回执，"思考深度都和假的一样"这句
        /// 抱怨里有一半是这么来的：不是没生效，是没有任何东西证明它生效了。
        /// 0 表示上游没报这个数（≠ 一定没思考）。
        reasoning_tokens: u32,
        /// 网关在 Anthropic 线路上补的思考**字符**数。那条路的上游根本不报思考 token
        /// （思考算进 output_tokens），所以 reasoning_tokens 恒为 0；字符数是那边唯一
        /// 真实可核对的思考量。两个数各显示各的，前端不拿它冒充 token。
        thinking_chars: u32,
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
    let cancel_id = cancellation_id(&config);
    let cancel_flag = cancel_id.as_deref().map(register_cancel);
    let _cancel_guard = cancel_id.map(CancelGuard);
    let payload = serde_json::json!({
        "model": config.model,
        "stream": false,
        "max_tokens": max_tokens,
        "temperature": config.temperature.unwrap_or(0.1),
        "messages": messages,
    });

    let client = &*HTTP;
    let completion_timeout = StreamTimeouts::for_config(&config).response_headers;
    let resp = post_chat_once(
        client,
        &url,
        &config,
        &payload,
        completion_timeout,
        cancel_flag.as_deref(),
    )
    .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = read_ai_error_body_cancellable(resp, cancel_flag.as_deref()).await?;
        return Err(format_ai_http_error(status, &text));
    }

    let v = read_ai_completion_body(resp, Duration::from_secs(10), cancel_flag.as_deref()).await?;
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

async fn read_ai_error_body(response: reqwest::Response) -> String {
    read_ai_error_body_with_limits(
        response,
        AI_ERROR_BODY_READ_TIMEOUT,
        MAX_AI_ERROR_BODY_BYTES,
    )
    .await
}

async fn read_ai_error_body_cancellable(
    response: reqwest::Response,
    cancel: Option<&AtomicBool>,
) -> Result<String, String> {
    let read = read_ai_error_body(response);
    tokio::pin!(read);
    loop {
        if request_was_cancelled(cancel) {
            return Err(CANCELLED_AI_REQUEST.to_string());
        }
        match tokio::time::timeout(CANCEL_POLL, &mut read).await {
            Ok(body) => return Ok(body),
            Err(_) => continue,
        }
    }
}

async fn read_ai_completion_body(
    response: reqwest::Response,
    timeout: Duration,
    cancel: Option<&AtomicBool>,
) -> Result<serde_json::Value, String> {
    let deadline = Instant::now() + timeout;
    let read = response.json::<serde_json::Value>();
    tokio::pin!(read);
    loop {
        if request_was_cancelled(cancel) {
            return Err(CANCELLED_AI_REQUEST.to_string());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("AI completion timed out reading the response body".to_string());
        }
        match tokio::time::timeout(remaining.min(CANCEL_POLL), &mut read).await {
            Ok(Ok(value)) => return Ok(value),
            Ok(Err(error)) => return Err(error.to_string()),
            Err(_) if deadline.saturating_duration_since(Instant::now()).is_zero() => {
                return Err("AI completion timed out reading the response body".to_string());
            }
            Err(_) => continue,
        }
    }
}

async fn read_ai_error_body_with_limits(
    mut response: reqwest::Response,
    timeout: Duration,
    max_bytes: usize,
) -> String {
    let read = async move {
        let capacity = response
            .content_length()
            .unwrap_or_default()
            .min(max_bytes as u64) as usize;
        let mut body = Vec::with_capacity(capacity);
        while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
            let remaining = max_bytes.saturating_sub(body.len());
            if chunk.len() > remaining {
                body.extend_from_slice(&chunk[..remaining]);
                break;
            }
            body.extend_from_slice(&chunk);
        }
        Ok::<Vec<u8>, String>(body)
    };

    match tokio::time::timeout(timeout, read).await {
        Ok(Ok(body)) => String::from_utf8_lossy(&body).into_owned(),
        Ok(Err(error)) => format!("failed to read AI error response body: {error}"),
        Err(_) => format!(
            "timed out reading AI error response body after {} seconds",
            duration_seconds_label(timeout)
        ),
    }
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
    if let Some(request_id) = config.request_id.as_deref().filter(|value| {
        (8..=128).contains(&value.len())
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    }) {
        rb = rb.header("x-ide-request-id", request_id);
    }
    if let Some(run_id) = config.ide_run_id.as_deref().filter(|value| {
        (8..=128).contains(&value.len())
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    }) {
        rb = rb.header("x-ide-run-id", run_id);
    }
    if let Some(step_index) = config
        .ide_step_index
        .filter(|value| (0..=10_000).contains(value))
    {
        rb = rb.header("x-ide-step-index", step_index.to_string());
    }
    if let Some(step_kind) = config.ide_step_kind.as_deref().filter(|value| {
        matches!(
            *value,
            "intent" | "main" | "aux" | "subagent" | "repair" | "learning" | "chat"
        )
    }) {
        rb = rb.header("x-ide-step-kind", step_kind);
    }
    if let Some(m) = config.ide_mode.as_deref().filter(|s| !s.is_empty()) {
        rb = rb.header("x-ide-mode", m);
    }
    if let Some(t) = config.ide_tools.as_deref().filter(|s| !s.is_empty()) {
        rb = rb.header("x-ide-tools", t);
    }
    if let Some(profile) = config.ide_semantic_profile.as_deref().filter(|profile| {
        profile.starts_with("2.5:")
            && profile.len() <= 1024
            && profile.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b':' | b',' | b'_')
            })
    }) {
        rb = rb.header("x-ide-semantic-profile", profile);
    }
    if let Some(region) = config.ide_region.as_deref().filter(|region| {
        (2..=8).contains(&region.len()) && region.bytes().all(|byte| byte.is_ascii_lowercase())
    }) {
        rb = rb.header("x-ide-region", region);
    }
    // 只放行已知档位。网关侧还会按会员套餐再钳一次，这里的白名单是防止把任意字符串
    // 当档位发出去。
    if let Some(tier) = config
        .michael_compression
        .as_deref()
        .map(str::trim)
        .filter(|s| matches!(*s, "1m" | "2m" | "5m"))
    {
        rb = rb.header("x-michael-compression", tier);
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

#[derive(Debug, Clone, Copy)]
struct ResponseHeadersDeadline {
    started: Instant,
    at: Instant,
    budget: Duration,
    unix_ms: u64,
}

impl ResponseHeadersDeadline {
    fn new(started: Instant, budget: Duration) -> Self {
        let unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .saturating_add(budget.as_millis())
            .min(u64::MAX as u128) as u64;
        Self {
            started,
            at: started + budget,
            budget,
            unix_ms,
        }
    }

    fn remaining(self) -> Duration {
        self.at.saturating_duration_since(Instant::now())
    }

    fn timeout_error(self) -> String {
        response_headers_timeout_error(self.budget, self.started.elapsed())
    }
}

fn with_response_deadline_header(
    request: reqwest::RequestBuilder,
    deadline: ResponseHeadersDeadline,
) -> reqwest::RequestBuilder {
    request.header(RESPONSE_DEADLINE_HEADER, deadline.unix_ms.to_string())
}

fn request_was_cancelled(cancel: Option<&AtomicBool>) -> bool {
    cancel.is_some_and(|flag| flag.load(Ordering::SeqCst))
}

/// Send one physical model request and return its first HTTP response or transport
/// error. A failure is terminal for this turn; the desktop and browser paths never
/// replay the same prompt automatically.
async fn post_chat_once(
    client: &reqwest::Client,
    url: &str,
    config: &AiConfig,
    payload: &serde_json::Value,
    response_headers_timeout: Duration,
    cancel: Option<&AtomicBool>,
) -> Result<reqwest::Response, String> {
    if request_was_cancelled(cancel) {
        return Err(CANCELLED_AI_REQUEST.to_string());
    }
    let deadline = ResponseHeadersDeadline::new(Instant::now(), response_headers_timeout);
    send_with_response_headers_deadline(
        with_response_deadline_header(
            with_ide_headers(client.post(url).bearer_auth(&config.api_key), config),
            deadline,
        )
        .json(payload),
        deadline,
        cancel,
    )
    .await
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
        // UI 档位原值：Gemini-3/Kimi/GLM 等家族只发 thinking/thinkingConfig 不发
        // reasoning_effort，此前 serde 直接丢掉 thinkingEffort → 全部掉进 35s/18s 标准窗。
        let ui_effort = config.thinking_effort.as_deref().unwrap_or_default().trim();
        let thinking_enabled = config
            .thinking
            .as_ref()
            .and_then(|value| value.get("type"))
            .and_then(|value| value.as_str())
            .is_some_and(|kind| {
                kind.eq_ignore_ascii_case("enabled") || kind.eq_ignore_ascii_case("adaptive")
            })
            || config.thinking_config.is_some();
        let has_thinking_budget = config.thinking_budget.is_some_and(|budget| budget > 0);
        let deep =
            |value: &str| value.eq_ignore_ascii_case("max") || value.eq_ignore_ascii_case("xhigh");
        // xhigh 与 max 同档：gpt-5.6 系把 xhigh 原样透传，此前它掉进 35s 标准窗——深度思考
        // 本身就是长时间无输出，窗口太短会被无进度看门狗掐掉再重试，用户设的思考深度形同虚设。
        let defaults = if has_thinking_budget || deep(effort) || deep(ui_effort) {
            (
                RESPONSE_HEADERS_TIMEOUT_SECS,
                EXTENDED_FIRST_STREAM_PROGRESS_TIMEOUT_SECS,
                EXTENDED_EMPTY_STREAM_PROGRESS_TIMEOUT_SECS,
                EXTENDED_STREAM_STALL_TIMEOUT_SECS,
            )
        } else if effort.eq_ignore_ascii_case("high")
            || ui_effort.eq_ignore_ascii_case("high")
            || thinking_enabled
        {
            (
                RESPONSE_HEADERS_TIMEOUT_SECS,
                HIGH_FIRST_STREAM_PROGRESS_TIMEOUT_SECS,
                HIGH_EMPTY_STREAM_PROGRESS_TIMEOUT_SECS,
                HIGH_STREAM_STALL_TIMEOUT_SECS,
            )
        } else {
            (
                RESPONSE_HEADERS_TIMEOUT_SECS,
                STANDARD_FIRST_STREAM_PROGRESS_TIMEOUT_SECS,
                STANDARD_EMPTY_STREAM_PROGRESS_TIMEOUT_SECS,
                STANDARD_STREAM_STALL_TIMEOUT_SECS,
            )
        };

        Self {
            response_headers: Duration::from_secs(defaults.0),
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

fn response_headers_timeout_error(timeout: Duration, elapsed: Duration) -> String {
    format!(
        "AI request timed out waiting for response headers after {} seconds (per-attempt {}-second deadline)",
        duration_seconds_label(elapsed.min(timeout)),
        duration_seconds_label(timeout),
    )
}

#[cfg(test)]
async fn send_with_response_headers_timeout(
    request: reqwest::RequestBuilder,
    timeout: Duration,
) -> Result<reqwest::Response, String> {
    let deadline = ResponseHeadersDeadline::new(Instant::now(), timeout);
    send_with_response_headers_deadline(request, deadline, None).await
}

async fn send_with_response_headers_deadline(
    request: reqwest::RequestBuilder,
    deadline: ResponseHeadersDeadline,
    cancel: Option<&AtomicBool>,
) -> Result<reqwest::Response, String> {
    let send = request.send();
    tokio::pin!(send);
    loop {
        if request_was_cancelled(cancel) {
            return Err(CANCELLED_AI_REQUEST.to_string());
        }
        let remaining = deadline.remaining();
        if remaining.is_zero() {
            return Err(deadline.timeout_error());
        }
        match tokio::time::timeout(remaining.min(CANCEL_POLL), &mut send).await {
            Ok(Ok(response)) => return Ok(response),
            Ok(Err(error)) => return Err(error.to_string()),
            Err(_) if deadline.remaining().is_zero() => return Err(deadline.timeout_error()),
            Err(_) => continue,
        }
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
        } else {
            // Response headers, SSE comments, and empty control frames do not buy
            // another timeout window. One physical attempt has sixty seconds from
            // request start to its first real reasoning/text/tool event.
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

/// Anthropic's native Messages stream uses top-level event objects instead of
/// OpenAI's `choices[].delta`. A gateway normally converts these, but accepting
/// the native events here keeps a direct Anthropic-compatible route from losing
/// the visible reasoning/text stream.
fn native_anthropic_event_has_real_progress(event: &serde_json::Value) -> bool {
    match event["type"].as_str() {
        Some("content_block_start") => {
            event["content_block"]["type"].as_str() == Some("tool_use")
                && (event["content_block"]["id"]
                    .as_str()
                    .is_some_and(|value| !value.is_empty())
                    || event["content_block"]["name"]
                        .as_str()
                        .is_some_and(|value| !value.is_empty()))
        }
        Some("content_block_delta") => match event["delta"]["type"].as_str() {
            Some("thinking_delta") => event["delta"]["thinking"]
                .as_str()
                .is_some_and(|value| !value.is_empty()),
            Some("text_delta") => event["delta"]["text"]
                .as_str()
                .is_some_and(|value| !value.is_empty()),
            Some("input_json_delta") => event["delta"]["partial_json"]
                .as_str()
                .is_some_and(|value| !value.is_empty()),
            _ => false,
        },
        _ => false,
    }
}

/// Map each provider's "why generation stopped" vocabulary onto the OpenAI spelling
/// so the client has exactly one set of values to reason about. Anthropic's native
/// stream reports `max_tokens` / `end_turn` / `tool_use` / `stop_sequence`; everything
/// OpenAI-compatible already uses the target vocabulary and passes through unchanged.
fn normalize_finish_reason(raw: &str) -> &str {
    match raw {
        "max_tokens" => "length",
        "end_turn" | "stop_sequence" => "stop",
        "tool_use" => "tool_calls",
        other => other,
    }
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
        attempt: None,
        status: None,
    });
}

#[cfg(test)]
mod ide_header_tests {
    use super::*;
    use std::io::{Read, Write};

    fn config() -> AiConfig {
        AiConfig {
            base_url: "https://example.invalid/v1".into(),
            api_key: "test".into(),
            model: "test-model".into(),
            max_tokens: None,
            temperature: None,
            reasoning_effort: None,
            thinking_budget: None,
            thinking_effort: None,
            thinking: None,
            thinking_config: None,
            request_id: None,
            cancel_id: None,
            ide_run_id: None,
            ide_step_index: None,
            ide_step_kind: None,
            ide_mode: Some("agent".into()),
            ide_tools: None,
            ide_semantic_profile: Some("2.5:engineering,design,existing_project".into()),
            ide_region: None,
            michael_compression: None,
            mc_prefix: None,
            mc_prefix_covered: None,
            ide_timezone: Some("America/Los_Angeles".into()),
            ide_utc_offset_minutes: Some(-420),
        }
    }

    #[test]
    fn relays_bounded_user_timezone_headers() {
        let mut cfg = config();
        cfg.request_id = Some("req_12345678".into());
        cfg.ide_run_id = Some("run_12345678".into());
        cfg.ide_step_index = Some(7);
        cfg.ide_step_kind = Some("subagent".into());
        let request = with_ide_headers(reqwest::Client::new().get("https://example.invalid"), &cfg)
            .build()
            .unwrap();
        assert_eq!(request.headers()["x-ide-request-id"], "req_12345678");
        assert_eq!(request.headers()["x-ide-run-id"], "run_12345678");
        assert_eq!(request.headers()["x-ide-step-index"], "7");
        assert_eq!(request.headers()["x-ide-step-kind"], "subagent");
        assert_eq!(
            request.headers()["x-ide-semantic-profile"],
            "2.5:engineering,design,existing_project"
        );
        assert_eq!(request.headers()["x-ide-timezone"], "America/Los_Angeles");
        assert_eq!(request.headers()["x-ide-utc-offset-minutes"], "-420");
    }

    #[test]
    fn cancellation_id_is_independent_from_gateway_settlement_id() {
        let decoded: AiConfig = serde_json::from_value(serde_json::json!({
            "baseUrl": "https://example.invalid/v1",
            "apiKey": "test",
            "model": "test-model",
            "requestId": "req_settlement_123",
            "cancelId": "req_physical_456"
        }))
        .unwrap();
        assert_eq!(decoded.request_id.as_deref(), Some("req_settlement_123"));
        assert_eq!(decoded.cancel_id.as_deref(), Some("req_physical_456"));

        let mut cfg = config();
        cfg.request_id = Some("req_settlement_123".into());
        cfg.cancel_id = Some("req_physical_456".into());

        assert_eq!(cancellation_id(&cfg).as_deref(), Some("req_physical_456"));
        let request = with_ide_headers(reqwest::Client::new().get("https://example.invalid"), &cfg)
            .build()
            .unwrap();
        assert_eq!(request.headers()["x-ide-request-id"], "req_settlement_123");
        assert!(!request.headers().contains_key("x-ide-cancel-id"));

        cfg.cancel_id = None;
        assert_eq!(cancellation_id(&cfg).as_deref(), Some("req_settlement_123"));
    }

    #[test]
    fn drops_invalid_timezone_headers() {
        let mut cfg = config();
        cfg.request_id = Some("bad\nrequest".into());
        cfg.ide_run_id = Some("bad\nrun".into());
        cfg.ide_step_index = Some(-1);
        cfg.ide_step_kind = Some("unknown".into());
        cfg.ide_timezone = Some("bad\ntimezone".into());
        cfg.ide_utc_offset_minutes = Some(900);
        let request = with_ide_headers(reqwest::Client::new().get("https://example.invalid"), &cfg)
            .build()
            .unwrap();
        assert!(!request.headers().contains_key("x-ide-timezone"));
        assert!(!request.headers().contains_key("x-ide-utc-offset-minutes"));
        assert!(!request.headers().contains_key("x-ide-request-id"));
        assert!(!request.headers().contains_key("x-ide-run-id"));
        assert!(!request.headers().contains_key("x-ide-step-index"));
        assert!(!request.headers().contains_key("x-ide-step-kind"));
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

    #[test]
    fn gateway_transport_keepalive_stays_in_the_background_window() {
        assert!(GATEWAY_TRANSPORT_KEEPALIVE_INTERVAL >= Duration::from_secs(45));
        assert!(GATEWAY_TRANSPORT_KEEPALIVE_INTERVAL <= Duration::from_secs(60));
        assert_eq!(MICHAEL_GATEWAY_HEALTH_URL, "https://code.mrday.one/health");
    }

    #[test]
    fn desktop_reqwest_keeps_http2_enabled() {
        let manifest = include_str!("../Cargo.toml");
        let reqwest = manifest
            .lines()
            .find(|line| line.trim_start().starts_with("reqwest ="))
            .expect("reqwest dependency");
        assert!(reqwest.contains("\"http2\""));
    }

    #[tokio::test]
    async fn gateway_transport_warmup_is_an_anonymous_head_request() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0_u8; 4096];
            let read = socket.read(&mut request).unwrap();
            tx.send(String::from_utf8_lossy(&request[..read]).to_string())
                .unwrap();
            socket
                .write_all(
                    b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: keep-alive\r\n\r\n",
                )
                .unwrap();
            socket.flush().unwrap();
        });

        let client = reqwest::Client::builder().build().unwrap();
        warm_gateway_transport_once(&client, &format!("http://{address}/health")).await;
        server.join().unwrap();

        let request = rx.recv().unwrap().to_ascii_lowercase();
        assert!(request.starts_with("head /health http/1.1\r\n"));
        assert!(!request.contains("authorization:"));
        assert!(!request.contains("content-length:"));
    }

    #[tokio::test]
    async fn ai_error_body_read_obeys_the_byte_cap() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let _ = socket.read(&mut [0_u8; 4096]);
            let body = "x".repeat(128);
            let response = format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).unwrap();
            socket.flush().unwrap();
        });

        let response = reqwest::Client::new()
            .get(format!("http://{address}/"))
            .send()
            .await
            .unwrap();
        let body = read_ai_error_body_with_limits(response, Duration::from_secs(1), 16).await;
        assert_eq!(body, "x".repeat(16));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn ai_error_body_read_obeys_the_total_timeout() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let _ = socket.read(&mut [0_u8; 4096]);
            socket
                .write_all(
                    b"HTTP/1.1 400 Bad Request\r\nContent-Length: 4\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
            socket.flush().unwrap();
            std::thread::sleep(Duration::from_millis(150));
        });

        let response = reqwest::Client::new()
            .get(format!("http://{address}/"))
            .send()
            .await
            .unwrap();
        let started = Instant::now();
        let body = read_ai_error_body_with_limits(response, Duration::from_millis(20), 64).await;
        assert_eq!(
            body,
            "timed out reading AI error response body after 0.02 seconds"
        );
        assert!(started.elapsed() < Duration::from_millis(100));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn non_streaming_completion_relays_request_id() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0_u8; 32 * 1024];
            let read = socket.read(&mut request).unwrap();
            tx.send(String::from_utf8_lossy(&request[..read]).to_string())
                .unwrap();
            let body = r#"{"choices":[{"message":{"content":"ok"}}]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).unwrap();
            socket.flush().unwrap();
        });

        let mut cfg = config();
        cfg.base_url = format!("http://{address}");
        cfg.request_id = Some("req_nonstream_123".into());
        let result = ai_complete(
            cfg,
            vec![serde_json::json!({"role": "user", "content": "hello"})],
            32,
        )
        .await
        .unwrap();
        server.join().unwrap();

        assert_eq!(result, "ok");
        let request = rx.recv().unwrap().to_ascii_lowercase();
        assert!(request.contains("x-ide-request-id: req_nonstream_123\r\n"));
    }

    #[tokio::test]
    async fn non_streaming_completion_cancellation_interrupts_header_wait() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let _ = socket.read(&mut [0_u8; 4096]);
            tx.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(600));
        });

        let request_id = "req_nonstream_cancel_headers_123".to_string();
        let mut cfg = config();
        cfg.base_url = format!("http://{address}");
        cfg.request_id = Some(request_id.clone());
        let task = tokio::spawn(ai_complete(
            cfg,
            vec![serde_json::json!({"role": "user", "content": "hello"})],
            32,
        ));
        tokio::task::spawn_blocking(move || rx.recv_timeout(Duration::from_secs(2)))
            .await
            .unwrap()
            .unwrap();
        let started = Instant::now();
        cancel_ai(request_id.clone());
        let result = tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result, Err(CANCELLED_AI_REQUEST.to_string()));
        assert!(started.elapsed() < Duration::from_millis(500));
        assert!(!CANCELS.lock().unwrap().contains_key(&request_id));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn non_streaming_completion_cancellation_interrupts_body_read() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let _ = socket.read(&mut [0_u8; 4096]);
            let partial = r#"{"choices":["#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 128\r\nConnection: keep-alive\r\n\r\n{partial}"
            );
            socket.write_all(response.as_bytes()).unwrap();
            socket.flush().unwrap();
            tx.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(600));
        });

        let request_id = "req_nonstream_cancel_body_123".to_string();
        let mut cfg = config();
        cfg.base_url = format!("http://{address}");
        cfg.request_id = Some(request_id.clone());
        let task = tokio::spawn(ai_complete(
            cfg,
            vec![serde_json::json!({"role": "user", "content": "hello"})],
            32,
        ));
        tokio::task::spawn_blocking(move || rx.recv_timeout(Duration::from_secs(2)))
            .await
            .unwrap()
            .unwrap();
        let started = Instant::now();
        cancel_ai(request_id.clone());
        let result = tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result, Err(CANCELLED_AI_REQUEST.to_string()));
        assert!(started.elapsed() < Duration::from_millis(500));
        assert!(!CANCELS.lock().unwrap().contains_key(&request_id));
        server.join().unwrap();
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
            thinking_effort: None,
            thinking: None,
            thinking_config: None,
            request_id: None,
            cancel_id: None,
            ide_run_id: None,
            ide_step_index: None,
            ide_step_kind: None,
            ide_mode: None,
            ide_tools: None,
            ide_semantic_profile: None,
            ide_region: None,
            michael_compression: None,
            mc_prefix: None,
            mc_prefix_covered: None,
            ide_timezone: None,
            ide_utc_offset_minutes: None,
        }
    }

    fn timeouts(reasoning_effort: Option<&str>, thinking_budget: Option<u32>) -> StreamTimeouts {
        StreamTimeouts::for_config_with_env(&config(reasoning_effort, thinking_budget), |_| None)
    }

    fn read_http_request(socket: &mut std::net::TcpStream) -> String {
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = Vec::new();
        loop {
            let mut chunk = [0_u8; 4096];
            let read = socket.read(&mut chunk).unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
            if let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("content-length:")
                            .or_else(|| line.strip_prefix("Content-Length:"))
                    })
                    .and_then(|value| value.trim().parse::<usize>().ok())
                    .unwrap_or_default();
                if request.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        String::from_utf8_lossy(&request).into_owned()
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
    fn every_attempt_gets_sixty_seconds_before_first_progress() {
        let standard = StreamTimeouts {
            response_headers: Duration::from_secs(60),
            first_progress: Duration::from_secs(60),
            empty_stream: Duration::from_secs(60),
            stall: Duration::from_secs(60),
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
                response_headers: Duration::from_secs(60),
                first_progress: Duration::from_secs(60),
                empty_stream: Duration::from_secs(60),
                stall: Duration::from_secs(90),
            }
        );
        let extended = StreamTimeouts {
            response_headers: Duration::from_secs(60),
            first_progress: Duration::from_secs(60),
            empty_stream: Duration::from_secs(60),
            stall: Duration::from_secs(120),
        };
        assert_eq!(timeouts(Some("max"), None), extended);
        // gpt-5.6 系把 xhigh 原样透传——它必须和 max 同档。
        assert_eq!(timeouts(Some("xhigh"), None), extended);
        assert_eq!(timeouts(Some("XHigh"), None), extended); // 大小写不敏感
        assert_eq!(timeouts(Some("high"), Some(32_000)), extended);
        assert_eq!(timeouts(Some("medium"), Some(1)), extended);
    }

    #[test]
    fn adaptive_thinking_uses_high_deadlines_without_a_legacy_budget() {
        let mut cfg = config(Some("medium"), None);
        cfg.thinking = Some(serde_json::json!({"type": "adaptive"}));

        assert_eq!(
            StreamTimeouts::for_config_with_env(&cfg, |_| None),
            StreamTimeouts {
                response_headers: Duration::from_secs(60),
                first_progress: Duration::from_secs(60),
                empty_stream: Duration::from_secs(60),
                stall: Duration::from_secs(90),
            },
            "adaptive Claude thinking gets the same full per-attempt pre-progress window"
        );
    }

    #[test]
    fn environment_overrides_are_independent_and_clamped() {
        let values = HashMap::from([
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
                response_headers: Duration::from_secs(60),
                first_progress: Duration::from_secs(60),
                empty_stream: Duration::from_secs(60),
                stall: Duration::from_secs(61),
            }
        );

        let invalid = HashMap::from([
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
            progress.remaining(started + Duration::from_secs(59)),
            Some(Duration::from_secs(1))
        );
        assert_eq!(progress.remaining(started + timeouts.first_progress), None);

        progress.record(started + Duration::from_secs(20));
        assert_eq!(
            progress.remaining(started + Duration::from_secs(79)),
            Some(Duration::from_secs(1))
        );
        assert_eq!(progress.remaining(started + Duration::from_secs(80)), None);
    }

    #[test]
    fn raw_stream_activity_without_real_delta_keeps_the_full_attempt_window() {
        let timeouts = timeouts(Some("high"), None);
        let started = Instant::now();
        let mut progress = StreamProgressDeadline::new(started, timeouts);

        assert_eq!(
            progress.remaining(started + Duration::from_secs(40)),
            Some(Duration::from_secs(20)),
            "before any bytes arrive, high reasoning may still wait for first progress"
        );

        let first_raw_chunk = started + Duration::from_secs(2);
        progress.record_activity(first_raw_chunk);
        assert_eq!(
            progress.remaining(started + timeouts.first_progress - Duration::from_secs(1)),
            Some(Duration::from_secs(1)),
            "raw bytes and heartbeats do not shorten the fixed pre-progress attempt window"
        );
        assert_eq!(progress.remaining(started + timeouts.first_progress), None);

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
            response_headers_timeout_error(timeouts.response_headers, Duration::from_secs(73)),
            "AI request timed out waiting for response headers after 73 seconds (per-attempt 73-second deadline)"
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
            "上游已开始流式传输，但 81 秒内没有生成有效内容，已停止本轮，请重试。"
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
            "AI request timed out waiting for response headers after 0.02 seconds (per-attempt 0.02-second deadline)"
        );
        server.join().unwrap();
    }

    #[tokio::test]
    async fn modern_claude_adaptive_thinking_is_forwarded_without_legacy_budget() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (request_tx, request_rx) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let request = read_http_request(&mut socket);
            request_tx.send(request).unwrap();

            let body =
                b"data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\ndata: [DONE]\n\n";
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            socket.write_all(headers.as_bytes()).unwrap();
            socket.write_all(body).unwrap();
            socket.flush().unwrap();
        });

        let channel: Channel<AiEvent> = Channel::new(|_| Ok(()));
        let mut cfg = config(Some("high"), None);
        cfg.base_url = format!("http://{address}");
        cfg.model = "claude-opus-4-8".into();
        cfg.thinking = Some(serde_json::json!({"type": "adaptive"}));

        ai_chat_inner(
            cfg,
            vec![serde_json::json!({"role": "user", "content": "hello"})],
            Some(vec![]),
            channel,
        )
        .await
        .unwrap();

        let request = request_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        server.join().unwrap();
        let body = request.split_once("\r\n\r\n").unwrap().1;
        let payload: serde_json::Value = serde_json::from_str(body).unwrap();
        assert_eq!(payload["reasoning_effort"], "high");
        assert_eq!(payload["thinking"]["type"], "adaptive");
        assert!(payload.get("thinking_budget").is_none());
        assert!(payload["thinking"].get("budget_tokens").is_none());
    }

    #[tokio::test]
    async fn native_anthropic_sse_relays_thinking_text_and_tool_json_in_order() {
        let frames = [
            serde_json::json!({
                "type": "message_start",
                "message": {"id": "msg_native", "type": "message"}
            }),
            serde_json::json!({
                "type": "content_block_start",
                "index": 0,
                "content_block": {"type": "thinking", "thinking": ""}
            }),
            serde_json::json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": {"type": "thinking_delta", "thinking": "check project"}
            }),
            serde_json::json!({
                "type": "content_block_start",
                "index": 1,
                "content_block": {"type": "text", "text": ""}
            }),
            serde_json::json!({
                "type": "content_block_delta",
                "index": 1,
                "delta": {"type": "text_delta", "text": "Done."}
            }),
            serde_json::json!({
                "type": "content_block_start",
                "index": 2,
                "content_block": {"type": "tool_use", "id": "toolu_native", "name": "read_file"}
            }),
            serde_json::json!({
                "type": "content_block_delta",
                "index": 2,
                "delta": {"type": "input_json_delta", "partial_json": "{\"path\":\"src/main.js\"}"}
            }),
            serde_json::json!({"type": "message_stop"}),
        ];
        let body = frames
            .iter()
            .map(|frame| {
                format!(
                    "event: {}\ndata: {}\n\n",
                    frame["type"].as_str().unwrap(),
                    frame
                )
            })
            .collect::<String>()
            .into_bytes();

        let (result, events) = run_raw_sse_body(body).await;

        result.unwrap();
        let kinds = non_metric_kinds(&events);
        assert_eq!(
            kinds,
            ["reasoning", "token", "toolCall", "toolCall", "done"]
        );
        assert_eq!(
            events
                .iter()
                .find(|event| event["kind"] == "reasoning")
                .unwrap()["delta"],
            "check project"
        );
        assert_eq!(
            events
                .iter()
                .find(|event| event["kind"] == "token")
                .unwrap()["delta"],
            "Done."
        );
        let tool_calls = events
            .iter()
            .filter(|event| event["kind"] == "toolCall")
            .collect::<Vec<_>>();
        assert_eq!(tool_calls[0]["index"], 2);
        assert_eq!(tool_calls[0]["id"], "toolu_native");
        assert_eq!(tool_calls[0]["name"], "read_file");
        assert_eq!(tool_calls[0]["arguments"], "");
        assert_eq!(tool_calls[1]["arguments"], "{\"path\":\"src/main.js\"}");
    }

    #[test]
    fn finish_reasons_are_normalized_onto_one_vocabulary() {
        // Anthropic native -> OpenAI spelling
        assert_eq!(normalize_finish_reason("max_tokens"), "length");
        assert_eq!(normalize_finish_reason("end_turn"), "stop");
        assert_eq!(normalize_finish_reason("stop_sequence"), "stop");
        assert_eq!(normalize_finish_reason("tool_use"), "tool_calls");
        // Already-OpenAI values pass through untouched
        assert_eq!(normalize_finish_reason("length"), "length");
        assert_eq!(normalize_finish_reason("tool_calls"), "tool_calls");
        assert_eq!(normalize_finish_reason("content_filter"), "content_filter");
    }

    // `finish_reason` was never parsed at all, so a response cut off by the output-token
    // limit looked identical to a complete one. That left the client's truncation guard
    // with only a JSON-shape heuristic — which cannot detect a tool-call argument object
    // that was severed at a point where it is still a valid JSON *prefix*.
    #[tokio::test]
    async fn openai_stream_reports_a_length_cutoff_to_the_client() {
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",",
            "\"function\":{\"name\":\"write_file\",\"arguments\":\"{\\\"path\\\":\\\"a.js\\\"}\"}}]}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"length\"}]}\n\n",
            "data: [DONE]\n\n",
        )
        .as_bytes()
        .to_vec();

        let (result, events) = run_raw_sse_body(body).await;

        result.unwrap();
        let finish = events
            .iter()
            .find(|event| event["kind"] == "finishReason")
            .expect("a length cutoff must reach the client");
        assert_eq!(finish["reason"], "length");
    }

    #[tokio::test]
    async fn anthropic_stream_reports_a_max_tokens_cutoff_to_the_client() {
        let frames = [
            serde_json::json!({"type": "message_start", "message": {"id": "m", "type": "message"}}),
            serde_json::json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": {"type": "text_delta", "text": "partial"}
            }),
            serde_json::json!({"type": "message_delta", "delta": {"stop_reason": "max_tokens"}}),
            serde_json::json!({"type": "message_stop"}),
        ];
        let body = frames
            .iter()
            .map(|frame| format!("event: {}\ndata: {}\n\n", frame["type"].as_str().unwrap(), frame))
            .collect::<String>()
            .into_bytes();

        let (result, events) = run_raw_sse_body(body).await;

        result.unwrap();
        let finish = events
            .iter()
            .find(|event| event["kind"] == "finishReason")
            .expect("Anthropic's max_tokens must reach the client");
        assert_eq!(finish["reason"], "length", "normalized to the OpenAI spelling");
    }

    #[tokio::test]
    async fn explicit_stream_options_rejection_is_not_replayed() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (request_tx, request_rx) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut first, _) = listener.accept().unwrap();
            let first_request = read_http_request(&mut first);
            let error_body =
                r#"{"error":{"message":"Unsupported parameter: stream_options is not supported"}}"#;
            let error_response = format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                error_body.len(),
                error_body
            );
            first.write_all(error_response.as_bytes()).unwrap();
            first.flush().unwrap();

            listener.set_nonblocking(true).unwrap();
            let deadline = Instant::now() + Duration::from_millis(150);
            let mut served = 1;
            while Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut replay, _)) => {
                        let _ = read_http_request(&mut replay);
                        served += 1;
                        break;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("gateway test listener failed: {error}"),
                }
            }
            request_tx.send((first_request, served)).unwrap();
        });

        let channel: Channel<AiEvent> = Channel::new(|_| Ok(()));
        let mut cfg = config(None, None);
        cfg.base_url = format!("http://{address}");
        let error = ai_chat_inner(
            cfg,
            vec![serde_json::json!({"role": "user", "content": "hello"})],
            Some(vec![]),
            channel,
        )
        .await
        .unwrap_err();

        let (first_request, served) = request_rx.recv().unwrap();
        assert!(first_request.contains("\"stream_options\""));
        assert!(error.contains("stream_options is not supported"));
        assert_eq!(served, 1);
        server.join().unwrap();
    }

    #[tokio::test]
    async fn ordinary_client_error_does_not_retry_the_request() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (count_tx, count_rx) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut first, _) = listener.accept().unwrap();
            let _ = read_http_request(&mut first);
            let body = r#"{"error":{"message":"invalid api key"}}"#;
            let response = format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            first.write_all(response.as_bytes()).unwrap();
            first.flush().unwrap();

            listener.set_nonblocking(true).unwrap();
            let deadline = Instant::now() + Duration::from_millis(150);
            let mut served = 1;
            while Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut socket, _)) => {
                        let _ = read_http_request(&mut socket);
                        served += 1;
                        break;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("gateway test listener failed: {error}"),
                }
            }
            count_tx.send(served).unwrap();
        });

        let channel: Channel<AiEvent> = Channel::new(|_| Ok(()));
        let mut cfg = config(None, None);
        cfg.base_url = format!("http://{address}");
        let error = ai_chat_inner(
            cfg,
            vec![serde_json::json!({"role": "user", "content": "hello"})],
            Some(vec![]),
            channel,
        )
        .await
        .unwrap_err();
        assert!(error.contains("invalid api key"));
        assert_eq!(count_rx.recv_timeout(Duration::from_secs(1)).unwrap(), 1);
        server.join().unwrap();
    }

    #[tokio::test]
    async fn gateway_http_error_response_is_not_retried() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (count_tx, count_rx) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let _ = socket.read(&mut [0_u8; 4096]);
            socket
                .write_all(
                    b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
            socket.flush().unwrap();
            count_tx.send(1).unwrap();
        });

        let mut cfg = config(None, None);
        cfg.base_url = format!("http://{address}");
        let response = post_chat_once(
            &reqwest::Client::new(),
            &format!("http://{address}/v1/chat/completions"),
            &cfg,
            &serde_json::json!({"model": "test", "stream": true, "messages": []}),
            Duration::from_secs(1),
            None,
        )
        .await
        .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::BAD_GATEWAY);
        assert_eq!(count_rx.recv_timeout(Duration::from_secs(1)).unwrap(), 1);
        server.join().unwrap();
    }

    #[tokio::test]
    async fn gateway_response_header_timeout_is_single_shot() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (count_tx, count_rx) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let _ = socket.read(&mut [0_u8; 4096]);
            std::thread::sleep(Duration::from_millis(35));
            drop(socket);

            listener.set_nonblocking(true).unwrap();
            let deadline = Instant::now() + Duration::from_millis(100);
            let mut served = 1;
            while Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut replay, _)) => {
                        let _ = replay.read(&mut [0_u8; 4096]);
                        served += 1;
                        break;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("gateway test listener failed: {error}"),
                }
            }
            count_tx.send(served).unwrap();
        });

        let mut cfg = config(None, None);
        cfg.base_url = format!("http://{address}");
        let started = Instant::now();
        let error = post_chat_once(
            &reqwest::Client::new(),
            &format!("http://{address}/v1/chat/completions"),
            &cfg,
            &serde_json::json!({"model": "test", "stream": true, "messages": []}),
            Duration::from_millis(15),
            None,
        )
        .await
        .unwrap_err();
        assert!(error.contains("timed out waiting for response headers"));
        assert!(
            started.elapsed() < Duration::from_millis(250),
            "response-header wait exceeded its configured bound"
        );
        assert_eq!(count_rx.recv_timeout(Duration::from_secs(1)).unwrap(), 1);
        server.join().unwrap();
    }

    #[tokio::test]
    async fn response_header_deadline_is_applied_once() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let read = socket.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..read]).to_ascii_lowercase();
            assert!(request.contains("x-ide-response-deadline-ms:"));
            std::thread::sleep(Duration::from_millis(100));
        });

        let mut cfg = config(None, None);
        cfg.base_url = format!("http://{address}");
        let started = Instant::now();
        let error = post_chat_once(
            &reqwest::Client::new(),
            &format!("http://{address}/v1/chat/completions"),
            &cfg,
            &serde_json::json!({"model": "test", "stream": true, "messages": []}),
            Duration::from_millis(80),
            None,
        )
        .await
        .unwrap_err();
        let elapsed = started.elapsed();
        assert!(error.contains("timed out waiting for response headers"));
        assert!(
            elapsed >= Duration::from_millis(70) && elapsed < Duration::from_millis(150),
            "response-header deadline was not applied once: {elapsed:?}"
        );
        server.join().unwrap();
    }

    #[tokio::test]
    async fn cancellation_interrupts_response_header_wait() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let _ = socket.read(&mut [0_u8; 4096]);
            std::thread::sleep(Duration::from_millis(200));
        });
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_task = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            cancel_task.store(true, Ordering::SeqCst);
        });

        let cfg = config(None, None);
        let started = Instant::now();
        let error = post_chat_once(
            &reqwest::Client::new(),
            &format!("http://{address}/v1/chat/completions"),
            &cfg,
            &serde_json::json!({"model": "test", "stream": true, "messages": []}),
            Duration::from_secs(1),
            Some(cancel.as_ref()),
        )
        .await
        .unwrap_err();
        assert_eq!(error, CANCELLED_AI_REQUEST);
        assert!(
            started.elapsed() < Duration::from_millis(180),
            "cancel did not interrupt the header wait"
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
        body.extend_from_slice(
            b"\"}}]}

data: [DONE]

",
        );

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
    // Register cancellation before DNS/connect/header wait. Previously registration
    // happened only after headers arrived, so Stop could not interrupt the exact hang
    // this watchdog is meant to bound.
    let cancel_id = cancellation_id(&config);
    let cancel_flag = cancel_id.as_deref().map(register_cancel);
    let _cancel_guard = cancel_id.map(CancelGuard);
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
    // 回发上一轮的前缀引用。网关据此从 Redis 取回历史摘要，客户端就只需要上传新增
    // 消息 —— 这是 2m/5m 档能真正达到的唯一途径。
    if let Some(tok) = config
        .mc_prefix
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.len() <= 200)
    {
        payload["mc_prefix"] = serde_json::json!(tok);
        if let Some(covered) = config.mc_prefix_covered.filter(|covered| *covered > 0) {
            payload["mc_prefix_covered"] = serde_json::json!(covered);
        }
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
    if let Some(ref thinking) = config.thinking {
        payload["thinking"] = thinking.clone();
    }
    if let Some(ref thinking_config) = config.thinking_config {
        payload["thinking_config"] = thinking_config.clone();
    }

    // Prompt-cache breakpoints are route capabilities. The Michael gateway adds them only after
    // selecting a native Anthropic connection; the desktop does not guess from model names or
    // mutate the stable prefix.

    let client = &*HTTP;
    let resp_result = post_chat_once(
        client,
        &url,
        &config,
        &payload,
        timeouts.response_headers,
        cancel_flag.as_deref(),
    )
    .await;
    let resp = match resp_result {
        Ok(response) => response,
        Err(error) if error == CANCELLED_AI_REQUEST => {
            let _ = on_event.send(AiEvent::Done);
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    if !resp.status().is_success() {
        let status = resp.status();
        let text = read_ai_error_body(resp).await;
        let message = format_ai_http_error(status, &text);
        let _ = on_event.send(AiEvent::Error {
            message: message.clone(),
        });
        return Err(message);
    }
    send_stream_metric(&on_event, stream_started, "responseHeaders", None);

    // 网关本轮签发的前缀引用。必须在消费 body 之前把响应头读出来。前端存下来，下一轮
    // 通过 config.mcPrefix 回发 —— 没有这一步，网关每轮签的令牌都进了垃圾桶，客户端
    // 只能整份上传历史。
    if let Some(tok) = resp
        .headers()
        .get("x-michael-compression-prefix")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.len() <= 200)
        .map(str::to_string)
    {
        // 覆盖条数缺失或解析失败时按 0 处理：宁可这一轮整份重传，也不能凭一个错的
        // 条数去裁历史 —— 裁错了模型收到的是错位的上下文，而且不会有任何报错。
        let covered = resp
            .headers()
            .get("x-michael-compression-covered")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(0);
        if covered > 0 {
            let _ = on_event.send(AiEvent::CompressionPrefix {
                token: tok,
                covered,
            });
        }
    }

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
    let mut progress = StreamProgressDeadline::new(stream_started, timeouts);
    let mut raw_stream_bytes: u64 = 0;
    let mut sent_first_chunk_metric = false;
    let mut sent_first_progress_metric = false;
    // A native Anthropic `tool_use` sends identity in `content_block_start` and
    // JSON fragments later in `input_json_delta`. Retain it so every fragment is
    // a complete ToolCall event for both the agent and ordinary-chat consumers.
    let mut native_anthropic_tools: HashMap<u32, (String, String)> = HashMap::new();
    // 首个有效输出前持续上报字节心跳：只发一次 firstChunk 会让前端永远显示首包字节数，
    // 用户无法区分"上游断了"和"模型还在深思/排队"（prefill 阶段不吐任何 token）。
    let mut last_bytes_metric_at = Instant::now();
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
                    message: "连接中断（网络波动），已保留生成的部分。".to_string(),
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
        // 只统计字节；activity 的判定放到 SSE 行解析处——`: ping` 心跳注释和
        // 空 role 预热帧不能算"开始输出"，否则 first_progress 大窗会被降级成 empty_stream 小窗。
        raw_stream_bytes = raw_stream_bytes.saturating_add(chunk.len() as u64);
        if !sent_first_chunk_metric {
            sent_first_chunk_metric = true;
            last_bytes_metric_at = Instant::now();
            send_stream_metric(
                &on_event,
                stream_started,
                "firstChunk",
                Some(raw_stream_bytes),
            );
        } else if !sent_first_progress_metric
            && last_bytes_metric_at.elapsed() >= Duration::from_secs(3)
        {
            // 心跳注释/预热帧也是字节：定期把累计字节数送给前端，停顿提示才能说清
            // "连接活着、模型还没开口"。不影响任何超时判定。
            last_bytes_metric_at = Instant::now();
            send_stream_metric(
                &on_event,
                stream_started,
                "streaming",
                Some(raw_stream_bytes),
            );
        }
        buf.extend_from_slice(&chunk);

        // Server-sent events are newline-delimited `data: {...}` lines. Scan with a
        // cursor and compact once per network chunk; draining the Vec for every SSE
        // line repeatedly shifted the remaining bytes when providers emitted a burst.
        let mut consumed = 0usize;
        while let Some(relative_pos) = buf[consumed..].iter().position(|&b| b == b'\n') {
            let line_end = consumed + relative_pos;
            let line_bytes = &buf[consumed..=line_end];
            consumed = line_end + 1;
            let line = match std::str::from_utf8(line_bytes) {
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
            progress.record_activity(Instant::now()); // 真正的 data 帧才算上游开始流式输出
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
                // 思考 token 的字段名各家不一样，全都认一遍：
                //   OpenAI / 兼容渠道 → completion_tokens_details.reasoning_tokens
                //   Anthropic 原生    → output_tokens_details.reasoning_tokens
                //   部分聚合渠道会平铺成顶层 reasoning_tokens
                let reasoning = usage["completion_tokens_details"]["reasoning_tokens"]
                    .as_u64()
                    .or_else(|| usage["output_tokens_details"]["reasoning_tokens"].as_u64())
                    .or_else(|| usage["reasoning_tokens"].as_u64())
                    .unwrap_or(0);
                let thinking_chars = usage["thinking_chars"].as_u64().unwrap_or(0);
                if prompt > 0 || completion > 0 {
                    let _ = on_event.send(AiEvent::Usage {
                        prompt_tokens: prompt as u32,
                        completion_tokens: completion as u32,
                        cached_tokens: cached as u32,
                        cache_creation_tokens: cache_creation as u32,
                        reasoning_tokens: reasoning as u32,
                        thinking_chars: thinking_chars as u32,
                    });
                }
            }
            // Direct Anthropic Messages API streaming uses native event objects.
            // The gateway generally normalizes them to OpenAI chunks, but direct
            // routes must not discard visible thinking/text merely because they
            // bypass that normalization. `message_stop` is Anthropic's [DONE].
            if v["type"].as_str() == Some("message_stop") {
                send_stream_metric(&on_event, stream_started, "done", Some(raw_stream_bytes));
                let _ = on_event.send(AiEvent::Done);
                return Ok(());
            }
            if native_anthropic_event_has_real_progress(&v) && !sent_first_progress_metric {
                progress.record(Instant::now());
                sent_first_progress_metric = true;
                send_stream_metric(
                    &on_event,
                    stream_started,
                    "firstProgress",
                    Some(raw_stream_bytes),
                );
            } else if native_anthropic_event_has_real_progress(&v) {
                progress.record(Instant::now());
            }
            match v["type"].as_str() {
                Some("content_block_start")
                    if v["content_block"]["type"].as_str() == Some("tool_use") =>
                {
                    if let Some(index) = v["index"].as_u64().and_then(|raw| u32::try_from(raw).ok())
                    {
                        let id = v["content_block"]["id"].as_str().unwrap_or("").to_string();
                        let name = v["content_block"]["name"]
                            .as_str()
                            .unwrap_or("")
                            .to_string();
                        native_anthropic_tools.insert(index, (id.clone(), name.clone()));
                        if !id.is_empty() || !name.is_empty() {
                            let _ = on_event.send(AiEvent::ToolCall {
                                index,
                                id,
                                name,
                                arguments: String::new(),
                            });
                        }
                    }
                }
                Some("content_block_delta") => match v["delta"]["type"].as_str() {
                    Some("thinking_delta") => {
                        if let Some(thinking) = v["delta"]["thinking"]
                            .as_str()
                            .filter(|text| !text.is_empty())
                        {
                            let _ = on_event.send(AiEvent::Reasoning {
                                delta: thinking.to_string(),
                            });
                        }
                    }
                    Some("text_delta") => {
                        if let Some(text) =
                            v["delta"]["text"].as_str().filter(|text| !text.is_empty())
                        {
                            let _ = on_event.send(AiEvent::Token {
                                delta: text.to_string(),
                            });
                        }
                    }
                    Some("input_json_delta") => {
                        if let (Some(index), Some(arguments)) = (
                            v["index"].as_u64().and_then(|raw| u32::try_from(raw).ok()),
                            v["delta"]["partial_json"]
                                .as_str()
                                .filter(|text| !text.is_empty()),
                        ) {
                            let (id, name) = native_anthropic_tools
                                .get(&index)
                                .cloned()
                                .unwrap_or_default();
                            let _ = on_event.send(AiEvent::ToolCall {
                                index,
                                id,
                                name,
                                arguments: arguments.to_string(),
                            });
                        }
                    }
                    _ => {}
                },
                // Anthropic reports why it stopped on `message_delta`, using its own
                // vocabulary (`max_tokens` / `end_turn` / `tool_use` / `stop_sequence`).
                Some("message_delta") => {
                    if let Some(reason) = v["delta"]["stop_reason"]
                        .as_str()
                        .filter(|reason| !reason.is_empty())
                    {
                        let _ = on_event.send(AiEvent::FinishReason {
                            reason: normalize_finish_reason(reason).to_string(),
                        });
                    }
                }
                _ => {}
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
            // Sibling of `delta`, not part of it: it arrives on the final chunk, where
            // `delta` is typically `{}`. `length` here is the provider telling us the
            // response was cut off — the client rejects any tool call in that turn.
            if let Some(reason) = v["choices"][0]["finish_reason"]
                .as_str()
                .filter(|reason| !reason.is_empty())
            {
                let _ = on_event.send(AiEvent::FinishReason {
                    reason: normalize_finish_reason(reason).to_string(),
                });
            }
        }
        if consumed > 0 {
            buf.drain(..consumed);
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
    let max = 800_000usize;
    let mut response = resp;
    let mut bytes =
        Vec::with_capacity(response.content_length().unwrap_or(0).min(max as u64) as usize);
    while bytes.len() < max {
        let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? else {
            break;
        };
        let remaining = max - bytes.len();
        bytes.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        if chunk.len() >= remaining {
            break;
        }
    }
    let raw = String::from_utf8_lossy(&bytes).to_string();

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
const MAX_SEARCH_RESPONSE_BYTES: usize = 800_000;

/// A search should be fast; bound it much tighter than a page fetch. A source that hasn't
/// answered in 8s is treated as blocked/dead so the race can settle on a working one.
fn search_client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(8))
        .build()
        .ok()
}

/// Search result pages are only parsed for a few short result cards. Do not let
/// an upstream error page or an accidentally unbounded response consume the
/// process heap before the 8-second search deadline can take effect.
async fn read_search_response_text(response: reqwest::Response) -> Result<String, reqwest::Error> {
    if !response.status().is_success() {
        return Ok(String::new());
    }

    let mut response = response;
    let capacity = response
        .content_length()
        .unwrap_or_default()
        .min(MAX_SEARCH_RESPONSE_BYTES as u64) as usize;
    let mut bytes = Vec::with_capacity(capacity);
    while bytes.len() < MAX_SEARCH_RESPONSE_BYTES {
        let Some(chunk) = response.chunk().await? else {
            break;
        };
        let remaining = MAX_SEARCH_RESPONSE_BYTES - bytes.len();
        bytes.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        if chunk.len() >= remaining {
            break;
        }
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
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
        Ok(r) => read_search_response_text(r)
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
        Ok(r) => read_search_response_text(r)
            .await
            .map(|h| parse_bing(&h))
            .unwrap_or_default(),
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
        Ok(r) => read_search_response_text(r)
            .await
            .map(|h| parse_google(&h))
            .unwrap_or_default(),
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
        Ok(r) => read_search_response_text(r)
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
/// `pub(crate)` 而非私有：knowledge.rs 的 ddg_surface_checked 在被反爬拦下时
/// 降级到这里 —— 那边是裸 reqwest，穿不过 202 挑战页，而这里是真 Chrome。
pub(crate) async fn browser_render_search(q: &str) -> Vec<(String, String, String)> {
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
        let mut end = (abs + 4000).min(html.len());
        // A byte limit can land in the middle of a multi-byte character. Check
        // the original string before slicing; slicing first is itself a panic.
        while end > abs && !html.is_char_boundary(end) {
            end -= 1;
        }
        let region = &html[abs..end];
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

#[cfg(test)]
mod search_result_parser_tests {
    use super::{parse_bing, read_search_response_text, MAX_SEARCH_RESPONSE_BYTES};
    use std::io::{Read, Write};

    #[test]
    fn bing_region_limit_never_slices_inside_utf8() {
        let mut html = String::from(
            r#"<li class="b_algo"><h2><a href="https://example.com">中文标题</a></h2><p>中文摘要</p>"#,
        );
        let result_start = html.find("b_algo").unwrap();
        let byte_limit = result_start + 4000;
        html.push_str(&"x".repeat(byte_limit - 1 - html.len()));
        html.push('中');
        html.push_str("</li>");

        assert!(!html.is_char_boundary(byte_limit));
        let results = parse_bing(&html);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "中文标题");
        assert_eq!(results[0].1, "https://example.com");
        assert_eq!(results[0].2, "中文摘要");
    }

    #[tokio::test]
    async fn search_response_read_stops_at_the_page_cap() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let _ = socket.read(&mut [0_u8; 4096]);
            let body = "x".repeat(MAX_SEARCH_RESPONSE_BYTES + 64);
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = socket.write_all(header.as_bytes());
            let _ = socket.write_all(body.as_bytes());
        });

        let response = reqwest::Client::new()
            .get(format!("http://{address}/"))
            .send()
            .await
            .unwrap();
        let body = read_search_response_text(response).await.unwrap();
        assert_eq!(body.len(), MAX_SEARCH_RESPONSE_BYTES);
        server.join().unwrap();
    }
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
