use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use rand::Rng;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

/// Shared, pooled HTTP client for upstream model calls. Building a fresh
/// `reqwest::Client` per request (the old behaviour) forced a brand-new TCP+TLS
/// handshake to the provider on every call — a large chunk of the "feels slow"
/// latency, and it compounds badly for an agent firing many sequential requests.
/// One pooled client keeps connections warm (keep-alive), so only the first call
/// to a host pays the handshake. No global timeout: streamed chat responses are
/// open-ended; only the connect phase is bounded (per-request timeouts are added
/// for the non-streaming calls that need them).
static GW_HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(16)
        .tcp_keepalive(Duration::from_secs(60))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});

/// Chat streams use HTTP/1.1 deliberately. Some aggregator/CDN combinations leave an
/// individual HTTP/2 stream stuck before response headers while the shared connection
/// remains established. Reusing that connection makes every retry hit the same poisoned
/// transport. HTTP/1.1 isolates in-flight requests; cancelling a header-stalled request
/// drops that connection, and the retry below can open a genuinely fresh one.
fn build_chat_http_client(pool_idle_per_host: usize) -> reqwest::Client {
    reqwest::Client::builder()
        .http1_only()
        .connect_timeout(Duration::from_secs(5))
        .pool_idle_timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(pool_idle_per_host)
        .tcp_keepalive(Duration::from_secs(30))
        .tcp_nodelay(true)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

static GW_CHAT_HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| build_chat_http_client(8));

const CHAT_UPSTREAM_MAX_ATTEMPTS_PER_ROUTE: u32 = 2;
const CHAT_UPSTREAM_ROUTE_COOLDOWN: Duration = Duration::from_secs(20);

/// What the IDE waits for response headers before it gives up. Reasoning effort
/// never changes this transport-health deadline; deeper thinking gets extra time
/// only after the HTTP response has opened.
/// Only read by the test that enforces the coupling — the value's job is to make the
/// client's deadline visible here so nobody widens the gateway budget past it.
#[cfg_attr(not(test), allow(dead_code))]
const CLIENT_HEADER_TIMEOUT: Duration = Duration::from_secs(15);
/// This supplier does not flush HTTP headers until its first SSE event, so response-header
/// latency includes model prefill. Production measurements put small chat near 2.3-3.9s and a
/// normal Agent/tool request at 5.5s. Use request-aware ceilings instead of treating 4s as a
/// transport failure and cancelling healthy Agent turns.
/// 5s 太紧：实测小聊天最慢就到 3.9s，只剩 1.1s 余量，一次比平常慢一点的 prefill 就
/// 会被当成传输故障掐掉。7s 给到约 1.8 倍余量，而总时长仍由 ROUTE_BUDGET(12s) 兜住，
/// 所以"动不动等 47s"那个问题不会因此回来。
const STANDARD_MAX_HEADER_WAIT: Duration = Duration::from_secs(7);
const AGENT_MAX_HEADER_WAIT: Duration = Duration::from_secs(8);
const DEEP_MAX_HEADER_WAIT: Duration = Duration::from_secs(10);
/// 对冲式首次等待：zyz 类中转的【第一条连接】经常拒 headers，换新连接后立刻就通
///（实测每次白等 7-8s，用户体感"卡半天→唰一下全出来"）。首次尝试只等 4s 就果断
/// 换连接；重试给满额，健康但慢的供应商由第二次兜底，不会误杀。仅在同路线还有
/// 重试机会时启用（多路线单尝试场景不压缩，避免慢供应商被连环跳过）。
const FIRST_ATTEMPT_HEADER_WAIT: Duration = Duration::from_secs(4);
const MAX_ERROR_BODY_WAIT: Duration = Duration::from_secs(2);
const ROUTE_BUDGET: Duration = Duration::from_secs(12);
const CLIENT_DEADLINE_MARGIN: Duration = Duration::from_millis(750);
const FAST_HEADER_RETRY_DELAY: Duration = Duration::from_millis(120);
const RESPONSE_DEADLINE_HEADER: &str = "x-ide-response-deadline-ms";

/// Total time the gateway may spend hunting for a working upstream route before it
/// must answer the client.
///
/// This has to stay comfortably under the client's own header timeout. When it
/// didn't, the client gave up first and fast-retried, and each retry opened a fresh
/// gateway request with its own set of upstream calls — a multiplying storm of
/// `/v1/messages` requests rather than one failure the user could read.
fn route_budget_for(_deep_thinking: bool) -> Duration {
    ROUTE_BUDGET
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

fn route_budget_with_client_deadline(
    deep_thinking: bool,
    client_deadline_ms: Option<u64>,
    now_ms: u64,
) -> Duration {
    let fallback = route_budget_for(deep_thinking);
    let Some(deadline_ms) = client_deadline_ms else {
        return fallback;
    };
    let remaining = Duration::from_millis(deadline_ms.saturating_sub(now_ms));
    fallback.min(remaining.saturating_sub(CLIENT_DEADLINE_MARGIN))
}

fn route_budget_for_headers(headers: &HeaderMap, deep_thinking: bool) -> Duration {
    let client_deadline_ms = headers
        .get(RESPONSE_DEADLINE_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok());
    route_budget_with_client_deadline(deep_thinking, client_deadline_ms, unix_time_ms())
}

/// Does this request ask the model to think before answering?
///
/// Thinking moves work into prefill, and this supplier withholds HTTP headers until its
/// first SSE event, so a thinking request legitimately takes longer to produce headers
/// than a plain one. That is what the deep budget (10s headers / 600s idle) exists for.
///
/// All three wire shapes have to be recognised, because they are not interchangeable
/// across models and the gateway emits different ones for different families:
///   * `reasoning_effort: max|xhigh`   — OpenAI-shaped request, deepest dials
///   * `thinking.budget_tokens > 0`    — Claude 3.7 / 4.6 explicit-budget form
///   * `thinking.type: adaptive`       — Claude 4.7+ / 5 / Fable / Mythos (NO budget field)
///
/// Missing the adaptive arm is a silent downgrade, not a visible error: the request keeps
/// working, just against a budget sized for a non-thinking turn, and fails as a 504 under
/// load. Keep this in sync with `anthropic_thinking`.
fn request_is_deep_thinking(body: &serde_json::Value) -> bool {
    let effort_is_deep = body
        .get("reasoning_effort")
        .and_then(|v| v.as_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("max") || e.eq_ignore_ascii_case("xhigh"));
    let explicit_budget = body
        .get("thinking")
        .and_then(|t| t.get("budget_tokens"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        > 0;
    let thinking_on = body
        .get("thinking")
        .and_then(|t| t.get("type"))
        .and_then(|v| v.as_str())
        .is_some_and(|t| t == "adaptive" || t == "enabled");
    effort_is_deep || explicit_budget || thinking_on
}

fn max_header_wait_for_request(deep_thinking: bool, agentic: bool) -> Duration {
    if deep_thinking {
        DEEP_MAX_HEADER_WAIT
    } else if agentic {
        AGENT_MAX_HEADER_WAIT
    } else {
        STANDARD_MAX_HEADER_WAIT
    }
}

async fn wait_for_upstream_retry(delay: Duration, deadline: Instant) -> bool {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() || delay >= remaining {
        return false;
    }
    tokio::time::sleep(delay).await;
    true
}

static CHAT_UPSTREAM_ROUTE_COOLDOWNS: LazyLock<Mutex<HashMap<uuid::Uuid, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
// 中转丢块自愈：jgy 等聚合中转在深思考超过 ~7.5K token 后会丢掉后面的 text/tool_use
// 块并谎报 end_turn（对照实验：budget 6000 → thinking+text+tool_use 正常；budget 24000
// → 只回 thinking 就 end_turn；官方 API 绝不会思考完直接收尾）。检出签名后该线路记
// 30 分钟"思考钳位"，期间 budget_tokens 压到实测安全值；健康线路不受影响，到期自动解除。
const THINKING_CLIP_COOLDOWN: Duration = Duration::from_secs(30 * 60);
const THINKING_CLIP_SAFE_BUDGET: i64 = 6000;
/// Learned header latency per upstream route, so the first-attempt cutover is measured
/// against how THIS route actually behaves instead of one global guess.
///
/// The fixed 4s first-attempt wait taxes every request on a reliably-slow route: the
/// route needs ~2s, we wait 4s, cut over, and pay again. Meanwhile a genuinely healthy
/// route that occasionally takes 5s gets killed for no reason. Both are the same bug —
/// a constant standing in for a measurement.
///
/// Stores an exponentially-weighted mean of SUCCESSFUL header latencies plus a count of
/// consecutive stalls. Bounded: one small entry per route id, no growth with traffic.
#[derive(Clone, Copy, Debug)]
struct RouteHeaderStats {
    /// EWMA of successful header latency, milliseconds.
    mean_ms: f64,
    /// Successful observations folded in so far (saturating).
    samples: u32,
    /// Consecutive first-attempt stalls; resets on any success.
    stall_streak: u32,
}

static ROUTE_HEADER_STATS: LazyLock<Mutex<HashMap<uuid::Uuid, RouteHeaderStats>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Fold a successful header latency into the route's memory.
fn record_header_success(route: uuid::Uuid, header_ms: u128) {
    let Ok(mut map) = ROUTE_HEADER_STATS.lock() else { return };
    let entry = map.entry(route).or_insert(RouteHeaderStats {
        mean_ms: header_ms as f64,
        samples: 0,
        stall_streak: 0,
    });
    // 0.3 weight: adapts within a handful of requests when a provider degrades, without
    // letting one outlier swing the cutover.
    entry.mean_ms = entry.mean_ms * 0.7 + (header_ms as f64) * 0.3;
    entry.samples = entry.samples.saturating_add(1);
    entry.stall_streak = 0;
}

/// Record that this route stalled before sending headers.
fn record_header_stall(route: uuid::Uuid) {
    let Ok(mut map) = ROUTE_HEADER_STATS.lock() else { return };
    let entry = map.entry(route).or_insert(RouteHeaderStats {
        mean_ms: 0.0,
        samples: 0,
        stall_streak: 0,
    });
    entry.stall_streak = entry.stall_streak.saturating_add(1);
}

/// How long to wait for headers on the FIRST attempt of a route that still has retries.
///
/// Unknown routes keep the previous fixed behaviour. Once a route has a real baseline,
/// wait a multiple of its own mean — slow-but-honest providers stop being killed, and
/// routes that habitually hang get cut over well before the old flat 4s.
fn adaptive_first_attempt_wait(route: uuid::Uuid, fallback: Duration) -> Duration {
    const MIN_WAIT: Duration = Duration::from_millis(900);
    const MAX_WAIT: Duration = Duration::from_secs(9);
    let Ok(map) = ROUTE_HEADER_STATS.lock() else { return fallback };
    let Some(stats) = map.get(&route) else { return fallback };
    // Need a few observations before trusting the mean.
    if stats.samples < 3 {
        return fallback;
    }
    // A route that just stalled repeatedly gets a shorter leash, so the cutover to a
    // fresh connection — which is what actually works on these relays — happens sooner.
    let multiplier = if stats.stall_streak >= 2 { 1.5 } else { 2.5 };
    let target_ms = (stats.mean_ms * multiplier).round().max(0.0) as u64;
    Duration::from_millis(target_ms).clamp(MIN_WAIT, MAX_WAIT)
}

static THINKING_CLIP_ROUTES: LazyLock<Mutex<HashMap<uuid::Uuid, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
/// The i18n pack cache is bounded because each entry holds a full ~630KB response
/// body and the key is a hash of (locale, entries) — a caller who varies one
/// character misses every time, so an unbounded map OOMs the gateway before the
/// upstream bill even becomes the bigger problem.
const I18N_PACK_CACHE_MAX_ENTRIES: usize = 64;
const I18N_PACK_CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
static I18N_PACK_CACHE: LazyLock<Mutex<HashMap<String, (Instant, serde_json::Value)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Insert into the pack cache, evicting expired entries first and then the oldest
/// ones until the map is back under its cap.
fn i18n_pack_cache_put(key: String, body: serde_json::Value) {
    let Ok(mut cache) = I18N_PACK_CACHE.lock() else {
        return;
    };
    let now = Instant::now();
    cache.retain(|_, (at, _)| now.duration_since(*at) < I18N_PACK_CACHE_TTL);
    while cache.len() >= I18N_PACK_CACHE_MAX_ENTRIES {
        let oldest = cache
            .iter()
            .min_by_key(|(_, (at, _))| *at)
            .map(|(k, _)| k.clone());
        match oldest {
            Some(k) => {
                cache.remove(&k);
            }
            None => break,
        }
    }
    cache.insert(key, (now, body));
}

/// Read a still-fresh cached pack.
fn i18n_pack_cache_get(key: &str) -> Option<serde_json::Value> {
    let cache = I18N_PACK_CACHE.lock().ok()?;
    let (at, body) = cache.get(key)?;
    if Instant::now().duration_since(*at) >= I18N_PACK_CACHE_TTL {
        return None;
    }
    Some(body.clone())
}

/// Per-user budget on cache-missing i18n pack generations. Sliding window, in
/// memory — this is an abuse fuse, not accounting, so it does not need to survive
/// a restart. A real UI needs a few packs per language; anything approaching this
/// ceiling is a loop or an attack.
const I18N_PACK_BUDGET_WINDOW: Duration = Duration::from_secs(60 * 60);
const I18N_PACK_BUDGET_PER_WINDOW: usize = 40;
static I18N_PACK_BUDGET: LazyLock<Mutex<HashMap<uuid::Uuid, Vec<Instant>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 匿名调用共用的预算身份。
///
/// 已发布的客户端（0.3.15）调这个接口不带任何凭据，硬拒绝会让它们整个界面翻译失效。
/// 但这条路会花运营方的上游余额，所以匿名额度是**全局共享的一小份**，而不是每人一份
/// —— 攻击者拿不到比这更多的量，正常用户的 UI 文案又只需要很少几次就能把缓存捂热。
const I18N_PACK_ANON_IDENTITY: uuid::Uuid = uuid::Uuid::nil();
const I18N_PACK_ANON_PER_WINDOW: usize = 30;

fn i18n_pack_charge_budget(user_id: uuid::Uuid) -> Result<(), AppError> {
    let Ok(mut budget) = I18N_PACK_BUDGET.lock() else {
        return Ok(());
    };
    let now = Instant::now();
    budget.retain(|_, hits| {
        hits.retain(|at| now.duration_since(*at) < I18N_PACK_BUDGET_WINDOW);
        !hits.is_empty()
    });
    let cap = if user_id == I18N_PACK_ANON_IDENTITY {
        I18N_PACK_ANON_PER_WINDOW
    } else {
        I18N_PACK_BUDGET_PER_WINDOW
    };
    let hits = budget.entry(user_id).or_default();
    if hits.len() >= cap {
        return Err(AppError {
            status: StatusCode::TOO_MANY_REQUESTS,
            msg: "语言包生成过于频繁，请稍后再试".into(),
        });
    }
    hits.push(now);
    Ok(())
}

fn chat_upstream_retry_base_delay_ms(attempt: u32) -> u64 {
    match attempt {
        0 => 250,
        1 => 650,
        2 => 1_300,
        3 => 2_500,
        _ => 4_000,
    }
}

fn chat_upstream_retry_delay(attempt: u32) -> Duration {
    let jitter_ms = rand::thread_rng().gen_range(0..=175);
    Duration::from_millis(chat_upstream_retry_base_delay_ms(attempt) + jitter_ms)
}

fn route_cooldown_remaining(id: uuid::Uuid, now: Instant) -> Option<Duration> {
    let mut guard = CHAT_UPSTREAM_ROUTE_COOLDOWNS.lock().ok()?;
    match guard.get(&id).copied() {
        Some(until) if until > now => Some(until - now),
        Some(_) => {
            guard.remove(&id);
            None
        }
        None => None,
    }
}

fn mark_route_cooldown(id: uuid::Uuid) {
    if let Ok(mut guard) = CHAT_UPSTREAM_ROUTE_COOLDOWNS.lock() {
        guard.insert(id, Instant::now() + CHAT_UPSTREAM_ROUTE_COOLDOWN);
    }
}

fn thinking_clip_active(id: uuid::Uuid) -> bool {
    let Ok(mut guard) = THINKING_CLIP_ROUTES.lock() else {
        return false;
    };
    match guard.get(&id).copied() {
        Some(until) if until > Instant::now() => true,
        Some(_) => {
            guard.remove(&id);
            false
        }
        None => false,
    }
}

fn mark_thinking_clip(id: uuid::Uuid) {
    if let Ok(mut guard) = THINKING_CLIP_ROUTES.lock() {
        guard.insert(id, Instant::now() + THINKING_CLIP_COOLDOWN);
    }
}

/// 钳位期内把已转换好的 Anthropic 请求体思考预算压到安全值。只降不升；
/// 没有 thinking 或预算本就不超时不动。返回是否真的钳了。
fn clip_thinking_budget(upstream_body: &mut serde_json::Value) -> bool {
    let Some(budget) = upstream_body
        .pointer("/thinking/budget_tokens")
        .and_then(|v| v.as_i64())
    else {
        return false;
    };
    if budget <= THINKING_CLIP_SAFE_BUDGET {
        return false;
    }
    if let Some(thinking) = upstream_body.get_mut("thinking").and_then(|t| t.as_object_mut()) {
        thinking.insert("budget_tokens".into(), json!(THINKING_CLIP_SAFE_BUDGET));
        return true;
    }
    false
}

fn chat_upstream_attempt_suffix(route_count: usize, attempts: u32, last_status: u16) -> String {
    if route_count <= 1 {
        format!("（已请求 {attempts} 次；当前只有 1 条同模型线路；最后状态 {last_status}）")
    } else {
        format!("（已请求 {attempts} 次 / {route_count} 条同模型线路；最后状态 {last_status}）")
    }
}

fn safe_upstream_error_excerpt(low: &str) -> String {
    let mut text = low
        .replace(['\r', '\n', '\t'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for marker in ["sk-", "sk_live_", "sk-proj-"] {
        if let Some(pos) = text.find(marker) {
            let end = (pos + 48).min(text.len());
            text.replace_range(pos..end, "[redacted-key]");
        }
    }
    text.chars().take(220).collect()
}

fn upstream_failure_status(status: u16, low: &str) -> StatusCode {
    let access_failure = matches!(status, 401 | 403)
        || low.contains("forbidden")
        || low.contains("unauthorized")
        || low.contains("invalid api key")
        || low.contains("invalid_api_key")
        || low.contains("permission denied")
        || low.contains("access denied")
        || low.contains("insufficient_balance")
        || low.contains("insufficient account balance")
        || low.contains("未授权")
        || low.contains("no available")
        || low.contains("没有可用");
    if access_failure {
        StatusCode::FAILED_DEPENDENCY
    } else {
        match status {
            429 => StatusCode::TOO_MANY_REQUESTS,
            502 => StatusCode::BAD_GATEWAY,
            503 => StatusCode::SERVICE_UNAVAILABLE,
            504 => StatusCode::GATEWAY_TIMEOUT,
            // A request-shape rejection is PERMANENT: the body is wrong, and resending
            // the identical body — here or from the IDE — can only fail again. The old
            // `_ => BAD_GATEWAY` catch-all dressed these up as transient 502s, so the
            // client's own retry loop re-sent them, which is how a single malformed
            // `thinking` block turned into a route-killing storm and a frozen IDE.
            // Pass the real status through so nobody retries it.
            400 => StatusCode::BAD_REQUEST,
            413 => StatusCode::PAYLOAD_TOO_LARGE,
            422 => StatusCode::UNPROCESSABLE_ENTITY,
            _ => StatusCode::BAD_GATEWAY,
        }
    }
}

/// Wrap an upstream byte stream with an IDLE timeout: if the provider (zyz et al.)
/// goes silent mid-response for too long (it occasionally stalls a stream), we
/// gracefully END the stream instead of leaving the IDE frozen forever. The client
/// then hits EOF, finalizes whatever it has, and unblocks — far better than an
/// infinite "跑着跑着卡住" hang. Generic over the byte type so we don't need to name
/// `bytes::Bytes` directly.
#[allow(dead_code)]
fn idle_guarded_stream<B, S>(
    upstream: S,
) -> impl futures_util::Stream<Item = Result<B, std::io::Error>> + Send + 'static
where
    S: futures_util::Stream<Item = reqwest::Result<B>> + Send + 'static,
    B: Send + 'static,
{
    use futures_util::StreamExt;
    // 180s: a "thinking" model can pause far longer than 30s — it reasons silently, or
    // composes a long tool-call argument (a full-file write) the relay forwards in bursts.
    // The old 30s cut those mid-stream (→ truncated tool call → empty write "内容为空").
    // 180s still bounds a truly-hung upstream so the client eventually auto-retries.
    let idle = std::time::Duration::from_secs(180);
    let upstream = Box::pin(upstream);
    futures_util::stream::unfold(upstream, move |mut s| async move {
        match tokio::time::timeout(idle, s.next()).await {
            Ok(Some(Ok(chunk))) => Some((Ok(chunk), s)),
            // upstream finished, errored, or went idle past the timeout → end here
            _ => None,
        }
    })
}

fn admin_only(claims: &Claims) -> ApiResult<()> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    Ok(())
}

/// Normalize an OpenAI-compatible base URL: ensure it ends with a `/v1` segment
/// (so `https://gateway.example` becomes `https://gateway.example/v1`). If the
/// caller already included `/v1` (or any `/v1/...`), leave it untouched.
fn api_base(base: &str) -> String {
    let b = base.trim().trim_end_matches('/');
    if b.ends_with("/v1") || b.contains("/v1/") {
        b.to_string()
    } else {
        format!("{}/v1", b)
    }
}

#[derive(sqlx::FromRow, Clone)]
pub struct Model {
    pub id: uuid::Uuid,
    pub label: String,
    pub provider: String,
    pub base_url: String,
    pub model_id: Option<String>,
    pub api_key: String,
    pub price_cents: i64,
    pub rate: f64,
    /// USD per 1,000,000 INPUT tokens (real-API unit). 0 = not set → bill the flat `rate`.
    pub input_price: f64,
    /// USD per 1,000,000 OUTPUT tokens. 0 = not set → bill the flat `rate`.
    pub output_price: f64,
    /// Per 1M CACHE-READ tokens (cheap). 0 = not set → fall back to 0.1× input_price.
    pub cache_read_price: f64,
    /// Per 1M CACHE-CREATE/write tokens (premium). 0 = not set → fall back to 1.25× input_price.
    pub cache_create_price: f64,
    /// Optional admin blurb shown in the IDE picker's hover card.
    pub description: String,
    pub active: bool,
    pub sort: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub enabled_models: Vec<String>,
    /// Billing mode: "rate" (token×price×倍率, default) or "per_call" (flat fee/call).
    pub billing_mode: String,
    /// Flat fee per call in cents, used only when billing_mode = "per_call".
    pub per_call_cents: i64,
    /// Same fee at micro-USD resolution (1 cent = 10 000). Whole cents floored a $0.0055 fee
    /// to 1 cent, which the admin form then redisplayed as "0.010" — the value appearing to
    /// revert. Free-model billing reads this; paid billing still rounds to cents.
    pub per_call_micro_usd: i64,
    /// Friendly display-name overrides: { raw_model_id → label shown in the IDE }.
    /// The IDE still sends the raw id upstream; this only renames the picker entry.
    pub model_names: serde_json::Value,
    /// Per-MODEL price overrides: { raw_model_id → {"in": usd_per_1M, "out": usd_per_1M} }.
    /// When an entry is set (in>0 or out>0) it WINS over the built-in official catalog for
    /// that model; empty → fall back to official, then the connection-level input/output
    /// price. Lets the admin price each enabled model individually. (倍率 still applies on top.)
    pub model_prices: serde_json::Value,
    /// Per-model billing override, same shape as `model_prices`:
    ///   { "<model_id>": { "mode": "rate"|"per_call"|"free", "per_call_cents": N } }
    /// A `models` row is a CONNECTION holding many `enabled_models`, so billing_mode /
    /// per_call_cents alone could only switch a whole channel. This overrides per model.
    pub model_billing: serde_json::Value,
    /// Upstream wire protocol: "anthropic" (native /v1/messages) or "openai" (/chat/completions
    /// compat). When "anthropic", the gateway translates the OpenAI request/response ⇄ Anthropic.
    pub protocol: String,
}

/// Per-MODEL (input, output) USD/1M price override from a connection's model_prices map.
/// Returns (0.0, 0.0) when this model has no override — compute_cost then uses the built-in
/// official price, then the connection-level fallback. Admin per-model prices beat both.
fn model_price_override(model_prices: &serde_json::Value, model_id: &str) -> (f64, f64) {
    match model_prices.get(model_id) {
        Some(p) => (
            p.get("in").and_then(|v| v.as_f64()).unwrap_or(0.0).max(0.0),
            p.get("out")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
                .max(0.0),
        ),
        None => (0.0, 0.0),
    }
}

/// Effective billing for ONE model id on a connection: the per-model override when present,
/// else the connection's own mode. Returns (mode, per_call_cents, is_free).
///
/// "free" is a billing TARGET, not a price: a free model still costs whatever its mode says
/// (flat per-call, or real token cost), it is just deducted from the daily free-points pool
/// instead of quota/wallet. That keeps one cost path — no second pricing engine to drift.
fn effective_billing(model: &Model, model_id: &str) -> (String, i64, bool) {
    let (m, c, f, _micro) = effective_billing_micro(model, model_id);
    (m, c, f)
}

/// As `effective_billing`, plus the per-call fee in micro-USD when the override carries one.
/// Whole `per_call_cents` cannot express a sub-cent fee, so free models read this instead.
fn effective_billing_micro(model: &Model, model_id: &str) -> (String, i64, bool, i64) {
    let micro = model
        .model_billing
        .get(model_id)
        .and_then(|v| v.get("per_call_micro_usd"))
        .and_then(|v| v.as_i64())
        .filter(|n| *n > 0)
        .unwrap_or(0);
    let (m, c, f) = effective_billing_inner(model, model_id);
    // Fall back to the whole-cent fee so an override written before micro support still bills.
    let micro = if micro > 0 {
        micro
    } else if model.per_call_micro_usd > 0 {
        model.per_call_micro_usd
    } else {
        c.max(0) * MICRO_USD_PER_CENT
    };
    (m, c, f, micro)
}

fn effective_billing_inner(model: &Model, model_id: &str) -> (String, i64, bool) {
    let ov = model.model_billing.get(model_id);
    let mode = ov
        .and_then(|v| v.get("mode"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_lowercase())
        .filter(|s| s == "rate" || s == "per_call" || s == "free")
        .unwrap_or_else(|| model.billing_mode.clone());
    let per_call = ov
        .and_then(|v| v.get("per_call_cents"))
        .and_then(|v| v.as_i64())
        .filter(|n| *n >= 0)
        .unwrap_or(model.per_call_cents);
    let is_free = mode == "free";
    // A free model priced per call still needs a flat fee; free + per_call_cents 0 means
    // "costs nothing", which is legitimate (fully free) — the points pool simply is not
    // touched. Map free → per_call only when a fee was actually configured.
    let cost_mode = if is_free {
        if per_call > 0 { "per_call".to_string() } else { "rate".to_string() }
    } else {
        mode
    };
    (cost_mode, per_call, is_free)
}

fn route_supports_prompt_cache(model: &Model) -> bool {
    model.protocol == "anthropic"
        && std::env::var("MICHAEL_PROMPT_CACHE").ok().as_deref() != Some("0")
}

/// True for any image-GENERATION model (bills PER-IMAGE, not per-token) across vendors:
/// OpenAI gpt-image / DALL·E, Google gemini *image* (gemini-3.1-flash-image-preview),
/// gpt-4o-image, etc. Guarantees image calls never fall through to $0 token billing.
/// Text/vision models never contain these substrings, so it won't misfire on them.
fn is_image_gen_model(model_id: &str) -> bool {
    let m = model_id.to_lowercase();
    m.contains("gpt-image")
        || m.contains("dall-e")
        || m.contains("dall_e")
        || m.contains("-image")
        || m.contains("image-preview")
        || m.contains("image-generation")
}

/// Look up a friendly display name for `mid` in a connection's model_names map,
/// falling back to the raw id when there's no override.
fn display_name_for(model_names: &serde_json::Value, mid: &str) -> String {
    model_names
        .get(mid)
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| mid.to_string())
}

/// Pick the final cost for one successful upstream call based on the connection's
/// billing mode. "per_call" → flat per_call_cents (token-count independent);
/// otherwise → real token billing via compute_cost. Centralized so EVERY billing
/// site (chat stream/non-stream, legacy chat, responses) stays consistent.
#[allow(clippy::too_many_arguments)]
fn resolve_cost(
    billing_mode: &str,
    per_call_cents: i64,
    usage: Option<&serde_json::Value>,
    model_id: &str,
    rate: f64,
    admin_in: f64,
    admin_out: f64,
    cache_read_price: f64,
    cache_create_price: f64,
    model_in: f64,
    model_out: f64,
) -> i64 {
    if billing_mode == "per_call" {
        let c = per_call_cents.max(0);
        tracing::info!("[billing] model={} mode=per_call → {}¢", model_id, c);
        return c;
    }
    compute_cost(
        usage,
        model_id,
        rate,
        admin_in,
        admin_out,
        cache_read_price,
        cache_create_price,
        model_in,
        model_out,
    )
}

fn usage_is_authoritative(usage: Option<&serde_json::Value>) -> bool {
    let Some(usage) = usage.filter(|value| value.is_object()) else {
        return false;
    };
    let has_nonnegative = |keys: &[&str]| {
        keys.iter().any(|key| {
            usage
                .get(*key)
                .and_then(|value| value.as_i64())
                .is_some_and(|value| value >= 0)
        })
    };
    has_nonnegative(&["prompt_tokens", "input_tokens"])
        && has_nonnegative(&["completion_tokens", "output_tokens"])
}

fn valid_ide_request_id(value: &str) -> bool {
    (8..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn ide_request_id(headers: &HeaderMap) -> ApiResult<Option<String>> {
    let Some(value) = headers.get("x-ide-request-id") else {
        return Ok(None);
    };
    let value = value
        .to_str()
        .map_err(|_| AppError::bad("x-ide-request-id 无效"))?
        .trim();
    if !valid_ide_request_id(value) {
        return Err(AppError::bad("x-ide-request-id 无效"));
    }
    Ok(Some(value.to_string()))
}

fn allowed_ids(m: &Model) -> Vec<String> {
    if !m.enabled_models.is_empty() {
        return m.enabled_models.clone();
    }
    match &m.model_id {
        Some(s) if !s.is_empty() => vec![s.clone()],
        _ => vec![],
    }
}

#[derive(Deserialize)]
pub struct I18nPackReq {
    pub locale: String,
    pub source_locale: Option<String>,
    pub entries: HashMap<String, String>,
}

fn i18n_pack_cache_key(locale: &str, entries: &HashMap<String, String>) -> String {
    let mut pairs: Vec<_> = entries.iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(b.0));
    let mut h = std::collections::hash_map::DefaultHasher::new();
    locale.hash(&mut h);
    for (k, v) in pairs {
        k.hash(&mut h);
        v.hash(&mut h);
    }
    format!("{}:{:016x}", locale, h.finish())
}

fn json_object_from_model_text(text: &str) -> Option<serde_json::Value> {
    let mut s = text.trim();
    if s.starts_with("```") {
        if let Some(pos) = s.find('\n') {
            s = &s[pos + 1..];
        }
        if let Some(pos) = s.rfind("```") {
            s = &s[..pos];
        }
    }
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    serde_json::from_str(&s[start..=end]).ok()
}

fn i18n_pack_payload(
    model_id: &str,
    source_locale: &str,
    locale: &str,
    entries: &HashMap<String, String>,
) -> serde_json::Value {
    json!({
        "model": model_id,
        "temperature": 0.1,
        "stream": false,
        "messages": [
            {
                "role": "system",
                "content": "You are a professional software UI localization engine. Return ONLY valid JSON. Translate UI strings accurately and naturally. Preserve placeholders like {name}, {count}, {path}, punctuation that belongs to variables, product names (Michael IDE, Git, MCP, Skills), code identifiers, file paths, shortcuts, and HTML/Markdown markers. Keep keys unchanged. Do not add explanations."
            },
            {
                "role": "user",
                "content": format!(
                    "Translate this Michael IDE UI language pack from {} to locale {}. Return JSON exactly as {{\"translations\":{{\"key\":\"translated text\"}}}}. Entries JSON:\n{}",
                    source_locale,
                    locale,
                    serde_json::to_string(entries).unwrap_or_else(|_| "{}".into())
                )
            }
        ]
    })
}

fn i18n_out_from_raw(
    entries: &HashMap<String, String>,
    raw: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut out = serde_json::Map::new();
    for (k, original) in entries {
        if let Some(text) = raw.get(k).and_then(|v| v.as_str()) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                out.insert(k.clone(), json!(trimmed));
                continue;
            }
        }
        out.insert(k.clone(), json!(original));
    }
    out
}

fn i18n_pack_body(
    locale: &str,
    source_locale: &str,
    translations: serde_json::Map<String, serde_json::Value>,
    source: &str,
) -> serde_json::Value {
    json!({
        "locale": locale,
        "source_locale": source_locale,
        "translations": translations,
        "source": source,
    })
}

async fn i18n_pack_from_model(
    m: &Model,
    model_id: &str,
    source_locale: &str,
    locale: &str,
    entries: &HashMap<String, String>,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let payload = i18n_pack_payload(model_id, source_locale, locale, entries);
    let url = format!("{}/chat/completions", api_base(&m.base_url));
    let resp = GW_HTTP
        .post(url)
        .header("Authorization", format!("Bearer {}", m.api_key))
        .json(&payload)
        .timeout(Duration::from_secs(90))
        .send()
        .await
        .map_err(|e| format!("{} / {} 请求失败: {e}", m.label, model_id))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "{} / {} 上游错误 {}: {}",
            m.label,
            model_id,
            status.as_u16(),
            safe_upstream_error_excerpt(&text)
        ));
    }
    let data: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| format!("{} / {} 返回非 JSON", m.label, model_id))?;
    let content = data
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let parsed = json_object_from_model_text(content)
        .ok_or_else(|| format!("{} / {} 没有返回可解析语言包 JSON", m.label, model_id))?;
    let raw = parsed
        .get("translations")
        .and_then(|v| v.as_object())
        .or_else(|| parsed.as_object())
        .ok_or_else(|| format!("{} / {} 语言包缺少 translations 对象", m.label, model_id))?;
    Ok(i18n_out_from_raw(entries, raw))
}

fn google_translate_locale(locale: &str) -> String {
    match locale.trim().replace('_', "-").as_str() {
        "zh-CN" | "zh-Hans" => "zh-CN".to_string(),
        "zh-TW" | "zh-Hant" => "zh-TW".to_string(),
        other => other.to_string(),
    }
}

fn google_translate_text(data: &serde_json::Value) -> Option<String> {
    let parts = data.get(0)?.as_array()?;
    let mut out = String::new();
    for part in parts {
        if let Some(text) = part.get(0).and_then(|v| v.as_str()) {
            out.push_str(text);
        }
    }
    if out.trim().is_empty() {
        None
    } else {
        Some(out)
    }
}

async fn google_translate_joined(
    source_locale: &str,
    locale: &str,
    joined: &str,
) -> Result<String, String> {
    let resp = GW_HTTP
        .get("https://translate.googleapis.com/translate_a/single")
        .query(&[
            ("client", "gtx"),
            ("sl", source_locale),
            ("tl", locale),
            ("dt", "t"),
            ("q", joined),
        ])
        .timeout(Duration::from_secs(25))
        .send()
        .await
        .map_err(|e| format!("公共翻译请求失败: {e}"))?;
    let status = resp.status();
    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("公共翻译返回非 JSON: {e}"))?;
    if !status.is_success() {
        return Err(format!("公共翻译错误 {}: {}", status.as_u16(), data));
    }
    google_translate_text(&data).ok_or_else(|| "公共翻译没有返回文本".to_string())
}

async fn google_translate_batch(
    source_locale: &str,
    locale: &str,
    texts: &[String],
) -> Result<Vec<String>, String> {
    if texts.is_empty() {
        return Ok(vec![]);
    }
    let marker = "<<<MICHAEL_I18N_SPLIT>>>";
    let joined = texts.join(&format!("\n{marker}\n"));
    let translated = google_translate_joined(source_locale, locale, &joined).await?;
    let parts: Vec<String> = translated
        .split(marker)
        .map(|s| s.trim_matches(['\n', '\r']).trim().to_string())
        .collect();
    if parts.len() == texts.len() {
        return Ok(parts);
    }
    if texts.len() == 1 {
        return Ok(vec![translated.trim().to_string()]);
    }

    let mut one_by_one = Vec::with_capacity(texts.len());
    for text in texts {
        let single = google_translate_joined(source_locale, locale, text).await?;
        let cleaned = single.trim();
        one_by_one.push(if cleaned.is_empty() {
            text.clone()
        } else {
            cleaned.to_string()
        });
    }
    Ok(one_by_one)
}

async fn i18n_pack_from_public_translate(
    source_locale: &str,
    locale: &str,
    entries: &HashMap<String, String>,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let tl = google_translate_locale(locale);
    let mut pairs: Vec<(&String, &String)> = entries.iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(b.0));

    let mut out = serde_json::Map::new();
    let mut batch_keys: Vec<String> = Vec::new();
    let mut batch_texts: Vec<String> = Vec::new();
    let mut batch_len = 0usize;
    for (key, text) in pairs {
        let projected = batch_len + text.len() + 32;
        if !batch_texts.is_empty() && projected > 3200 {
            let translated = google_translate_batch(source_locale, &tl, &batch_texts).await?;
            for (k, v) in batch_keys.drain(..).zip(translated) {
                out.insert(k, json!(if v.trim().is_empty() { "" } else { v.trim() }));
            }
            batch_texts.clear();
            batch_len = 0;
        }
        batch_keys.push(key.clone());
        batch_texts.push(text.clone());
        batch_len += text.len() + 32;
    }
    if !batch_texts.is_empty() {
        let translated = google_translate_batch(source_locale, &tl, &batch_texts).await?;
        for (k, v) in batch_keys.drain(..).zip(translated) {
            out.insert(k, json!(if v.trim().is_empty() { "" } else { v.trim() }));
        }
    }

    Ok(i18n_out_from_raw(entries, &out))
}

/// POST /api/i18n/pack — generate a UI language pack for any BCP-47 locale.
///
/// The IDE ships core packs (zh/en/ja) locally. For every other selected language
/// it posts the English key-value base here; the server asks an active configured
/// model to translate it into the requested locale, caches the result in memory,
/// and returns a plain `{ translations: { key: text } }` object. This gives every
/// language in the picker a real loading path without bundling hundreds of huge
/// hand-maintained JSON files into the desktop app.
pub async fn i18n_pack(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<I18nPackReq>,
) -> ApiResult<Json<serde_json::Value>> {
    // This endpoint spends real money: it drives a configured upstream model with
    // the platform's own api_key. It used to be the one paid route in the gateway
    // that required no credential at all, so anyone could burn the operator's
    // upstream balance anonymously and unattributably.
    // 鉴权是**软**的：花钱的是缓存未命中那条路，所以未鉴权的请求允许读缓存、但绝不
    // 允许触发上游调用。
    //
    // 硬拒绝会打断所有**已发布**的客户端：0.3.15 调这个接口时不带任何 Authorization，
    // 一上线就是整个界面翻译失效。而它们要的几乎都是同一批 UI 文案，任何一个已登录
    // 客户端都会把缓存捂热，所以读缓存这条路对它们基本总是命中。
    let user_id = auth_any_user(&state, &headers).await.ok();
    let locale = req.locale.trim().replace('_', "-");
    if locale.is_empty() || locale.len() > 32 {
        return Err(AppError::bad("locale 不能为空"));
    }
    let source_locale = req.source_locale.unwrap_or_else(|| "en".to_string());
    let entries: HashMap<String, String> = req
        .entries
        .into_iter()
        .filter(|(k, v)| {
            !k.trim().is_empty() && k.len() <= 96 && !v.trim().is_empty() && v.len() <= 900
        })
        .take(700)
        .collect();
    if entries.is_empty() {
        return Err(AppError::bad("entries 不能为空"));
    }

    let cache_key = i18n_pack_cache_key(&locale, &entries);
    if let Some(v) = i18n_pack_cache_get(&cache_key) {
        return Ok(Json(v));
    }
    // 缓存没命中，且没有凭据 —— 到此为止。这一步之后才是花运营方钱的地方，
    // 而"任何人都能烧运营方的上游余额"正是加鉴权要堵的洞。
    // 匿名走全局共享的小额预算（已发布客户端不带凭据，硬拒绝就是界面翻译全废）。
    let user_id = user_id.unwrap_or(I18N_PACK_ANON_IDENTITY);

    // A cache miss is what costs money, so budget the misses per user. Legitimate
    // use is a handful of packs per locale; the 2026-07-25 incident was a single
    // client cache-miss loop that produced ~340k requests in a day.
    i18n_pack_charge_budget(user_id)?;

    let models = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE active = true AND api_key <> '' ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await?;

    let mut failures: Vec<String> = Vec::new();
    for m in &models {
        let mut ids = allowed_ids(m);
        if ids.is_empty() {
            if let Some(id) = &m.model_id {
                if !id.trim().is_empty() {
                    ids.push(id.clone());
                }
            }
        }
        ids.dedup();
        // 翻译是纯机械活：按官方单价升序挑模型，没有价格的排最后。此前是字母序，
        // claude-fable-5（$10/$50）排在 haiku/opus 前面，每个语言包批次都用最贵的
        // 旗舰翻译 UI 文案，纯烧钱（用户实测账单抓到）。
        ids.sort_by(|a, b| {
            let price = |id: &str| official_price(id).map(|(i, o)| i + o).unwrap_or(f64::MAX);
            price(a)
                .partial_cmp(&price(b))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.cmp(b))
        });
        if ids.is_empty() {
            failures.push(format!("{} 未配置 model_id", m.label));
            continue;
        }
        for model_id in ids {
            match i18n_pack_from_model(m, &model_id, &source_locale, &locale, &entries).await {
                Ok(out) => {
                    let body =
                        i18n_pack_body(&locale, &source_locale, out, "model_generated_cached");
                    i18n_pack_cache_put(cache_key, body.clone());
                    return Ok(Json(body));
                }
                Err(e) => failures.push(e),
            }
        }
    }

    let out = i18n_pack_from_public_translate(&source_locale, &locale, &entries)
        .await
        .map_err(|e| AppError {
            status: StatusCode::BAD_GATEWAY,
            msg: format!(
                "语言包生成失败；模型线路失败 {} 条，公共翻译也失败: {}",
                failures.len(),
                e
            ),
        })?;
    let body = i18n_pack_body(&locale, &source_locale, out, "public_translate_cached");
    i18n_pack_cache_put(cache_key, body.clone());
    Ok(Json(body))
}

/// Mask a secret for display: keep the last 4 chars.
fn mask(key: &str) -> String {
    if key.len() <= 4 {
        return "••••".into();
    }
    format!("••••{}", &key[key.len() - 4..])
}

// ---------- admin: list / create / delete ----------
/// GET /api/admin/models — full list for management (api_key masked).
pub async fn admin_list(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let rows = sqlx::query_as::<_, Model>("SELECT * FROM models ORDER BY sort, created_at")
        .fetch_all(&state.db)
        .await?;
    let list: Vec<serde_json::Value> = rows
        .iter()
        .map(|m| {
            json!({
                "id": m.id, "label": m.label, "provider": m.provider, "base_url": m.base_url,
                "model_id": m.model_id, "api_key_masked": mask(&m.api_key), "has_key": !m.api_key.is_empty(),
                "price_cents": m.price_cents, "rate": m.rate, "active": m.active, "sort": m.sort, "created_at": m.created_at,
                "input_price": m.input_price, "output_price": m.output_price,
                "cache_read_price": m.cache_read_price, "cache_create_price": m.cache_create_price,
                "description": m.description,
                "enabled_models": m.enabled_models,
                "billing_mode": m.billing_mode, "per_call_cents": m.per_call_cents,
                "per_call_micro_usd": m.per_call_micro_usd,
                "model_names": m.model_names,
                "model_prices": m.model_prices,
                "model_billing": m.model_billing,
                "protocol": m.protocol,
            })
        })
        .collect();
    Ok(Json(json!(list)))
}

#[derive(Debug, Deserialize)]
pub struct ModelEstimateReq {
    pub channel_rate_id: uuid::Uuid,
    pub connection_id: uuid::Uuid,
    pub model_id: String,
    pub calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_creation_tokens: i64,
    pub sales_cny: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct QuotaEstimateReq {
    pub channel_rate_id: uuid::Uuid,
    pub connection_id: uuid::Uuid,
    pub model_id: String,
    pub visible_quota_usd: f64,
    pub sales_cny: f64,
    pub target_margin_percent: f64,
}

const USER_QUOTA_RAW_USD_PER_VISIBLE_USD: f64 = 6.63;
const MAX_ESTIMATE_TOKENS_PER_CALL: i64 = 10_000_000_000;
const MAX_ESTIMATE_CALLS: i64 = 1_000_000;
const MAX_ESTIMATE_MONEY: f64 = 1_000_000_000.0;

#[derive(Debug)]
struct QuotaPackageProjection {
    quota_raw_usd: f64,
    provider_usd_capacity: f64,
    channel_cost_cny: f64,
    profit_cny: f64,
    margin_percent: f64,
    break_even_multiplier: f64,
    target_multiplier: f64,
    break_even_sales_cny: f64,
    target_sales_cny: f64,
    safe_visible_quota_usd: f64,
}

fn round_multiplier_up(value: f64) -> f64 {
    (value * 100.0).ceil() / 100.0
}

fn project_quota_package(
    visible_quota_usd: f64,
    sales_cny: f64,
    usd_per_cny: f64,
    multiplier: f64,
    target_margin_percent: f64,
) -> QuotaPackageProjection {
    let quota_raw_usd = visible_quota_usd * USER_QUOTA_RAW_USD_PER_VISIBLE_USD;
    let provider_usd_capacity = quota_raw_usd / multiplier;
    let channel_cost_cny = provider_usd_capacity / usd_per_cny;
    let profit_cny = sales_cny - channel_cost_cny;
    let margin_percent = profit_cny / sales_cny * 100.0;
    let break_even_multiplier = quota_raw_usd / (sales_cny * usd_per_cny);
    let target_cost_ratio = 1.0 - target_margin_percent / 100.0;
    let target_multiplier = break_even_multiplier / target_cost_ratio;
    let target_sales_cny = channel_cost_cny / target_cost_ratio;
    let safe_visible_quota_usd = sales_cny * usd_per_cny * target_cost_ratio * multiplier
        / USER_QUOTA_RAW_USD_PER_VISIBLE_USD;
    QuotaPackageProjection {
        quota_raw_usd,
        provider_usd_capacity,
        channel_cost_cny,
        profit_cny,
        margin_percent,
        break_even_multiplier,
        target_multiplier,
        break_even_sales_cny: channel_cost_cny,
        target_sales_cny,
        safe_visible_quota_usd,
    }
}

/// POST /api/admin/model-estimate - project one model workload using the exact
/// server-side price priority, cache prices, connection multiplier and rounding.
pub async fn admin_model_estimate(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<ModelEstimateReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;

    if !(1..=MAX_ESTIMATE_CALLS).contains(&req.calls) {
        return Err(AppError::bad("调用次数需在 1 到 1000000 之间"));
    }
    for (label, value) in [
        ("普通输入 Token", req.input_tokens),
        ("输出 Token", req.output_tokens),
        ("缓存读取 Token", req.cache_read_tokens),
        ("缓存写入 Token", req.cache_creation_tokens),
    ] {
        if !(0..=MAX_ESTIMATE_TOKENS_PER_CALL).contains(&value) {
            return Err(AppError::bad(format!(
                "{label} 需在 0 到 {MAX_ESTIMATE_TOKENS_PER_CALL} 之间"
            )));
        }
    }
    if req.input_tokens == 0
        && req.output_tokens == 0
        && req.cache_read_tokens == 0
        && req.cache_creation_tokens == 0
    {
        return Err(AppError::bad("至少填写一种 Token 数量"));
    }
    if req
        .sales_cny
        .is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err(AppError::bad("销售总价必须是有效的非负数"));
    }

    let model = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = $1")
        .bind(req.connection_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::bad("模型连接不存在"))?;
    let model_id = req.model_id.trim();
    if model_id.is_empty() || !allowed_ids(&model).iter().any(|id| id == model_id) {
        return Err(AppError::bad("该连接没有开放这个模型"));
    }
    if is_image_gen_model(model_id) {
        return Err(AppError::bad("图片模型按张计费，不能使用 Token 推算器"));
    }

    let (channel_name, usd_per_cny) = sqlx::query_as::<_, (String, f64)>(
        "SELECT name, usd_per_cny FROM channel_rates WHERE id = $1",
    )
    .bind(req.channel_rate_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::bad("渠道汇率不存在"))?;

    let (model_in, model_out) = model_price_override(&model.model_prices, model_id);
    let (input_price, output_price, price_source) = if model_in > 0.0 || model_out > 0.0 {
        (model_in, model_out, "model_override")
    } else if let Some((input, output)) = official_price(model_id) {
        (input, output, "official_catalog")
    } else if model.input_price > 0.0 || model.output_price > 0.0 {
        (model.input_price, model.output_price, "connection_fallback")
    } else {
        return Err(AppError::bad(
            "该模型没有可用价格，请在连接编辑里填写单模型输入/输出价",
        ));
    };

    let cache_read_price = if model.cache_read_price > 0.0 {
        model.cache_read_price
    } else {
        input_price * CACHE_READ_FACTOR
    };
    let cache_creation_price = if model.cache_create_price > 0.0 {
        model.cache_create_price
    } else {
        input_price * CACHE_WRITE_FACTOR
    };
    let route_rate = model.rate.max(0.0);
    let provider_usd_per_call = projected_provider_usd(
        req.input_tokens,
        req.output_tokens,
        req.cache_read_tokens,
        req.cache_creation_tokens,
        input_price,
        output_price,
        cache_read_price,
        cache_creation_price,
    );
    let usage = json!({
        "input_tokens": req.input_tokens,
        "output_tokens": req.output_tokens,
        "cache_read_input_tokens": req.cache_read_tokens,
        "cache_creation_input_tokens": req.cache_creation_tokens,
    });
    let billed_cents_per_call = resolve_cost(
        &model.billing_mode,
        model.per_call_cents,
        Some(&usage),
        model_id,
        route_rate,
        model.input_price,
        model.output_price,
        model.cache_read_price,
        model.cache_create_price,
        model_in,
        model_out,
    );
    let calls = req.calls as f64;
    let provider_usd_total = provider_usd_per_call * calls;
    let channel_cost_cny = provider_usd_total / usd_per_cny;
    let billed_raw_usd = billed_cents_per_call as f64 / 100.0 * calls;
    let visible_quota_usd = billed_raw_usd / USER_QUOTA_RAW_USD_PER_VISIBLE_USD;
    let profit_cny = req.sales_cny.map(|sales| sales - channel_cost_cny);
    let margin_percent = req.sales_cny.and_then(|sales| {
        if sales > 0.0 {
            Some((sales - channel_cost_cny) / sales * 100.0)
        } else {
            None
        }
    });

    Ok(Json(json!({
        "channel": { "id": req.channel_rate_id, "name": channel_name, "usd_per_cny": usd_per_cny },
        "connection": { "id": model.id, "label": model.label, "rate": route_rate, "billing_mode": model.billing_mode },
        "model": { "id": model_id, "name": display_name_for(&model.model_names, model_id) },
        "calls": req.calls,
        "tokens_per_call": {
            "input": req.input_tokens,
            "output": req.output_tokens,
            "cache_read": req.cache_read_tokens,
            "cache_creation": req.cache_creation_tokens,
        },
        "prices_per_million": {
            "input": input_price,
            "output": output_price,
            "cache_read": cache_read_price,
            "cache_creation": cache_creation_price,
            "source": price_source,
        },
        "provider_usd_per_call": provider_usd_per_call,
        "provider_usd_total": provider_usd_total,
        "channel_cost_cny": channel_cost_cny,
        "billed_cents_per_call": billed_cents_per_call,
        "billed_raw_usd": billed_raw_usd,
        "visible_quota_usd": visible_quota_usd,
        "quota_raw_usd_per_visible_usd": USER_QUOTA_RAW_USD_PER_VISIBLE_USD,
        "sales_cny": req.sales_cny,
        "profit_cny": profit_cny,
        "margin_percent": margin_percent,
        "break_even_cny": channel_cost_cny,
    })))
}

/// POST /api/admin/quota-estimate - calculate the worst-case cost when a user
/// spends an entire visible quota package on a rate-billed model connection.
pub async fn admin_quota_estimate(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<QuotaEstimateReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    if !req.visible_quota_usd.is_finite()
        || req.visible_quota_usd <= 0.0
        || req.visible_quota_usd > MAX_ESTIMATE_MONEY
    {
        return Err(AppError::bad("用户套餐额度必须是有效的正数"));
    }
    if !req.sales_cny.is_finite() || req.sales_cny <= 0.0 || req.sales_cny > MAX_ESTIMATE_MONEY {
        return Err(AppError::bad("销售总价必须是有效的正数"));
    }
    if !req.target_margin_percent.is_finite() || !(0.0..100.0).contains(&req.target_margin_percent)
    {
        return Err(AppError::bad("目标利润率需在 0% 到 100% 之间"));
    }

    let model = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = $1")
        .bind(req.connection_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::bad("模型连接不存在"))?;
    let model_id = req.model_id.trim();
    if model_id.is_empty() || !allowed_ids(&model).iter().any(|id| id == model_id) {
        return Err(AppError::bad("该连接没有开放这个模型"));
    }
    if model.billing_mode == "per_call" {
        return Err(AppError::bad("套餐额度模式只支持倍率计费模型"));
    }
    let multiplier = model.rate.max(0.0);
    if multiplier <= 0.0 {
        return Err(AppError::bad("模型连接倍率必须大于 0"));
    }

    let (channel_name, usd_per_cny) = sqlx::query_as::<_, (String, f64)>(
        "SELECT name, usd_per_cny FROM channel_rates WHERE id = $1",
    )
    .bind(req.channel_rate_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::bad("渠道汇率不存在"))?;

    let projection = project_quota_package(
        req.visible_quota_usd,
        req.sales_cny,
        usd_per_cny,
        multiplier,
        req.target_margin_percent,
    );
    let break_even_multiplier_rounded = round_multiplier_up(projection.break_even_multiplier);
    let target_multiplier_rounded = round_multiplier_up(projection.target_multiplier);
    let status = if multiplier + f64::EPSILON < projection.break_even_multiplier {
        "loss"
    } else if multiplier + f64::EPSILON < projection.target_multiplier {
        "below_target"
    } else {
        "healthy"
    };

    Ok(Json(json!({
        "channel": { "id": req.channel_rate_id, "name": channel_name, "usd_per_cny": usd_per_cny },
        "connection": { "id": model.id, "label": model.label, "rate": multiplier, "billing_mode": model.billing_mode },
        "model": { "id": model_id, "name": display_name_for(&model.model_names, model_id) },
        "visible_quota_usd": req.visible_quota_usd,
        "quota_raw_usd": projection.quota_raw_usd,
        "quota_raw_usd_per_visible_usd": USER_QUOTA_RAW_USD_PER_VISIBLE_USD,
        "provider_usd_capacity": projection.provider_usd_capacity,
        "channel_cost_cny": projection.channel_cost_cny,
        "sales_cny": req.sales_cny,
        "profit_cny": projection.profit_cny,
        "margin_percent": projection.margin_percent,
        "break_even_sales_cny": projection.break_even_sales_cny,
        "target_sales_cny": projection.target_sales_cny,
        "break_even_multiplier": projection.break_even_multiplier,
        "break_even_multiplier_rounded": break_even_multiplier_rounded,
        "target_margin_percent": req.target_margin_percent,
        "target_multiplier": projection.target_multiplier,
        "target_multiplier_rounded": target_multiplier_rounded,
        "safe_visible_quota_usd": projection.safe_visible_quota_usd,
        "recommended_multiplier": if status == "healthy" { multiplier } else { target_multiplier_rounded },
        "status": status,
    })))
}

#[derive(Deserialize)]
pub struct ModelReq {
    pub label: String,
    pub provider: Option<String>,
    pub base_url: String,
    pub model_id: Option<String>,
    pub api_key: String,
    pub rate: Option<f64>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub cache_read_price: Option<f64>,
    pub cache_create_price: Option<f64>,
    pub description: Option<String>,
    pub sort: Option<i32>,
    pub billing_mode: Option<String>,
    pub per_call_cents: Option<i64>,
    pub per_call_micro_usd: Option<i64>,
}

/// POST /api/admin/models — create a provider connection (admin). model_id is
/// optional; the exposed models are chosen later via the edit/enabled set.
pub async fn admin_create(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<ModelReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    if req.label.trim().is_empty() || req.base_url.trim().is_empty() {
        return Err(AppError::bad("名称 / baseUrl 不能为空"));
    }
    let bmode = match req.billing_mode.as_deref() {
        Some("per_call") => "per_call",
        _ => "rate",
    };
    let id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO models (label, provider, base_url, model_id, api_key, rate, input_price, output_price, description, sort, billing_mode, per_call_cents, cache_read_price, cache_create_price) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14) RETURNING id",
    )
    .bind(req.label.trim())
    .bind(req.provider.unwrap_or_default())
    .bind(req.base_url.trim().trim_end_matches('/'))
    .bind(req.model_id.unwrap_or_default().trim())
    .bind(req.api_key.trim())
    .bind(req.rate.unwrap_or(1.0).max(0.0))
    .bind(req.input_price.unwrap_or(0.0).max(0.0))
    .bind(req.output_price.unwrap_or(0.0).max(0.0))
    .bind(req.description.unwrap_or_default().trim())
    .bind(req.sort.unwrap_or(0))
    .bind(bmode)
    .bind(req.per_call_cents.unwrap_or(0).max(0))
    .bind(req.cache_read_price.unwrap_or(0.0).max(0.0))
    .bind(req.cache_create_price.unwrap_or(0.0).max(0.0))
    .fetch_one(&state.db)
    .await?;
    Ok(Json(json!({ "ok": true, "id": id })))
}

/// DELETE /api/admin/models/:id (admin).
pub async fn admin_delete(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let res = sqlx::query("DELETE FROM models WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::bad("模型不存在"));
    }
    Ok(Json(json!({ "ok": true })))
}

/// GET /api/admin/models/:id/available — proxy the provider's model catalogue
/// (OpenAI-compatible GET /models) using this connection's key.
pub async fn admin_available(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let m = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::bad("模型连接不存在"))?;
    if m.api_key.is_empty() {
        return Err(AppError::bad("该连接未配置 API Key"));
    }
    let url = format!("{}/models", api_base(&m.base_url));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::internal(e.to_string()))?;
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", m.api_key))
        .send()
        .await
        .map_err(|e| AppError::internal(format!("拉取模型列表失败: {e}")))?;
    let status = resp.status();
    let data: serde_json::Value = resp.json().await.unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        return Err(AppError {
            status: axum::http::StatusCode::BAD_GATEWAY,
            msg: format!("供应商错误 {}: {}", status.as_u16(), data),
        });
    }
    let ids: Vec<String> = data
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.get("id").and_then(|i| i.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Ok(Json(json!({ "models": ids, "enabled": m.enabled_models })))
}

#[derive(Deserialize)]
pub struct UpdateReq {
    pub label: Option<String>,
    pub provider: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>, // empty/missing = keep existing
    pub rate: Option<f64>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub cache_read_price: Option<f64>,
    pub cache_create_price: Option<f64>,
    pub description: Option<String>,
    pub active: Option<bool>,
    pub sort: Option<i32>,
    pub enabled_models: Option<Vec<String>>,
    pub billing_mode: Option<String>,
    pub per_call_cents: Option<i64>,
    pub per_call_micro_usd: Option<i64>,
    /// { raw_model_id → friendly display name }. Replaces the whole map when present.
    pub model_names: Option<serde_json::Value>,
    /// { raw_model_id → {"in", "out"} } per-model price overrides. Replaces the whole map.
    pub model_prices: Option<serde_json::Value>,
    pub model_billing: Option<serde_json::Value>,
    /// "anthropic" | "openai" — upstream wire protocol for this connection.
    pub protocol: Option<String>,
}

/// POST /api/admin/models/:id — update a connection (incl. enabled model set). admin.
pub async fn admin_update(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<UpdateReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let m = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::bad("模型连接不存在"))?;
    let label = req
        .label
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(m.label);
    let provider = req.provider.unwrap_or(m.provider);
    let base_url = req
        .base_url
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(m.base_url);
    let api_key = match req.api_key {
        Some(k) if !k.trim().is_empty() => k.trim().to_string(),
        _ => m.api_key,
    };
    let rate = req.rate.unwrap_or(m.rate).max(0.0);
    let input_price = req.input_price.unwrap_or(m.input_price).max(0.0);
    let output_price = req.output_price.unwrap_or(m.output_price).max(0.0);
    let cache_read_price = req.cache_read_price.unwrap_or(m.cache_read_price).max(0.0);
    let cache_create_price = req
        .cache_create_price
        .unwrap_or(m.cache_create_price)
        .max(0.0);
    let description = req
        .description
        .map(|s| s.trim().to_string())
        .unwrap_or(m.description);
    let active = req.active.unwrap_or(m.active);
    let sort = req.sort.unwrap_or(m.sort);
    let enabled = req.enabled_models.unwrap_or(m.enabled_models);
    let billing_mode = match req.billing_mode.as_deref() {
        Some("per_call") => "per_call".to_string(),
        Some("rate") => "rate".to_string(),
        _ => m.billing_mode, // unspecified → keep existing
    };
    let per_call_cents = req.per_call_cents.unwrap_or(m.per_call_cents).max(0);
    let per_call_micro_usd = req
        .per_call_micro_usd
        .unwrap_or(m.per_call_micro_usd)
        .max(0);
    let model_billing = req
        .model_billing
        .filter(|v| v.is_object())
        .unwrap_or(m.model_billing);
    // 次数模式 with a zero fee bills exactly nothing, silently. But a zero CONNECTION fee is
    // perfectly valid when every model carries its own price, which is how per-model pricing
    // is meant to be used — so check the RESOLVED outcome per model, not the connection field
    // in isolation. Reject only models that would actually end up charging nothing:
    // per-call with no fee anywhere, and not 免费 (免费 is floored at billing time, so it is
    // capped by the points pool rather than unlimited).
    if billing_mode == "per_call" && per_call_cents == 0 && per_call_micro_usd == 0 {
        let unpriced: Vec<String> = enabled
            .iter()
            .filter(|mid| {
                let ov = model_billing.get(mid.as_str());
                let mode = ov
                    .and_then(|v| v.get("mode"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| s == "rate" || s == "per_call" || s == "free")
                    .unwrap_or_else(|| billing_mode.clone());
                if mode == "free" || mode == "rate" {
                    return false; // points-capped, or billed by tokens — both fine
                }
                let fee = ov
                    .and_then(|v| v.get("per_call_micro_usd"))
                    .and_then(|v| v.as_i64())
                    .or_else(|| ov.and_then(|v| v.get("per_call_cents")).and_then(|v| v.as_i64()))
                    .unwrap_or(0);
                fee <= 0
            })
            .cloned()
            .collect();
        if !unpriced.is_empty() {
            return Err(AppError::bad(format!(
                "次数模式下这些模型没有价格，调用将不计费：{}。请给它们单独填「次费$」，或设置渠道级「每次调用收费」。",
                unpriced.join("、")
            )));
        }
    }
    // model_names / model_prices: replace the whole map when the client sends one; keep existing otherwise.
    let model_names = req
        .model_names
        .filter(|v| v.is_object())
        .unwrap_or(m.model_names);
    let model_prices = req
        .model_prices
        .filter(|v| v.is_object())
        .unwrap_or(m.model_prices);
    let protocol = match req.protocol.as_deref() {
        Some("openai") => "openai".to_string(),
        Some("anthropic") => "anthropic".to_string(),
        _ => m.protocol, // unspecified → keep existing
    };
    sqlx::query("UPDATE models SET label=$1, provider=$2, base_url=$3, api_key=$4, rate=$5, active=$6, sort=$7, enabled_models=$8, input_price=$9, output_price=$10, description=$11, billing_mode=$12, per_call_cents=$13, model_names=$14, cache_read_price=$15, cache_create_price=$16, model_prices=$17, protocol=$18, model_billing=$20, per_call_micro_usd=$21 WHERE id=$19")
        .bind(&label)
        .bind(&provider)
        .bind(&base_url)
        .bind(&api_key)
        .bind(rate)
        .bind(active)
        .bind(sort)
        .bind(&enabled)
        .bind(input_price)
        .bind(output_price)
        .bind(&description)
        .bind(&billing_mode)
        .bind(per_call_cents)
        .bind(&model_names)
        .bind(cache_read_price)
        .bind(cache_create_price)
        .bind(&model_prices)
        .bind(&protocol)
        .bind(id)
        .bind(&model_billing)
        .bind(per_call_micro_usd)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

// ---------- IDE-facing: list active models (safe fields, no secrets) ----------
/// GET /api/models — active models for the IDE (no api_key / base_url leaked).
pub async fn list_for_client(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE active = true ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await?;
    let mut list = Vec::new();
    for m in &rows {
        for mid in allowed_ids(m) {
            let name = display_name_for(&m.model_names, &mid);
            let (model_in, model_out) = model_price_override(&m.model_prices, &mid);
            let (input_price, output_price, price_source) = if model_in > 0.0 || model_out > 0.0 {
                (model_in, model_out, "model_override")
            } else if m.input_price > 0.0 || m.output_price > 0.0 {
                (m.input_price, m.output_price, "backend")
            } else if let Some((official_in, official_out)) = official_price(&mid) {
                (official_in, official_out, "catalog")
            } else {
                (0.0, 0.0, "unset")
            };
            list.push(json!({
                "conn_id": m.id,
                "group": m.label,
                "provider": m.provider,
                "model_id": mid.clone(),
                "name": name,
                "price_cents": m.price_cents,
                // Expose the display price the admin configured for the IDE picker.
                // No api_key/base_url is leaked; just the model's visible input/output
                // price so the client can show exactly what the backend is using.
                "input_price": input_price,
                "output_price": output_price,
                "price_source": price_source,
                "rate": m.rate,
                "description": m.description,
                // 每模型真实上下文窗口（tokens）：客户端上下文表和棘轮压缩阈值都靠它，
                // 不下发就只能靠客户端猜（GPT-5 曾被猜成 128K，白扔 3/4 窗口）。
                "context_window": official_context(&mid),
                // Full native list so the client can show every window a model really offers,
                // instead of collapsing a genuine choice down to the default.
                "context_windows": official_contexts(&mid)
                    .into_iter()
                    .map(|(tokens, beta)| serde_json::json!({ "tokens": tokens, "beta": beta }))
                    .collect::<Vec<_>>(),
            }));
        }
    }
    Ok(Json(json!(list)))
}

// ---------- IDE-facing: proxy a chat completion, billing credits ----------
/// POST /api/models/:id/chat — forwards an OpenAI-style chat request to the
/// model's provider, deducts the model's price from the caller's credits, and
/// returns the upstream JSON. Non-streaming.
pub async fn chat(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
    headers: HeaderMap,
    Json(mut body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    let request_id = ide_request_id(&headers)?;
    let model = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = $1 AND active = true")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::bad("模型不存在或已停用"))?;

    // pre-check: need a positive balance when the model isn't free. per_call mode
    // (with per_call_cents > 0) also requires balance even if rate/io-price are 0.
    // Which pool pays decides which balance to gate on. A free-flagged model must NOT be
    // blocked by an empty wallet — that is the whole point — but it must still be blocked by
    // an empty points pool, or "free" would silently become unlimited.
    let _pre_mid = body
        .get("model")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| model.model_id.clone().unwrap_or_default());
    let (_pre_mode, _pre_percall, pre_free) = effective_billing(&model, &_pre_mid);
    if pre_free {
        if free_points_balance(&state, uid).await <= 0 {
            return Err(AppError {
                status: axum::http::StatusCode::PAYMENT_REQUIRED,
                msg: "今日免费额度已用完，明天 0 点重置（或改用付费模型）".into(),
            });
        }
    } else {
        let not_free = model.rate > 0.0
            || model.input_price > 0.0
            || model.output_price > 0.0
            || (model.billing_mode == "per_call" && model.per_call_cents > 0);
        if not_free {
            let bal: i64 = sqlx::query_scalar("SELECT credits_cents FROM users WHERE id = $1")
                .bind(uid)
                .fetch_one(&state.db)
                .await?;
            if bal <= 0 {
                return Err(AppError {
                    status: axum::http::StatusCode::PAYMENT_REQUIRED,
                    msg: "额度不足，请充值".into(),
                });
            }
        }
    }

    // forward to the provider (OpenAI-compatible /chat/completions)
    if !body.is_object() {
        return Err(AppError::bad("请求体需为 JSON 对象"));
    }
    // honour the requested model when it's in this connection's enabled set
    let allowed = allowed_ids(&model);
    let requested = body.get("model").and_then(|v| v.as_str()).map(String::from);
    let chosen = match requested {
        Some(r) if allowed.contains(&r) => r,
        _ => allowed.first().cloned().unwrap_or_default(),
    };
    if chosen.is_empty() {
        return Err(AppError::bad("该连接未开放任何模型，请在后台编辑勾选"));
    }
    body["model"] = json!(chosen);

    // Weak-vision models (deepseek/minimax/glm/…) can't read images well. If the
    // request has images and the chosen model isn't vision-native, let gpt-5.5
    // describe the images first, then hand the text to the chosen model.
    if needs_vision_help(&chosen) {
        vision_preprocess(&state, &mut body).await;
    }

    let url = format!("{}/chat/completions", api_base(&model.base_url));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| AppError::internal(e.to_string()))?;
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", model.api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("模型调用失败: {e}")))?;
    let status = resp.status();
    let data: serde_json::Value = resp
        .json()
        .await
        .unwrap_or_else(|_| json!({ "error": "上游返回非 JSON" }));
    if !status.is_success() {
        return Err(AppError {
            status: axum::http::StatusCode::BAD_GATEWAY,
            msg: format!("模型供应商错误 {}: {}", status.as_u16(), data),
        });
    }

    // bill on success: per_call flat fee, or real token usage × official price × 倍率.
    let (model_in, model_out) = model_price_override(&model.model_prices, &chosen);
    let usage_val = data.get("usage");
    let usage_reported = usage_is_authoritative(usage_val);
    if !usage_reported {
        tracing::warn!(model = %chosen, "provider omitted authoritative usage; rate billing is zero");
    }
    let (eff_mode, eff_percall, free_pool, free_micro) = effective_billing_micro(&model, &chosen);
    let cost = resolve_cost(
        &eff_mode,
        eff_percall,
        usage_val.filter(|_| usage_reported),
        &chosen,
        model.rate,
        model.input_price,
        model.output_price,
        model.cache_read_price,
        model.cache_create_price,
        model_in,
        model_out,
    );
    let mut tokens = extract_bill_tokens(
        usage_val.filter(|_| usage_reported),
        &chosen,
        !usage_reported,
    );
    tokens.request_id = request_id;
    // Same step classification as the main chat path — otherwise this handler's rows land in
    // model_usage with NULL mode/tool_turn and the routing report silently under-counts.
    tokens.mode = step_mode(&headers);
    tokens.tool_turn = step_is_tool_turn(&body);
    bill(&state, uid, model.id, cost, false, &tokens, free_pool, free_micro).await;
    Ok(Json(data))
}

// ---------- admin: usage stats ----------
/// GET /api/admin/model-usage — recent usage + totals (admin).
pub async fn admin_usage(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM model_usage")
        .fetch_one(&state.db)
        .await?;
    let spent: i64 =
        sqlx::query_scalar("SELECT COALESCE(SUM(cost_cents),0)::bigint FROM model_usage")
            .fetch_one(&state.db)
            .await?;
    Ok(Json(json!({ "calls": calls, "spent_cents": spent })))
}

/// GET /api/usage — a logged-in user's own recent usage + current balance.
pub async fn user_usage(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    type UsageRow = (
        i64,
        i64,
        i64,
        i64,
        i64,
        String,
        bool,
        chrono::DateTime<chrono::Utc>,
        i64,
    );
    let rows: Vec<UsageRow> =
        sqlx::query_as(
            "SELECT cost_cents, prompt_tokens, completion_tokens, cached_tokens, cache_creation_tokens, model_name, estimated, created_at, free_milli_points_spent \
             FROM model_usage WHERE user_id = $1 ORDER BY created_at DESC LIMIT 200",
        )
        .bind(uid)
        .fetch_all(&state.db)
        .await?;
    let list: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let reported = !r.6;
            json!({
                "cost_cents": r.0,
                "prompt_tokens": if reported { Some(r.1) } else { None },
                "completion_tokens": if reported { Some(r.2) } else { None },
                "cached_tokens": if reported { Some(r.3) } else { None },
                "cache_creation_tokens": if reported { Some(r.4) } else { None },
                "model": r.5,
                "estimated": r.6,
                // 点 spent from the daily free pool. 0 for paid calls, so the client can
                // render "40 点" rows without a second endpoint.
                "free_points_spent": r.8 as f64 / MILLI as f64,
                "usage_reported": reported,
                "time": r.7,
            })
        })
        .collect();
    let (credits, plan): (i64, String) =
        sqlx::query_as("SELECT credits_cents, plan FROM users WHERE id = $1")
            .bind(uid)
            .fetch_one(&state.db)
            .await?;
    let total_spent: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(cost_cents),0)::bigint FROM model_usage WHERE user_id = $1",
    )
    .bind(uid)
    .fetch_one(&state.db)
    .await?;
    Ok(Json(json!({
        "credits_cents": credits,
        "plan": plan,
        "total_spent_cents": total_spent,
        "recent": list,
    })))
}

/// GET /api/usage/settlement/:request_id — the exact row that was charged for
/// one IDE model request. Token fields are null unless the upstream supplied a
/// complete authoritative usage object; cost_cents is always the amount that
/// was actually deducted by the billing transaction.
pub async fn usage_settlement(
    State(state): State<AppState>,
    claims: Claims,
    Path(request_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !valid_ide_request_id(&request_id) {
        return Err(AppError::bad("request_id 无效"));
    }
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    type SettlementRow = (
        i64,
        i64,
        i64,
        i64,
        i64,
        bool,
        Option<String>,
        Option<chrono::DateTime<chrono::Utc>>,
        i64,
    );
    let row: SettlementRow = sqlx::query_as(
        "SELECT COALESCE(SUM(cost_cents), 0)::bigint, \
                COALESCE(SUM(prompt_tokens), 0)::bigint, \
                COALESCE(SUM(completion_tokens), 0)::bigint, \
                COALESCE(SUM(cached_tokens), 0)::bigint, \
                COALESCE(SUM(cache_creation_tokens), 0)::bigint, \
                COALESCE(bool_and(NOT estimated), false), \
                MAX(model_name), MAX(created_at), COUNT(*)::bigint \
         FROM model_usage WHERE user_id = $1 AND request_id = $2",
    )
    .bind(uid)
    .bind(&request_id)
    .fetch_one(&state.db)
    .await?;
    if row.8 == 0 {
        return Err(AppError {
            status: StatusCode::NOT_FOUND,
            msg: "结算记录尚未生成".into(),
        });
    }
    let reported = row.5;
    Ok(Json(json!({
        "request_id": request_id,
        "cost_cents": row.0,
        "prompt_tokens": if reported { Some(row.1) } else { None },
        "completion_tokens": if reported { Some(row.2) } else { None },
        "cached_tokens": if reported { Some(row.3) } else { None },
        "cache_creation_tokens": if reported { Some(row.4) } else { None },
        "model": row.6.unwrap_or_default(),
        "usage_reported": reported,
        "time": row.7,
        "attempt_count": row.8,
    })))
}

// ================= Michael API keys + OpenAI-compatible gateway =================

#[derive(sqlx::FromRow)]
struct ApiKeyRow {
    id: uuid::Uuid,
    label: String,
    api_key: String,
    email: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

fn gen_api_key() -> String {
    let mut rng = rand::thread_rng();
    let hex: String = (0..40)
        .map(|_| std::char::from_digit(rng.gen_range(0..16), 16).unwrap())
        .collect();
    format!("sk-michael-{hex}")
}

fn mask_key(k: &str) -> String {
    if k.len() <= 8 {
        return "••••".into();
    }
    format!("{}…{}", &k[..11.min(k.len())], &k[k.len() - 4..])
}

#[derive(Deserialize)]
pub struct ApiKeyReq {
    pub label: Option<String>,
    pub email: Option<String>,
}

/// POST /api/admin/apikeys — generate a gateway key for the admin (or a given user's email).
pub async fn admin_create_apikey(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<ApiKeyReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let uid = match req
        .email
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        Some(email) => sqlx::query_scalar::<_, uuid::Uuid>("SELECT id FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::bad("用户不存在"))?,
        None => {
            uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?
        }
    };
    let key = gen_api_key();
    sqlx::query("INSERT INTO api_keys (user_id, api_key, label) VALUES ($1,$2,$3)")
        .bind(uid)
        .bind(&key)
        .bind(req.label.unwrap_or_default())
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "ok": true, "api_key": key })))
}

/// GET /api/admin/apikeys — list keys (masked) with their owner email.
pub async fn admin_list_apikeys(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let rows = sqlx::query_as::<_, ApiKeyRow>(
        "SELECT k.id, k.label, k.api_key, u.email, k.created_at, k.last_used_at \
         FROM api_keys k LEFT JOIN users u ON u.id = k.user_id ORDER BY k.created_at DESC LIMIT 200",
    )
    .fetch_all(&state.db)
    .await?;
    let list: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| json!({ "id": r.id, "label": r.label, "email": r.email, "key_masked": mask_key(&r.api_key), "created_at": r.created_at, "last_used_at": r.last_used_at }))
        .collect();
    Ok(Json(json!(list)))
}

/// DELETE /api/admin/apikeys/:id
pub async fn admin_delete_apikey(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let res = sqlx::query("DELETE FROM api_keys WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::bad("密钥不存在"));
    }
    Ok(Json(json!({ "ok": true })))
}

/// GET /api/ide-key — return a stable API key bound to THE LOGGED-IN USER (creating
/// it once), so the IDE can auto-configure a per-user key. REQUIRES a valid login JWT
/// (the `Claims` extractor 401s otherwise). This is deliberate: previously this was
/// public and returned the *first admin's* key — anyone could fetch it (full-gateway
/// leak) and every anonymous caller's usage billed the admin. Now each caller gets
/// THEIR OWN key, billed to THEIR account. The desktop IDE already authenticates chat
/// with the login JWT directly; this endpoint is for clients that want a stable key.
pub async fn ide_key(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    let existing: Option<String> = sqlx::query_scalar("SELECT api_key FROM api_keys WHERE user_id = $1 AND label = 'ide-auto' ORDER BY created_at LIMIT 1")
        .bind(uid)
        .fetch_optional(&state.db)
        .await?;
    let key = match existing {
        Some(k) => k,
        None => {
            let k = gen_api_key();
            sqlx::query(
                "INSERT INTO api_keys (user_id, api_key, label) VALUES ($1, $2, 'ide-auto')",
            )
            .bind(uid)
            .bind(&k)
            .execute(&state.db)
            .await?;
            k
        }
    };
    Ok(Json(json!({ "api_key": key })))
}

/// A model id whose vision is weak/absent → route images through gpt-5.5 first.
fn needs_vision_help(model_id: &str) -> bool {
    let m = model_id.to_lowercase();
    let native = m.contains("gpt")
        || m.contains("gemini")
        || m.contains("claude")
        || m.contains("vision")
        || m.contains("-vl")
        || m.contains("image")
        || m.contains("o3")
        || m.contains("o4");
    !native
}

/// If the request carries images, ask gpt-5.5 to describe them, then rewrite the
/// messages to plain text (description injected) so a non-vision model can work
/// from it. No-op if there are no images or no gpt-5.5 connection is configured.
async fn vision_preprocess(state: &AppState, body: &mut serde_json::Value) {
    let mut images: Vec<serde_json::Value> = Vec::new();
    if let Some(msgs) = body.get("messages").and_then(|m| m.as_array()) {
        for m in msgs {
            if let Some(arr) = m.get("content").and_then(|c| c.as_array()) {
                for part in arr {
                    if part.get("type").and_then(|t| t.as_str()) == Some("image_url") {
                        images.push(part.clone());
                    }
                }
            }
        }
    }
    if images.is_empty() {
        return;
    }
    // best-effort: have gpt-5.5 describe the images (may fail → we still strip them)
    let mut desc: Option<String> = None;
    let conns = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE active = true ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    if let Some(vconn) = conns.into_iter().find(|m| {
        allowed_ids(m)
            .iter()
            .any(|id| id.eq_ignore_ascii_case("gpt-5.5"))
    }) {
        let mut vcontent = vec![json!({
            "type": "text",
            "text": "请详细、客观地描述这些图片的全部内容（文字、数据、图表、代码、界面元素、布局、配色等），让一个无法读图的模型也能据此完成工作。只输出描述本身。"
        })];
        vcontent.extend(images.clone());
        let payload = json!({ "model": "gpt-5.5", "messages": [{ "role": "user", "content": vcontent }], "stream": false });
        if let Ok(client) = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()
        {
            let url = format!("{}/chat/completions", api_base(&vconn.base_url));
            if let Ok(r) = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", vconn.api_key))
                .json(&payload)
                .send()
                .await
            {
                if let Ok(d) = r.json::<serde_json::Value>().await {
                    if let Some(s) = d["choices"][0]["message"]["content"].as_str() {
                        if !s.trim().is_empty() {
                            desc = Some(s.to_string());
                        }
                    }
                }
            }
        }
    }
    let note = match desc {
        Some(d) => format!("【图片内容（由 GPT-5.5 视觉识别）】：\n{}", d),
        None => "【图片】（视觉识别暂不可用，无法读取图片内容）".to_string(),
    };
    // ALWAYS strip images → plain text so a non-vision model never chokes on them.
    if let Some(msgs) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        let mut last_img: Option<usize> = None;
        for (i, m) in msgs.iter_mut().enumerate() {
            if let Some(arr) = m.get("content").and_then(|c| c.as_array()).cloned() {
                let mut text = String::new();
                let mut had = false;
                for part in &arr {
                    match part.get("type").and_then(|t| t.as_str()) {
                        Some("text") => {
                            if let Some(t) = part.get("text").and_then(|x| x.as_str()) {
                                if !text.is_empty() {
                                    text.push('\n');
                                }
                                text.push_str(t);
                            }
                        }
                        Some("image_url") => had = true,
                        _ => {}
                    }
                }
                m["content"] = json!(text);
                if had {
                    last_img = Some(i);
                }
            }
        }
        if let Some(idx) = last_img {
            if let Some(m) = msgs.get_mut(idx) {
                let cur = m
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                m["content"] = json!(format!("{}\n\n{}", cur, note));
            }
        }
    }
}

/// Cents to bill from an upstream `usage` object, CACHE-AWARE. `rate` is cents per
/// 1000 tokens (same unit as the non-streaming path). Cached (read) input tokens are
/// charged at CACHE_READ_FACTOR of the rate so caching savings reach the user;
/// Anthropic cache-CREATION at CACHE_WRITE_FACTOR. Handles both usage shapes —
/// OpenAI/DeepSeek: `prompt_tokens` INCLUDES cached; Anthropic: `input_tokens`
/// EXCLUDES cached (cache_read/creation reported separately). Returns None when the
/// upstream reported no usable token counts, so the caller falls back to a flat fee.
#[allow(dead_code)] // kept for an optional token-based billing mode (currently flat)
fn cost_from_usage(u: &serde_json::Value, rate: f64) -> Option<i64> {
    const CACHE_READ_FACTOR: f64 = 0.1; // cached reads ~10% of input price
    const CACHE_WRITE_FACTOR: f64 = 1.25; // Anthropic cache creation ~125%
                                          // Sanity ceiling: a malformed/huge upstream usage must never saturate to i64::MAX
                                          // and zero out a user's balance. No single call legitimately costs $10k.
    const COST_CEILING: f64 = 1_000_000.0;
    let completion = u
        .get("completion_tokens")
        .and_then(|v| v.as_f64())
        .or_else(|| u.get("output_tokens").and_then(|v| v.as_f64()));
    let prompt = u
        .get("prompt_tokens")
        .and_then(|v| v.as_f64())
        .or_else(|| u.get("input_tokens").and_then(|v| v.as_f64()));
    let (completion, prompt) = match (completion, prompt) {
        (Some(c), Some(p)) => (c, p),
        // Some providers report only total_tokens — bill that flat (matches the
        // non-streaming path's formula).
        _ => {
            let total = u.get("total_tokens").and_then(|v| v.as_f64())?;
            return Some((total / 1000.0 * rate).round().clamp(0.0, COST_CEILING) as i64);
        }
    };
    let cache_read = u.get("cache_read_input_tokens").and_then(|v| v.as_f64()); // Anthropic
    let cache_creation = u
        .get("cache_creation_input_tokens")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let cached = u
        .get("prompt_tokens_details")
        .and_then(|d| d.get("cached_tokens"))
        .and_then(|v| v.as_f64())
        .or_else(|| u.get("prompt_cache_hit_tokens").and_then(|v| v.as_f64())) // DeepSeek
        .or(cache_read)
        .unwrap_or(0.0);
    let billable_input = if cache_read.is_some() {
        // Anthropic shape: input_tokens EXCLUDES cached.
        prompt + cached * CACHE_READ_FACTOR + cache_creation * CACHE_WRITE_FACTOR
    } else {
        // OpenAI/DeepSeek shape: prompt_tokens INCLUDES cached.
        (prompt - cached).max(0.0) + cached * CACHE_READ_FACTOR
    };
    Some(
        ((billable_input + completion) / 1000.0 * rate)
            .round()
            .clamp(0.0, COST_CEILING) as i64,
    )
}

/// Official public list prices (USD per 1,000,000 tokens) per model — the REAL cost basis
/// for billing (the default when no per-model override is set). Per-MODEL, not per-connection,
/// because one connection (e.g. the zyz aggregator) exposes many models at different prices.
/// Matched by FAMILY substring so date/`-preview` suffixes still resolve (e.g.
/// `claude-haiku-4-5-20251001`, `gemini-3.1-pro-preview`). Sources: vendor pricing pages, 2026-07
/// (Anthropic prices from the claude-api skill; Gemini/GPT/DeepSeek/MiniMax from vendor pages).
/// Returns (input, output). None → caller falls back to the connection-level price, then 0.
/// Official context-window sizes (tokens) per model family — the client meters context
/// usage and triggers ratchet compression against this number, so a wrong guess either
/// wastes most of the window (guessing 128K for a 400K model) or blows the request.
/// Keep in sync with provider docs; unknown models fall back client-side.
/// Every native context window a model genuinely offers, ascending, with the upstream beta
/// header each one requires (None = available by default).
///
/// A list rather than a single number because some models really do offer more than one native
/// window, and collapsing that to one hid a real capability: Sonnet 4/4.5 ship 200K by default
/// and 1M behind `context-1m`, and Gemini 1.5 Pro offers 1M and 2M. This is NOT the same axis as
/// michael-compression's 1M/2M/5M tiers — those are windows this gateway manufactures on top of
/// whatever the model natively has.
///
/// Anything listed with Some(beta) MUST have that header actually sent upstream (see the
/// anthropic-beta wiring at the request builder), or the option is a 413 with extra steps.
fn official_contexts(model_id: &str) -> Vec<(i64, Option<&'static str>)> {
    let m = model_id.to_lowercase();
    if m.contains("claude") || m.contains("sonnet") || m.contains("opus") || m.contains("haiku") || m.contains("fable") {
        if m.contains("opus-4-6") || m.contains("opus-4-7") || m.contains("opus-4-8")
            || m.contains("opus-5") || m.contains("sonnet-4-6") || m.contains("sonnet-5")
            || m.contains("fable-5") || m.contains("mythos-5")
        {
            // 1M is both default and maximum on these — one native window, not a choice.
            return vec![(1_000_000, None)];
        }
        if m.contains("sonnet-4") {
            // Sonnet 4 / 4.5: 200K default, 1M behind the beta header (Anthropic requires a
            // sufficient usage tier for it; upstream may still refuse, which surfaces as a
            // normal upstream error rather than a silent truncation).
            return vec![(200_000, None), (1_000_000, Some("context-1m-2025-08-07"))];
        }
        return vec![(200_000, None)];
    }
    if m.contains("gpt-5") || m.contains("codex") {
        return vec![(400_000, None)];
    }
    if m.contains("gemini") {
        if m.contains("1.5") && m.contains("pro") {
            return vec![(1_000_000, None), (2_000_000, None)];
        }
        return vec![(1_000_000, None)];
    }
    if m.contains("grok") || m.contains("xai") {
        return vec![(256_000, None)];
    }
    if m.contains("minimax") {
        return vec![(1_000_000, None)];
    }
    if m.contains("kimi") || m.contains("moonshot") || m.contains("k2") {
        return vec![(256_000, None)];
    }
    if m.contains("deepseek") {
        return vec![(128_000, None)];
    }
    if m.contains("glm") || m.contains("qwen") {
        return vec![(128_000, None)];
    }
    vec![]
}

/// The DEFAULT native window — the first entry of official_contexts. Kept as the single number
/// that budgeting and michael-compression plan against, so adding a beta-gated larger option
/// never silently inflates anyone's budget.
fn official_context(model_id: &str) -> Option<i64> {
    official_contexts(model_id).first().map(|(tokens, _)| *tokens)
}
fn official_price(model_id: &str) -> Option<(f64, f64)> {
    let m = model_id.to_lowercase();
    // ---- Anthropic Claude (official list price) ----
    if m.contains("claude") {
        if m.contains("fable-5") || m.contains("mythos-5") {
            return Some((10.0, 50.0));
        }
        if m.contains("opus-4") {
            return Some((5.0, 25.0));
        } // opus 4.6 / 4.7 / 4.8
          // sonnet 5 standard $3/$15 (intro $2/$10 through 2026-08-31; use the durable list price);
          // sonnet 4.6 is also $3/$15.
        if m.contains("sonnet-5") || m.contains("sonnet-4") {
            return Some((3.0, 15.0));
        }
        if m.contains("haiku-4") {
            return Some((1.0, 5.0));
        } // haiku 4.5 (+date suffix)
    }
    // ---- Google Gemini (official list price, standard ≤200K-context tier) ----
    if m.contains("gemini") {
        if m.contains("image") {
            return None;
        } // image gen → billed per-image, not per-token
        if m.contains("3.5-flash") || m.contains("3-5-flash") {
            return Some((1.5, 9.0));
        }
        if m.contains("pro") {
            return Some((2.0, 12.0));
        } // gemini 3.1 pro (-preview)
        if m.contains("flash") {
            return Some((0.5, 3.0));
        } // gemini 3 flash tier
    }
    // ---- Z.ai GLM (official list price, USD/1M, docs.z.ai 2026-07) ----
    if m.contains("glm") {
        // 顺序敏感：更具体的变体（airx/air/x/flashx）必须先于裸版本号匹配
        if m.contains("5.2") || m.contains("5.1") {
            return Some((1.40, 4.40));
        }
        if m.contains("glm-5") || m.contains("glm5") {
            return Some((1.00, 3.20));
        }
        if m.contains("flashx") {
            return Some((0.07, 0.40));
        }
        if m.contains("airx") {
            return Some((1.10, 4.50));
        }
        if m.contains("air") {
            return Some((0.20, 1.10));
        }
        if m.contains("4.5-x") || m.contains("4.5x") {
            return Some((2.20, 8.90));
        }
        if m.contains("4.7") || m.contains("4.6") || m.contains("4.5") {
            return Some((0.60, 2.20));
        }
        return Some((0.60, 2.20)); // 其余 GLM 变体按 4.x 主档兜底
    }
    // ---- xAI Grok (official list price, USD/1M, x.ai 2026-07) ----
    if m.contains("grok") {
        if m.contains("code-fast") || m.contains("code_fast") {
            return Some((0.20, 1.50));
        }
        if m.contains("4.20") {
            return Some((2.0, 6.0));
        }
        if m.contains("4.5") {
            return Some((2.0, 6.0));
        }
        if m.contains("4.3") {
            return Some((1.25, 2.50));
        }
        if m.contains("4.1") || m.contains("4-fast") || m.contains("fast") {
            return Some((0.20, 0.50)); // grok-4.1 / grok-4.1-fast / grok-4-fast 量产档
        }
        if m.contains("build") {
            return Some((1.0, 2.0));
        }
        if m.contains("3-mini") || m.contains("3 mini") {
            return Some((0.30, 0.50));
        }
        if m.contains("grok-4") || m.contains("grok-3") {
            return Some((3.0, 15.0)); // grok-4 / grok-3 旗舰旧档
        }
        return Some((2.0, 6.0)); // 未知新 Grok 按当前旗舰档兜底
    }
    // ---- OpenAI GPT / DeepSeek / MiniMax (exact ids as the aggregator exposes them) ----
    let p = match m.as_str() {
        "gpt-5.5" => (5.0, 30.0),
        "gpt-5.4" => (2.5, 15.0),
        "deepseek-v4-flash" => (0.14, 0.28),
        "deepseek-v4-pro" => (0.435, 0.87),
        "minimax-m3" => (0.30, 1.20),
        "minimax-m2.7" | "minimax-m2.7-highspeed" => (0.25, 1.00),
        "minimax-m2.5" | "minimax-m2.5-highspeed" => (0.30, 1.20),
        "minimax-m2.1" | "minimax-m2.1-highspeed" => (0.15, 0.60),
        "minimax-m2" => (0.10, 0.40),
        _ => return None,
    };
    Some(p)
}

const CACHE_READ_FACTOR: f64 = 0.1;
const CACHE_WRITE_FACTOR: f64 = 1.25;

#[allow(clippy::too_many_arguments)]
fn projected_provider_usd(
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_creation_tokens: i64,
    input_price: f64,
    output_price: f64,
    cache_read_price: f64,
    cache_creation_price: f64,
) -> f64 {
    (input_tokens as f64 * input_price
        + output_tokens as f64 * output_price
        + cache_read_tokens as f64 * cache_read_price
        + cache_creation_tokens as f64 * cache_creation_price)
        / 1_000_000.0
}

/// REAL billing — actual token usage × the model's REAL (official) price × the
/// connection's 倍率 (markup multiplier):
///   cost_cents = (input_tok·off_in + output_tok·off_out) / 1e6 · 100 · rate
/// `off_in/off_out` come from the per-model official catalog, falling back to the admin's
/// per-connection input/output price override when a model isn't catalogued. `rate` is the
/// connection's 倍率 (e.g. 3 = bill 3× the real cost; the operator's margin, hidden from
/// users). Uses ONLY the upstream's authoritative `usage`; no usage / no price → 0 (never
/// guesses). Cache-aware (cached input 0.1×). Hard $50/call ceiling.
#[allow(clippy::too_many_arguments)]
fn compute_cost(
    usage: Option<&serde_json::Value>,
    model_id: &str,
    rate: f64,
    admin_in: f64,
    admin_out: f64,
    cache_read_price: f64,
    cache_create_price: f64,
    model_in: f64,
    model_out: f64,
) -> i64 {
    const COST_CEILING_CENTS: f64 = 5000.0; // $50/call backstop — no legit single call hits this
    let u = match usage {
        Some(u) if u.is_object() => u,
        _ => return 0,
    };
    let completion = u
        .get("completion_tokens")
        .and_then(|v| v.as_f64())
        .or_else(|| u.get("output_tokens").and_then(|v| v.as_f64()))
        .unwrap_or(0.0);
    let prompt = u
        .get("prompt_tokens")
        .and_then(|v| v.as_f64())
        .or_else(|| u.get("input_tokens").and_then(|v| v.as_f64()))
        // Only total_tokens reported → the non-output remainder is input.
        .or_else(|| {
            u.get("total_tokens")
                .and_then(|v| v.as_f64())
                .map(|t| (t - completion).max(0.0))
        })
        .unwrap_or(0.0);
    if prompt <= 0.0 && completion <= 0.0 {
        return 0;
    }
    // Price priority: admin's PER-MODEL override (model_in/out, set in the backend per enabled
    // model) wins; else the built-in official catalog; else the connection-level input/output
    // price. This lets each checked model be priced individually while keeping the catalog default.
    let (off_in, off_out) = if model_in > 0.0 || model_out > 0.0 {
        (model_in, model_out)
    } else {
        official_price(model_id).unwrap_or((admin_in, admin_out))
    };
    if off_in <= 0.0 && off_out <= 0.0 {
        return 0; // no known price for this model → can't compute a real cost
    }
    let cache_read = u.get("cache_read_input_tokens").and_then(|v| v.as_f64()); // Anthropic
    let cache_creation = u
        .get("cache_creation_input_tokens")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let cached = u
        .get("prompt_tokens_details")
        .and_then(|d| d.get("cached_tokens"))
        .and_then(|v| v.as_f64())
        .or_else(|| u.get("prompt_cache_hit_tokens").and_then(|v| v.as_f64())) // DeepSeek
        .or(cache_read)
        .unwrap_or(0.0);
    // Per-token CACHE prices: admin's explicit price if set (>0), else the old factor off
    // input. cache READ is cheap, cache CREATE/write is a premium — billed separately now.
    let read_price = if cache_read_price > 0.0 {
        cache_read_price
    } else {
        off_in * CACHE_READ_FACTOR
    };
    let write_price = if cache_create_price > 0.0 {
        cache_create_price
    } else {
        off_in * CACHE_WRITE_FACTOR
    };
    // Split input into plain (full price) + cache-read + cache-create, bill each at its own
    // unit price; output at off_out. Then × 倍率. Anthropic reports input EXCLUDING cached;
    // OpenAI/DeepSeek report prompt INCLUDING cached reads (and no separate write count).
    let (plain_input, read_tok, write_tok) = if cache_read.is_some() {
        (prompt, cached, cache_creation) // Anthropic shape
    } else {
        ((prompt - cached).max(0.0), cached, 0.0) // OpenAI / DeepSeek shape
    };
    let usd = (plain_input * off_in
        + read_tok * read_price
        + write_tok * write_price
        + completion * off_out)
        / 1_000_000.0;
    let uncapped = (usd * 100.0 * rate.max(0.0)).round();
    let cents = uncapped.clamp(0.0, COST_CEILING_CENTS) as i64;
    // The ceiling is a backstop, not a policy — if it ever fires, both the charge AND
    // the model_usage row understate what the upstream actually cost, so reconciliation
    // would silently come up short. Make that loud instead of invisible.
    if uncapped > COST_CEILING_CENTS {
        tracing::error!(
            model = %model_id,
            computed_cents = uncapped as i64,
            capped_to = cents,
            "single-call cost exceeded the ceiling; charge and usage record both understate true upstream cost"
        );
    }
    // Detailed breakdown so we can trace "why was this call charged X" — appears
    // in `docker logs server-backend-1` at INFO level.
    tracing::info!(
        "[billing] model={} prompt={} completion={} cache_read={} cache_create={} | in_price={} read_price={:.4} write_price={:.4} out_price={} → usd={:.6} rate={} → {}¢",
        model_id, prompt as i64, completion as i64, read_tok as i64, write_tok as i64,
        off_in, read_price, write_price, off_out, usd, rate, cents
    );
    cents
}

/// Pull the final `usage` object out of an accumulated OpenAI-style SSE stream. With
/// `stream_options.include_usage` the upstream emits a trailing `data:` chunk whose
/// `usage` carries the real prompt/completion token counts; we scan every `data:` line
/// and keep the LAST one that actually has token fields. None if the stream never
/// reported usage (caller then bills the flat fee).
fn parse_usage_from_sse(acc: &[u8]) -> Option<serde_json::Value> {
    let text = String::from_utf8_lossy(acc);
    let mut last: Option<serde_json::Value> = None;
    for line in text.lines() {
        let payload = match line.trim_start().strip_prefix("data:") {
            Some(p) => p.trim(),
            None => continue,
        };
        if payload.is_empty() || payload == "[DONE]" {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
            if let Some(u) = v.get("usage") {
                let has_tokens = [
                    "prompt_tokens",
                    "completion_tokens",
                    "total_tokens",
                    "input_tokens",
                    "output_tokens",
                ]
                .iter()
                .any(|k| u.get(*k).and_then(|x| x.as_f64()).is_some());
                if has_tokens {
                    last = Some(u.clone());
                }
            }
        }
    }
    last
}

/// Incrementally validates OpenAI-compatible SSE before each upstream chunk is
/// forwarded. A terminal marker alone is insufficient when an earlier frame was
/// malformed: that frame may contain the missing suffix of a file-writing tool.
#[derive(Clone, Debug, Default)]
struct ToolArgumentRules {
    required: Vec<String>,
    min_lengths: std::collections::HashMap<String, usize>,
}

fn validate_streamed_tool_arguments(
    provider: &str,
    name: &str,
    raw_arguments: &str,
    rules: Option<&ToolArgumentRules>,
) -> Result<String, String> {
    let arguments = if raw_arguments.trim().is_empty() {
        "{}".to_string()
    } else {
        raw_arguments.to_string()
    };
    let parsed: serde_json::Value = serde_json::from_str(&arguments).map_err(|error| {
        format!("{provider} tool call {name:?} produced incomplete arguments JSON: {error}")
    })?;
    let object = parsed
        .as_object()
        .ok_or_else(|| format!("{provider} tool call {name:?} arguments must be a JSON object"))?;
    if let Some(rules) = rules {
        let missing = rules
            .required
            .iter()
            .filter(|key| !object.contains_key(key.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(format!(
                "{provider} tool call {name:?} is missing required arguments: {}",
                missing.join(", ")
            ));
        }
        for (key, min_length) in &rules.min_lengths {
            let Some(value) = object.get(key) else {
                continue;
            };
            let text = value.as_str().ok_or_else(|| {
                format!("{provider} tool call {name:?} argument {key:?} must be a string")
            })?;
            if text.chars().count() < *min_length {
                return Err(format!(
                    "{provider} tool call {name:?} argument {key:?} is shorter than minLength {min_length}"
                ));
            }
        }
    }
    Ok(arguments)
}

#[derive(Default)]
struct OpenAiToolStream {
    name: String,
    arguments: String,
}

#[derive(Default)]
struct OpenAiSseValidator {
    buf: Vec<u8>,
    done_seen: bool,
    tool_calls: std::collections::HashMap<(u64, u64), OpenAiToolStream>,
    tool_argument_rules: std::collections::HashMap<String, ToolArgumentRules>,
}

impl OpenAiSseValidator {
    fn with_tool_argument_rules(
        tool_argument_rules: std::collections::HashMap<String, ToolArgumentRules>,
    ) -> Self {
        Self {
            tool_argument_rules,
            ..Self::default()
        }
    }

    fn record_tool_calls(&mut self, event: &serde_json::Value) -> Result<(), String> {
        let Some(choices) = event.get("choices").and_then(|value| value.as_array()) else {
            return Ok(());
        };
        for (choice_position, choice) in choices.iter().enumerate() {
            let choice_index = choice
                .get("index")
                .and_then(|value| value.as_u64())
                .unwrap_or(choice_position as u64);
            let calls = choice
                .pointer("/delta/tool_calls")
                .or_else(|| choice.pointer("/message/tool_calls"));
            let Some(calls) = calls.and_then(|value| value.as_array()) else {
                continue;
            };
            for (call_position, call) in calls.iter().enumerate() {
                let tool_index = call
                    .get("index")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(call_position as u64);
                let stream = self
                    .tool_calls
                    .entry((choice_index, tool_index))
                    .or_default();
                let Some(function) = call.get("function") else {
                    continue;
                };
                let function = function
                    .as_object()
                    .ok_or_else(|| "OpenAI SSE tool call function must be an object".to_string())?;
                if let Some(name) = function.get("name") {
                    let name = name.as_str().ok_or_else(|| {
                        "OpenAI SSE tool call function.name must be a string".to_string()
                    })?;
                    if !name.is_empty() {
                        stream.name = name.to_string();
                    }
                }
                if let Some(arguments) = function.get("arguments") {
                    let arguments = arguments.as_str().ok_or_else(|| {
                        "OpenAI SSE tool call function.arguments must be a string".to_string()
                    })?;
                    stream.arguments.push_str(arguments);
                }
            }
        }
        Ok(())
    }

    fn validate_tool_calls(&self) -> Result<(), String> {
        for stream in self.tool_calls.values() {
            if stream.name.is_empty() {
                return Err("OpenAI SSE tool call ended without function.name".to_string());
            }
            validate_streamed_tool_arguments(
                "OpenAI",
                &stream.name,
                &stream.arguments,
                self.tool_argument_rules.get(&stream.name),
            )?;
        }
        Ok(())
    }

    fn push(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.buf.extend_from_slice(bytes);
        while let Some(newline) = self.buf.iter().position(|&byte| byte == b'\n') {
            let raw: Vec<u8> = self.buf.drain(..=newline).collect();
            let line = std::str::from_utf8(&raw)
                .map_err(|error| format!("OpenAI SSE contains invalid UTF-8: {error}"))?
                .trim();
            let Some(payload) = line.strip_prefix("data:").map(str::trim) else {
                continue;
            };
            if payload.is_empty() {
                continue;
            }
            if payload == "[DONE]" {
                if self.done_seen {
                    return Err(
                        "OpenAI SSE contains more than one terminal data: [DONE]".to_string()
                    );
                }
                // Validate before the caller forwards the chunk containing [DONE]. This
                // prevents clients from observing a successful terminal event for a
                // truncated tool call and also keeps that response out of the cache.
                self.validate_tool_calls()?;
                self.done_seen = true;
                continue;
            }
            if self.done_seen {
                return Err("OpenAI SSE contains data after terminal data: [DONE]".to_string());
            }
            let event = serde_json::from_str::<serde_json::Value>(payload)
                .map_err(|error| format!("OpenAI SSE contains malformed JSON: {error}"))?;
            self.record_tool_calls(&event)?;
        }
        Ok(())
    }

    fn finish(&self) -> Result<(), String> {
        if !self.buf.iter().all(u8::is_ascii_whitespace) {
            return Err("OpenAI upstream stream ended with an incomplete SSE frame".to_string());
        }
        if !self.done_seen {
            return Err("OpenAI upstream stream ended without terminal data: [DONE]".to_string());
        }
        self.validate_tool_calls()
    }
}

#[cfg(test)]
fn validate_openai_sse_eof(bytes: &[u8]) -> Result<(), String> {
    let mut validator = OpenAiSseValidator::default();
    validator.push(bytes)?;
    validator.finish()
}

#[cfg(test)]
fn validate_openai_sse_with_rules(
    bytes: &[u8],
    rules: std::collections::HashMap<String, ToolArgumentRules>,
) -> Result<(), String> {
    let mut validator = OpenAiSseValidator::with_tool_argument_rules(rules);
    validator.push(bytes)?;
    validator.finish()
}

/// Strip ALL `cache_control` before forwarding. PROVEN via per-call fingerprints that
/// the [tools+system] prefix is byte-IDENTICAL on every call (16+ consecutive calls,
/// same sys_hash + tools_hash) — yet the relay (zyz) still bills cache CREATION (a 1.25×
/// write premium) on nearly every call and serves reads only sporadically (its prompt
/// cache appears per-instance behind a load balancer, so identical calls keep missing).
/// So on this relay cache_control is, on average, a pure write premium. Stripping it →
/// flat 1× billing. The real win (write-once, then 0.1× reads) needs a RELIABLE-caching
/// upstream (Anthropic / Bedrock direct, or LiteLLM), not this relay.
fn strip_cache_control(body: &mut serde_json::Value) {
    if let Some(msgs) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        for m in msgs.iter_mut() {
            if let Some(content) = m.get_mut("content") {
                if let Some(blocks) = content.as_array_mut() {
                    for b in blocks.iter_mut() {
                        if let Some(o) = b.as_object_mut() {
                            o.remove("cache_control");
                        }
                    }
                }
            }
        }
    }
    if let Some(tools) = body.get_mut("tools").and_then(|t| t.as_array_mut()) {
        for t in tools.iter_mut() {
            if let Some(o) = t.as_object_mut() {
                o.remove("cache_control");
            }
        }
    }
}

/// Deterministic cache key for a chat request. serde_json serializes Map keys sorted, so
/// the same request always produces the same key.
///
/// Scoped PER USER and hashed with SHA-256. Both matter:
///
/// * The key used to be global, so an entry stored by one account could be served to a
///   different one. Scoping to the caller means a collision can only hit your own history.
/// * `DefaultHasher` is a hash-table primitive, not a digest: not collision resistant,
///   `DefaultHasher::new()` is specified to use fixed zero keys (so anyone can reproduce it
///   offline and grind for a colliding body), and Rust reserves the right to change the
///   algorithm between releases. The old "128-bit" claim did not hold either — the second
///   hash fed a constant plus the SAME bytes to the SAME keyed function, so it is
///   correlated with the first, not independent.
fn gw_cache_key(uid: uuid::Uuid, body: &serde_json::Value) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(uid.as_bytes());
    h.update(b"\x00"); // domain separator so the uid cannot run into the body
    h.update(serde_json::to_vec(body).unwrap_or_default());
    format!("gwc:{:x}", h.finalize())
}

fn response_cache_safe(bytes: &[u8]) -> bool {
    // Tool-call arguments contain the user's full tracking number. The native tool
    // masks its result, but caching the model response would retain the original
    // argument in Redis. A false positive only costs one cache miss.
    !bytes
        .windows(b"track_shipment".len())
        .any(|window| window == b"track_shipment")
}

/// POST /v1/chat/completions — OpenAI-compatible gateway. Auth via a Michael API
/// key (Bearer). Resolves `model` to the connection that exposes it, forwards
/// the request (streaming passthrough), and bills the key owner's credits.
/// Repair malformed `tool_calls[*].function.arguments` strings from upstream relays.
/// Specifically targets the `'{}'` + `'{...}'` concatenation bug seen on Claude-via-
/// OpenAI-compat relays, where the placeholder `{}` is glued to the real args JSON
/// instead of replaced. We detect this exact pattern and keep only the trailing JSON.
fn fix_tool_call_arguments(data: &mut serde_json::Value) {
    let choices = match data.get_mut("choices").and_then(|c| c.as_array_mut()) {
        Some(c) => c,
        None => return,
    };
    for ch in choices {
        let tcs = match ch
            .pointer_mut("/message/tool_calls")
            .and_then(|t| t.as_array_mut())
        {
            Some(t) => t,
            None => continue,
        };
        for tc in tcs {
            let args_val = match tc.pointer_mut("/function/arguments") {
                Some(v) => v,
                None => continue,
            };
            let s = match args_val.as_str() {
                Some(s) => s.to_string(),
                None => continue,
            };
            // Strip a literal leading `{}` followed by another JSON object — that's
            // the exact concatenation bug. Don't touch valid single-object strings.
            let trimmed = s.trim_start();
            if let Some(rest) = trimmed.strip_prefix("{}") {
                let rest = rest.trim_start();
                if rest.starts_with('{') && serde_json::from_str::<serde_json::Value>(rest).is_ok()
                {
                    *args_val = serde_json::Value::String(rest.to_string());
                    continue;
                }
            }
            // Fallback: try to parse; if it fails, attempt to locate the last valid
            // JSON object in the string (handles `xxx{...}` garbage prefix).
            if serde_json::from_str::<serde_json::Value>(&s).is_err() {
                if let Some(last_open) = s.rfind('{') {
                    let candidate = &s[last_open..];
                    if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                        *args_val = serde_json::Value::String(candidate.to_string());
                    }
                }
            }
        }
    }
}

/// Resolve a caller to a user id from either an api_key or a login JWT (Bearer).
/// Used by free, auth-gated endpoints (knowledge base) that need a valid user but
/// don't bill.
pub(crate) async fn auth_any_user(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<uuid::Uuid, AppError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::unauthorized("缺少 API Key"))?;
    match sqlx::query_scalar::<_, uuid::Uuid>("SELECT user_id FROM api_keys WHERE api_key = $1")
        .bind(&token)
        .fetch_optional(&state.db)
        .await?
    {
        Some(u) => Ok(u),
        None => crate::auth::user_from_jwt(&state.cfg, &token)
            .ok_or_else(|| AppError::unauthorized("登录已失效或密钥无效")),
    }
}

/// How many unsettled upstream calls one user may have in flight at once.
/// Generous enough for the IDE agent's parallel tool calls, small enough that the
/// worst-case overdraft is bounded instead of open-ended.
const MAX_INFLIGHT_PER_USER: i64 = 8;
/// Backstop TTL on the in-flight counter, in case a process dies without releasing.
const INFLIGHT_TTL_SECS: u64 = 15 * 60;

/// RAII counter for a user's unsettled upstream calls. Held across the upstream
/// request; decrements on drop so every exit path (error, early return, panic)
/// releases it.
pub(crate) struct InFlightGuard {
    redis: redis::aio::ConnectionManager,
    key: String,
}

impl InFlightGuard {
    async fn acquire(state: &AppState, uid: uuid::Uuid) -> Result<Self, AppError> {
        let key = format!("inflight:{uid}");
        let mut redis = state.redis.clone();
        let n: i64 = redis::cmd("INCR")
            .arg(&key)
            .query_async(&mut redis)
            .await
            .unwrap_or(0);
        // A Redis hiccup returns 0 here; fail open rather than locking every user out
        // of a working gateway over a cache blip.
        if n > 0 {
            let _: Result<(), redis::RedisError> = redis::cmd("EXPIRE")
                .arg(&key)
                .arg(INFLIGHT_TTL_SECS)
                .query_async(&mut redis)
                .await;
        }
        if n > MAX_INFLIGHT_PER_USER {
            let _: Result<(), redis::RedisError> =
                redis::cmd("DECR").arg(&key).query_async(&mut redis).await;
            return Err(AppError {
                status: StatusCode::TOO_MANY_REQUESTS,
                msg: "并发请求过多，请稍后再试".into(),
            });
        }
        Ok(Self { redis, key })
    }
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        let mut redis = self.redis.clone();
        let key = std::mem::take(&mut self.key);
        tokio::spawn(async move {
            let _: Result<(), redis::RedisError> =
                redis::cmd("DECR").arg(&key).query_async(&mut redis).await;
        });
    }
}

/// Resolve a caller AND require that they actually have something to spend, for
/// endpoints that consume a paid third-party service (Tripo3D / ElevenLabs / HF …).
///
/// `auth_any_user` alone only proves "some registered account", which let any free
/// signup burn the operator's third-party balance without limit. This adds the same
/// access gate `/v1/chat/completions` uses. It does not price the call — per-endpoint
/// pricing is still a product decision — it only ensures the caller is a paying user
/// and that abuse has a ceiling.
pub(crate) async fn require_paid_access(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<uuid::Uuid, AppError> {
    let uid = auth_any_user(state, headers).await?;
    let (plan, plan_exp, q_total, q_window, credits): (
        String,
        Option<chrono::DateTime<chrono::Utc>>,
        i64,
        i64,
        i64,
    ) = sqlx::query_as(
        "SELECT plan, plan_expires_at, quota_total_cents, quota_window_cents, credits_cents \
         FROM users WHERE id = $1",
    )
    .bind(uid)
    .fetch_one(&state.db)
    .await?;
    let plan_active = plan != "none" && plan_exp.is_none_or(|e| e > chrono::Utc::now());
    let quota_ok = plan_active && q_total > 0 && q_window > 0;
    if !quota_ok && credits <= 0 {
        return Err(AppError {
            status: StatusCode::PAYMENT_REQUIRED,
            msg: "该功能需要有效会员或额度".into(),
        });
    }
    asset_gen_charge_budget(uid)?;
    Ok(uid)
}

/// Per-user ceiling on asset generations. These calls are slow and expensive
/// upstream (and `generate_music` spawns a local MusicGen subprocess with no
/// concurrency limit), so cap them even for paying users until real per-call
/// billing exists.
const ASSET_GEN_WINDOW: Duration = Duration::from_secs(60 * 60);
const ASSET_GEN_PER_WINDOW: usize = 60;
static ASSET_GEN_BUDGET: LazyLock<Mutex<HashMap<uuid::Uuid, Vec<Instant>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn asset_gen_charge_budget(user_id: uuid::Uuid) -> Result<(), AppError> {
    let Ok(mut budget) = ASSET_GEN_BUDGET.lock() else {
        return Ok(());
    };
    let now = Instant::now();
    budget.retain(|_, hits| {
        hits.retain(|at| now.duration_since(*at) < ASSET_GEN_WINDOW);
        !hits.is_empty()
    });
    let hits = budget.entry(user_id).or_default();
    if hits.len() >= ASSET_GEN_PER_WINDOW {
        return Err(AppError {
            status: StatusCode::TOO_MANY_REQUESTS,
            msg: "资源生成过于频繁，请稍后再试".into(),
        });
    }
    hits.push(now);
    Ok(())
}

/// POST /api/knowledge/search — agentic-RAG retrieval over the curated domain
/// knowledge corpus. Body: { query, domain?, top_k? }. Free (no billing); auth
/// only to prevent open abuse. Returns the most relevant best-practice sections.
pub async fn knowledge_search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    auth_any_user(&state, &headers).await?;
    let query = body
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if query.is_empty() {
        return Err(AppError::bad("缺少 query"));
    }
    let domain = body
        .get("domain")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());
    let top_k = body.get("top_k").and_then(|v| v.as_u64()).unwrap_or(6) as usize;
    let hits = crate::knowledge::search(query, domain, top_k);
    Ok(Json(json!({ "results": hits })))
}

/// GET /api/knowledge/domains — list the available knowledge domains + their topics
/// so the agent (or the IDE) can see what's covered.
pub async fn knowledge_domains(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    auth_any_user(&state, &headers).await?;
    let idx = crate::knowledge::get();
    let domains: Vec<_> = idx
        .domains
        .iter()
        .map(|(d, t)| json!({ "domain": d, "topics": t }))
        .collect();
    Ok(Json(json!({ "domains": domains })))
}

/// Per-call token detail for the model_usage audit trail.
#[derive(Clone, Default)]
struct BillTokens {
    prompt: i64,
    completion: i64,
    cached: i64,
    cache_creation: i64,
    model_name: String,
    estimated: bool,
    request_id: Option<String>,
    // ---- step-type instrumentation (for model-routing analysis) ----
    // We can already see WHAT was spent; these say WHAT KIND OF WORK bought it, so the
    // share of expensive calls that were mechanical tool dispatch becomes measurable
    // instead of guessed. All optional: nothing downstream depends on them.
    /// Which IDE surface asked (agent / chat / explorer / plan / reviewer), from x-ide-mode.
    mode: Option<String>,
    /// True when this continues an agent loop — the last input message was a tool result
    /// rather than a human turn. These are the calls that repeat many times per task.
    tool_turn: Option<bool>,
    /// First tool the model called back; None when it answered in prose. A call whose
    /// entire output is one tool dispatch is the prime routing candidate.
    emitted_tool: Option<String>,
}

// ---- step-type classification (pure, no extra model call) ------------------
//
// Routing decisions need to know what KIND of work a call did. All three signals are
// already in the request/response we handle, so this costs a couple of string scans and
// never touches the network.

/// Which IDE surface issued the call. Same header prompts::assemble_into keys off.
fn step_mode(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-ide-mode")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty() && s.len() <= 32)
}

/// Is this a continuation of an agent loop rather than a fresh human turn?
/// True when the last input message is a tool result — these are the calls that repeat
/// many times per task and therefore dominate cost.
/// Is this request a continuation after tool execution (rather than a fresh user turn)?
///
/// Checking only the LAST message was wrong and recorded `false` on every request in
/// production — 1440 NULL / 0 true out of 1545 rows. The IDE deliberately appends ephemeral
/// `user` nudges AFTER the tool results (the "last message gets the most attention" trick),
/// so a tool turn's final message is almost always `user`, never `tool`.
///
/// Scan back instead, and stop at the first assistant message that made no tool calls — that
/// is the boundary of the current tool cycle. Anything tool-shaped inside it means this is a
/// tool turn. Handles both wire shapes: OpenAI `role:"tool"`, and Anthropic tool results,
/// which arrive as a `user` message whose content array carries a `tool_result` block.
fn step_is_tool_turn(body: &serde_json::Value) -> Option<bool> {
    let msgs = body.get("messages")?.as_array()?;
    for m in msgs.iter().rev().take(12) {
        let role = m.get("role").and_then(|v| v.as_str()).unwrap_or("");
        if role == "tool" || role == "function" {
            return Some(true);
        }
        // Anthropic shape: user message containing a tool_result content block.
        if role == "user" {
            if let Some(parts) = m.get("content").and_then(|v| v.as_array()) {
                if parts.iter().any(|p| {
                    p.get("type").and_then(|v| v.as_str()) == Some("tool_result")
                }) {
                    return Some(true);
                }
            }
        }
        // An assistant turn that called tools keeps us inside the cycle; one that did not
        // ends it — anything older belongs to a previous exchange.
        if role == "assistant" {
            let called = m.get("tool_calls").and_then(|v| v.as_array()).is_some_and(|a| !a.is_empty());
            if !called {
                return Some(false);
            }
            return Some(true);
        }
    }
    Some(false)
}

/// The first tool the model called back, or None when it answered in prose.
/// Scans the accumulated OpenAI-shape response; bounded and allocation-light.
fn step_emitted_tool(text: &str) -> Option<String> {
    // Matches both the streaming delta shape and the non-streaming message shape:
    //   "function":{"name":"read_file"     /    "function": { "name": "read_file"
    let key = "\"function\"";
    let mut from = 0usize;
    while let Some(f) = text[from..].find(key) {
        let start = from + f + key.len();
        let window = &text[start..text.len().min(start + 160)];
        if let Some(n) = window.find("\"name\"") {
            let rest = &window[n + 6..];
            if let Some(q1) = rest.find('"') {
                if let Some(q2) = rest[q1 + 1..].find('"') {
                    let name = &rest[q1 + 1..q1 + 1 + q2];
                    if !name.is_empty() && name.len() <= 64 {
                        return Some(name.to_string());
                    }
                }
            }
        }
        from = start;
    }
    None
}

/// Extract BillTokens from a provider usage JSON (OpenAI or Anthropic shape).
fn extract_bill_tokens(
    usage: Option<&serde_json::Value>,
    model_name: &str,
    estimated: bool,
) -> BillTokens {
    let u = match usage.and_then(|v| if v.is_object() { Some(v) } else { None }) {
        Some(v) => v,
        None => {
            return BillTokens {
                model_name: model_name.to_string(),
                estimated,
                ..Default::default()
            }
        }
    };
    let gi = |keys: &[&str]| -> i64 {
        for k in keys {
            if let Some(n) = u.get(*k).and_then(|x| x.as_i64()) {
                return n;
            }
        }
        0
    };
    let cached = gi(&["cache_read_input_tokens"])
        .max(
            u.pointer("/prompt_tokens_details/cached_tokens")
                .and_then(|x| x.as_i64())
                .unwrap_or(0),
        )
        .max(gi(&["prompt_cache_hit_tokens"]));
    BillTokens {
        prompt: gi(&["prompt_tokens", "input_tokens"]),
        completion: gi(&["completion_tokens", "output_tokens"]),
        cached,
        cache_creation: gi(&["cache_creation_input_tokens"]),
        model_name: model_name.to_string(),
        estimated,
        request_id: None,
        // Filled by the caller, which is the only place that can see the request headers
        // and the model's reply. Left None here so usage extraction stays a pure function.
        mode: None,
        tool_turn: None,
        emitted_tool: None,
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct FusedCharge {
    quota_cents: i64,
    wallet_cents: i64,
}

impl FusedCharge {
    fn total_cents(&self) -> i64 {
        self.quota_cents.saturating_add(self.wallet_cents)
    }
}

fn split_fused_charge(
    requested_cost: i64,
    use_quota: bool,
    quota_total: i64,
    quota_window: i64,
    quota_weekly_cap: i64,
    quota_week_used: i64,
    credits: i64,
) -> FusedCharge {
    let requested = requested_cost.max(0);
    let quota_available = if use_quota {
        let weekly_available = if quota_weekly_cap > 0 {
            quota_weekly_cap.saturating_sub(quota_week_used.max(0))
        } else {
            requested
        };
        requested
            .min(quota_total.max(0))
            .min(quota_window.max(0))
            .min(weekly_available.max(0))
    } else {
        0
    };
    let quota_cents = requested.min(quota_available);
    // Whatever quota can't cover lands on the wallet **in full**, even past the
    // available balance — `credits_cents` is allowed to go negative.
    //
    // This used to be clamped with `.min(credits.max(0))`, which meant a user who
    // overshot their balance simply didn't pay the difference: the access gate only
    // checks that the balance is positive, and settlement happens after the upstream
    // call, so every overshoot was silently written off while the operator still paid
    // upstream. Recording it as debt costs the user nothing they didn't spend, makes
    // `model_usage.cost_cents` equal the real cost, and lets the existing
    // `credits <= 0` gate refuse the next request until they top up (a top-up nets
    // against the debt). The in-flight cap bounds how much can accrue at once.
    let overflow = requested.saturating_sub(quota_cents);
    // 超出配额的部分怎么落，取决于这是**谁**的超支：
    //
    // · 按量付费（use_quota=false，或套餐配额本来就是 0）：全额记为债务，允许 credits
    //   为负。此前这里被 `.min(credits.max(0))` 钳住，等于用户超支的那部分直接免单
    //   —— 门禁只看余额是否为正、结算又发生在上游调用之后，所以每一次超支都被静默
    //   写掉，而运营方照付上游。记成债务不会多收他没花的钱，还能让 `credits <= 0`
    //   的门禁挡住下一次请求，充值时自动净额抵扣。
    //
    // · 纯订阅（本轮确实动用了套餐配额）：**不制造钱包债务**。固定价套餐的用户每个
    //   配额窗口末尾都会有一次请求超出剩余配额，全额落到钱包的话，他每个窗口都在
    //   为套餐内的正常使用累积负债 —— 那是他买套餐时就付过的钱。这一小段由运营方
    //   吸收，且天然被"单次请求"的规模限制住。
    let wallet_cents = if use_quota && quota_cents > 0 {
        overflow.min(credits.max(0))
    } else {
        overflow
    };
    FusedCharge {
        quota_cents,
        wallet_cents,
    }
}

/// Deduct cost from the user's quota/credits and log the model_usage row with token detail.
/// Module-scope so chat_completions, responses_proxy, and image_generations all share it.
/// Write one model_usage row. Extracted so the free-points path records identical history to
/// the quota/wallet path — free is a payment source, not a reason to lose usage data.
async fn record_usage_row(
    state: &AppState,
    uid: uuid::Uuid,
    conn_id: uuid::Uuid,
    cost_cents: i64,
    free_milli_points_spent: i64,
    tokens: &BillTokens,
) {
    if let Err(error) = sqlx::query(
        "INSERT INTO model_usage (user_id, model_id, cost_cents, prompt_tokens, completion_tokens, cached_tokens, cache_creation_tokens, model_name, estimated, request_id, ide_mode, is_tool_turn, emitted_tool, free_milli_points_spent) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
    )
    .bind(uid)
    .bind(conn_id)
    .bind(cost_cents)
    .bind(tokens.prompt)
    .bind(tokens.completion)
    .bind(tokens.cached)
    .bind(tokens.cache_creation)
    .bind(&tokens.model_name)
    .bind(tokens.estimated)
    .bind(&tokens.request_id)
    .bind(tokens.mode.as_deref())
    .bind(tokens.tool_turn)
    .bind(tokens.emitted_tool.as_deref())
    .bind(free_milli_points_spent)
    .execute(&state.db)
    .await
    {
        tracing::error!(%error, "failed to insert free-pool usage row");
    }
}

/// Daily free allowance in 点. The operator prices in 点: ¥0.5 = 10 点, so 1 点 = ¥0.05 and
/// the ¥2 daily allowance is exactly 40 点.
pub const FREE_POINTS_DAILY: i64 = 40;

/// The pool is STORED in milli-点 (1 点 = 1000). Whole 点 could not express a small per-call
/// fee: the deduction rounded up, so any non-zero cost cost a full 点 and a 40-点 allowance
/// was always exactly 40 calls regardless of price. Integers throughout — no floats in the
/// money path — just three more decimal places.
pub const MILLI: i64 = 1_000;
pub const FREE_MILLI_POINTS_DAILY: i64 = FREE_POINTS_DAILY * MILLI;

/// Micro-USD per raw cent (1 cent = 10 000 micro-USD). Per-model fees are stored in micro-USD
/// so a $0.003 fee survives; whole cents floored it to zero and the model became free.
pub const MICRO_USD_PER_CENT: i64 = 10_000;

/// Micro-USD that one milli-点 buys. 1 点 = RAW_CENTS_PER_POINT cents, so
/// 1 milli-点 = 5 cents × 10 000 / 1000 = 50 micro-USD.
pub const MICRO_USD_PER_MILLI_POINT: i64 = RAW_CENTS_PER_POINT * MICRO_USD_PER_CENT / MILLI;

/// Milli-点 owed for a call costing `micro_usd` of real provider spend. Rounds UP at
/// milli-点 resolution, so a priced call always costs something (never free by rounding),
/// but a $0.003 call costs 60 milli-点 (0.06 点) rather than a whole one.
pub fn milli_points_for_micro_usd(micro_usd: i64) -> i64 {
    if micro_usd <= 0 {
        return 0;
    }
    (micro_usd + MICRO_USD_PER_MILLI_POINT - 1) / MICRO_USD_PER_MILLI_POINT
}

/// Raw provider cents that one 点 buys.
///
/// DERIVATION (the one assumption in this file, single-sourced so it is changed in one place):
///   • the client's credit denomination is exact — 663 raw cents = $1.00 of visible credit
///   • at ≈¥7.2 per $1.00 of visible credit, 1 点 (¥0.05) ≈ $0.00694 ≈ 4.6 raw cents
/// Rounded UP to 5, which makes each point buy slightly more than its strict value — the
/// error therefore favours the user, never silently overcharges them. If the platform's
/// ¥-per-credit-dollar changes, this is the only number to touch.
pub const RAW_CENTS_PER_POINT: i64 = 5;

/// Points owed for a call that cost `raw_cents` of real provider spend. Rounds UP so a
/// sub-point call still costs 1 点 — otherwise a cheap-enough free model would be unlimited.
pub fn points_for_raw_cents(raw_cents: i64) -> i64 {
    if raw_cents <= 0 {
        return 0;
    }
    (raw_cents + RAW_CENTS_PER_POINT - 1) / RAW_CENTS_PER_POINT
}

/// Read the caller's free-points balance, granting today's allowance first if the stored
/// date is not today. Lazy grant instead of a cron sweep: no scheduler to fail, users who
/// never call cost nothing, and "resets to zero daily" is automatic — yesterday's remainder
/// is overwritten, never carried.
async fn free_points_balance(state: &AppState, uid: uuid::Uuid) -> i64 {
    let row: Result<Option<(i64,)>, _> = sqlx::query_as(
        "UPDATE users SET \
           free_points = CASE WHEN free_points_date IS DISTINCT FROM CURRENT_DATE \
                              THEN $2 ELSE free_points END, \
           free_points_date = CURRENT_DATE \
         WHERE id = $1 RETURNING free_points",
    )
    .bind(uid)
    .bind(FREE_MILLI_POINTS_DAILY)
    .fetch_optional(&state.db)
    .await;
    row.ok().flatten().map(|(n,)| n).unwrap_or(0)
}

/// Spend from the daily free pool. Returns what was actually deducted — the pool floors at
/// zero rather than going negative, so a request that outruns the remaining points is
/// partially charged and the rest is simply free. The pre-gate below refuses the call before
/// it reaches here when the pool is already empty, so this is the tail case only.
/// Spend from the daily pool, in milli-点. `micro_usd` is the call's real provider cost at
/// micro-USD resolution — either the per-model flat fee, or token cost converted up from
/// cents — so per-call and volume billing both land in the same conversion.
async fn spend_free_points(state: &AppState, uid: uuid::Uuid, micro_usd: i64) -> i64 {
    let points = milli_points_for_micro_usd(micro_usd);
    if points <= 0 {
        return 0;
    }
    let _ = free_points_balance(state, uid).await; // ensure today's grant exists first
    let row: Result<Option<(i64,)>, _> = sqlx::query_as(
        "UPDATE users SET free_points = GREATEST(0, free_points - $2) \
         WHERE id = $1 RETURNING free_points",
    )
    .bind(uid)
    .bind(points)
    .fetch_optional(&state.db)
    .await;
    match row.ok().flatten() {
        Some(_) => points,
        None => 0,
    }
}

async fn bill(
    state: &AppState,
    uid: uuid::Uuid,
    conn_id: uuid::Uuid,
    cost: i64,
    use_quota: bool,
    tokens: &BillTokens,
    free_pool: bool,
    free_micro_usd: i64,
) {
    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(error) => {
            tracing::error!(%error, "failed to begin billing transaction");
            return;
        }
    };
    let requested_cost = cost.max(0);
    // Free models bill against the daily points pool, never quota or wallet. Done here rather
    // than at each call site so no biller can forget it: every path that charges a free model
    // lands in this one branch, and the model_usage row below is still written (so usage
    // history and the routing report stay complete — free is a payment source, not a
    // shadow-billing hole).
    if free_pool {
        // Prefer the model's own micro-USD fee (per-call billing, which may be sub-cent);
        // otherwise convert the token-billed cost up from whole cents. Volume billing and
        // per-call billing therefore both convert to 点 through one path.
        let micro = if free_micro_usd > 0 {
            free_micro_usd
        } else {
            requested_cost.max(0) * MICRO_USD_PER_CENT
        };
        // FLOOR: a 免费 model must always consume something, even when no fee is configured.
        // Without this, "free + no fee" spent 0 点 — so the model was not merely free, it was
        // UNCAPPED: the daily allowance never moved and there was nothing to run out of,
        // which defeats the entire pool. One milli-点 (0.001 点) is negligible to a real user
        // and still guarantees the cap eventually binds.
        let points = spend_free_points(state, uid, micro).await.max(1);
        // Deduct the floor when the fee itself produced nothing.
        if micro <= 0 {
            let _ = sqlx::query(
                "UPDATE users SET free_points = GREATEST(0, free_points - 1) WHERE id = $1",
            )
            .bind(uid)
            .execute(&state.db)
            .await;
        }
        // cost_cents stays the REAL provider cost (so operator-side reporting is honest);
        // free_points_spent carries what the user actually paid, in 点.
        record_usage_row(state, uid, conn_id, requested_cost, points, tokens).await;
        let _ = tx.rollback().await;
        return;
    }
    let charge = if requested_cost == 0 {
        FusedCharge::default()
    } else {
        let balances: Option<(i64, i64, i64, i64, i64)> = match sqlx::query_as(
            "SELECT quota_total_cents, quota_window_cents, quota_weekly_cap_cents, \
                    quota_week_used_cents, credits_cents \
             FROM users WHERE id = $1 FOR UPDATE",
        )
        .bind(uid)
        .fetch_optional(&mut *tx)
        .await
        {
            Ok(row) => row,
            Err(error) => {
                tracing::error!(%error, "failed to lock balances for billing");
                return;
            }
        };
        match balances {
            Some((quota_total, quota_window, quota_weekly_cap, quota_week_used, credits)) => {
                split_fused_charge(
                    requested_cost,
                    use_quota,
                    quota_total,
                    quota_window,
                    quota_weekly_cap,
                    quota_week_used,
                    credits,
                )
            }
            None => FusedCharge::default(),
        }
    };
    let actual_cost = charge.total_cents();
    if actual_cost > 0 {
        if let Err(error) = sqlx::query(
            "UPDATE users SET quota_total_cents = quota_total_cents - $1, \
             quota_window_cents = quota_window_cents - $1, \
             quota_week_used_cents = quota_week_used_cents + $1, \
             credits_cents = credits_cents - $2 WHERE id = $3",
        )
        .bind(charge.quota_cents)
        .bind(charge.wallet_cents)
        .bind(uid)
        .execute(&mut *tx)
        .await
        {
            tracing::error!(%error, "failed to deduct fused quota and credits");
            return;
        }
    }
    if let Err(error) = sqlx::query(
        "INSERT INTO model_usage (user_id, model_id, cost_cents, prompt_tokens, completion_tokens, cached_tokens, cache_creation_tokens, model_name, estimated, request_id, ide_mode, is_tool_turn, emitted_tool) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
    )
    .bind(uid)
    .bind(conn_id)
    .bind(actual_cost)
    .bind(tokens.prompt)
    .bind(tokens.completion)
    .bind(tokens.cached)
    .bind(tokens.cache_creation)
    .bind(&tokens.model_name)
    .bind(tokens.estimated)
    .bind(&tokens.request_id)
    .bind(tokens.mode.as_deref())
    .bind(tokens.tool_turn)
    .bind(tokens.emitted_tool.as_deref())
    .execute(&mut *tx)
    .await
    {
        tracing::error!(%error, "failed to insert billing settlement");
        return;
    }
    if let Err(error) = tx.commit().await {
        tracing::error!(%error, "failed to commit billing transaction");
    }
}

// ============ Anthropic protocol bridge (OpenAI ⇄ Anthropic Messages API) ============
// A connection with protocol="anthropic" talks the NATIVE Anthropic /v1/messages API instead
// of the OpenAI-compat /chat/completions wrapper. Native = reliable prompt caching (0.1× reads,
// proven working on this upstream) + correct tool-call streaming (the compat wrapper stalled /
// garbled Claude tool writes). The IDE still speaks OpenAI, so the gateway translates the
// request → Anthropic and the response (streaming + non-streaming) → OpenAI. protocol="openai"
// paths are completely untouched.

/// Flatten an OpenAI message `content` (string OR array of parts) to plain text.
fn oai_content_text(content: Option<&serde_json::Value>) -> String {
    match content {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Array(parts)) => parts
            .iter()
            .filter_map(|p| {
                if p.get("type").and_then(|t| t.as_str()) == Some("text") {
                    p.get("text").and_then(|t| t.as_str()).map(String::from)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// OpenAI user `content` → Anthropic content (plain string, or blocks incl. images).
fn oai_content_to_anthropic(content: Option<&serde_json::Value>) -> serde_json::Value {
    match content {
        Some(serde_json::Value::Array(parts)) => {
            let mut blocks: Vec<serde_json::Value> = Vec::new();
            for p in parts {
                match p.get("type").and_then(|t| t.as_str()) {
                    Some("text") => {
                        if let Some(t) = p.get("text").and_then(|v| v.as_str()) {
                            blocks.push(json!({"type":"text","text":t}));
                        }
                    }
                    Some("image_url") => {
                        if let Some(u) = p.pointer("/image_url/url").and_then(|v| v.as_str()) {
                            if let Some(rest) = u.strip_prefix("data:") {
                                if let Some((meta, data)) = rest.split_once(',') {
                                    let media = meta.split(';').next().unwrap_or("image/png");
                                    blocks.push(json!({"type":"image","source":{"type":"base64","media_type":media,"data":data}}));
                                }
                            } else {
                                blocks
                                    .push(json!({"type":"image","source":{"type":"url","url":u}}));
                            }
                        }
                    }
                    _ => {}
                }
            }
            if blocks.is_empty() {
                json!("")
            } else {
                json!(blocks)
            }
        }
        Some(serde_json::Value::String(s)) => json!(s.clone()),
        _ => json!(""),
    }
}

/// Extended-thinking config for a Claude model on the native Anthropic path, or None.
/// Adaptive thinking (Opus/Sonnet 4.x+, Fable, Mythos) is Anthropic's smartest mode and
/// auto-scales depth (minimal on trivial calls, deep on hard ones) — the single biggest IQ
/// lever for the coding agent. Verified live against the upstream: adaptive is accepted, and
/// replayed tool_use turns WITHOUT preserved thinking blocks are tolerated (200, not 400), so
/// no thinking-signature round-trip through the OpenAI-format history is needed.
/// Haiku stays fast/cheap (no thinking); 3.7 uses the older explicit-budget form; 3.5 none.
/// Respects the client's per-model control: `reasoning_effort` present = thinking ON, absent /
/// "off" = OFF (the IDE defaults Claude to "medium" and drops the field on "off").
/// Master off-switch: env MICHAEL_ANTHROPIC_THINKING=0.
fn anthropic_thinking(model: &str, effort: Option<&str>) -> Option<serde_json::Value> {
    if std::env::var("MICHAEL_ANTHROPIC_THINKING").ok().as_deref() == Some("0") {
        return None;
    }
    let eff = match effort {
        Some(e) if !e.is_empty() && e != "off" => e,
        _ => return None,
    };
    let m = model.to_lowercase();
    if m.contains("haiku") {
        return None;
    } // fast tier → keep it fast
    if m.contains("claude-3-5") || m.contains("claude-3.5") {
        return None;
    } // pre-thinking
    if m.contains("claude-3-7") || m.contains("claude-3.7") {
        // 3.7 → explicit budget
        let budget = match eff {
            "low" => 4000,
            "high" | "max" => 12000,
            _ => 8000,
        };
        return Some(json!({"type":"enabled","budget_tokens":budget}));
    }
    // 4.6 及更早（不含上面已处理的 3.5/3.7）：仍接受显式预算。
    // 历史背景：早期聚合上游（zyz 等）对 {"type":"adaptive"} 静默忽略——请求 200 但一个
    // thinking_delta 都不回，IDE 的思考卡永远是空的；换成 enabled+budget_tokens 后同一
    // 路线能正常回思考流。那个兜底当时是对的，但它被套用到了**所有** claude 模型上。
    if m.contains("claude-4-6") || m.contains("claude-opus-4-6") || m.contains("claude-sonnet-4-6")
    {
        let budget = match eff {
            "low" => 4096,
            "high" => 24000,
            "max" | "xhigh" => 32000,
            _ => 12000,
        };
        return Some(json!({"type":"enabled","budget_tokens":budget}));
    }
    if m.contains("claude") || m.contains("fable") || m.contains("mythos") {
        // Sonnet 5 / Opus 5 / Opus 4.8 / 4.7 / Fable 5 / Mythos 5 REMOVED the explicit-budget
        // form: `{"type":"enabled","budget_tokens":N}` is rejected with a hard 400
        //   "thinking.type.enabled is not supported for this model.
        //    use thinking.type.adaptive and output_config.effort"
        // The old zyz workaround above was therefore sending a request that can never
        // succeed on the current upstream (polly.modelbridge.cc → real Claude API), and the
        // 400 was being reclassified as a retryable 502 (see `upstream_failure_status`), so
        // the IDE re-sent the same impossible request every ~2s — measured in production on
        // 2026-08-01, 29 rejections in six hours, each with attempted_sends=1: the gateway
        // gave up correctly, the CLIENT was the retry loop, and the user just saw a frozen
        // editor. Depth is expressed with output_config.effort instead (set by the caller);
        // adaptive lets the model choose how much to think per turn.
        // display:"summarized" is REQUIRED to get any visible thinking out of this family.
        // On 4.6 the default was "summarized", which is why its 已思考 card worked. On
        // 4.7/4.8/5/Sonnet 5/Fable/Mythos the default flipped to "omitted": thinking blocks
        // still stream, but their text is an EMPTY STRING. The SSE bridge only emits
        // reasoning_content when the delta is non-empty (models.rs ~4543) and the client only
        // raises a reasoning event for non-empty text — so "omitted" produces zero deltas and
        // the card never appears. Nothing downstream is broken; it is correctly dropping empty
        // strings. Raw chain-of-thought is never returned on this family regardless; summarized
        // is the only visible form there is.
        return Some(json!({"type":"adaptive","display":"summarized"}));
    }
    None
}

/// OpenAI /chat/completions body → Anthropic /v1/messages body.
#[cfg(test)]
fn oai_to_anthropic(body: &serde_json::Value) -> Result<serde_json::Value, String> {
    oai_to_anthropic_with_cache(body, true)
}

fn oai_to_anthropic_with_cache(
    body: &serde_json::Value,
    prompt_cache: bool,
) -> Result<serde_json::Value, String> {
    let mut system_parts: Vec<serde_json::Value> = Vec::new();
    let mut messages: Vec<serde_json::Value> = Vec::new();
    if let Some(msgs) = body.get("messages").and_then(|m| m.as_array()) {
        for m in msgs {
            match m.get("role").and_then(|r| r.as_str()).unwrap_or("user") {
                "system" => {
                    let s = oai_content_text(m.get("content"));
                    if !s.is_empty() {
                        let mut block = serde_json::Map::new();
                        block.insert("type".into(), json!("text"));
                        block.insert("text".into(), json!(s));
                        // The gateway-injected Prompt Graph message is first. Cache it separately
                        // from later dynamic Skill/system messages so those can change without
                        // invalidating the stable production prefix.
                        if prompt_cache && system_parts.is_empty() {
                            block.insert("cache_control".into(), json!({"type":"ephemeral"}));
                        }
                        system_parts.push(serde_json::Value::Object(block));
                    }
                }
                "tool" => {
                    // OpenAI tool result → Anthropic user turn w/ a tool_result block. Consecutive
                    // tool results MUST be grouped into one user turn (Anthropic requirement).
                    let tcid = m.get("tool_call_id").and_then(|v| v.as_str()).unwrap_or("");
                    let block = json!({"type":"tool_result","tool_use_id":tcid,"content":oai_content_text(m.get("content"))});
                    let can_group = messages.last().is_some_and(|last| {
                        last.get("role").and_then(|r| r.as_str()) == Some("user")
                            && last
                                .get("content")
                                .and_then(|c| c.as_array())
                                .is_some_and(|a| {
                                    a.iter().all(|b| {
                                        b.get("type").and_then(|t| t.as_str())
                                            == Some("tool_result")
                                    })
                                })
                    });
                    if can_group {
                        if let Some(arr) = messages
                            .last_mut()
                            .and_then(|l| l.get_mut("content"))
                            .and_then(|c| c.as_array_mut())
                        {
                            arr.push(block);
                        }
                    } else {
                        messages.push(json!({"role":"user","content":[block]}));
                    }
                }
                "assistant" => {
                    let mut blocks: Vec<serde_json::Value> = Vec::new();
                    let s = oai_content_text(m.get("content"));
                    if !s.is_empty() {
                        blocks.push(json!({"type":"text","text":s}));
                    }
                    if let Some(tcs) = m.get("tool_calls").and_then(|t| t.as_array()) {
                        for tc in tcs {
                            let id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            let name = tc
                                .pointer("/function/name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let args = tc.pointer("/function/arguments").ok_or_else(|| {
                                format!(
                                    "assistant tool call {name:?} (id {id:?}) is missing function.arguments"
                                )
                            })?;
                            let input = match args {
                                serde_json::Value::String(args) => serde_json::from_str(args)
                                    .map_err(|err| {
                                        format!(
                                            "assistant tool call {name:?} (id {id:?}) has malformed function.arguments JSON: {err}"
                                        )
                                    })?,
                                serde_json::Value::Object(_) => args.clone(),
                                _ => {
                                    return Err(format!(
                                        "assistant tool call {name:?} (id {id:?}) has non-object function.arguments"
                                    ));
                                }
                            };
                            if !input.is_object() {
                                return Err(format!(
                                    "assistant tool call {name:?} (id {id:?}) function.arguments must decode to a JSON object"
                                ));
                            }
                            blocks
                                .push(json!({"type":"tool_use","id":id,"name":name,"input":input}));
                        }
                    }
                    if blocks.is_empty() {
                        blocks.push(json!({"type":"text","text":"(no content)"}));
                    }
                    messages.push(json!({"role":"assistant","content":blocks}));
                }
                _ => messages.push(
                    json!({"role":"user","content":oai_content_to_anthropic(m.get("content"))}),
                ),
            }
        }
    }
    let mut out = serde_json::Map::new();
    if let Some(model) = body.get("model") {
        out.insert("model".into(), model.clone());
    }
    out.insert("messages".into(), json!(messages));
    if !system_parts.is_empty() {
        out.insert("system".into(), json!(system_parts));
    }
    // Extended thinking — ALWAYS use the gateway's model-aware config, never the client's
    // `thinking` field. Newer models (Sonnet 5, Opus 4.7/4.8, Fable 5) REJECT the old
    // `{"type":"enabled","budget_tokens":N}` format with a 400/502 — they require
    // `{"type":"adaptive"}` + `output_config.effort`. The IDE client may still send the old
    // format; the gateway normalises it here per-model.
    let model_str = body.get("model").and_then(|v| v.as_str()).unwrap_or("");
    let effort = body
        .get("reasoning_effort")
        .and_then(|v| v.as_str())
        .or_else(|| {
            // 客户端只发 thinking:{budget_tokens} 不发 reasoning_effort（IDE Claude 族的
            // 真实形状）时，按预算推档——以前这里一律写死 "high"，用户转盘上的
            // low/medium/max 全被压平成 high，max 的 64K 输出余量也永远打不中。
            // 档位边界与 IDE budgets{low:4096, medium:12000, high:24000, max:32000} 对齐。
            body.get("thinking").map(|t| {
                match t.get("budget_tokens").and_then(|v| v.as_i64()).unwrap_or(0) {
                    b if b > 24000 => "max",
                    b if b > 12000 => "high",
                    b if b > 4096 => "medium",
                    b if b > 0 => "low",
                    _ => "high", // 无预算的裸 thinking 开关（Kimi/GLM 形状）保持旧行为
                }
            })
        });
    let thinking = anthropic_thinking(model_str, effort);
    let thinking_on = thinking.is_some();
    // Anthropic REQUIRES max_tokens. Map from OpenAI, else a generous default.
    let mut max_tokens = body
        .get("max_tokens")
        .and_then(|v| v.as_i64())
        .or_else(|| body.get("max_completion_tokens").and_then(|v| v.as_i64()))
        .filter(|n| *n > 0)
        .unwrap_or(8192);
    // For adaptive thinking: no budget_tokens, just ensure a generous max_tokens.
    // For budget-based (3.7): ensure max_tokens > budget_tokens.
    if thinking_on {
        let budget = thinking
            .as_ref()
            .and_then(|t| t.get("budget_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        // Give the deepest effort real headroom to think long. Both "high" and "max" map to
        // output_config.effort="high" (Anthropic's top knob), so the ONLY thing that makes
        // "max" deeper than "high" is more max_tokens room for adaptive thinking to stretch.
        // Without this the top UI dial is a no-op. Gated by effort so low/medium stay lean;
        // weak/fast models never reach here (thinking is None for haiku/3.5/non-Claude).
        let floor = match effort {
            Some("max") => 64000,
            Some("high") => 40000,
            _ => 32000,
        };
        let min_mt = (budget + 8000).max(floor);
        if max_tokens < min_mt {
            max_tokens = min_mt;
        }
    }
    let max_tokens = max_tokens.clamp(1, 128000);
    out.insert("max_tokens".into(), json!(max_tokens));
    if let Some(t) = &thinking {
        out.insert("thinking".into(), t.clone());
        // 不发 output_config.effort：实测聚合上游（zyz）一旦收到 effort 就把思考流
        // 换成一句 "Compatibility reasoning summary." 摘要，完整原始思考全部丢失；
        // 只发 thinking.budget_tokens 时上游按原文回思考流。思考深度已由
        // budget_tokens + max_tokens 下限（见上）控制，effort 不再需要。
    }
    // Native Anthropic requests do not forward OpenAI sampling knobs. New Claude
    // models reject temperature/top_p even when thinking is off, while omitting
    // them preserves the provider default for every model generation.
    for k in ["stream", "stop"] {
        if let Some(v) = body.get(k) {
            out.insert(k.to_string(), v.clone());
        }
    }
    if let Some(tools) = body.get("tools").and_then(|t| t.as_array()) {
        let atools: Vec<serde_json::Value> = tools
            .iter()
            .filter_map(|t| {
                let f = t.get("function")?;
                let name = f.get("name")?.as_str()?;
                let mut a = serde_json::Map::new();
                a.insert("name".into(), json!(name));
                if let Some(d) = f.get("description") {
                    a.insert("description".into(), d.clone());
                }
                a.insert(
                    "input_schema".into(),
                    f.get("parameters")
                        .cloned()
                        .unwrap_or_else(|| json!({"type":"object","properties":{}})),
                );
                Some(serde_json::Value::Object(a))
            })
            .collect();
        if !atools.is_empty() {
            let mut atools = atools;
            // Prompt caching breakpoint #1: last tool. tools 在 Anthropic 请求序列
            // 最前（tools→system→messages），断点打在末个工具上把整个工具表缓存住。
            if prompt_cache {
                if let Some(last) = atools.last_mut().and_then(|v| v.as_object_mut()) {
                    last.insert("cache_control".into(), json!({"type":"ephemeral"}));
                }
            }
            out.insert("tools".into(), json!(atools));
        }
    }
    // Prompt caching breakpoint #3: rolling conversation breakpoint. 不打在"最后一条
    // 消息"上——IDE 的尾部是易变区（运行草稿纸/自提醒/协调 nudge 每轮增删），断点挂
    // 在那里下一轮永远对不上前缀，实测整段历史 0 命中。打在【最后一条含 tool_result
    // 的消息】上：工具结果是 append-only 的稳定履历（压缩轮外逐字节不动），下一轮的
    // 前缀能一路匹配到这里，历史大头以 0.1× 读回。没有工具结果时退回最后一条消息。
    if prompt_cache {
        if let Some(arr) = out.get_mut("messages").and_then(|m| m.as_array_mut()) {
            let anchor = arr
                .iter()
                .rposition(|m| {
                    m.get("content")
                        .and_then(|c| c.as_array())
                        .is_some_and(|blocks| {
                            blocks.iter().any(|b| {
                                b.get("type").and_then(|t| t.as_str()) == Some("tool_result")
                            })
                        })
                })
                .or_else(|| arr.len().checked_sub(1));
            if let Some(idx) = anchor {
                let last_msg = &mut arr[idx];
                if let Some(blocks) = last_msg.get_mut("content").and_then(|c| c.as_array_mut()) {
                    if let Some(obj) = blocks.last_mut().and_then(|b| b.as_object_mut()) {
                        // tool_result 的 content 是嵌套结构也允许挂 cache_control（块级均可）。
                        obj.insert("cache_control".into(), json!({"type":"ephemeral"}));
                    }
                } else if let Some(text) = last_msg
                    .get("content")
                    .and_then(|c| c.as_str())
                    .map(String::from)
                {
                    if let Some(obj) = last_msg.as_object_mut() {
                        obj.insert(
                        "content".into(),
                        json!([{"type":"text","text":text,"cache_control":{"type":"ephemeral"}}]),
                    );
                    }
                }
            }
        }
    }
    if let Some(tc) = body.get("tool_choice") {
        let atc = match tc.as_str() {
            Some("auto") => Some(json!({"type":"auto"})),
            Some("required") => Some(json!({"type":"any"})),
            Some("none") => None,
            _ => tc
                .pointer("/function/name")
                .and_then(|n| n.as_str())
                .map(|n| json!({"type":"tool","name":n})),
        };
        if let Some(v) = atc {
            out.insert("tool_choice".into(), v);
        }
    }
    Ok(serde_json::Value::Object(out))
}

/// Anthropic usage → an object carrying BOTH Anthropic token names (so compute_cost bills
/// cache-correctly) and OpenAI names (so OpenAI clients read prompt/completion tokens).
fn anthropic_usage_merged(au: &serde_json::Value) -> serde_json::Value {
    let g = |k: &str| au.get(k).and_then(|v| v.as_i64()).unwrap_or(0);
    let (it, ot) = (g("input_tokens"), g("output_tokens"));
    json!({
        "input_tokens": it, "output_tokens": ot,
        "cache_read_input_tokens": g("cache_read_input_tokens"),
        "cache_creation_input_tokens": g("cache_creation_input_tokens"),
        "prompt_tokens": it, "completion_tokens": ot, "total_tokens": it + ot,
    })
}

/// Anthropic non-streaming response → OpenAI /chat/completions response.
fn anthropic_to_oai(av: &serde_json::Value, model: &str) -> serde_json::Value {
    let mut text = String::new();
    let mut tool_calls: Vec<serde_json::Value> = Vec::new();
    if let Some(content) = av.get("content").and_then(|c| c.as_array()) {
        for b in content {
            match b.get("type").and_then(|t| t.as_str()) {
                Some("text") => {
                    if let Some(t) = b.get("text").and_then(|v| v.as_str()) {
                        text.push_str(t);
                    }
                }
                Some("tool_use") => {
                    let input = b.get("input").cloned().unwrap_or_else(|| json!({}));
                    tool_calls.push(json!({
                        "id": b.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                        "type": "function",
                        "function": {"name": b.get("name").and_then(|v| v.as_str()).unwrap_or(""), "arguments": serde_json::to_string(&input).unwrap_or_else(|_| "{}".into())}
                    }));
                }
                _ => {}
            }
        }
    }
    let finish = match av.get("stop_reason").and_then(|v| v.as_str()) {
        Some("tool_use") => "tool_calls",
        Some("max_tokens") => "length",
        _ => "stop",
    };
    let mut message = serde_json::Map::new();
    message.insert("role".into(), json!("assistant"));
    message.insert(
        "content".into(),
        if text.is_empty() && !tool_calls.is_empty() {
            serde_json::Value::Null
        } else {
            json!(text)
        },
    );
    if !tool_calls.is_empty() {
        message.insert("tool_calls".into(), json!(tool_calls));
    }
    json!({
        "id": av.get("id").cloned().unwrap_or_else(|| json!("chatcmpl-anthropic")),
        "object": "chat.completion", "model": model,
        "choices": [{"index": 0, "message": serde_json::Value::Object(message), "finish_reason": finish}],
        "usage": anthropic_usage_merged(av.get("usage").unwrap_or(&json!({}))),
    })
}

fn tool_argument_rules(
    body: &serde_json::Value,
) -> std::collections::HashMap<String, ToolArgumentRules> {
    body.get("tools")
        .and_then(|tools| tools.as_array())
        .into_iter()
        .flatten()
        .filter_map(|tool| {
            let function = tool.get("function")?;
            let name = function.get("name")?.as_str()?.to_string();
            let required = function
                .pointer("/parameters/required")
                .and_then(|required| required.as_array())
                .into_iter()
                .flatten()
                .filter_map(|key| key.as_str().map(str::to_string))
                .collect::<Vec<_>>();
            let min_lengths = function
                .pointer("/parameters/properties")
                .and_then(|properties| properties.as_object())
                .into_iter()
                .flatten()
                .filter_map(|(key, schema)| {
                    schema
                        .get("minLength")
                        .and_then(|value| value.as_u64())
                        .and_then(|value| usize::try_from(value).ok())
                        .map(|value| (key.clone(), value))
                })
                .collect();
            Some((
                name,
                ToolArgumentRules {
                    required,
                    min_lengths,
                },
            ))
        })
        .collect()
}

/// Stateful converter: Anthropic Messages SSE stream → OpenAI chat.completions SSE stream.
/// Fed raw upstream bytes via `push` (handles chunk-split events); emits ready-to-forward
/// OpenAI `data:` lines. Accumulates usage for billing. `finish` emits the terminal chunks.
struct AnthToolStream {
    tool_index: i64,
    name: String,
    arguments: String,
    stopped: bool,
}

struct AnthSse {
    buf: Vec<u8>,
    model: String,
    role_sent: bool,
    next_tool_idx: i64,
    tool_blocks: std::collections::HashMap<i64, AnthToolStream>,
    tool_argument_rules: std::collections::HashMap<String, ToolArgumentRules>,
    message_stop_seen: bool,
    // 中转丢块签名追踪：只见 thinking、不见任何 text/tool_use 就 end_turn。
    saw_thinking_block: bool,
    saw_answer_block: bool,
    input_tokens: i64,
    output_tokens: i64,
    input_usage_reported: bool,
    output_usage_reported: bool,
    cache_read: i64,
    cache_create: i64,
    stop_reason: String,
}
impl AnthSse {
    #[cfg(test)]
    fn new(model: &str) -> Self {
        Self::with_tool_argument_rules(model, std::collections::HashMap::new())
    }

    #[cfg(test)]
    fn with_required_tool_args(
        model: &str,
        required_tool_args: std::collections::HashMap<String, Vec<String>>,
    ) -> Self {
        let rules = required_tool_args
            .into_iter()
            .map(|(name, required)| {
                (
                    name,
                    ToolArgumentRules {
                        required,
                        min_lengths: std::collections::HashMap::new(),
                    },
                )
            })
            .collect();
        Self::with_tool_argument_rules(model, rules)
    }

    fn with_tool_argument_rules(
        model: &str,
        tool_argument_rules: std::collections::HashMap<String, ToolArgumentRules>,
    ) -> Self {
        AnthSse {
            buf: Vec::new(),
            model: model.to_string(),
            role_sent: false,
            next_tool_idx: 0,
            tool_blocks: std::collections::HashMap::new(),
            tool_argument_rules,
            message_stop_seen: false,
            saw_thinking_block: false,
            saw_answer_block: false,
            input_tokens: 0,
            output_tokens: 0,
            input_usage_reported: false,
            output_usage_reported: false,
            cache_read: 0,
            cache_create: 0,
            stop_reason: "stop".into(),
        }
    }

    /// 故障签名：完整收流却只有思考、没有任何 text/tool_use 块，且 stop_reason 是
    /// end_turn（映射后为 "stop"）。官方 API 不会这样收尾——这是中转深思考超限丢块。
    fn thinking_only_end_turn(&self) -> bool {
        self.saw_thinking_block && !self.saw_answer_block && self.stop_reason == "stop"
    }

    fn validated_tool_arguments(&self, block: &AnthToolStream) -> Result<String, String> {
        validate_streamed_tool_arguments(
            "Anthropic",
            &block.name,
            &block.arguments,
            self.tool_argument_rules.get(&block.name),
        )
    }
    fn chunk(&self, delta: serde_json::Value, finish: Option<&str>) -> Vec<u8> {
        let choice = json!({"index":0,"delta":delta,"finish_reason": match finish { Some(f) => json!(f), None => serde_json::Value::Null }});
        format!(
            "data: {}\n\n",
            json!({"object":"chat.completion.chunk","model":self.model,"choices":[choice]})
        )
        .into_bytes()
    }
    fn ensure_role(&mut self, out: &mut Vec<u8>) {
        if !self.role_sent {
            out.extend(self.chunk(json!({"role":"assistant","content":""}), None));
            self.role_sent = true;
        }
    }
    /// Record any token counts this event carries, from either `usage` or
    /// `message.usage`, regardless of the event's `type`.
    ///
    /// Counts only ever increase: a relay that reports a running `output_tokens` on
    /// several events must not have its final (largest) figure replaced by an earlier
    /// partial one, and cache figures behave the same way. Nothing is inferred — a
    /// field that never arrives leaves its `*_usage_reported` flag false, so billing
    /// still refuses to charge for tokens the provider never confirmed.
    fn harvest_usage(&mut self, ev: &serde_json::Value) {
        for pointer in ["/usage", "/message/usage"] {
            let Some(u) = ev.pointer(pointer) else {
                continue;
            };
            if !u.is_object() {
                continue;
            }
            let read = |key: &str| u.get(key).and_then(|v| v.as_i64()).filter(|v| *v >= 0);
            if let Some(v) = read("input_tokens") {
                if v >= self.input_tokens {
                    self.input_tokens = v;
                }
                self.input_usage_reported = true;
            }
            if let Some(v) = read("output_tokens") {
                if v >= self.output_tokens {
                    self.output_tokens = v;
                }
                self.output_usage_reported = true;
            }
            if let Some(v) = read("cache_read_input_tokens") {
                self.cache_read = self.cache_read.max(v);
            }
            if let Some(v) = read("cache_creation_input_tokens") {
                self.cache_create = self.cache_create.max(v);
            }
        }
    }
    fn push(&mut self, bytes: &[u8]) -> Result<Vec<u8>, String> {
        self.buf.extend_from_slice(bytes);
        let mut out: Vec<u8> = Vec::new();
        while let Some(nl) = self.buf.iter().position(|&b| b == b'\n') {
            let raw: Vec<u8> = self.buf.drain(0..=nl).collect();
            let line = std::str::from_utf8(&raw)
                .map_err(|err| format!("Anthropic SSE contains invalid UTF-8: {err}"))?
                .trim();
            let data = match line.strip_prefix("data:") {
                Some(d) => d.trim(),
                None => continue,
            };
            if data.is_empty() {
                continue;
            }
            let ev: serde_json::Value = serde_json::from_str(data)
                .map_err(|err| format!("invalid Anthropic SSE JSON: {err}"))?;
            // Harvest usage from WHEREVER it appears, before the per-type handling.
            //
            // Anthropic's own spec carries final token counts in `message_delta`, and
            // that is all this parser used to read (plus input from `message_start`).
            // Relays in front of the real API don't all follow that: some attach the
            // final `usage` to `message_stop`, some to a top-level `usage` on another
            // event. When it landed anywhere else, `output_usage_reported` stayed false,
            // `usage_is_authoritative()` returned false, and `compute_cost` billed the
            // call as **zero** — production was logging "provider omitted authoritative
            // usage" for ~18% of Claude calls, opus-5 included.
            //
            // This only records numbers the provider actually sent (never estimates),
            // and only ever moves a count upward, so an early partial figure can't
            // overwrite a larger final one.
            self.harvest_usage(&ev);
            match ev.get("type").and_then(|t| t.as_str()) {
                // Token counts are handled by `harvest_usage` above for every event
                // type; the per-type arms below only deal with content and control flow.
                Some("message_start") => {
                    self.ensure_role(&mut out);
                }
                Some("content_block_start") => {
                    let idx = ev.get("index").and_then(|v| v.as_i64()).ok_or_else(|| {
                        "Anthropic content_block_start is missing a numeric index".to_string()
                    })?;
                    let cb = ev.get("content_block");
                    match cb.and_then(|c| c.get("type")).and_then(|t| t.as_str()) {
                        Some("thinking") | Some("redacted_thinking") => self.saw_thinking_block = true,
                        Some("text") | Some("tool_use") => self.saw_answer_block = true,
                        _ => {}
                    }
                    if cb.and_then(|c| c.get("type")).and_then(|t| t.as_str()) == Some("tool_use") {
                        if self.tool_blocks.contains_key(&idx) {
                            return Err(format!(
                                "Anthropic tool_use reused content block index {idx}"
                            ));
                        }
                        let ti = self.next_tool_idx;
                        self.next_tool_idx += 1;
                        let id = cb
                            .and_then(|c| c.get("id"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let name = cb
                            .and_then(|c| c.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let input = cb
                            .and_then(|c| c.get("input"))
                            .cloned()
                            .unwrap_or_else(|| json!({}));
                        let initial_arguments = match input.as_object() {
                            Some(input) if input.is_empty() => String::new(),
                            Some(_) => serde_json::to_string(&input).map_err(|err| {
                                format!(
                                    "Anthropic tool_use {name:?} contains unserializable input: {err}"
                                )
                            })?,
                            None => {
                                return Err(format!(
                                    "Anthropic tool_use {name:?} input must be a JSON object"
                                ));
                            }
                        };
                        self.tool_blocks.insert(
                            idx,
                            AnthToolStream {
                                tool_index: ti,
                                name: name.to_string(),
                                arguments: initial_arguments.clone(),
                                stopped: false,
                            },
                        );
                        self.ensure_role(&mut out);
                        out.extend(self.chunk(json!({"tool_calls":[{"index":ti,"id":id,"type":"function","function":{"name":name,"arguments":initial_arguments}}]}), None));
                    }
                }
                Some("content_block_delta") => {
                    match ev.pointer("/delta/type").and_then(|t| t.as_str()) {
                        Some("text_delta") => {
                            if let Some(t) = ev.pointer("/delta/text").and_then(|v| v.as_str()) {
                                self.saw_answer_block = true;
                                self.ensure_role(&mut out);
                                out.extend(self.chunk(json!({"content": t}), None));
                            }
                        }
                        Some("thinking_delta") => {
                            if let Some(t) = ev.pointer("/delta/thinking").and_then(|v| v.as_str())
                            {
                                self.saw_thinking_block = true;
                                self.ensure_role(&mut out);
                                out.extend(self.chunk(json!({"reasoning_content": t}), None));
                            }
                        }
                        Some("input_json_delta") => {
                            let idx = ev.get("index").and_then(|v| v.as_i64()).ok_or_else(|| {
                                "Anthropic input_json_delta is missing a numeric content block index"
                                    .to_string()
                            })?;
                            let pj = ev
                                .pointer("/delta/partial_json")
                                .and_then(|v| v.as_str())
                                .ok_or_else(|| {
                                    format!(
                                        "Anthropic input_json_delta for index {idx} is missing partial_json"
                                    )
                                })?;
                            let block = self.tool_blocks.get_mut(&idx).ok_or_else(|| {
                                format!(
                                    "Anthropic input_json_delta references unknown content block index {idx}"
                                )
                            })?;
                            if block.stopped {
                                return Err(format!(
                                    "Anthropic input_json_delta arrived after content_block_stop for index {idx}"
                                ));
                            }
                            block.arguments.push_str(pj);
                            let ti = block.tool_index;
                            out.extend(self.chunk(
                                json!({"tool_calls":[{"index":ti,"function":{"arguments": pj}}]}),
                                None,
                            ));
                        }
                        _ => {}
                    }
                }
                Some("message_delta") => {
                    if let Some(sr) = ev.pointer("/delta/stop_reason").and_then(|v| v.as_str()) {
                        self.stop_reason = match sr {
                            "tool_use" => "tool_calls",
                            "max_tokens" => "length",
                            _ => "stop",
                        }
                        .into();
                    }
                }
                Some("content_block_stop") => {
                    if let Some(idx) = ev.get("index").and_then(|v| v.as_i64()) {
                        if let Some(block) = self.tool_blocks.get(&idx) {
                            if block.stopped {
                                return Err(format!(
                                    "Anthropic content block index {idx} stopped more than once"
                                ));
                            }
                            let arguments = self.validated_tool_arguments(block)?;
                            let emit_empty_object = block.arguments.trim().is_empty();
                            let ti = block.tool_index;
                            if emit_empty_object {
                                out.extend(self.chunk(json!({"tool_calls":[{"index":ti,"function":{"arguments":arguments}}]}), None));
                            }
                        }
                        if let Some(block) = self.tool_blocks.get_mut(&idx) {
                            block.stopped = true;
                        }
                    }
                }
                Some("message_stop") => {
                    self.message_stop_seen = true;
                }
                Some("error") => {
                    let message = ev
                        .pointer("/error/message")
                        .and_then(|value| value.as_str())
                        .unwrap_or("unknown Anthropic streaming error");
                    return Err(format!("Anthropic streaming error: {message}"));
                }
                _ => {} // ping / text block stops → emit nothing
            }
        }
        Ok(out)
    }
    fn usage(&self) -> serde_json::Value {
        json!({
            "input_tokens": self.input_tokens, "output_tokens": self.output_tokens,
            "cache_read_input_tokens": self.cache_read, "cache_creation_input_tokens": self.cache_create,
            "prompt_tokens": self.input_tokens, "completion_tokens": self.output_tokens,
            "total_tokens": self.input_tokens + self.output_tokens,
        })
    }
    fn usage_is_authoritative(&self) -> bool {
        self.input_usage_reported && self.output_usage_reported
    }
    fn finish(&self) -> Result<Vec<u8>, String> {
        if !self.buf.iter().all(u8::is_ascii_whitespace) {
            return Err("Anthropic stream ended with an incomplete SSE frame".to_string());
        }
        if !self.message_stop_seen {
            return Err("Anthropic stream ended before message_stop".to_string());
        }
        for block in self.tool_blocks.values() {
            if !block.stopped {
                return Err(format!(
                    "Anthropic stream ended before tool_use {:?} completed",
                    block.name
                ));
            }
            self.validated_tool_arguments(block)?;
        }
        let mut out = self.chunk(json!({}), Some(&self.stop_reason));
        out.extend(format!("data: {}\n\n", json!({"object":"chat.completion.chunk","model":self.model,"choices":[],"usage":self.usage()})).into_bytes());
        out.extend_from_slice(b"data: [DONE]\n\n");
        Ok(out)
    }
}

/// POST /v1/audio/transcriptions — OpenAI-compatible speech-to-text for the IDE's voice input.
/// Auth via a Michael API key (or the login JWT), same as chat. Forwards the uploaded clip to the
/// configured Whisper upstream (Groq's free whisper-large-v3 by default) and returns its JSON
/// verbatim. Does NOT use the DB `models` connections — those aggregators don't do audio (404).
pub async fn audio_transcriptions(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    // ---- auth (mirror chat_completions: api_keys row, else login JWT) ----
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::unauthorized("缺少 API Key"))?;
    let _uid: uuid::Uuid = match sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT user_id FROM api_keys WHERE api_key = $1",
    )
    .bind(&token)
    .fetch_optional(&state.db)
    .await?
    {
        Some(u) => u,
        None => crate::auth::user_from_jwt(&state.cfg, &token)
            .ok_or_else(|| AppError::unauthorized("登录已失效或密钥无效"))?,
    };
    // Transcription burns a paid third-party key. Identity alone is not enough —
    // require the same access the chat route does, and cap the per-user rate.
    require_paid_access(&state, &headers).await?;

    if state.cfg.transcribe_api_key.is_empty() {
        return Err(AppError::bad("转写服务未配置"));
    }

    // ---- read the multipart form: file (required) + optional language ----
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_name = "speech.m4a".to_string();
    let mut content_type = "audio/mp4".to_string();
    let mut language: Option<String> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad(format!("表单解析失败: {e}")))?
    {
        match field.name().unwrap_or("") {
            "file" => {
                if let Some(n) = field.file_name() {
                    if !n.is_empty() {
                        file_name = n.to_string();
                    }
                }
                if let Some(ct) = field.content_type() {
                    if ct.contains('/') {
                        content_type = ct.to_string();
                    }
                }
                file_bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| AppError::bad(format!("读取音频失败: {e}")))?
                        .to_vec(),
                );
            }
            "language" => language = field.text().await.ok().filter(|s| !s.is_empty()),
            _ => {
                let _ = field.bytes().await;
            }
        }
    }
    let file_bytes = file_bytes.ok_or_else(|| AppError::bad("缺少音频文件"))?;
    if file_bytes.len() < 256 {
        return Err(AppError::bad("音频太短或为空"));
    }

    // ---- forward to the Whisper upstream ----
    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(file_name)
        .mime_str(&content_type)
        .map_err(|e| AppError::bad(format!("音频类型无效: {e}")))?;
    let mut form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", state.cfg.transcribe_model.clone())
        .text("response_format", "json");
    if let Some(l) = language {
        form = form.text("language", l);
    }

    let resp = GW_HTTP
        .post(&state.cfg.transcribe_url)
        .header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", state.cfg.transcribe_api_key),
        )
        .multipart(form)
        .send()
        .await
        .map_err(|e| AppError::bad(format!("转写上游连接失败: {e}")))?;

    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let ctype = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();
    let body = resp
        .bytes()
        .await
        .map_err(|e| AppError::bad(format!("转写上游读取失败: {e}")))?;

    Ok(Response::builder()
        .status(status)
        .header(axum::http::header::CONTENT_TYPE, ctype)
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response()))
}

pub async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    let request_id = ide_request_id(&headers)?;
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::unauthorized("缺少 API Key"))?;
    let uid: uuid::Uuid = match sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT user_id FROM api_keys WHERE api_key = $1",
    )
    .bind(&token)
    .fetch_optional(&state.db)
    .await?
    {
        Some(u) => {
            let _ = sqlx::query("UPDATE api_keys SET last_used_at = now() WHERE api_key = $1")
                .bind(&token)
                .execute(&state.db)
                .await;
            u
        }
        // Also accept the login JWT directly (the IDE authenticates with it).
        None => crate::auth::user_from_jwt(&state.cfg, &token)
            .ok_or_else(|| AppError::unauthorized("登录已失效或密钥无效"))?,
    };

    if !body.is_object() {
        return Err(AppError::bad("请求体需为 JSON 对象"));
    }
    // Never trust desktop/provider-agnostic cache markers. Strip them before route selection;
    // native Anthropic routes add gateway-owned breakpoints after the actual connection is known.
    strip_cache_control(&mut body);
    // L0 server-side assembly: when the IDE opts in (x-ide-mode header), inject the system
    // prompt + requested tool schemas from the registry HERE, so the client ships neither.
    // No header → no-op (existing behavior untouched).
    crate::prompts::assemble_into(&headers, &mut body)
        .map_err(|err| AppError::internal(format!("IDE prompt graph unavailable: {err}")))?;
    let model_id = body
        .get("model")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| AppError::bad("缺少 model"))?;

    let conns = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE active = true ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await?;
    let candidates: Vec<Model> = conns
        .into_iter()
        .filter(|m| allowed_ids(m).contains(&model_id))
        .collect();
    let route_count = candidates.len();
    let primary_conn = candidates
        .first()
        .cloned()
        .ok_or_else(|| AppError::bad(format!("模型 {model_id} 不可用")))?;

    // Refill the 5h30m window + reset the weekly counter when due.
    sqlx::query(
        "UPDATE users SET \
         quota_window_cents = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN LEAST(quota_window_cap_cents, quota_total_cents) ELSE quota_window_cents END, \
         quota_window_reset_at = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN now() + interval '5 hours 30 minutes' ELSE quota_window_reset_at END, \
         quota_week_used_cents = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN 0 ELSE quota_week_used_cents END, \
         quota_week_reset_at = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN now() + interval '7 days' ELSE quota_week_reset_at END \
         WHERE id = $1",
    )
    .bind(uid)
    .execute(&state.db)
    .await?;

    // Access gate: active-membership quota (window/total/weekly) OR pay-as-you-go credits.
    let (plan, plan_exp, q_total, q_window, q_weekly_cap, q_week_used, credits): (String, Option<chrono::DateTime<chrono::Utc>>, i64, i64, i64, i64, i64) =
        sqlx::query_as("SELECT plan, plan_expires_at, quota_total_cents, quota_window_cents, quota_weekly_cap_cents, quota_week_used_cents, credits_cents FROM users WHERE id = $1")
            .bind(uid)
            .fetch_one(&state.db)
            .await?;
    let plan_active = plan != "none" && plan_exp.is_none_or(|e| e > chrono::Utc::now());
    let quota_ok = plan_active
        && q_total > 0
        && q_window > 0
        && (q_weekly_cap == 0 || q_week_used < q_weekly_cap);
    let use_quota = quota_ok;
    // Free-flagged models are paid from the daily 点 pool, so the quota/credits gate below
    // does not apply to them — and crucially, passing that gate must NOT let a user keep
    // calling a free model on an empty pool. Without this the allowance was decorative: any
    // member with quota could use free models forever at 0 点.
    //
    // Checked across every candidate route, since one model can be served by more than one
    // connection; if ANY route that could serve this request bills from the pool, the pool is
    // what must have room.
    let free_here = candidates.iter().any(|c| effective_billing(c, &model_id).2);
    if free_here && free_points_balance(&state, uid).await <= 0 {
        return Err(AppError {
            status: StatusCode::PAYMENT_REQUIRED,
            msg: "今日免费额度已用完，明天 0 点重置（或改用付费模型）".into(),
        });
    }
    if !free_here && !quota_ok && credits <= 0 {
        let msg = if plan_active && q_total <= 0 {
            "总额度已用完"
        } else if plan_active && q_window <= 0 {
            "本时段额度已用完，请等待刷新（每 5.5 小时）"
        } else if plan_active && q_weekly_cap > 0 && q_week_used >= q_weekly_cap {
            "本周额度已用完"
        } else {
            "请先开通会员或充值额度"
        };
        return Err(AppError {
            status: StatusCode::PAYMENT_REQUIRED,
            msg: msg.into(),
        });
    }

    // The gate above only proves the balance is positive, not that it covers this
    // call, and settlement happens after the upstream responds. Serially that lets a
    // user overspend by exactly one request per top-up (the next request is refused);
    // concurrently there was nothing bounding how many unsettled requests could pass
    // the same positive-balance check at once, so N parallel calls multiplied the
    // overdraft by N. This caps unsettled in-flight requests per user, which closes
    // the amplification without changing how anything is priced.
    //
    // Redis-backed so the cap holds across gateway instances. The guard releases on
    // drop — including the early-return and panic paths — and the key carries a TTL so
    // a hard crash can't strand a user at their limit.
    let inflight_guard = InFlightGuard::acquire(&state, uid).await?;

    // michael-compression：严格 opt-in。没有请求档位时这里直接返回，body 一个字节都不动，
    // 现有流量的行为与这个特性上线前完全一致。
    let mut compression_applied: Option<crate::compression::Tier> = None;
    // 签发的前缀令牌必须回传给客户端，否则续传这条腿是断的：网关每轮都签一个新令牌
    // 写进 Redis，客户端从不回发，于是 Redis 只写不读，而客户端每轮都得上传完整历史
    // —— 2m/5m 两档在物理上根本达不到。
    let mut compression_prefix: Option<(String, usize)> = None;
    // 总开关默认关闭（MICHAEL_COMPRESSION_ENABLED）。发布前审查在这条链路上确认了多处
    // 会破坏线上请求的缺陷，最严重的是 compression_write_back 把每条消息重写成
    // {role, content} —— tool_calls / tool_call_id 全部丢失，而 agent 模式发的正是
    // 这些，上游会直接拒收。开关打开前必须先修完；关着时 body 一个字节都不动。
    if state.cfg.compression_enabled {
        let requested_tier = compression_tier_from(&headers, &body);
        if let Some(requested) = requested_tier {
            // 档位是付费能力：按会员套餐钳位。超出权限时下调而不是拒绝，用户仍然拿到他
            // 买到的那一档，而不是在长对话跑到一半时被打断。
            let allowed = crate::compression::max_tier_for_plan(&plan, plan_active, credits);
            match crate::compression::clamp_tier(requested, allowed) {
                Some(tier) => {
                    if tier != requested {
                        tracing::info!(
                            %uid, plan = %plan, requested = requested.as_str(), granted = tier.as_str(),
                            "michael-compression: 请求档位超出套餐权限，已下调"
                        );
                    }
                    // `mc_prefix` 必须由 apply 先读取。旧顺序在调用 apply 之前就把它删了，
                    // 导致服务端永远拿不到客户端回传的前缀，Redis 只写不读。
                    compression_prefix =
                        apply_michael_compression(&state, &mut body, &model_id, tier, uid).await?;
                    compression_applied = Some(tier);
                }
                None => {
                    tracing::info!(
                        %uid, plan = %plan,
                        "michael-compression: 当前套餐不含该能力，本轮不压缩"
                    );
                }
            }
        }
        // 所有 Michael 私有协议字段都必须在任何上游请求之前移除；放在 apply 之后，既能
        // 让压缩层读取前缀，又不会把字段泄漏给供应商。
        compression_strip_protocol_fields(&mut body);
    }

    let streaming = body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    // Force the upstream to emit a final usage chunk so streaming billing reads real
    // (cache-discounted) tokens. This MUST overwrite whatever the client sent: with
    // `entry().or_insert_with()` a caller could pass
    // `"stream_options":{"include_usage":false}`, the upstream would never emit usage,
    // `parse_usage_from_sse` returned None and `compute_cost` billed 0 — unlimited free
    // flagship inference for anyone holding a valid key.
    if streaming {
        if let Some(obj) = body.as_object_mut() {
            let opts = obj
                .entry("stream_options")
                .or_insert_with(|| serde_json::json!({}));
            if !opts.is_object() {
                *opts = serde_json::json!({});
            }
            if let Some(opts) = opts.as_object_mut() {
                opts.insert("include_usage".into(), serde_json::Value::Bool(true));
            }
        }
    }
    // ── Gateway response cache ────────────────────────────────────────────────
    // Identical request (same model + messages + params) → serve the stored
    // response: NO upstream call, 0 cost. Real caching the user controls, working
    // for EVERY model regardless of whether the upstream caches. Best-effort: any
    // Redis hiccup or miss just falls through to a normal upstream call. The quota
    // gate already ran above, so a hit still requires access — it just costs nothing.
    let ckey = gw_cache_key(uid, &body);
    {
        let mut rconn = state.redis.clone();
        let hit: Option<Vec<u8>> = redis::cmd("GET")
            .arg(&ckey)
            .query_async(&mut rconn)
            .await
            .ok()
            .flatten();
        if let Some(bytes) = hit.filter(|bytes| response_cache_safe(bytes)) {
            bill(
                &state,
                uid,
                primary_conn.id,
                0,
                use_quota,
                &BillTokens {
                    model_name: model_id.clone(),
                    request_id: request_id.clone(),
                    ..Default::default()
                },
                false,
                0,
            )
            .await; // record a 0-cost cache hit
            let ct = if streaming {
                "text/event-stream"
            } else {
                "application/json"
            };
            // 缓存命中也必须回传压缩头。压缩在这之前就已经跑过、前缀也签发了，
            // 但这条返回路径原来只带 x-gateway-cache —— 客户端于是拿不到令牌，
            // 下一轮只能整份重传，续传链在"恰好命中缓存"的那一轮被悄悄打断。
            let mut cache_builder = Response::builder()
                .status(StatusCode::OK)
                .header(axum::http::header::CONTENT_TYPE, ct)
                .header("x-gateway-cache", "hit")
                .header("cache-control", "no-cache");
            if let Some(tier) = compression_applied {
                cache_builder =
                    cache_builder.header("x-michael-compression-applied", tier.as_str());
            }
            if let Some((tok, covered)) = compression_prefix.as_ref() {
                cache_builder = cache_builder
                    .header("x-michael-compression-prefix", tok.as_str())
                    .header("x-michael-compression-covered", covered.to_string());
            }
            return cache_builder
                .body(Body::from(bytes))
                .map_err(|e| AppError::internal(e.to_string()));
        }
    }
    // ── max_tokens guardrail for thinking (all protocols) ───────────────────
    // Chinese aggregators (zyz etc.) convert reasoning_effort / thinking to Anthropic thinking
    // with budget_tokens; if max_tokens < budget_tokens the upstream rejects. The native
    // Anthropic path (oai_to_anthropic) handles this, but OpenAI-protocol connections pass
    // body through unchanged — so bump max_tokens here before the fork.
    {
        let has_thinking = body.get("thinking").is_some()
            || body
                .get("reasoning_effort")
                .and_then(|v| v.as_str())
                .is_some_and(|e| !e.is_empty() && e != "off");
        if has_thinking {
            let budget = body
                .pointer("/thinking/budget_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let min_mt = (budget + 8000).max(32000);
            let cur_mt = body.get("max_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
            if cur_mt < min_mt {
                if let Some(obj) = body.as_object_mut() {
                    obj.insert("max_tokens".into(), json!(min_mt.min(128000)));
                }
            }
        }
    }
    let tool_argument_rules = tool_argument_rules(&body);
    // Pooled client (warm keep-alive connections) instead of a fresh handshake
    // per request. Streaming stays open-ended; non-streaming gets a sane cap.
    //
    // Upstream providers (zyz et al.) intermittently return 502/503/504/429 or
    // drop a kept-alive connection mid-flight — the user just sees "网关又出问题".
    // Retry such *transient* failures with bounded exponential backoff so a
    // short provider flap is absorbed instead of surfaced. We only retry BEFORE
    // streaming the body has started (a send error or a bad status line), so no
    // half-streamed response is ever double-sent, and billing still happens once,
    // after success. Failed routes get a short in-memory cooldown so the next
    // request prefers another same-model route when the admin has configured one.
    let model_name = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("该模型")
        .to_string();
    // Map an upstream error to a friendly, actionable Chinese message.
    fn friendly_upstream(status: u16, low: &str) -> String {
        if low.contains("insufficient_balance") || low.contains("insufficient account balance") {
            "上游供应商账户余额不足。请在后台为该模型线路充值，或切换到其他可用线路。".into()
        } else if low.contains("forbidden") || low.contains("未授权") {
            "上游暂不可用（供应商未授权 / 账户异常）。请换个模型，或联系模型供应商开通 / 续费。"
                .into()
        } else if low.contains("no available") || low.contains("没有可用") {
            "上游暂无可用账号。请换个模型，或稍后再试。".into()
        } else if status == 429
            || low.contains("rate")
            || low.contains("frequent")
            || low.contains("过于频繁")
        {
            "请求过于频繁，请稍后再试。".into()
        } else if status == 401 || low.contains("unauthorized") || low.contains("invalid api key") {
            "上游密钥无效。请在后台「模型系统」更新该连接的 API Key。".into()
        } else if status == 400 {
            let detail = safe_upstream_error_excerpt(low);
            if detail.is_empty() {
                "上游拒绝了请求（400），但没有返回更细原因。".into()
            } else {
                format!("上游拒绝了请求（400）：{detail}")
            }
        } else {
            "上游暂时不可用，请换个模型或稍后再试。".into()
        }
    }
    // 深思考只放宽响应头之后的首个有效 token / stream idle 窗口。响应头代表线路健康，
    // 在它出现前，普通与深思请求共用同一个短 transport deadline。
    let deep_thinking = request_is_deep_thinking(&body);
    let agentic_request = headers
        .get("x-ide-mode")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|mode| mode != "chat")
        || body
            .get("tools")
            .and_then(|tools| tools.as_array())
            .is_some_and(|tools| !tools.is_empty());
    let max_header_wait = max_header_wait_for_request(deep_thinking, agentic_request);
    // Send with retry — but ONLY retry *transient* failures (502/503/504/429 or a
    // dropped connection). "forbidden / unauthorized / no available account" is
    // PERSISTENT: retrying just makes the user wait ~15s for the same error, so we
    // fail FAST with a friendly message. Billing only happens after a success.
    let (resp, conn) = {
        let mut success = None;
        let mut err_status = 502u16;
        let mut err_low = String::new();
        let mut selected_conn = None;
        let mut attempted_sends = 0u32;
        let now = Instant::now();
        // Total budget for finding a working upstream route, and a per-attempt ceiling
        // on the header wait.
        //
        // Two things went wrong without these. (1) `GW_HTTP` sets only a
        // `connect_timeout`, and the streaming path deliberately skips `.timeout()`
        // (reqwest would apply it to the whole body and cut long answers off), so once
        // the TCP connect succeeded `req.send()` waited for response headers
        // *indefinitely* — a provider that accepts the connection and then stalls hung
        // the gateway forever. (2) Even when attempts did fail, 6 tries plus backoff
        // could burn 40s+ before the client heard anything.
        //
        // Either way the IDE hit its own header timeout (20s, 45s for deep thinking),
        // gave up, and fast-retried — which starts a *fresh* gateway request and a
        // fresh set of upstream calls while the abandoned ones are still open upstream.
        // That is the "extra /v1/messages calls keep coming" storm. The gateway must
        // therefore always answer before the client's deadline, even if that means
        // abandoning retries.
        //
        // A healthy upstream sends headers in well under a second even when the first
        // token is far away (that wait is bounded separately by the client's
        // first-progress timeout), so a header wait this long means a broken route, not
        // a thinking model.
        let route_budget = route_budget_for_headers(&headers, deep_thinking);
        let route_deadline = now + route_budget;
        let mut ordered_candidates: Vec<&Model> = Vec::with_capacity(candidates.len());
        let mut cooled_candidates: Vec<&Model> = Vec::new();
        for candidate in &candidates {
            if route_count > 1 && route_cooldown_remaining(candidate.id, now).is_some() {
                cooled_candidates.push(candidate);
            } else {
                ordered_candidates.push(candidate);
            }
        }
        ordered_candidates.extend(cooled_candidates);

        'routes: for candidate in ordered_candidates {
            // protocol="anthropic" → native /v1/messages with translated OpenAI⇄Anthropic body;
            // else OpenAI-compat /chat/completions passthrough. Multiple active connections may
            // expose the same model id; try the next line when the current one is dead instead of
            // failing the whole IDE request on the first 502.
            let candidate_anthropic = candidate.protocol == "anthropic";
            let candidate_url = if candidate_anthropic {
                format!("{}/messages", api_base(&candidate.base_url))
            } else {
                format!("{}/chat/completions", api_base(&candidate.base_url))
            };
            let mut candidate_upstream_body = if candidate_anthropic {
                match oai_to_anthropic_with_cache(&body, route_supports_prompt_cache(candidate)) {
                    Ok(v) => v,
                    Err(err) => {
                        err_status = 400;
                        err_low =
                            format!("Anthropic request conversion failed: {err}").to_lowercase();
                        continue;
                    }
                }
            } else {
                serde_json::Value::Null
            };
            // 该线路正处于"深思考丢块"钳位期：把思考预算压到实测安全值再发。
            if candidate_anthropic
                && thinking_clip_active(candidate.id)
                && clip_thinking_budget(&mut candidate_upstream_body)
            {
                tracing::info!(
                    route_id = %candidate.id,
                    clipped_budget = THINKING_CLIP_SAFE_BUDGET,
                    "route recently dropped post-thinking blocks; thinking budget clipped for this request"
                );
            }
            let mut route_attempts = 0u32;
            let mut route_failed_transient = false;
            // With several equivalent routes, probe each route once before spending time on a
            // duplicate attempt. With the single Claude route currently configured, permit one
            // fast retry on a fresh HTTP/1.1 connection even while the route is cooling.
            let candidate_max_attempts = if route_count > 1 {
                1
            } else {
                CHAT_UPSTREAM_MAX_ATTEMPTS_PER_ROUTE
            };
            for attempt in 0u32..candidate_max_attempts {
                // Out of budget: stop probing and let the caller report the last error,
                // so the client gets a real response instead of timing out and retrying.
                let remaining = route_deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    tracing::warn!(
                        model = %model_id,
                        attempted_sends,
                        budget_secs = route_budget.as_secs(),
                        "upstream route budget exhausted; answering the client instead of retrying further"
                    );
                    break 'routes;
                }
                // The first attempt uses the warm HTTP/1.1 pool. Every retry owns a client with
                // no idle pool, guaranteeing it cannot reuse the connection that just stalled.
                let chat_client = if attempt == 0 {
                    GW_CHAT_HTTP.clone()
                } else {
                    build_chat_http_client(0)
                };
                let req0 = chat_client.post(&candidate_url);
                let mut req = if candidate_anthropic {
                    // 1M context must be explicitly enabled upstream. Anthropic's own API ships
                    // 1M by default on Opus 4.6+/Sonnet 4.6+/Fable, but resellers front it behind
                    // the same beta flag Anthropic uses for Sonnet 4/4.5 — observed verbatim:
                    //   400 {"error":"1m context is fully available; please enable 1m context and retry"}
                    // So: whenever this model's native window is >= 1M, or it has a beta-gated
                    // 1M entry, send the flag. It is a no-op where 1M is already default, and it
                    // is the difference between working and a hard 400 where it is not.
                    let wants_1m = official_contexts(&model_id)
                        .iter()
                        .any(|(tokens, _)| *tokens >= 1_000_000);
                    let mut r = req0
                        .header("x-api-key", &candidate.api_key)
                        .header("anthropic-version", "2023-06-01");
                    if wants_1m {
                        r = r.header("anthropic-beta", "context-1m-2025-08-07");
                    }
                    r.json(&candidate_upstream_body)
                } else {
                    req0.header("Authorization", format!("Bearer {}", candidate.api_key))
                        .json(&body)
                };
                if !streaming {
                    req = req.timeout(Duration::from_secs(120));
                }
                route_attempts += 1;
                attempted_sends += 1;
                // `tokio::time::timeout` around `send()` bounds only the header phase —
                // `send()` resolves as soon as the status line and headers arrive, so the
                // response body/stream that follows is untouched. That is the piece
                // reqwest's own `.timeout()` cannot express for a streaming response.
                let header_wait = if attempt == 0 && candidate_max_attempts > 1 {
                    // Measured, not guessed: a route with a known baseline is judged
                    // against itself; an unknown one keeps FIRST_ATTEMPT_HEADER_WAIT.
                    let adaptive =
                        adaptive_first_attempt_wait(candidate.id, FIRST_ATTEMPT_HEADER_WAIT);
                    remaining.min(max_header_wait).min(adaptive)
                } else {
                    remaining.min(max_header_wait)
                };
                let send_started = Instant::now();
                let sent = match tokio::time::timeout(header_wait, req.send()).await {
                    Ok(result) => {
                        let header_ms = send_started.elapsed().as_millis();
                        match &result {
                            Ok(response) => {
                                // Feed the route's own baseline so the next first-attempt
                                // cutover is judged against measured behaviour.
                                record_header_success(candidate.id, header_ms);
                                tracing::info!(
                                    model = %model_id,
                                    route_id = %candidate.id,
                                    attempt = attempt + 1,
                                    fresh_connection = attempt > 0,
                                    upstream_status = response.status().as_u16(),
                                    upstream_header_ms = header_ms,
                                    "upstream response headers received"
                                )
                            }
                            Err(error) => tracing::warn!(
                                model = %model_id,
                                route_id = %candidate.id,
                                attempt = attempt + 1,
                                fresh_connection = attempt > 0,
                                upstream_header_ms = header_ms,
                                error = %error,
                                "upstream request failed before response headers"
                            ),
                        }
                        result
                    }
                    Err(_) => {
                        // Dropping the future cancels the request, which is what stops
                        // abandoned calls from piling up at the provider.
                        err_status = 504;
                        err_low = format!(
                            "upstream sent no response headers within {}s",
                            header_wait.as_secs()
                        );
                        record_header_stall(candidate.id);
                        tracing::warn!(
                            model = %model_id,
                            url = %candidate_url,
                            attempt = attempt + 1,
                            waited_ms = header_wait.as_millis(),
                            "upstream stalled before response headers"
                        );
                        route_failed_transient = true;
                        if attempt + 1 >= candidate_max_attempts {
                            break;
                        }
                        if !wait_for_upstream_retry(FAST_HEADER_RETRY_DELAY, route_deadline).await {
                            break 'routes;
                        }
                        tracing::info!(
                            model = %model_id,
                            route_id = %candidate.id,
                            next_attempt = attempt + 2,
                            "retrying header-stalled route on a fresh connection"
                        );
                        continue;
                    }
                };
                match sent {
                    Ok(r) if r.status().is_success() => {
                        success = Some(r);
                        selected_conn = Some(candidate.clone());
                        break 'routes;
                    }
                    Ok(r) => {
                        err_status = r.status().as_u16();
                        let error_body_wait = route_deadline
                            .saturating_duration_since(Instant::now())
                            .min(MAX_ERROR_BODY_WAIT);
                        if error_body_wait.is_zero() {
                            route_failed_transient = true;
                            break;
                        }
                        err_low = match tokio::time::timeout(error_body_wait, r.text()).await {
                            Ok(Ok(text)) => text.to_lowercase(),
                            Ok(Err(error)) => error.to_string().to_lowercase(),
                            Err(_) => {
                                err_status = 504;
                                route_failed_transient = true;
                                tracing::warn!(
                                    model = %model_id,
                                    url = %candidate_url,
                                    waited_ms = error_body_wait.as_millis(),
                                    "upstream error response body stalled; cancelling route"
                                );
                                break;
                            }
                        };
                        let persistent = err_status == 401
                            || err_status == 403
                            || err_low.contains("forbidden")
                            || err_low.contains("unauthorized")
                            || err_low.contains("invalid api key")
                            || err_low.contains("未授权")
                            || err_low.contains("no available")
                            || err_low.contains("没有可用");
                        let transient = matches!(err_status, 502 | 503 | 504 | 429);
                        // A 400 that names the REQUEST as the problem is deterministic:
                        // the same body will be rejected by every remaining candidate, so
                        // failing over just multiplies one bad request by the route count
                        // while the user watches a spinner. Give up immediately and let the
                        // real upstream message reach them. (401/403 still fail over — those
                        // are per-route credentials, and another route may well be fine.)
                        if err_status == 400
                            && (err_low.contains("invalid_request_error")
                                || err_low.contains("is not supported for this model")
                                || err_low.contains("extra inputs are not permitted")
                                || err_low.contains("unexpected keyword"))
                        {
                            tracing::warn!(
                                model = %model_id,
                                excerpt = %safe_upstream_error_excerpt(&err_low),
                                "upstream rejected the request body; not failing over"
                            );
                            break 'routes;
                        }
                        if persistent || !transient {
                            break;
                        }
                        if attempt + 1 >= candidate_max_attempts {
                            route_failed_transient = true;
                            break;
                        }
                        if !wait_for_upstream_retry(
                            chat_upstream_retry_delay(attempt),
                            route_deadline,
                        )
                        .await
                        {
                            break 'routes;
                        }
                    }
                    // A send error means the request almost certainly never reached the
                    // server (incl. a stale pooled connection) — safe to re-send.
                    Err(e) => {
                        err_status = 502;
                        err_low = e.to_string().to_lowercase();
                        if attempt + 1 >= candidate_max_attempts {
                            route_failed_transient = true;
                            break;
                        }
                        if !wait_for_upstream_retry(
                            chat_upstream_retry_delay(attempt),
                            route_deadline,
                        )
                        .await
                        {
                            break 'routes;
                        }
                    }
                }
            }
            if route_failed_transient {
                mark_route_cooldown(candidate.id);
                tracing::warn!(
                    model = %model_name,
                    provider = %candidate.provider,
                    label = %candidate.label,
                    attempts = route_attempts,
                    status = err_status,
                    "chat upstream route exhausted transient retries; cooling route"
                );
            }
        }
        match (success, selected_conn) {
            (Some(r), Some(c)) => (r, c),
            (None, _) => {
                let downstream_status = upstream_failure_status(err_status, &err_low);
                tracing::warn!(
                    model = %model_name,
                    upstream_status = err_status,
                    downstream_status = downstream_status.as_u16(),
                    error_excerpt = %safe_upstream_error_excerpt(&err_low),
                    attempted_sends,
                    route_count,
                    "returning classified upstream failure"
                );
                let msg = format!(
                    "【{model_name}】{}{}",
                    friendly_upstream(err_status, &err_low),
                    chat_upstream_attempt_suffix(route_count, attempted_sends, err_status)
                );
                if headers.contains_key("x-ide-mode") {
                    return Response::builder()
                        .status(downstream_status)
                        .header(
                            axum::http::header::CONTENT_TYPE,
                            "text/plain; charset=utf-8",
                        )
                        .body(Body::from(msg))
                        .map_err(|e| AppError::internal(e.to_string()));
                }
                return Err(AppError {
                    status: downstream_status,
                    msg,
                });
            }
            _ => unreachable!("success response and selected connection are set together"),
        }
    };
    let status = resp.status();
    let anthropic = conn.protocol == "anthropic";

    if streaming {
        // 深思考请求（xhigh/max/带 thinking 预算）静默期可超 3 分钟：固定 180s 的上游
        // 空闲斩会在客户端窗口放宽后成为顶层杀手，这里跟档位一起放宽。`deep_thinking`
        // 在路由预算处已算好（见上方），这里直接复用。
        // Tee the upstream SSE: forward bytes to the client UNCHANGED while
        // accumulating the full stream so a complete response can be cached. Billing is
        // REAL: the trailing include_usage chunk gives true token counts → official price
        // × 倍率 (see compute_cost). Cache hits bill 0 (handled at the cache-hit return
        // above). 180s idle guard preserved inline.
        let ct = resp
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/event-stream")
            .to_string();
        let st = state.clone();
        let cid = conn.id;
        let rate = conn.rate;
        let admin_in = conn.input_price;
        let admin_out = conn.output_price;
        let cache_read_price = conn.cache_read_price;
        let cache_create_price = conn.cache_create_price;
        // Per-model override wins over the connection default; `free_pool` routes the charge
        // to the daily points pool instead of quota/wallet.
        let req_model = model_id.clone();
        let (bmode, percall, free_pool, free_micro) = effective_billing_micro(&conn, &model_id);
        let request_id_task = request_id.clone();
        // 思考钳位探测：只对"开了思考的 Anthropic 原生请求"检测丢块签名。
        let thinking_clip_probe = anthropic
            && (body.get("thinking").is_some()
                || body
                    .get("reasoning_effort")
                    .and_then(|v| v.as_str())
                    .is_some_and(|e| !e.is_empty() && e != "off"));
        let (model_in, model_out) = model_price_override(&conn.model_prices, &model_id);
        let ckey_task = ckey.clone();
        // Step-type signals must be read here: `body` is moved into the pump task below.
        let step_mode_task = step_mode(&headers);
        let step_tool_turn_task = step_is_tool_turn(&body);
        // Absorb short provider bursts without making the billing/cache pump stop reading the
        // upstream while Hyper or nginx drains a handful of tiny SSE frames.
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<axum::body::Bytes, std::io::Error>>(256);
        // Move the in-flight guard into the pump task: the handler returns as soon as
        // the response head is ready, but the request is not settled until this task
        // finishes billing. Dropping the guard at handler return would have left the
        // whole streaming window — the case that actually matters — uncounted.
        let inflight = inflight_guard;
        tokio::spawn(async move {
            let _inflight = inflight;
            use futures_util::StreamExt;
            let mut upstream = Box::pin(resp.bytes_stream());
            // 180s (was 30s): a thinking model pauses to reason / composes a long file write
            // silently → the 30s guard cut the stream mid-tool-call (truncated args → empty
            // write "内容为空"). 180s lets those through while still bounding a real hang.
            // 深思考档（xhigh/max/thinking 预算）放宽到 600s——否则客户端 180s 窗和这里打平，
            // 超 3 分钟的静默深思仍会被网关先掐。
            let idle = std::time::Duration::from_secs(if deep_thinking { 600 } else { 180 });
            let mut acc: Vec<u8> = Vec::new(); // OpenAI-shape SSE bytes, for the response cache (capped 1MB)
                                               // Bounded tail for OpenAI usage extraction (the include_usage chunk is the LAST event;
                                               // a >1MB response would miss it in the capped acc). Unused on the anthropic path — there
                                               // usage comes from the converter's accumulated counts.
            let mut tail: Vec<u8> = Vec::new();
            let mut complete = false;
            let mut client_closed = false;
            let mut stream_failure: Option<String> = None;
            // anthropic connections: translate the upstream Anthropic SSE → OpenAI SSE on the fly.
            let mut conv = if anthropic {
                Some(AnthSse::with_tool_argument_rules(
                    &req_model,
                    tool_argument_rules.clone(),
                ))
            } else {
                None
            };
            let mut openai_validator = if anthropic {
                None
            } else {
                Some(OpenAiSseValidator::with_tool_argument_rules(
                    tool_argument_rules,
                ))
            };
            // SSE heartbeat: Chinese carrier NATs kill TCP connections idle >30-60s.
            // During model "thinking" the upstream is silent → zero bytes flow to the
            // client → NAT drops it → "网络波动". Fix: send an SSE comment (`: ping\n\n`)
            // every 15s of upstream silence. SSE comments are ignored by compliant parsers.
            let hb_interval = std::time::Duration::from_secs(15);
            let mut last_data = tokio::time::Instant::now();
            let response_opened_at = tokio::time::Instant::now();
            let mut first_upstream_chunk = true;
            // When the client hangs up we keep draining the upstream instead of
            // bailing out. The upstream keeps generating (and keeps charging the
            // operator) either way, and the token counts only arrive in the FINAL
            // usage event — abandoning the stream early meant `parse_usage_from_sse`
            // found nothing, `compute_cost` billed 0, and disconnecting mid-stream
            // was a free-inference button. Draining bounded by DRAIN_AFTER_CLOSE and
            // by the existing idle-stall check.
            const DRAIN_AFTER_CLOSE: std::time::Duration = std::time::Duration::from_secs(120);
            let mut closed_at: Option<tokio::time::Instant> = None;
            loop {
                if let Some(at) = closed_at {
                    if at.elapsed() >= DRAIN_AFTER_CLOSE {
                        tracing::warn!(
                            model = %req_model,
                            "client gone; usage frame did not arrive within drain window — billing what was measured"
                        );
                        break;
                    }
                }
                match tokio::time::timeout(hb_interval, upstream.next()).await {
                    Ok(Some(Ok(chunk))) => {
                        last_data = tokio::time::Instant::now();
                        if first_upstream_chunk {
                            first_upstream_chunk = false;
                            tracing::info!(
                                model = %req_model,
                                request_id = request_id_task.as_deref().unwrap_or(""),
                                first_upstream_chunk_ms = response_opened_at.elapsed().as_millis(),
                                chunk_bytes = chunk.len(),
                                "first upstream stream chunk received"
                            );
                        }
                        let fwd: Vec<u8> = match conv.as_mut() {
                            Some(c) => match c.push(chunk.as_ref()) {
                                Ok(fwd) => fwd,
                                Err(err) => {
                                    stream_failure = Some(err);
                                    break;
                                }
                            },
                            None => {
                                if let Some(validator) = openai_validator.as_mut() {
                                    if let Err(err) = validator.push(chunk.as_ref()) {
                                        stream_failure = Some(err);
                                        break;
                                    }
                                }
                                chunk.to_vec()
                            }
                        };
                        if !fwd.is_empty() {
                            if acc.len() < 1_000_000 {
                                acc.extend_from_slice(&fwd);
                            }
                            if conv.is_none() {
                                tail.extend_from_slice(&fwd);
                                if tail.len() > 131_072 {
                                    let cut = tail.len() - 65_536;
                                    tail.drain(0..cut);
                                }
                            }
                            if !client_closed
                                && tx.send(Ok(axum::body::Bytes::from(fwd))).await.is_err()
                            {
                                client_closed = true;
                                closed_at = Some(tokio::time::Instant::now());
                            }
                        }
                    }
                    Ok(None) => {
                        if conv.is_some() {
                            // Anthropic completion is validated by AnthSse::finish below.
                            complete = true;
                        } else {
                            match openai_validator
                                .as_ref()
                                .expect("OpenAI validator")
                                .finish()
                            {
                                Ok(()) => complete = true,
                                Err(err) => stream_failure = Some(err),
                            }
                        }
                        break;
                    }
                    Ok(Some(Err(err))) => {
                        stream_failure = Some(format!("upstream stream read failed: {err}"));
                        break;
                    }
                    Err(_elapsed) => {
                        if last_data.elapsed() >= idle {
                            stream_failure = Some(format!(
                                "upstream stream stalled for {} seconds",
                                idle.as_secs()
                            ));
                            break; // real stall — upstream dead for 180s
                        }
                        // Send SSE heartbeat to keep the client connection alive
                        if !client_closed
                            && tx
                                .send(Ok(axum::body::Bytes::from_static(b": ping\n\n")))
                                .await
                                .is_err()
                        {
                            client_closed = true;
                            closed_at = Some(tokio::time::Instant::now());
                        }
                    }
                }
            }
            // Anthropic bills from its native usage events; OpenAI-compatible streams
            // bill from the trailing include_usage chunk. Missing/incomplete usage is
            // never guessed: rate billing is zero and the settlement says unreported.
            let (usage, usage_reported) = if let Some(c) = conv.as_ref() {
                if complete {
                    match c.finish() {
                        Ok(fin) => {
                            if acc.len() < 1_000_000 {
                                acc.extend_from_slice(&fin);
                            }
                            if !client_closed
                                && tx.send(Ok(axum::body::Bytes::from(fin))).await.is_err()
                            {
                                client_closed = true;
                            }
                        }
                        Err(err) => {
                            complete = false;
                            stream_failure = Some(err);
                        }
                    }
                } else if stream_failure.is_none() && !client_closed {
                    stream_failure = Some(
                        "Anthropic upstream stream ended before protocol completion".to_string(),
                    );
                }
                (c.usage(), c.usage_is_authoritative())
            } else {
                match parse_usage_from_sse(&tail) {
                    Some(u) if usage_is_authoritative(Some(&u)) => (u, true),
                    _ => (json!({}), false),
                }
            };
            if !usage_reported {
                tracing::warn!(model = %req_model, "provider omitted authoritative usage; rate billing is zero");
            }
            // 中转丢块自愈：完整收流但只有思考、没有任何正文/工具块——按线路记 30 分钟
            // 思考钳位。客户端对 reasoning-only 轮有 250ms 快速重试，下一发立即走钳位请求。
            let relay_dropped_blocks = complete
                && thinking_clip_probe
                && conv.as_ref().is_some_and(|c| c.thinking_only_end_turn());
            if relay_dropped_blocks {
                mark_thinking_clip(cid);
                tracing::warn!(
                    model = %req_model,
                    route_id = %cid,
                    "upstream returned thinking-only end_turn (relay dropped post-thinking blocks); clipping this route's thinking budget for 30 minutes"
                );
            }
            if let Some(err) = stream_failure.take() {
                complete = false;
                // 第二个丢块签名：思考开启时上游把工具参数流掐断（incomplete arguments
                // JSON / 流中断在 tool_use 中途）。同样按线路记思考钳位——IDE 的整轮
                // 重试会立刻换成低思考预算的请求，而不是原样重掷再被掐一次。
                if thinking_clip_probe
                    && (err.contains("produced incomplete arguments JSON")
                        || err.contains("ended before protocol completion"))
                {
                    mark_thinking_clip(cid);
                    tracing::warn!(
                        model = %req_model,
                        route_id = %cid,
                        "upstream cut a tool-argument stream mid-flight with thinking on; clipping this route's thinking budget for 30 minutes"
                    );
                }
                if !client_closed {
                    tracing::warn!(model = %req_model, error = %err, "upstream model stream failed protocol validation");
                    let _ = tx
                        .send(Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            err,
                        )))
                        .await;
                }
            }
            let cost = resolve_cost(
                &bmode,
                percall,
                usage_reported.then_some(&usage),
                &req_model,
                rate,
                admin_in,
                admin_out,
                cache_read_price,
                cache_create_price,
                model_in,
                model_out,
            );
            let mut tokens = extract_bill_tokens(
                usage_reported.then_some(&usage),
                &req_model,
                !usage_reported,
            );
            tokens.request_id = request_id_task;
            tokens.mode = step_mode_task;
            tokens.tool_turn = step_tool_turn_task;
            // What did the model actually DO? A reply that is nothing but one tool dispatch
            // is the clearest routing candidate; prose replies are where reasoning happens.
            tokens.emitted_tool = step_emitted_tool(&String::from_utf8_lossy(&acc));
            // Cache the FULL (OpenAI-shape) stream for identical future requests (only when complete).
            // 中转丢块的坏流（只有思考）绝不缓存：客户端的快速重试请求体逐字节相同，
            // 命中缓存就会拿回同一份坏流，钳位后的重试永远打不到上游。
            if complete && !relay_dropped_blocks && !acc.is_empty() && acc.len() < 1_000_000 && response_cache_safe(&acc) {
                let mut rconn = st.redis.clone();
                let _: Result<(), redis::RedisError> = redis::cmd("SET")
                    .arg(&ckey_task)
                    .arg(acc)
                    .arg("EX")
                    .arg(3600i64)
                    .query_async(&mut rconn)
                    .await;
            }
            bill(&st, uid, cid, cost, use_quota, &tokens, free_pool, free_micro).await;
        });
        let body_stream = futures_util::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        });
        let mut builder = Response::builder()
            .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
            .header(axum::http::header::CONTENT_TYPE, ct)
            .header("cache-control", "no-cache")
            .header("x-accel-buffering", "no");
        // 让调用方知道**实际生效**的档位——套餐不够时请求会被静默下调，不回传的话
        // 用户会以为自己拿到了 5M。
        if let Some(tier) = compression_applied {
            builder = builder.header("x-michael-compression-applied", tier.as_str());
            if let Some((tok, covered)) = compression_prefix.as_ref() {
                builder = builder.header("x-michael-compression-prefix", tok.as_str());
                // 覆盖条数必须一起回传，否则客户端不知道该省略前几条：整份上传既撞
                // 3.5MB 字节上限（5M 档因此不可达），又会让早期内容同时以摘要和原文
                // 出现、上下文重复膨胀。口径是"开头 system 块之后的第 N 条起"。
                builder = builder.header("x-michael-compression-covered", covered.to_string());
            }
        }
        let out = builder
            .body(Body::from_stream(body_stream))
            .map_err(|e| AppError::internal(e.to_string()))?;
        Ok(out)
    } else {
        let raw: serde_json::Value = resp
            .json()
            .await
            .unwrap_or_else(|_| json!({ "error": "上游返回非 JSON" }));
        if !status.is_success() {
            return Err(AppError {
                status: StatusCode::BAD_GATEWAY,
                msg: format!("模型供应商错误 {}: {}", status.as_u16(), raw),
            });
        }
        // Anthropic native response → OpenAI shape for the IDE (usage kept in a form compute_cost bills).
        let mut data = if anthropic {
            anthropic_to_oai(&raw, &model_id)
        } else {
            raw
        };
        // Repair upstream's malformed `tool_calls[*].function.arguments`. Some relays
        // (Claude→OpenAI-compat translators) concat the initial empty-arg placeholder
        // `"{}"` with the actual JSON, producing `'{}{"path":"."}'` which clients then
        // parse as `{}` (silent empty args). Strip leading `{}` when followed by `{`.
        fix_tool_call_arguments(&mut data);
        // Cache the successful response for identical future requests.
        if let Ok(bytes) = serde_json::to_vec(&data) {
            if !bytes.is_empty() && bytes.len() < 1_000_000 && response_cache_safe(&bytes) {
                let mut rconn = state.redis.clone();
                let _: Result<(), redis::RedisError> = redis::cmd("SET")
                    .arg(&ckey)
                    .arg(bytes)
                    .arg("EX")
                    .arg(3600i64)
                    .query_async(&mut rconn)
                    .await;
            }
        }
        let mut free_pool = false;
        let mut free_micro = 0i64;
        let (cost, tokens) = if is_image_gen_model(&model_id) {
            let per = if conn.per_call_cents > 0 {
                conn.per_call_cents
            } else {
                (30.0 * conn.rate).round() as i64
            };
            (
                per.clamp(0, 5000),
                BillTokens {
                    model_name: model_id.clone(),
                    request_id: request_id.clone(),
                    ..Default::default()
                },
            )
        } else {
            let usage_val = data.get("usage");
            let usage_reported = usage_is_authoritative(usage_val);
            if !usage_reported {
                tracing::warn!(model = %model_id, "provider omitted authoritative usage; rate billing is zero");
            }
            let (model_in, model_out) = model_price_override(&conn.model_prices, &model_id);
            let (eff_mode, eff_percall, eff_free, eff_micro) = effective_billing_micro(&conn, &model_id);
            free_pool = eff_free;
            free_micro = eff_micro;
            let cost = resolve_cost(
                &eff_mode,
                eff_percall,
                usage_val.filter(|_| usage_reported),
                &model_id,
                conn.rate,
                conn.input_price,
                conn.output_price,
                conn.cache_read_price,
                conn.cache_create_price,
                model_in,
                model_out,
            );
            let mut tokens = extract_bill_tokens(
                usage_val.filter(|_| usage_reported),
                &model_id,
                !usage_reported,
            );
            tokens.request_id = request_id.clone();
            tokens.mode = step_mode(&headers);
            tokens.tool_turn = step_is_tool_turn(&body);
            tokens.emitted_tool = step_emitted_tool(&serde_json::to_string(&data).unwrap_or_default());
            (cost, tokens)
        };
        bill(&state, uid, conn.id, cost, use_quota, &tokens, free_pool, free_micro).await;
        let mut resp = Json(data).into_response();
        if let Some((tok, covered)) = compression_prefix.as_ref() {
            if let Ok(v) = axum::http::HeaderValue::from_str(tok) {
                resp.headers_mut().insert("x-michael-compression-prefix", v);
            }
            if let Ok(v) = axum::http::HeaderValue::from_str(&covered.to_string()) {
                resp.headers_mut()
                    .insert("x-michael-compression-covered", v);
            }
        }
        if let Some(tier) = compression_applied {
            if let Ok(v) = axum::http::HeaderValue::from_str(tier.as_str()) {
                resp.headers_mut()
                    .insert("x-michael-compression-applied", v);
            }
        }
        Ok(resp)
    }
}

/// OpenAI Responses API proxy — forwards POST /v1/responses to the upstream that
/// owns the requested model. Used by the IDE's image-generation fallback chain:
/// 中转站 like LaoZhang/Codex wrap gpt-image-2 behind ChatGPT Plus accounts via
/// this endpoint with the image_generation built-in tool.
///
/// Smart model rewrite: if the IDE sends `model=gpt-image-2`, we route to the
/// matching connection (UI生图密钥) BUT swap body.model to `gpt-5.4` before
/// forwarding — because the Responses API requires a mainline text model in
/// the `model` field (the image model itself is fixed by `tools.image_generation`).
pub async fn responses_proxy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    let request_id = ide_request_id(&headers)?;
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::unauthorized("缺少 API Key"))?;
    let uid: uuid::Uuid = match sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT user_id FROM api_keys WHERE api_key = $1",
    )
    .bind(&token)
    .fetch_optional(&state.db)
    .await?
    {
        Some(u) => {
            let _ = sqlx::query("UPDATE api_keys SET last_used_at = now() WHERE api_key = $1")
                .bind(&token)
                .execute(&state.db)
                .await;
            u
        }
        None => crate::auth::user_from_jwt(&state.cfg, &token)
            .ok_or_else(|| AppError::unauthorized("登录已失效或密钥无效"))?,
    };

    let model_id = body
        .get("model")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| AppError::bad("缺少 model"))?;

    let conns = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE active = true ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await?;
    let conn = conns
        .into_iter()
        .find(|m| allowed_ids(m).contains(&model_id))
        .ok_or_else(|| AppError::bad(format!("模型 {model_id} 不可用")))?;

    // Same quota refill + check as image_generations.
    sqlx::query(
        "UPDATE users SET \
         quota_window_cents = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN LEAST(quota_window_cap_cents, quota_total_cents) ELSE quota_window_cents END, \
         quota_window_reset_at = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN now() + interval '5 hours 30 minutes' ELSE quota_window_reset_at END, \
         quota_week_used_cents = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN 0 ELSE quota_week_used_cents END, \
         quota_week_reset_at = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN now() + interval '7 days' ELSE quota_week_reset_at END \
         WHERE id = $1",
    )
    .bind(uid)
    .execute(&state.db)
    .await?;

    let (plan, plan_exp, q_total, q_window, q_weekly_cap, q_week_used, credits): (
        String,
        Option<chrono::DateTime<chrono::Utc>>,
        i64,
        i64,
        i64,
        i64,
        i64,
    ) = sqlx::query_as(
        "SELECT plan, plan_expires_at, quota_total_cents, quota_window_cents, \
         quota_weekly_cap_cents, quota_week_used_cents, credits_cents FROM users WHERE id = $1",
    )
    .bind(uid)
    .fetch_one(&state.db)
    .await?;
    let plan_active = plan != "none" && plan_exp.is_none_or(|e| e > chrono::Utc::now());
    let quota_ok = plan_active
        && q_total > 0
        && q_window > 0
        && (q_weekly_cap == 0 || q_week_used < q_weekly_cap);
    let use_quota = quota_ok;
    if !quota_ok && credits <= 0 {
        return Err(AppError {
            status: StatusCode::PAYMENT_REQUIRED,
            msg: "请先开通会员或充值额度".into(),
        });
    }

    // Same per-user concurrency ceiling chat_completions uses. Without it these two
    // billed paths had no cap at all, so the bounded-overdraft guarantee that
    // InFlightGuard exists to provide simply did not hold here.
    let _inflight_guard = InFlightGuard::acquire(&state, uid).await?;
    // Always ensure image_generation tool is present for image models.
    let is_image_model = model_id.to_lowercase().contains("gpt-image")
        || model_id.to_lowercase().contains("dall-e")
        || model_id.to_lowercase().contains("dall_e");
    if is_image_model {
        if let Some(obj) = body.as_object_mut() {
            let has_image_tool = obj
                .get("tools")
                .and_then(|t| t.as_array())
                .map(|a| a.iter().any(|t| t["type"] == "image_generation"))
                .unwrap_or(false);
            if !has_image_tool {
                let mut tools = obj
                    .get("tools")
                    .and_then(|t| t.as_array())
                    .cloned()
                    .unwrap_or_default();
                tools.push(serde_json::json!({"type": "image_generation"}));
                obj.insert("tools".into(), serde_json::json!(tools));
            }
        }
    }

    let url = format!("{}/responses", api_base(&conn.base_url));

    // Two-stage attempt for image models:
    //   stage 1: forward AS-IS — relay routes to real gpt-image-2 (full HD output).
    //   stage 2: when stage 1 fails with "no Plus OAuth account" (relay's HD account
    //   pool is empty), swap model → "gpt-5.4" and retry — relay's mainline-wrap
    //   path doesn't need a Plus account but caps output at ~940×627.
    // For non-image models, stage 2 is skipped (no model swap makes sense).
    async fn send_once(
        url: &str,
        api_key: &str,
        body: &serde_json::Value,
    ) -> Result<reqwest::Response, (u16, String)> {
        for attempt in 0u32..3 {
            match GW_HTTP
                .post(url)
                .header("Authorization", format!("Bearer {api_key}"))
                .json(body)
                .timeout(std::time::Duration::from_secs(180))
                .send()
                .await
            {
                Ok(r) if r.status().is_success() => return Ok(r),
                Ok(r) => {
                    let st = r.status().as_u16();
                    let txt = r.text().await.unwrap_or_default();
                    let transient = matches!(st, 502 | 503 | 504 | 429);
                    if !transient || attempt == 2 {
                        return Err((st, txt));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                Err(e) => {
                    if attempt == 2 {
                        return Err((0, e.to_string()));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
        Err((0, "exhausted".into()))
    }

    let resp = match send_once(&url, &conn.api_key, &body).await {
        Ok(r) => r,
        Err((st, msg)) if is_image_model && msg.to_lowercase().contains("no active plus oauth") => {
            // HD pool empty → fall back to mainline-wrap (model=gpt-5.4) for low-res but functional output.
            tracing::info!(
                "[responses] {model_id} HD pool empty, falling back to gpt-5.4 mainline-wrap"
            );
            if let Some(obj) = body.as_object_mut() {
                obj.insert("model".into(), serde_json::json!("gpt-5.4"));
            }
            match send_once(&url, &conn.api_key, &body).await {
                Ok(r) => r,
                Err((st2, msg2)) => {
                    return Err(AppError {
                        status: StatusCode::BAD_GATEWAY,
                        msg: format!(
                            "【{model_id}】responses 双路径都失败: HD={} | mainline={}: {}",
                            st,
                            st2,
                            msg2.chars().take(150).collect::<String>()
                        ),
                    });
                }
            }
        }
        Err((st, msg)) => {
            return Err(AppError {
                status: StatusCode::BAD_GATEWAY,
                msg: format!(
                    "【{model_id}】responses 上游不可用 ({}): {}",
                    st,
                    msg.chars().take(200).collect::<String>()
                ),
            });
        }
    };

    let data: serde_json::Value = resp
        .json()
        .await
        .unwrap_or_else(|_| serde_json::json!({"error": "上游返回非 JSON"}));
    let has_error = data.get("error").is_some();

    // Bill: image models = per-image (per_call_cents if set, else 30分×倍率), text = per-token.
    if !has_error {
        if is_image_gen_model(&model_id) {
            let mut n_images = data
                .get("output")
                .and_then(|o| o.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter(|i| i["type"] == "image_generation_call")
                        .count() as f64
                })
                .unwrap_or(0.0);
            // Non-OpenAI image models (e.g. gemini-*-image) may not emit `image_generation_call`;
            // a successful image call still costs → bill at least 1 image, never $0.
            if n_images == 0.0 {
                n_images = 1.0;
            }
            let cost = if conn.per_call_cents > 0 {
                (conn.per_call_cents as f64 * n_images).round().min(5000.0) as i64
            } else {
                (30.0 * n_images * conn.rate).round().min(5000.0) as i64
            };
            bill(
                &state,
                uid,
                conn.id,
                cost,
                use_quota,
                &BillTokens {
                    model_name: model_id.clone(),
                    estimated: true,
                    request_id: request_id.clone(),
                    ..Default::default()
                },
                false,
                0,
            )
            .await;
        } else {
            let usage = data.get("usage");
            let usage_reported = usage_is_authoritative(usage);
            let (model_in, model_out) = model_price_override(&conn.model_prices, &model_id);
            let (eff_mode, eff_percall, eff_free, eff_micro) = effective_billing_micro(&conn, &model_id);
            let free_pool = eff_free;
            let free_micro = eff_micro;
            let cost = resolve_cost(
                &eff_mode,
                eff_percall,
                usage.filter(|_| usage_reported),
                &model_id,
                conn.rate,
                conn.input_price,
                conn.output_price,
                conn.cache_read_price,
                conn.cache_create_price,
                model_in,
                model_out,
            );
            let mut tokens =
                extract_bill_tokens(usage.filter(|_| usage_reported), &model_id, !usage_reported);
            tokens.request_id = request_id.clone();
            bill(&state, uid, conn.id, cost, use_quota, &tokens, free_pool, free_micro).await;
        }
    }

    Ok(Json(data).into_response())
}

/// Image generation endpoint — proxies to upstream /images/generations.
/// Same auth + quota as chat_completions; bills per-image (official price × 倍率).
pub async fn image_generations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    let request_id = ide_request_id(&headers)?;
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::unauthorized("缺少 API Key"))?;
    let uid: uuid::Uuid = match sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT user_id FROM api_keys WHERE api_key = $1",
    )
    .bind(&token)
    .fetch_optional(&state.db)
    .await?
    {
        Some(u) => {
            let _ = sqlx::query("UPDATE api_keys SET last_used_at = now() WHERE api_key = $1")
                .bind(&token)
                .execute(&state.db)
                .await;
            u
        }
        None => crate::auth::user_from_jwt(&state.cfg, &token)
            .ok_or_else(|| AppError::unauthorized("登录已失效或密钥无效"))?,
    };

    let model_id = body
        .get("model")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| AppError::bad("缺少 model"))?;

    let conns = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE active = true ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await?;
    let conn = conns
        .into_iter()
        .find(|m| allowed_ids(m).contains(&model_id))
        .ok_or_else(|| AppError::bad(format!("模型 {model_id} 不可用")))?;

    // Quota refill + check (same as chat_completions).
    sqlx::query(
        "UPDATE users SET \
         quota_window_cents = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN LEAST(quota_window_cap_cents, quota_total_cents) ELSE quota_window_cents END, \
         quota_window_reset_at = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN now() + interval '5 hours 30 minutes' ELSE quota_window_reset_at END, \
         quota_week_used_cents = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN 0 ELSE quota_week_used_cents END, \
         quota_week_reset_at = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN now() + interval '7 days' ELSE quota_week_reset_at END \
         WHERE id = $1",
    )
    .bind(uid)
    .execute(&state.db)
    .await?;

    let (plan, plan_exp, q_total, q_window, q_weekly_cap, q_week_used, credits): (
        String,
        Option<chrono::DateTime<chrono::Utc>>,
        i64,
        i64,
        i64,
        i64,
        i64,
    ) = sqlx::query_as(
        "SELECT plan, plan_expires_at, quota_total_cents, quota_window_cents, \
         quota_weekly_cap_cents, quota_week_used_cents, credits_cents FROM users WHERE id = $1",
    )
    .bind(uid)
    .fetch_one(&state.db)
    .await?;
    let plan_active = plan != "none" && plan_exp.is_none_or(|e| e > chrono::Utc::now());
    let quota_ok = plan_active
        && q_total > 0
        && q_window > 0
        && (q_weekly_cap == 0 || q_week_used < q_weekly_cap);
    let use_quota = quota_ok;
    if !quota_ok && credits <= 0 {
        let msg = if plan_active && q_total <= 0 {
            "总额度已用完"
        } else if plan_active && q_window <= 0 {
            "本时段额度已用完，请等待刷新（每 5.5 小时）"
        } else if plan_active && q_weekly_cap > 0 && q_week_used >= q_weekly_cap {
            "本周额度已用完"
        } else {
            "请先开通会员或充值额度"
        };
        return Err(AppError {
            status: StatusCode::PAYMENT_REQUIRED,
            msg: msg.into(),
        });
    }

    // Proxy to upstream /images/generations with retry for transient failures.
    // Same per-user concurrency ceiling chat_completions uses. Without it these two
    // billed paths had no cap at all, so the bounded-overdraft guarantee that
    // InFlightGuard exists to provide simply did not hold here.
    let _inflight_guard = InFlightGuard::acquire(&state, uid).await?;
    let url = format!("{}/images/generations", api_base(&conn.base_url));
    let resp = {
        let mut success = None;
        let mut last_err = String::new();
        for attempt in 0u32..3 {
            match GW_HTTP
                .post(&url)
                .header("Authorization", format!("Bearer {}", conn.api_key))
                .json(&body)
                .timeout(std::time::Duration::from_secs(120))
                .send()
                .await
            {
                Ok(r) if r.status().is_success() => {
                    success = Some(r);
                    break;
                }
                Ok(r) => {
                    let st = r.status().as_u16();
                    last_err = r.text().await.unwrap_or_default();
                    let transient = matches!(st, 502 | 503 | 504 | 429);
                    if !transient || attempt == 2 {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                Err(e) => {
                    last_err = e.to_string();
                    if attempt == 2 {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
        match success {
            Some(r) => r,
            None => {
                return Err(AppError {
                    status: StatusCode::BAD_GATEWAY,
                    msg: format!(
                        "【{model_id}】生图上游不可用: {}",
                        last_err.chars().take(200).collect::<String>()
                    ),
                });
            }
        }
    };

    let mut data: serde_json::Value = resp
        .json()
        .await
        .unwrap_or_else(|_| json!({"error": "上游返回非 JSON"}));

    // Async task support: some upstreams queue large-size requests and return a task_id.
    if data.get("status").and_then(|s| s.as_str()) == Some("queued")
        || data.get("status").and_then(|s| s.as_str()) == Some("running")
    {
        if let Some(task_id) = data
            .get("task_id")
            .and_then(|t| t.as_str())
            .map(String::from)
        {
            let poll_url = format!(
                "{}/images/generations/{}",
                api_base(&conn.base_url),
                task_id
            );
            for _ in 0..60 {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                if let Ok(pr) = GW_HTTP
                    .get(&poll_url)
                    .header("Authorization", format!("Bearer {}", conn.api_key))
                    .timeout(std::time::Duration::from_secs(15))
                    .send()
                    .await
                {
                    if let Ok(pv) = pr.json::<serde_json::Value>().await {
                        let st = pv.get("status").and_then(|s| s.as_str()).unwrap_or("");
                        if st == "failed" {
                            data = pv;
                            break;
                        }
                        if st == "completed" {
                            data = pv;
                            break;
                        }
                    }
                }
            }
        }
    }

    // Fix relative URLs: some upstreams return "/api/v1/gen/..." instead of full URLs.
    if let Some(arr) = data.get_mut("data").and_then(|d| d.as_array_mut()) {
        let origin = conn.base_url.trim_end_matches('/');
        for item in arr.iter_mut() {
            if let Some(u) = item.get("url").and_then(|v| v.as_str()).map(String::from) {
                if u.starts_with('/') {
                    item["url"] = json!(format!("{}{}", origin, u));
                }
            }
        }
    }

    let has_error = data.get("error").is_some()
        || data.get("status").and_then(|s| s.as_str()) == Some("failed");

    // Bill per image: per_call_cents × n_images (if set), else 30分 × n_images × 倍率.
    let n_images = data
        .get("data")
        .and_then(|d| d.as_array())
        .map(|a| a.len() as f64)
        .unwrap_or(0.0);
    if !has_error && n_images > 0.0 {
        let cost = if conn.per_call_cents > 0 {
            (conn.per_call_cents as f64 * n_images).round().min(5000.0) as i64
        } else {
            (30.0 * n_images * conn.rate).round().min(5000.0) as i64
        };
        if cost > 0 {
            bill(
                &state,
                uid,
                conn.id,
                cost,
                use_quota,
                &BillTokens {
                    model_name: model_id.clone(),
                    estimated: true,
                    request_id: request_id.clone(),
                    ..Default::default()
                },
                false,
                0,
            )
            .await;
        }
    }

    Ok(Json(data).into_response())
}

#[cfg(test)]
mod billing_tests {
    use super::{
        anthropic_thinking, anthropic_to_oai, chat_upstream_attempt_suffix,
        chat_upstream_retry_base_delay_ms, clip_thinking_budget, compute_cost, is_image_gen_model,
        mark_thinking_clip, model_price_override, oai_to_anthropic, official_price,
        parse_usage_from_sse, project_quota_package, projected_provider_usd, resolve_cost,
        response_cache_safe, round_multiplier_up, split_fused_charge, thinking_clip_active,
        tool_argument_rules, upstream_failure_status, validate_openai_sse_eof,
        validate_openai_sse_with_rules, AnthSse, FusedCharge, OpenAiSseValidator,
        THINKING_CLIP_ROUTES, THINKING_CLIP_SAFE_BUDGET,
    };
    use std::time::{Duration as ClipDuration, Instant as ClipInstant};
    use axum::http::StatusCode;
    use serde_json::json;

    #[test]
    fn fused_charge_spills_from_quota_into_wallet() {
        assert_eq!(
            split_fused_charge(23, true, 10, 10, 0, 0, 100),
            FusedCharge {
                quota_cents: 10,
                wallet_cents: 13,
            }
        );
    }

    #[test]
    fn fused_charge_respects_weekly_quota_cap() {
        assert_eq!(
            split_fused_charge(23, true, 100, 100, 20, 15, 100),
            FusedCharge {
                quota_cents: 5,
                wallet_cents: 18,
            }
        );
    }

    /// 纯订阅用户不得因为**套餐内的正常使用**背上钱包债务。
    ///
    /// 固定价套餐每个配额窗口末尾必然有一次请求超出剩余配额。若把这部分全额落到钱包，
    /// 一个从没充过值的订阅用户就会每个窗口都累积一次负债 —— 而那是他买套餐时已经
    /// 付过的钱。这一小段由运营方吸收，规模天然被"单次请求"限制住。
    #[test]
    fn subscription_quota_overshoot_does_not_create_wallet_debt() {
        let charge = split_fused_charge(23, true, 10, 10, 0, 0, 0);
        assert_eq!(
            charge,
            FusedCharge {
                quota_cents: 10,
                wallet_cents: 0,
            },
            "零余额的订阅用户，超出配额的部分不该变成负债"
        );

        // 有余额时照常从钱包扣，但只扣到余额为止，同样不制造负债。
        let partial = split_fused_charge(23, true, 10, 10, 0, 0, 5);
        assert_eq!(
            partial,
            FusedCharge {
                quota_cents: 10,
                wallet_cents: 5,
            }
        );
    }

    /// 反过来：按量付费用户超支仍然全额记债，不能免单。
    #[test]
    fn pay_as_you_go_overspend_still_becomes_debt() {
        let charge = split_fused_charge(500, false, 0, 0, 0, 0, 20);
        assert_eq!(
            charge,
            FusedCharge {
                quota_cents: 0,
                wallet_cents: 500,
            },
            "没动用套餐配额时，超出余额的部分必须记为债务，否则每次超支都被静默免单"
        );
    }

    #[test]
    fn fused_charge_uses_wallet_without_eligible_quota() {
        assert_eq!(
            split_fused_charge(23, false, 100, 100, 0, 0, 0),
            FusedCharge {
                quota_cents: 0,
                wallet_cents: 23,
            }
        );
    }

    #[test]
    /// Overspend is recorded as debt, not written off.
    ///
    /// This test previously asserted the opposite — that the wallet portion was
    /// clamped to the available balance (23 requested, 4 available → charge 14 and
    /// forgive 9). That clamp was the bug: the access gate only checks that the
    /// balance is positive and settlement happens after the upstream call, so every
    /// overshoot was silently free while the operator still paid upstream. The full
    /// cost is now charged, `credits_cents` may go negative, and the existing
    /// `credits <= 0` gate refuses the next request until the user tops up.
    fn fused_charge_records_overspend_as_debt() {
        // 按量付费（本轮没动用任何套餐配额）：全额记债，允许 credits 变负。
        let charge = split_fused_charge(23, false, 0, 0, 0, 0, 4);
        assert_eq!(
            charge,
            FusedCharge {
                quota_cents: 0,
                wallet_cents: 23,
            }
        );
        assert_eq!(
            charge.total_cents(),
            23,
            "the settled amount must equal the true cost so model_usage can be reconciled"
        );
    }

    #[test]
    /// A user with no funds at all still gets charged the real amount, so the debt is
    /// visible and the next request is refused.
    fn fused_charge_bills_full_cost_with_empty_wallet() {
        let charge = split_fused_charge(500, false, 0, 0, 0, 0, 0);
        assert_eq!(
            charge,
            FusedCharge {
                quota_cents: 0,
                wallet_cents: 500,
            }
        );
    }

    #[test]
    fn shipment_tool_calls_are_never_response_cached() {
        assert!(response_cache_safe(
            br#"data: {\"content\":\"ordinary answer\"}"#
        ));
        assert!(!response_cache_safe(
            br#"data: {\"name\":\"track_shipment\",\"arguments\":\"{\\\"tracking_number\\\":\\\"1Z999AA10123456784\\\"}\"}"#
        ));
    }

    #[test]
    fn chat_gateway_transient_retry_backoff_is_bounded() {
        assert_eq!(chat_upstream_retry_base_delay_ms(0), 250);
        assert_eq!(chat_upstream_retry_base_delay_ms(1), 650);
        assert_eq!(chat_upstream_retry_base_delay_ms(2), 1_300);
        assert_eq!(chat_upstream_retry_base_delay_ms(3), 2_500);
        assert_eq!(chat_upstream_retry_base_delay_ms(4), 4_000);
        assert_eq!(chat_upstream_retry_base_delay_ms(99), 4_000);
    }

    #[test]
    fn chat_gateway_error_suffix_reports_single_route_retries() {
        assert_eq!(
            chat_upstream_attempt_suffix(1, 6, 502),
            "（已请求 6 次；当前只有 1 条同模型线路；最后状态 502）"
        );
        assert_eq!(
            chat_upstream_attempt_suffix(3, 12, 504),
            "（已请求 12 次 / 3 条同模型线路；最后状态 504）"
        );
    }

    #[test]
    fn chat_gateway_maps_permanent_upstream_access_failures_to_failed_dependency() {
        assert_eq!(
            upstream_failure_status(401, "invalid api key"),
            StatusCode::FAILED_DEPENDENCY
        );
        assert_eq!(
            upstream_failure_status(403, "provider rejected this model"),
            StatusCode::FAILED_DEPENDENCY
        );
        assert_eq!(
            upstream_failure_status(500, "no available provider account"),
            StatusCode::FAILED_DEPENDENCY
        );
        assert_eq!(
            upstream_failure_status(402, "insufficient_balance"),
            StatusCode::FAILED_DEPENDENCY
        );
    }

    #[test]
    fn chat_gateway_preserves_retryable_upstream_statuses() {
        assert_eq!(
            upstream_failure_status(429, "rate limited"),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            upstream_failure_status(502, "bad gateway"),
            StatusCode::BAD_GATEWAY
        );

        // Regression (2026-08-01 outage): a permanent request rejection must NOT be
        // dressed up as a transient 502. It used to fall through `_ => BAD_GATEWAY`,
        // so the IDE's retry loop re-sent the same rejected body until the route died
        // and the editor hung. These three statuses mean "the body is wrong" — the
        // client must see that and stop, not retry.
        assert_eq!(
            upstream_failure_status(
                400,
                "\"thinking.type.enabled\" is not supported for this model."
            ),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            upstream_failure_status(413, "request entity too large"),
            StatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(
            upstream_failure_status(422, "unprocessable"),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        // ...but a 400 whose text is an access/billing failure keeps its 424 mapping,
        // so the "switch account / top up" path in the IDE still triggers.
        assert_eq!(
            upstream_failure_status(400, "insufficient_balance"),
            StatusCode::FAILED_DEPENDENCY
        );
        assert_eq!(
            upstream_failure_status(503, "service unavailable"),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            upstream_failure_status(504, "header timeout"),
            StatusCode::GATEWAY_TIMEOUT
        );
    }

    // per_call mode bills the flat fee, ignoring token usage entirely.
    #[test]
    /// A `models` row is a CONNECTION holding many enabled_models, so billing_mode /
    /// per_call_cents alone could only switch a WHOLE channel — "make this one model per-call"
    /// was impossible, which is exactly what the operator hit. model_billing overrides per id.
    #[test]
    fn model_billing_overrides_the_connection_default() {
        // mode override: connection is rate, one model is per_call
        let billing = json!({ "gpt-5.5": { "mode": "per_call", "per_call_cents": 7 } });
        let ov = billing.get("gpt-5.5");
        let mode = ov
            .and_then(|v| v.get("mode"))
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_lowercase())
            .filter(|s| s == "rate" || s == "per_call" || s == "free")
            .unwrap_or_else(|| "rate".to_string());
        assert_eq!(mode, "per_call", "per-model override must beat the channel default");
        // an unlisted model keeps the connection default
        assert!(billing.get("claude-opus-5").is_none());
        // a junk mode is rejected, not silently honored
        let junk = json!({ "m": { "mode": "PER-CALL" } });
        let jm = junk
            .get("m")
            .and_then(|v| v.get("mode"))
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_lowercase())
            .filter(|s| s == "rate" || s == "per_call" || s == "free");
        assert!(jm.is_none(), "unknown mode must fall back, never be trusted");
    }

    /// "free" is a payment TARGET, not a price: the cost is still computed the normal way and
    /// still recorded in model_usage — it is merely deducted from the daily points pool. If
    /// free silently meant zero-cost, usage history and the routing report would go blind.
    #[test]
    fn free_mode_still_costs_and_maps_to_a_real_cost_mode() {
        // free + a configured per-call fee bills that flat fee (against points)
        assert_eq!(
            resolve_cost("per_call", 3, None, "free-model", 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            3,
        );
        // free with no fee falls through to token billing, which with zero prices is 0 —
        // legitimately free, and the points pool is simply untouched.
        assert_eq!(
            resolve_cost("rate", 0, None, "free-model", 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            0,
        );
    }

    /// The operator prices in 点: ¥0.5 = 10 点 → 1 点 = ¥0.05 → the ¥2 daily allowance is
    /// exactly 40 点. Pin the arithmetic so a future edit cannot quietly desync the two.
    #[test]
    fn daily_allowance_is_two_yuan_worth_of_points() {
        assert_eq!(super::FREE_POINTS_DAILY, 40);
        // ¥0.5 buys 10 点, so the daily grant is ¥2.00 exactly.
        let yuan_per_point = 0.5_f64 / 10.0;
        assert!((super::FREE_POINTS_DAILY as f64 * yuan_per_point - 2.0).abs() < 1e-9);
    }

    /// Regression: the free gate must exist on the MAIN chat path, not only the legacy
    /// handler. It was added to `chat` first, and `chat_completions` — the endpoint the IDE
    /// actually calls — kept passing free requests through on quota alone, so the allowance
    /// was decorative: a member with quota could use free models forever at 0 点.
    #[test]
    fn free_gate_guards_the_main_chat_path() {
        // Read at RUNTIME, not include_str!: embedding the very file being compiled makes
        // cargo's change detection lag by a build, so the assertion can pass against stale
        // bytes — which it did, hiding a removed gate for one run.
        // Read at RUNTIME (include_str! of the file being compiled lags a build), and search
        // ONLY the non-test half — the first cut counted this test's own assertion literals,
        // so it matched itself and could never fail. Both mutations sailed through it.
        let full = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/models.rs"))
            .expect("read models.rs");
        // Cut at the TEST MODULE, not the first `#[cfg(test)]` — there is a cfg(test) helper
        // far earlier in this file, and truncating there hid the very gate being asserted.
        let src = &full[..full.find("mod billing_tests").unwrap_or(full.len())];
        let n = src.matches("今日免费额度已用完").count();
        assert!(n >= 2, "the free-pool gate must guard both chat handlers, found {n}");
        assert!(
            src.contains("candidates.iter().any(|c| effective_billing(c, &model_id).2)"),
            "the main path must decide freeness across every candidate route",
        );
    }

    /// The operator could not enter a $0.003 per-call fee: whole cents floored it to 0 (the
    /// "minimum value" they hit), and whole 点 then rounded every call up to 1 点, so a 40-点
    /// allowance was always exactly 40 calls whatever the price. Both floors are gone.
    /// The CONNECTION-level fee had the same whole-cent floor as the per-model one: entering
    /// 0.0055 computed round(0.55) = 1 cent and the form redisplayed "0.010", which reads as
    /// the value reverting. Both levels must now carry micro-USD.
    /// A 免费 model with no fee used to spend 0 点 — so it was not "free within a daily cap",
    /// it was UNCAPPED: the allowance never moved and nothing could run out. And 次数模式 with
    /// a zero fee billed nothing at all while the admin form reported success. Both silent
    /// zeros are now closed: one at runtime (floor), one at save time (refusal).
    /// Regression: the classifier recorded `false` on EVERY production request (1440 NULL /
    /// 0 true of 1545 rows) because it read only the last message, and the IDE appends
    /// ephemeral user nudges after tool results. Routing data was therefore blind.
    #[test]
    fn tool_turns_are_detected_behind_trailing_nudges() {
        use super::step_is_tool_turn as t;

        // OpenAI shape with a trailing nudge — the real production case that recorded false.
        let with_nudge = json!({"messages":[
            {"role":"user","content":"do it"},
            {"role":"assistant","tool_calls":[{"id":"c1"}]},
            {"role":"tool","tool_call_id":"c1","content":"file bytes"},
            {"role":"user","content":"[行动门禁] keep going"}
        ]});
        assert_eq!(t(&with_nudge), Some(true), "a trailing nudge must not hide the tool result");

        // Anthropic shape: tool_result inside a user message's content array.
        let anthropic = json!({"messages":[
            {"role":"user","content":"do it"},
            {"role":"assistant","content":[{"type":"tool_use","id":"c1"}]},
            {"role":"user","content":[{"type":"tool_result","tool_use_id":"c1","content":"x"}]}
        ]});
        assert_eq!(t(&anthropic), Some(true), "Anthropic tool_result blocks count too");

        // A genuine fresh user turn is NOT a tool turn.
        let fresh = json!({"messages":[
            {"role":"user","content":"hi"},
            {"role":"assistant","content":"hello"},
            {"role":"user","content":"now do something"}
        ]});
        assert_eq!(t(&fresh), Some(false));

        // A prose-only assistant reply ends the cycle — older tool calls belong to a
        // previous exchange and must not leak into this turn's classification.
        let previous_cycle = json!({"messages":[
            {"role":"assistant","tool_calls":[{"id":"old"}]},
            {"role":"tool","tool_call_id":"old","content":"x"},
            {"role":"assistant","content":"done, here is the answer"},
            {"role":"user","content":"thanks, next task"}
        ]});
        assert_eq!(t(&previous_cycle), Some(false));
    }

    #[test]
    fn zero_fee_cannot_silently_mean_unlimited() {
        let full = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/models.rs"))
            .expect("read models.rs");
        let src = &full[..full.find("mod billing_tests").unwrap_or(full.len())];

        // runtime floor on the free path
        assert!(
            src.contains("spend_free_points(state, uid, micro).await.max(1)"),
            "a free-flagged call must always consume at least one milli-点",
        );
        // Save-time refusal for per-call with no fee — but resolved PER MODEL, not on the
        // connection field alone. A zero connection fee is legitimate when every model
        // carries its own price; the first cut rejected that and blocked a correct setup.
        assert!(
            src.contains(r#"billing_mode == "per_call" && per_call_cents == 0 && per_call_micro_usd == 0"#),
            "saving 次数模式 with no price anywhere must be refused",
        );
        assert!(
            src.contains("let unpriced: Vec<String> = enabled"),
            "the refusal must inspect each enabled model's resolved price, not just the channel field",
        );
        assert!(
            src.contains(r#"if mode == "free" || mode == "rate""#),
            "免费 (points-capped) and 倍率 (token-billed) models must not be flagged unpriced",
        );
        // the floor is the SMALLEST possible spend — it must not overcharge a priced call
        assert_eq!(super::milli_points_for_micro_usd(1), 1);
        assert!(super::milli_points_for_micro_usd(55_000) > 1, "a real fee still costs its real amount");
    }

    #[test]
    fn connection_fee_keeps_sub_cent_precision() {
        let full = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/models.rs"))
            .expect("read models.rs");
        let src = &full[..full.find("mod billing_tests").unwrap_or(full.len())];
        assert!(src.contains("pub per_call_micro_usd: i64"), "connection carries a micro fee");
        assert!(
            src.contains("model.per_call_micro_usd > 0"),
            "the free path must prefer the connection's micro fee over rounded cents",
        );
        // $0.0055 = 5500 micro-USD → 110 milli-点, i.e. 0.11 点 — NOT rounded to a whole cent
        // and NOT rounded up to a whole 点.
        assert_eq!(super::milli_points_for_micro_usd(5_500), 110);
        // the old lossy path would have produced 1 cent = 10 000 micro = 200 milli-点
        assert_ne!(super::milli_points_for_micro_usd(5_500), 200);
    }

    #[test]
    fn sub_cent_fees_survive_and_convert_proportionally() {
        use super::{milli_points_for_micro_usd as mp, MICRO_USD_PER_CENT, MICRO_USD_PER_MILLI_POINT};

        // $0.003 = 3000 micro-USD. It must NOT round to zero…
        let three_tenths_of_a_cent = 3_000;
        assert!(three_tenths_of_a_cent > 0);
        // …and must cost a real, sub-点 amount: 3000 / 50 = 60 milli-点 = 0.06 点.
        assert_eq!(MICRO_USD_PER_MILLI_POINT, 50);
        assert_eq!(mp(three_tenths_of_a_cent), 60);

        // A 40-点 daily pool therefore buys ~666 such calls, not 40.
        assert_eq!(super::FREE_MILLI_POINTS_DAILY / mp(three_tenths_of_a_cent), 666);

        // Volume billing converts through the same path: whole-cent token cost scaled up.
        assert_eq!(mp(1 * MICRO_USD_PER_CENT), 200, "1 cent = 0.2 点");
        assert_eq!(mp(super::RAW_CENTS_PER_POINT * MICRO_USD_PER_CENT), super::MILLI, "5 cents = 1 点");

        // Still never free by rounding: any positive cost costs at least one milli-点.
        assert_eq!(mp(1), 1);
        assert_eq!(mp(0), 0);
        assert_eq!(mp(-9), 0);
    }

    #[test]
    fn points_round_up_so_cheap_calls_are_never_free() {
        use super::points_for_raw_cents as pts;
        assert_eq!(pts(0), 0, "a genuinely zero-cost call spends nothing");
        assert_eq!(pts(-5), 0, "negative cost cannot refund points");
        // Anything that costs real money costs at least one 点 — otherwise a sub-point model
        // would be unlimited and the daily cap would mean nothing.
        assert_eq!(pts(1), 1);
        assert_eq!(pts(super::RAW_CENTS_PER_POINT), 1);
        assert_eq!(pts(super::RAW_CENTS_PER_POINT + 1), 2);
        // The whole daily pool corresponds to a bounded amount of real spend.
        assert_eq!(
            pts(super::RAW_CENTS_PER_POINT * super::FREE_POINTS_DAILY),
            super::FREE_POINTS_DAILY,
        );
    }

    #[test]
    fn per_call_mode_flat_fee() {
        let usage = json!({"prompt_tokens": 999999, "completion_tokens": 50000});
        // Huge usage, but per_call mode → exactly per_call_cents regardless.
        assert_eq!(
            resolve_cost(
                "per_call",
                20,
                Some(&usage),
                "claude-opus-4-8",
                5.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            20
        );
        // Even with no usage at all, per_call still charges the flat fee.
        assert_eq!(
            resolve_cost(
                "per_call",
                35,
                None,
                "claude-opus-4-8",
                5.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            35
        );
        // Negative per_call_cents floored to 0.
        assert_eq!(
            resolve_cost("per_call", -5, None, "x", 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            0
        );
    }

    // rate mode delegates to compute_cost (real token billing), unchanged.
    #[test]
    fn rate_mode_delegates_to_token_billing() {
        let usage =
            json!({"prompt_tokens": 22000, "completion_tokens": 2000, "total_tokens": 24000});
        // (22000·5 + 2000·25)/1e6 = $0.16 = 16¢ × 1.0 rate.
        assert_eq!(
            resolve_cost(
                "rate",
                999,
                Some(&usage),
                "claude-opus-4-8",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            16
        );
        // per_call_cents is IGNORED in rate mode.
        assert_eq!(
            resolve_cost(
                "rate",
                999,
                Some(&usage),
                "claude-opus-4-8",
                3.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            48
        );
        // Empty/unknown mode string → treated as rate (safe default).
        assert_eq!(
            resolve_cost(
                "",
                999,
                Some(&usage),
                "claude-opus-4-8",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            16
        );
    }

    // The per-model official catalog returns the published $/1M prices; unknown → None.
    #[test]
    fn official_catalog() {
        assert_eq!(official_price("claude-opus-4-8"), Some((5.0, 25.0)));
        assert_eq!(official_price("CLAUDE-OPUS-4-6"), Some((5.0, 25.0))); // case-insensitive
        assert_eq!(official_price("gpt-5.5"), Some((5.0, 30.0)));
        assert_eq!(official_price("gpt-5.4"), Some((2.5, 15.0)));
        assert_eq!(official_price("deepseek-v4-flash"), Some((0.14, 0.28)));
        assert_eq!(official_price("minimax-m3"), Some((0.30, 1.20)));
        assert_eq!(official_price("some-unknown-model"), None);
    }

    // REAL billing = (in·off_in + out·off_out)/1e6 · 100 · 倍率. Normal agent turn on
    // Claude Opus ($5/$25), 22k in + 2k out:
    //   (22000·5 + 2000·25)/1e6 = $0.16 = 16¢ real cost. × 倍率 3 → 48¢ billed.
    #[test]
    fn real_cost_times_rate() {
        let usage =
            json!({"prompt_tokens": 22000, "completion_tokens": 2000, "total_tokens": 24000});
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                3.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            48
        ); // ×3
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            16
        ); // ×1 = real cost
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                2.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            32
        ); // ×2
    }

    #[test]
    fn model_estimate_separates_provider_cost_from_user_multiplier() {
        let usd = projected_provider_usd(
            100_000, // $0.50 plain input
            10_000,  // $0.25 output
            50_000,  // $0.025 cache read
            20_000,  // $0.125 cache creation
            5.0, 25.0, 0.5, 6.25,
        );
        assert!((usd - 0.9).abs() < f64::EPSILON);

        let usage = json!({
            "input_tokens": 100_000,
            "output_tokens": 10_000,
            "cache_read_input_tokens": 50_000,
            "cache_creation_input_tokens": 20_000,
        });
        assert_eq!(
            resolve_cost(
                "rate",
                0,
                Some(&usage),
                "custom-model",
                0.8,
                5.0,
                25.0,
                0.5,
                6.25,
                0.0,
                0.0,
            ),
            72
        );
    }

    #[test]
    fn quota_package_estimate_recommends_break_even_and_target_multipliers() {
        let projection = project_quota_package(1000.0, 288.0, 10.0, 0.8, 20.0);
        assert!((projection.quota_raw_usd - 6630.0).abs() < 1e-9);
        assert!((projection.provider_usd_capacity - 8287.5).abs() < 1e-9);
        assert!((projection.channel_cost_cny - 828.75).abs() < 1e-9);
        assert!((projection.profit_cny + 540.75).abs() < 1e-9);
        assert!((projection.margin_percent + 187.76041666666669).abs() < 1e-9);
        assert!((projection.break_even_multiplier - 2.3020833333333335).abs() < 1e-9);
        assert!((projection.target_multiplier - 2.877604166666667).abs() < 1e-9);
        assert_eq!(round_multiplier_up(projection.break_even_multiplier), 2.31);
        assert_eq!(round_multiplier_up(projection.target_multiplier), 2.88);
    }

    // gpt-5.5 ($5/$30), 22k+2k, ×1: (110000+60000)/1e6 = $0.17 = 17¢.
    #[test]
    fn gpt55_real_cost() {
        let usage = json!({"prompt_tokens": 22000, "completion_tokens": 2000});
        assert_eq!(
            compute_cost(Some(&usage), "gpt-5.5", 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            17
        );
    }

    // Cheap model on a SMALL call rounds toward 0; the SAME model on a big agentic call
    // bills real money. deepseek-v4-flash ($0.14/$0.28), ×1:
    //   22k+2k  → $0.00364 → 0¢ (sub-cent).   200k+10k → $0.0308 → 3¢.
    #[test]
    fn cheap_model_scales_with_size() {
        let small = json!({"prompt_tokens": 22000, "completion_tokens": 2000});
        let big = json!({"prompt_tokens": 200000, "completion_tokens": 10000});
        assert_eq!(
            compute_cost(
                Some(&small),
                "deepseek-v4-flash",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            0
        );
        assert_eq!(
            compute_cost(
                Some(&big),
                "deepseek-v4-flash",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            3
        );
    }

    // An uncatalogued model falls back to the admin's per-connection input/output price.
    //   admin $2/$10, 22k+2k, ×1: (44000+20000)/1e6 = $0.064 = 6.4¢ → 6¢.
    #[test]
    fn admin_override_fallback() {
        let usage = json!({"prompt_tokens": 22000, "completion_tokens": 2000});
        assert_eq!(
            compute_cost(
                Some(&usage),
                "mystery-model",
                1.0,
                2.0,
                10.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            6
        );
        // No catalog AND no admin price → can't know the real cost → 0.
        assert_eq!(
            compute_cost(
                Some(&usage),
                "mystery-model",
                3.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            0
        );
    }

    // A PER-MODEL price override WINS over the built-in official catalog. claude-opus-4-8's
    // catalog price is $5/$25, but with a per-model override of $1/$2 the bill uses $1/$2:
    //   22k·$1 + 2k·$2 = 26000/1e6 = $0.026 = 2.6¢ → 3¢ (×1). Catalog would give 16¢.
    #[test]
    fn per_model_price_override_wins() {
        let usage = json!({"prompt_tokens": 22000, "completion_tokens": 2000});
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
                2.0
            ),
            3
        );
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                3.0,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
                2.0
            ),
            8
        ); // ×3 → 7.8→8
           // No override (0,0) → catalog price used (16¢), proving the override is what changed it.
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            16
        );
    }

    // model_price_override reads {in,out} from the connection map; missing/empty → (0,0).
    #[test]
    fn model_price_override_reads_map() {
        let mp = json!({"claude-opus-4-8": {"in": 1.5, "out": 2.5}, "gpt-5.5": {}});
        assert_eq!(model_price_override(&mp, "claude-opus-4-8"), (1.5, 2.5));
        assert_eq!(model_price_override(&mp, "gpt-5.5"), (0.0, 0.0)); // empty entry → no override
        assert_eq!(model_price_override(&mp, "absent"), (0.0, 0.0));
        assert_eq!(model_price_override(&json!({}), "anything"), (0.0, 0.0));
    }

    // Image-gen models (any vendor) must be detected so they bill per-image, never $0-tokens.
    #[test]
    fn image_gen_models_detected_across_vendors() {
        for id in [
            "gpt-image-1",
            "gpt-image-2",
            "dall-e-3",
            "gemini-3.1-flash-image-preview",
            "gpt-4o-image",
        ] {
            assert!(is_image_gen_model(id), "should be image: {id}");
        }
        // text / vision models must NOT be treated as image-gen (else they'd bill a flat image fee):
        for id in [
            "claude-opus-4-8",
            "gemini-3.5-flash",
            "gemini-3.1-pro-preview",
            "gpt-5.5",
            "deepseek-v4-pro",
        ] {
            assert!(!is_image_gen_model(id), "should NOT be image: {id}");
        }
    }

    // ---- Anthropic protocol bridge ----
    #[test]
    fn oai_to_anthropic_translates_system_tools_and_toolcalls() {
        let body = json!({
            "model": "claude-haiku-4-5-20251001", "max_tokens": 100,
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "read foo"},
                {"role": "assistant", "content": "ok", "tool_calls": [{"id": "c1", "type": "function", "function": {"name": "read_file", "arguments": "{\"path\":\"foo\"}"}}]},
                {"role": "tool", "tool_call_id": "c1", "content": "file body"}
            ],
            "tools": [{"type": "function", "function": {"name": "read_file", "description": "read", "parameters": {"type": "object", "properties": {"path": {"type": "string"}}}}}]
        });
        let a = oai_to_anthropic(&body).unwrap();
        // system hoisted out of messages, as a block array carrying the cache breakpoint
        assert_eq!(a["system"][0]["text"], json!("You are helpful."));
        assert_eq!(a["system"][0]["cache_control"]["type"], json!("ephemeral"));
        // 3 canonical breakpoints: last tool + system + conversation tail (Anthropic max 4)
        assert_eq!(a["tools"][0]["cache_control"]["type"], json!("ephemeral"));
        let tail = a["messages"].as_array().unwrap().last().unwrap().clone();
        let tail_last_block = tail["content"].as_array().unwrap().last().unwrap().clone();
        assert_eq!(tail_last_block["cache_control"]["type"], json!("ephemeral"));
        assert_eq!(a["max_tokens"], json!(100)); // haiku (fast tier) → no thinking bump
        assert!(
            a.get("thinking").is_none(),
            "haiku stays fast — no extended thinking"
        );
        let msgs = a["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 3); // system removed; user, assistant, tool-result-user
        assert_eq!(msgs[1]["role"], "assistant");
        assert_eq!(msgs[1]["content"][1]["type"], "tool_use");
        assert_eq!(msgs[1]["content"][1]["name"], "read_file");
        assert_eq!(msgs[1]["content"][1]["input"]["path"], "foo"); // arguments string parsed to object
        assert_eq!(msgs[2]["role"], "user");
        assert_eq!(msgs[2]["content"][0]["type"], "tool_result");
        assert_eq!(msgs[2]["content"][0]["tool_use_id"], "c1");
        assert_eq!(a["tools"][0]["name"], "read_file");
        assert!(a["tools"][0]["input_schema"]["properties"]["path"].is_object()); // parameters → input_schema
        assert!(a["tools"][0].get("parameters").is_none());
    }

    #[test]
    fn oai_to_anthropic_rejects_malformed_historical_tool_arguments() {
        let error = oai_to_anthropic(&json!({
            "model": "claude-sonnet-5",
            "messages": [{
                "role": "assistant",
                "tool_calls": [{
                    "id": "call_write_1",
                    "type": "function",
                    "function": {
                        "name": "write_file",
                        "arguments": "{\"path\":\"server/index.js\",\"content\":"
                    }
                }]
            }]
        }))
        .unwrap_err();

        assert!(error.contains("write_file"));
        assert!(error.contains("call_write_1"));
        assert!(error.contains("malformed"));
    }

    /// 4.7+/5/Fable must explicitly ask for summarized display, or the thinking text comes
    /// back as an EMPTY STRING and the 已思考 card never renders. This is the entire reason
    /// 4.6 showed thinking and 4.7 did not: the two families take different branches whose
    /// `display` defaults differ, and `display` was never set anywhere in the stack.
    #[test]
    fn adaptive_thinking_must_ask_for_summarized_display() {
        for model in ["claude-opus-4-7", "claude-opus-4-8", "claude-opus-5", "claude-sonnet-5", "claude-fable-5"] {
            let t = anthropic_thinking(model, Some("high")).expect("thinking must be requested");
            assert_eq!(t["type"], "adaptive", "{model} must use adaptive");
            assert_eq!(
                t["display"], "summarized",
                "{model}: without display=summarized the thinking streams back empty"
            );
        }
        // 4.6 takes the older explicit-budget branch, whose display default is already
        // summarized — it must NOT gain a display field.
        let t46 = anthropic_thinking("claude-opus-4-6", Some("high")).expect("4.6 requests thinking");
        assert_eq!(t46["type"], "enabled", "4.6 keeps the explicit-budget form");
        assert!(t46.get("display").is_none(), "4.6 must not gain a display field");
    }

    #[test]
    fn oai_to_anthropic_enables_thinking_and_drops_temp() {
        // Opus 4.8 + reasoning_effort → adaptive thinking on; temperature/top_p dropped;
        // max_tokens gets headroom; output_config.effort must NOT be sent (it collapses the
        // upstream thinking stream into a one-line summary, and adaptive defaults fine
        // without it).
        let body = json!({
            "model": "claude-opus-4-8", "max_tokens": 4096, "temperature": 0.7, "top_p": 0.9,
            "reasoning_effort": "high",
            "messages": [{"role": "user", "content": "hi"}]
        });
        let a = oai_to_anthropic(&body).unwrap();
        assert_eq!(a["thinking"], json!({"type":"adaptive","display":"summarized"}));
        assert!(
            a.get("output_config").is_none(),
            "output_config.effort must be omitted or upstream returns summarized thinking"
        );
        assert_eq!(a["max_tokens"], json!(40000)); // high effort gets extra thinking headroom
        assert!(
            a.get("temperature").is_none(),
            "temperature must be dropped when thinking is on"
        );
        assert!(
            a.get("top_p").is_none(),
            "top_p must be dropped when thinking is on"
        );

        // Fable 5 → adaptive too (it rejects budget_tokens like the rest of the 5 family).
        assert_eq!(
            oai_to_anthropic(
                &json!({"model":"claude-fable-5","reasoning_effort":"medium","messages":[]})
            )
            .unwrap()["thinking"],
            json!({"type":"adaptive","display":"summarized"})
        );

        // No reasoning_effort (user chose "off" → IDE drops the field) → NO thinking.
        // Sampling knobs are still omitted because current Claude models reject them.
        let off = oai_to_anthropic(&json!({
            "model":"claude-opus-4-8","max_tokens":4096,"temperature":0.5,"top_p":0.9,"messages":[]
        }))
        .unwrap();
        assert!(off.get("thinking").is_none());
        assert_eq!(off["max_tokens"], json!(4096));
        assert!(off.get("temperature").is_none());
        assert!(off.get("top_p").is_none());
    }

    #[test]
    fn thinking_normalized_per_model() {
        // Opus 4.8 with reasoning_effort: gateway normalizes to adaptive. The client may
        // still send the legacy enabled+budget_tokens shape; the gateway must rewrite it,
        // because forwarding it verbatim is a 400 on every model from 4.7 onward.
        let a = oai_to_anthropic(&json!({
            "model": "claude-opus-4-8",
            "reasoning_effort": "max",
            "thinking": {"type": "enabled", "budget_tokens": 32000},
            "messages": [{"role": "user", "content": "hi"}]
        }))
        .unwrap();
        assert_eq!(a["thinking"], json!({"type":"adaptive","display":"summarized"}));
        assert!(a["max_tokens"].as_i64().unwrap() >= 32000);
        assert!(a.get("output_config").is_none()); // effort knob dropped to keep raw thinking

        // Sonnet 5: adaptive as well — enabled+budget_tokens is rejected outright.
        let s5 = oai_to_anthropic(&json!({
            "model": "claude-sonnet-5",
            "reasoning_effort": "high",
            "thinking": {"type": "enabled", "budget_tokens": 16000},
            "messages": []
        }))
        .unwrap();
        assert_eq!(s5["thinking"], json!({"type":"adaptive","display":"summarized"}));

        // Claude 3.7: explicit budget is correct (gateway generates it, not client).
        let b = oai_to_anthropic(&json!({
            "model": "claude-3-7-sonnet-20250219",
            "reasoning_effort": "high",
            "messages": []
        }))
        .unwrap();
        assert_eq!(b["thinking"]["type"], "enabled");
        assert!(b["thinking"]["budget_tokens"].as_i64().unwrap() > 0);
        assert!(b["max_tokens"].as_i64().unwrap() >= 32000);

        // Haiku: no thinking even with effort
        let h = oai_to_anthropic(&json!({
            "model": "claude-haiku-4-5",
            "reasoning_effort": "high",
            "messages": []
        }))
        .unwrap();
        assert!(
            h.get("thinking").is_none(),
            "haiku should not have thinking"
        );
    }

    #[test]
    fn anthropic_thinking_gate_by_model() {
        // Modern Claude (4.7+/5/Fable/Mythos) REMOVED the explicit-budget form: sending
        // {"type":"enabled","budget_tokens":N} is a hard 400 —
        //   "thinking.type.enabled is not supported for this model.
        //    use thinking.type.adaptive and output_config.effort"
        // This is not a preference; it is the upstream contract, observed in production
        // (gateway logs, 2026-08-01, claude-sonnet-5 → 400 on every attempt).
        assert_eq!(
            anthropic_thinking("claude-opus-4-8", Some("medium")),
            Some(json!({"type":"adaptive","display":"summarized"}))
        );
        assert_eq!(
            anthropic_thinking("claude-sonnet-5", Some("high")),
            Some(json!({"type":"adaptive","display":"summarized"}))
        );
        assert_eq!(
            anthropic_thinking("claude-fable-5", Some("low")),
            Some(json!({"type":"adaptive","display":"summarized"}))
        );
        // 4.6 still accepts the explicit budget (deprecated but functional there) — it is
        // the one branch the old aggregator workaround is still valid for.
        assert_eq!(
            anthropic_thinking("claude-sonnet-4-6", Some("high")),
            Some(json!({"type":"enabled","budget_tokens":24000}))
        );
        assert_eq!(
            anthropic_thinking("claude-haiku-4-5-20251001", Some("high")),
            None
        ); // fast tier
        assert_eq!(anthropic_thinking("gpt-5.5", Some("high")), None); // non-Claude
                                                                       // effort absent / "off" → thinking off (respect the user's control).
        assert_eq!(anthropic_thinking("claude-opus-4-8", None), None);
        assert_eq!(anthropic_thinking("claude-opus-4-8", Some("off")), None);
    }

    #[test]
    fn anthropic_to_oai_maps_content_tools_usage() {
        let av = json!({
            "id": "msg_1",
            "content": [{"type": "text", "text": "Hello"}, {"type": "tool_use", "id": "t1", "name": "get_time", "input": {"tz": "Asia/Tokyo"}}],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 10, "output_tokens": 5, "cache_read_input_tokens": 3, "cache_creation_input_tokens": 0}
        });
        let o = anthropic_to_oai(&av, "claude-opus-4-8");
        assert_eq!(o["choices"][0]["finish_reason"], "tool_calls");
        assert_eq!(o["choices"][0]["message"]["content"], "Hello");
        assert_eq!(o["choices"][0]["message"]["tool_calls"][0]["id"], "t1");
        assert_eq!(
            o["choices"][0]["message"]["tool_calls"][0]["function"]["name"],
            "get_time"
        );
        assert!(
            o["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"]
                .as_str()
                .unwrap()
                .contains("Asia/Tokyo")
        );
        assert_eq!(o["usage"]["input_tokens"], 10); // Anthropic name (compute_cost reads this)
        assert_eq!(o["usage"]["prompt_tokens"], 10); // OpenAI name (clients read this)
        assert_eq!(o["usage"]["cache_read_input_tokens"], 3);
    }

    #[test]
    fn thinking_only_end_turn_signature_is_detected_and_healthy_streams_pass() {
        // 中转丢块签名：只回 thinking 就 end_turn → 命中。
        let mut c = AnthSse::new("claude-opus-4-6");
        let _ = c.push(b"data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\"}}\n\n").unwrap();
        let _ = c.push(b"data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"planning...\"}}\n\n").unwrap();
        let _ = c.push(b"data: {\"type\":\"content_block_stop\",\"index\":0}\n\n").unwrap();
        let _ = c.push(b"data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":7553}}\n\n").unwrap();
        let _ = c.push(b"data: {\"type\":\"message_stop\"}\n\n").unwrap();
        assert!(c.thinking_only_end_turn(), "thinking-only end_turn must be flagged as a relay drop");

        // 健康流：thinking 后跟 text/tool_use → 不命中。
        let mut healthy = AnthSse::new("claude-opus-4-6");
        let _ = healthy.push(b"data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\"}}\n\n").unwrap();
        let _ = healthy.push(b"data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n").unwrap();
        let _ = healthy.push(b"data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\n").unwrap();
        assert!(!healthy.thinking_only_end_turn());

        // 工具收尾（stop_reason=tool_use）永不误报。
        let mut tooled = AnthSse::new("claude-opus-4-6");
        let _ = tooled.push(b"data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"x\"}}\n\n").unwrap();
        let _ = tooled.push(b"data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"t1\",\"name\":\"write_file\",\"input\":{}}}\n\n").unwrap();
        let _ = tooled.push(b"data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"}}\n\n").unwrap();
        assert!(!tooled.thinking_only_end_turn());

        // 无思考的普通回答（end_turn）不命中——签名要求见过思考块。
        let mut plain = AnthSse::new("claude-opus-4-6");
        let _ = plain.push(b"data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}}\n\n").unwrap();
        let _ = plain.push(b"data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\n").unwrap();
        assert!(!plain.thinking_only_end_turn());
    }

    #[test]
    fn clip_thinking_budget_only_lowers_oversized_budgets() {
        // 超限预算 → 钳到安全值。
        let mut big = json!({"model":"claude-opus-4-6","thinking":{"type":"enabled","budget_tokens":24000},"max_tokens":40000});
        assert!(clip_thinking_budget(&mut big));
        assert_eq!(big.pointer("/thinking/budget_tokens"), Some(&json!(THINKING_CLIP_SAFE_BUDGET)));
        // 本就安全的预算不动。
        let mut small = json!({"thinking":{"type":"enabled","budget_tokens":4096}});
        assert!(!clip_thinking_budget(&mut small));
        assert_eq!(small.pointer("/thinking/budget_tokens"), Some(&json!(4096)));
        // 没开思考不动。
        let mut off = json!({"model":"claude-opus-4-6","max_tokens":8192});
        assert!(!clip_thinking_budget(&mut off));
        assert!(off.get("thinking").is_none());
    }

    #[test]
    fn thinking_clip_route_marking_expires_and_isolates_routes() {
        let bad = uuid::Uuid::new_v4();
        let good = uuid::Uuid::new_v4();
        assert!(!thinking_clip_active(bad));
        mark_thinking_clip(bad);
        assert!(thinking_clip_active(bad), "marked route must be clipped");
        assert!(!thinking_clip_active(good), "healthy routes must not be affected");
        if let Ok(mut guard) = THINKING_CLIP_ROUTES.lock() {
            guard.insert(bad, ClipInstant::now() - ClipDuration::from_secs(1));
        }
        assert!(!thinking_clip_active(bad), "expired clip must auto-release");
    }

    #[test]
    fn anth_sse_converts_stream_to_openai() {
        // Event shapes copied verbatim from a real zyz streaming response (tool call).
        let mut c = AnthSse::new("claude-opus-4-8");
        let mut out: Vec<u8> = Vec::new();
        out.extend(c.push(b"event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"m1\",\"usage\":{\"input_tokens\":15,\"cache_read_input_tokens\":46,\"cache_creation_input_tokens\":0,\"output_tokens\":0}}}\n\n").unwrap());
        out.extend(
            c.push(b"event: ping\ndata: {\"type\":\"ping\"}\n\n")
                .unwrap(),
        );
        out.extend(c.push(b"event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"tooluse_1\",\"name\":\"get_time\",\"input\":{}}}\n\n").unwrap());
        out.extend(c.push(b"event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"tz\\\": \\\"As\"}}\n\n").unwrap());
        out.extend(c.push(b"event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"ia/Tokyo\\\"}\"}}\n\n").unwrap());
        out.extend(c.push(b"event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n").unwrap());
        out.extend(c.push(b"event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"},\"usage\":{\"output_tokens\":18,\"input_tokens\":15,\"cache_read_input_tokens\":46}}\n\n").unwrap());
        out.extend(
            c.push(b"event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n")
                .unwrap(),
        );
        out.extend(c.finish().unwrap());
        // Parse the emitted OpenAI SSE back (no key-order assumptions).
        let s = String::from_utf8(out).unwrap();
        let (mut role, mut id, mut name, mut args, mut finish, mut done, mut idx) = (
            false,
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            false,
            -1i64,
        );
        for line in s.lines() {
            let d = match line.strip_prefix("data:") {
                Some(x) => x.trim(),
                None => continue,
            };
            if d == "[DONE]" {
                done = true;
                continue;
            }
            let v: serde_json::Value = serde_json::from_str(d).unwrap();
            let delta = &v["choices"][0]["delta"];
            if delta["role"] == "assistant" {
                role = true;
            }
            if let Some(tcs) = delta["tool_calls"].as_array() {
                for tc in tcs {
                    if let Some(i) = tc["index"].as_i64() {
                        idx = i;
                    }
                    if let Some(x) = tc["id"].as_str() {
                        if !x.is_empty() {
                            id = x.into();
                        }
                    }
                    if let Some(n) = tc["function"]["name"].as_str() {
                        if !n.is_empty() {
                            name = n.into();
                        }
                    }
                    if let Some(a) = tc["function"]["arguments"].as_str() {
                        args.push_str(a);
                    }
                }
            }
            if let Some(f) = v["choices"][0]["finish_reason"].as_str() {
                finish = f.into();
            }
        }
        assert!(role, "role bootstrap chunk emitted");
        assert_eq!(id, "tooluse_1");
        assert_eq!(name, "get_time");
        assert_eq!(idx, 0);
        assert_eq!(args, "{\"tz\": \"Asia/Tokyo\"}"); // input_json_delta pieces concatenated
        assert_eq!(finish, "tool_calls");
        assert!(done);
        let u = c.usage(); // accumulated for billing (cache-aware)
        assert_eq!(u["input_tokens"], 15);
        assert_eq!(u["output_tokens"], 18);
        assert_eq!(u["cache_read_input_tokens"], 46);
    }

    #[test]
    fn anth_sse_preserves_non_empty_tool_input_from_block_start() {
        let required = std::collections::HashMap::from([(
            "write_file".to_string(),
            vec!["path".to_string(), "content".to_string()],
        )]);
        let mut c = AnthSse::with_required_tool_args("claude-sonnet-5", required);
        let mut out = c
            .push(b"data: {\"type\":\"content_block_start\",\"index\":2,\"content_block\":{\"type\":\"tool_use\",\"id\":\"write_1\",\"name\":\"write_file\",\"input\":{\"path\":\"server/index.js\",\"content\":\"module.exports = {};\"}}}\n\n")
            .unwrap();
        out.extend(
            c.push(b"data: {\"type\":\"content_block_stop\",\"index\":2}\n\n")
                .unwrap(),
        );
        out.extend(
            c.push(
                b"data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"}}\n\n",
            )
            .unwrap(),
        );
        out.extend(c.push(b"data: {\"type\":\"message_stop\"}\n\n").unwrap());
        out.extend(c.finish().unwrap());

        let mut arguments = String::new();
        for line in String::from_utf8(out).unwrap().lines() {
            let Some(data) = line.strip_prefix("data:").map(str::trim) else {
                continue;
            };
            if data == "[DONE]" {
                continue;
            }
            let event: serde_json::Value = serde_json::from_str(data).unwrap();
            if let Some(fragment) =
                event["choices"][0]["delta"]["tool_calls"][0]["function"]["arguments"].as_str()
            {
                arguments.push_str(fragment);
            }
        }
        let parsed: serde_json::Value = serde_json::from_str(&arguments).unwrap();
        assert_eq!(parsed["path"], "server/index.js");
        assert_eq!(parsed["content"], "module.exports = {};");
    }

    #[test]
    fn anth_sse_rejects_unknown_tool_delta_index() {
        let mut c = AnthSse::new("claude-sonnet-5");
        let error = c
            .push(b"data: {\"type\":\"content_block_delta\",\"index\":7,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{}\"}}\n\n")
            .unwrap_err();
        assert!(error.contains("unknown content block index 7"));
    }

    #[test]
    fn anth_sse_rejects_clean_eof_without_message_stop() {
        let mut c = AnthSse::new("claude-sonnet-5");
        c.push(
            b"data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":1}}}\n\n",
        )
        .unwrap();
        c.push(b"data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\n")
            .unwrap();
        let error = c.finish().unwrap_err();
        assert!(error.contains("before message_stop"));
    }

    #[test]
    fn anth_sse_rejects_incomplete_or_missing_required_tool_arguments() {
        let required = std::collections::HashMap::from([(
            "write_file".to_string(),
            vec!["path".to_string(), "content".to_string()],
        )]);
        let mut incomplete = AnthSse::with_required_tool_args("claude-sonnet-5", required.clone());
        incomplete
            .push(b"data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"w1\",\"name\":\"write_file\",\"input\":{}}}\n\n")
            .unwrap();
        incomplete
            .push(b"data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"path\\\":\\\"server/index.js\\\"\"}}\n\n")
            .unwrap();
        let error = incomplete
            .push(b"data: {\"type\":\"content_block_stop\",\"index\":0}\n\n")
            .unwrap_err();
        assert!(error.contains("incomplete arguments JSON"));

        let mut missing = AnthSse::with_required_tool_args("claude-sonnet-5", required);
        missing
            .push(b"data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"w2\",\"name\":\"write_file\",\"input\":{}}}\n\n")
            .unwrap();
        let error = missing
            .push(b"data: {\"type\":\"content_block_stop\",\"index\":0}\n\n")
            .unwrap_err();
        assert!(error.contains("missing required arguments: path, content"));
    }

    #[test]
    fn anth_sse_rejects_empty_schema_constrained_tool_arguments() {
        let body = json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "write_file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "minLength": 1},
                            "content": {"type": "string", "minLength": 1}
                        },
                        "required": ["path", "content"]
                    }
                }
            }]
        });
        let mut stream =
            AnthSse::with_tool_argument_rules("claude-sonnet-5", tool_argument_rules(&body));
        stream
            .push(b"data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"w3\",\"name\":\"write_file\",\"input\":{\"path\":\"src/a.js\",\"content\":\"\"}}}\n\n")
            .unwrap();
        let error = stream
            .push(b"data: {\"type\":\"content_block_stop\",\"index\":0}\n\n")
            .unwrap_err();
        assert!(error.contains("argument \"content\" is shorter than minLength 1"));
    }

    #[test]
    fn anth_sse_rejects_invalid_utf8_even_when_message_stop_follows() {
        let mut c = AnthSse::new("claude-sonnet-5");
        let mut bytes = b"data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"".to_vec();
        bytes.push(0xff);
        bytes.extend_from_slice(b"\"}}\n\ndata: {\"type\":\"message_stop\"}\n\n");

        let error = c.push(&bytes).unwrap_err();

        assert!(error.contains("invalid UTF-8"));
        assert!(c.finish().is_err());
    }

    // The official catalog must cover every token model live on the gateway, matched by
    // family so date/`-preview` suffixes still resolve. (Image models → per-image, None here.)
    #[test]
    fn official_catalog_covers_live_models() {
        assert_eq!(official_price("claude-fable-5"), Some((10.0, 50.0)));
        assert_eq!(official_price("claude-opus-4-8"), Some((5.0, 25.0)));
        assert_eq!(official_price("claude-opus-4-6"), Some((5.0, 25.0)));
        assert_eq!(official_price("claude-sonnet-5"), Some((3.0, 15.0)));
        assert_eq!(official_price("claude-sonnet-4-6"), Some((3.0, 15.0)));
        assert_eq!(
            official_price("claude-haiku-4-5-20251001"),
            Some((1.0, 5.0))
        ); // date suffix matches
        assert_eq!(official_price("gemini-3.1-pro-preview"), Some((2.0, 12.0)));
        assert_eq!(official_price("gemini-3.5-flash"), Some((1.5, 9.0)));
        assert_eq!(official_price("gemini-3.1-flash-image-preview"), None); // image → per-image billing
        assert_eq!(official_price("gpt-5.5"), Some((5.0, 30.0)));
        assert_eq!(official_price("gpt-5.4"), Some((2.5, 15.0)));
        assert_eq!(official_price("deepseek-v4-flash"), Some((0.14, 0.28)));
        assert_eq!(official_price("deepseek-v4-pro"), Some((0.435, 0.87)));
        assert_eq!(official_price("MiniMax-M3"), Some((0.30, 1.20))); // case-insensitive
        assert_eq!(official_price("MiniMax-M2.7-highspeed"), Some((0.25, 1.00)));
        assert_eq!(official_price("MiniMax-M2.1"), Some((0.15, 0.60)));
        assert_eq!(official_price("MiniMax-M2"), Some((0.10, 0.40)));
        assert_eq!(official_price("some-unknown-model"), None); // → connection fallback, then 0
    }

    // GLM / Grok 走"透传任意 id"的连接（连接价 0、无按模型覆盖），此前不在目录里 → 一直按
    // 0 计费。默认定价 = 官方牌价进目录（docs.z.ai / x.ai，2026-07），连接倍率照常乘在上面。
    #[test]
    fn official_catalog_covers_glm_and_grok_families() {
        assert_eq!(official_price("glm-5.2"), Some((1.40, 4.40)));
        assert_eq!(official_price("GLM-5.1"), Some((1.40, 4.40))); // case-insensitive
        assert_eq!(official_price("glm-5"), Some((1.00, 3.20)));
        assert_eq!(official_price("glm-4.7-flashx"), Some((0.07, 0.40)));
        assert_eq!(official_price("glm-4.7"), Some((0.60, 2.20)));
        assert_eq!(official_price("glm-4.6"), Some((0.60, 2.20)));
        assert_eq!(official_price("glm-4.5-airx"), Some((1.10, 4.50))); // airx 先于 air
        assert_eq!(official_price("glm-4.5-air"), Some((0.20, 1.10)));
        assert_eq!(official_price("glm-4.5-x"), Some((2.20, 8.90)));
        assert_eq!(official_price("glm-4.5"), Some((0.60, 2.20)));
        assert_eq!(official_price("glm-next"), Some((0.60, 2.20))); // 未知变体按 4.x 主档兜底

        assert_eq!(official_price("grok-code-fast-1"), Some((0.20, 1.50)));
        assert_eq!(official_price("grok-4.20"), Some((2.0, 6.0)));
        assert_eq!(official_price("grok-4.5"), Some((2.0, 6.0)));
        assert_eq!(official_price("grok-4.3"), Some((1.25, 2.50)));
        assert_eq!(official_price("grok-4.1-fast"), Some((0.20, 0.50)));
        assert_eq!(official_price("grok-4-fast"), Some((0.20, 0.50)));
        assert_eq!(official_price("grok-build-0.1"), Some((1.0, 2.0)));
        assert_eq!(official_price("grok-3-mini"), Some((0.30, 0.50)));
        assert_eq!(official_price("grok-4-0709"), Some((3.0, 15.0)));
        assert_eq!(official_price("grok-3"), Some((3.0, 15.0)));
        assert_eq!(official_price("grok-x-new"), Some((2.0, 6.0))); // 未知新 Grok 按旗舰档兜底
    }

    // No usage reported → 0 (never guesses token counts).
    #[test]
    fn no_usage_is_zero() {
        assert_eq!(
            compute_cost(None, "claude-opus-4-8", 3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            0
        );
        assert_eq!(
            compute_cost(
                Some(&json!({})),
                "claude-opus-4-8",
                3.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            0
        );
    }

    // Anthropic-style field names (input_tokens/output_tokens) are honored.
    #[test]
    fn anthropic_field_names() {
        let usage = json!({"input_tokens": 22000, "output_tokens": 2000});
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            16
        );
    }

    // OpenAI cached-prompt shape: cached input billed at 0.1×. opus, prompt 10000 (8000
    // cached), completion 0, ×1: billable = 2000 + 800 = 2800; 2800·5/1e6 = $0.014 → 1¢.
    #[test]
    fn cached_input_discount() {
        let usage = json!({"prompt_tokens": 10000, "completion_tokens": 0,
                           "prompt_tokens_details": {"cached_tokens": 8000}});
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            1
        );
    }

    // A malformed/huge usage can never drain a balance — capped at $50 (5000¢).
    #[test]
    fn ceiling_caps_runaway() {
        let usage = json!({"prompt_tokens": 999_999_999i64, "completion_tokens": 999_999_999i64});
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                3.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            5000
        );
    }

    // Pull the trailing usage chunk out of a real-shaped SSE stream and bill it.
    #[test]
    fn sse_usage_extraction() {
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n\
                   data: {\"choices\":[],\"usage\":{\"prompt_tokens\":22000,\"completion_tokens\":2000,\"total_tokens\":24000}}\n\n\
                   data: [DONE]\n\n";
        let u = parse_usage_from_sse(sse.as_bytes()).expect("usage present");
        assert_eq!(u.get("prompt_tokens").and_then(|v| v.as_i64()), Some(22000));
        assert_eq!(
            compute_cost(
                Some(&u),
                "claude-opus-4-8",
                3.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            48
        );
    }

    // The 64KB usage tail can begin MID-LINE (cut from a bigger stream): leading garbage
    // is skipped and the trailing usage still extracted.
    #[test]
    fn sse_usage_from_truncated_tail() {
        let tail = "ent\":\" tokens\"}}]}\n\n\
                    data: {\"choices\":[],\"usage\":{\"prompt_tokens\":50000,\"completion_tokens\":3000}}\n\n\
                    data: [DONE]\n\n";
        let u = parse_usage_from_sse(tail.as_bytes()).expect("usage present in tail");
        assert_eq!(
            u.get("completion_tokens").and_then(|v| v.as_i64()),
            Some(3000)
        );
    }

    #[test]
    fn openai_sse_clean_eof_without_done_is_incomplete() {
        let partial = b"data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n\
                        data: {\"choices\":[],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":1}}\n\n";
        let error = validate_openai_sse_eof(partial).unwrap_err();
        assert!(error.contains("without terminal data: [DONE]"));

        // A marker mentioned inside JSON content is not an SSE terminal event.
        let embedded = b"data: {\"choices\":[{\"delta\":{\"content\":\"data: [DONE]\"}}]}\n\n";
        assert!(validate_openai_sse_eof(embedded).is_err());
    }

    #[test]
    fn openai_sse_done_line_marks_clean_eof_complete() {
        let complete = b"data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\r\n\r\n\
                         data:[DONE]\r\n\r\n";
        assert_eq!(validate_openai_sse_eof(complete), Ok(()));
    }

    #[test]
    fn openai_sse_rejects_malformed_json_before_done() {
        let stream = b"data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"write_file\",\"arguments\":\"{}\"}}]}}]}\n\n\
                       data: {malformed\n\n\
                       data: [DONE]\n\n";

        let error = validate_openai_sse_eof(stream).unwrap_err();

        assert!(error.contains("malformed JSON"));
    }

    #[test]
    fn openai_sse_rejects_incomplete_missing_and_empty_required_tool_arguments() {
        let body = json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "write_file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "minLength": 1},
                            "content": {"type": "string", "minLength": 1}
                        },
                        "required": ["path", "content"]
                    }
                }
            }]
        });
        let rules = tool_argument_rules(&body);

        let incomplete = b"data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"write_file\",\"arguments\":\"{\\\"path\\\":\\\"src/a.js\\\",\\\"content\\\":\"}}]}}]}\n\ndata: [DONE]\n\n";
        let error = validate_openai_sse_with_rules(incomplete, rules.clone()).unwrap_err();
        assert!(error.contains("incomplete arguments JSON"));

        let missing = b"data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"write_file\",\"arguments\":\"{\\\"path\\\":\\\"src/a.js\\\"}\"}}]}}]}\n\ndata: [DONE]\n\n";
        let error = validate_openai_sse_with_rules(missing, rules.clone()).unwrap_err();
        assert!(error.contains("missing required arguments: content"));

        let empty = b"data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"write_file\",\"arguments\":\"{\\\"path\\\":\\\"src/a.js\\\",\\\"content\\\":\\\"\\\"}\"}}]}}]}\n\ndata: [DONE]\n\n";
        let error = validate_openai_sse_with_rules(empty, rules).unwrap_err();
        assert!(error.contains("argument \"content\" is shorter than minLength 1"));
    }

    #[test]
    fn openai_sse_rejects_terminal_event_before_incomplete_tool_call_can_complete() {
        let body = json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "write_file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string"},
                            "content": {"type": "string"}
                        },
                        "required": ["path", "content"]
                    }
                }
            }]
        });
        let mut validator =
            OpenAiSseValidator::with_tool_argument_rules(tool_argument_rules(&body));
        validator
            .push(b"data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"write_file\",\"arguments\":\"{\\\"path\\\":\\\"src/a.js\\\",\\\"content\\\":\"}}]}}]}\n\n")
            .unwrap();

        // The streaming caller validates a chunk before forwarding it, so this
        // error keeps [DONE] from reaching the client or being cached as success.
        let error = validator.push(b"data: [DONE]\n\n").unwrap_err();

        assert!(error.contains("incomplete arguments JSON"));
        assert!(!validator.done_seen);
    }

    #[test]
    fn openai_sse_accumulates_complete_tool_argument_fragments() {
        let body = json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "write_file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "minLength": 1},
                            "content": {"type": "string", "minLength": 1}
                        },
                        "required": ["path", "content"]
                    }
                }
            }]
        });
        let stream = b"data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"write_file\",\"arguments\":\"{\\\"path\\\":\\\"src/a.js\\\",\"}}]}}]}\n\ndata: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"content\\\":\\\"ok\\\"}\"}}]}}]}\n\ndata: [DONE]\n\n";

        assert_eq!(
            validate_openai_sse_with_rules(stream, tool_argument_rules(&body)),
            Ok(())
        );
    }

    #[test]
    fn openai_sse_rejects_invalid_utf8_before_done() {
        let mut stream = b"data: {\"choices\":[{\"delta\":{\"content\":\"".to_vec();
        stream.push(0xff);
        stream.extend_from_slice(b"\"}}]}\n\ndata: [DONE]\n\n");

        let error = validate_openai_sse_eof(&stream).unwrap_err();

        assert!(error.contains("invalid UTF-8"));
    }

    // A stream that never reported usage → None → caller bills 0.
    #[test]
    fn sse_no_usage() {
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\ndata: [DONE]\n\n";
        assert!(parse_usage_from_sse(sse.as_bytes()).is_none());
    }
}

#[cfg(test)]
mod cache_price_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn explicit_cache_prices_used() {
        // Anthropic shape: 1000 plain input + 2000 cache_read + 500 cache_create + 300 output
        let u = json!({"input_tokens":1000,"output_tokens":300,"cache_read_input_tokens":2000,"cache_creation_input_tokens":500});
        // off_in=5, off_out=25 (official claude). explicit read=0.5, create=6.5. rate=1.
        // usd = (1000*5 + 2000*0.5 + 500*6.5 + 300*25)/1e6 = (5000+1000+3250+7500)/1e6 = 16750/1e6
        // cents = 16750/1e6 *100 *1 = 1.675 → round 2
        let c = compute_cost(
            Some(&u),
            "claude-opus-4-6",
            1.0,
            0.0,
            0.0,
            0.5,
            6.5,
            0.0,
            0.0,
        );
        assert_eq!(c, 2, "explicit cache prices: got {}", c);
        // with cache prices = 0 → falls back to factors (read 0.1*5=0.5, write 1.25*5=6.25)
        // usd = (1000*5 + 2000*0.5 + 500*6.25 + 300*25)/1e6 = (5000+1000+3125+7500)/1e6=16625 → 1.66 → 2
        let c2 = compute_cost(
            Some(&u),
            "claude-opus-4-6",
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        assert_eq!(c2, 2, "factor fallback: got {}", c2);
    }
}

#[cfg(test)]
mod authoritative_usage_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn complete_usage_shapes_are_authoritative() {
        assert!(usage_is_authoritative(Some(
            &json!({"prompt_tokens": 0, "completion_tokens": 0})
        )));
        assert!(usage_is_authoritative(Some(
            &json!({"input_tokens": 800, "output_tokens": 300})
        )));
        assert!(!usage_is_authoritative(Some(
            &json!({"prompt_tokens": 800})
        )));
        assert!(!usage_is_authoritative(None));
    }

    #[test]
    fn extract_bill_tokens_openai_shape() {
        let u = json!({"prompt_tokens": 500, "completion_tokens": 100,
                        "prompt_tokens_details": {"cached_tokens": 50}});
        let bt = extract_bill_tokens(Some(&u), "gpt-5.5", false);
        assert_eq!(bt.prompt, 500);
        assert_eq!(bt.completion, 100);
        assert_eq!(bt.cached, 50);
        assert_eq!(bt.model_name, "gpt-5.5");
        assert!(!bt.estimated);
    }

    #[test]
    fn extract_bill_tokens_anthropic_shape() {
        let u = json!({"input_tokens": 800, "output_tokens": 300,
                        "cache_read_input_tokens": 200,
                        "cache_creation_input_tokens": 450});
        let bt = extract_bill_tokens(Some(&u), "claude-opus-4-8", false);
        assert_eq!(bt.prompt, 800);
        assert_eq!(bt.completion, 300);
        assert_eq!(bt.cached, 200);
        assert_eq!(bt.cache_creation, 450);
    }

    #[test]
    fn extract_bill_tokens_none_returns_zeros() {
        let bt = extract_bill_tokens(None, "test", true);
        assert_eq!(bt.prompt, 0);
        assert_eq!(bt.completion, 0);
        assert_eq!(bt.cache_creation, 0);
        assert!(bt.estimated);
    }

    #[test]
    fn anth_sse_never_estimates_missing_output_usage() {
        let mut c = AnthSse::new("claude-opus-4-8");
        let bytes = b"data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":1000}}}\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello world, this is a test response with some content.\"}}\n";
        c.push(bytes).unwrap();
        let u = c.usage();
        assert_eq!(u["input_tokens"], 1000);
        assert_eq!(u["output_tokens"], 0);
        assert!(!c.usage_is_authoritative());
    }

    #[test]
    fn rate_billing_without_usage_is_zero() {
        let cost = resolve_cost(
            "rate",
            999,
            None,
            "claude-opus-4-8",
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        assert_eq!(cost, 0);
    }
}

#[cfg(test)]
mod anth_usage_harvest_tests {
    use super::*;

    /// Relays that attach the final `usage` to `message_stop` instead of
    /// `message_delta` used to be billed as ZERO: only `message_delta` was inspected,
    /// so `output_usage_reported` stayed false and `compute_cost` returned 0.
    /// Production was logging "provider omitted authoritative usage" for ~18% of
    /// Claude calls, opus-5 among them.
    #[test]
    fn usage_on_message_stop_is_authoritative() {
        let mut c = AnthSse::new("claude-opus-5");
        c.push(
            b"data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":1200}}}\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}}\ndata: {\"type\":\"content_block_stop\",\"index\":0}\ndata: {\"type\":\"message_stop\",\"usage\":{\"input_tokens\":1200,\"output_tokens\":340}}\n",
        )
        .expect("stream parses");
        let u = c.usage();
        assert_eq!(u["input_tokens"], 1200);
        assert_eq!(u["output_tokens"], 340);
        assert!(
            c.usage_is_authoritative(),
            "usage reported on message_stop must count as authoritative"
        );
    }

    /// A running counter must not be walked backwards by a later smaller figure.
    #[test]
    fn running_output_counts_only_move_upward() {
        let mut c = AnthSse::new("claude-sonnet-5");
        c.push(
            b"data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":10}}}\ndata: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":500}}\ndata: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":7}}\n",
        )
        .expect("stream parses");
        assert_eq!(c.usage()["output_tokens"], 500);
    }

    /// Harvesting must never invent numbers: a stream that reports no output tokens at
    /// all stays non-authoritative, so billing still refuses to charge for it.
    #[test]
    fn missing_output_usage_is_still_not_authoritative() {
        let mut c = AnthSse::new("claude-opus-5");
        c.push(
            b"data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":90}}}\ndata: {\"type\":\"message_stop\"}\n",
        )
        .expect("stream parses");
        assert!(!c.usage_is_authoritative());
        assert_eq!(c.usage()["output_tokens"], 0);
    }
}

#[cfg(test)]
mod michael_compression_wiring_tests {
    use super::*;

    /// 档位必须两种写法都能进来，且**不给就是不启用**——这是这个特性对现有流量零影响的
    /// 全部保证。
    #[test]
    fn tier_is_opt_in_from_header_or_body() {
        let empty = serde_json::json!({});
        assert!(compression_tier_from(&HeaderMap::new(), &empty).is_none());

        let mut h = HeaderMap::new();
        h.insert("x-michael-compression", "2m".parse().unwrap());
        assert_eq!(
            compression_tier_from(&h, &empty),
            Some(crate::compression::Tier::M2)
        );

        let body = serde_json::json!({ "michael_compression": "5m" });
        assert_eq!(
            compression_tier_from(&HeaderMap::new(), &body),
            Some(crate::compression::Tier::M5)
        );
        // 无法识别的值当作没请求，而不是报错打断聊天。
        let bad = serde_json::json!({ "michael_compression": "9m" });
        assert!(compression_tier_from(&HeaderMap::new(), &bad).is_none());
    }

    /// 顺序/角色原样保留，且**开头的 system 被钉住不参与压缩**。
    ///
    /// 服务端组装的 L0 系统提示词就在 messages[0]，而逐字尾部是从末尾往前取的 ——
    /// 不钉住的话压缩一触发它必然落进被压前缀，整套行为准则被一段 600 token 的
    /// 摘要替换掉。
    #[test]
    fn messages_are_read_in_order_with_leading_system_pinned() {
        let body = serde_json::json!({
            "messages": [
                { "role": "system", "content": "规则" },
                { "role": "user", "content": "问题" },
                { "role": "assistant", "content": [{ "type": "text", "text": "回答" }] },
            ]
        });
        let (pinned, msgs) = compression_plan_input(&body);
        assert_eq!(pinned, 1, "开头的 system 必须被钉住");
        assert_eq!(msgs.len(), 2, "可压缩部分不含被钉住的 system");
        assert_eq!(msgs[0].text, "问题");
        // 多模态内容按其文本部分参与规划。
        assert_eq!(msgs[1].text, "回答");
        assert!(msgs.iter().all(|m| m.tokens > 0));
    }

    #[test]
    fn nontext_content_is_never_silently_summarized() {
        let text = serde_json::json!({
            "role": "user",
            "content": [{ "type": "text", "text": "only text" }]
        });
        let image = serde_json::json!({
            "role": "user",
            "content": [
                { "type": "text", "text": "inspect this" },
                { "type": "image_url", "image_url": { "url": "data:image/png;base64,AA==" } }
            ]
        });
        let unknown = serde_json::json!({
            "role": "user",
            "content": [{ "type": "future_media", "data": "opaque" }]
        });
        assert!(!compression_message_has_nontext_content(&text));
        assert!(compression_message_has_nontext_content(&image));
        assert!(compression_message_has_nontext_content(&unknown));
    }

    /// tool_calls 里的负载必须计入体积。
    ///
    /// agent 模式下 write_file / multi_edit 的**整个文件内容都在
    /// tool_calls[].function.arguments 里**，而 content 是 null。只数 content 的话
    /// 最大的那些消息全被估成 0 token，规划器认为"没超窗口"什么都不压 —— 压缩在最
    /// 需要它的场景下恰好不工作。
    #[test]
    fn tool_call_payloads_count_toward_size() {
        let big = "x".repeat(4000);
        let body = serde_json::json!({
            "messages": [
                { "role": "user", "content": "改文件" },
                { "role": "assistant", "content": serde_json::Value::Null, "tool_calls": [
                    { "id": "c1", "type": "function", "function": { "name": "write_file", "arguments": big } }
                ]},
            ]
        });
        let (_, msgs) = compression_plan_input(&body);
        assert_eq!(msgs.len(), 2);
        assert!(
            msgs[1].tokens > 500,
            "带 tool_calls 的消息不能被估成 0 token，实测 {}",
            msgs[1].tokens
        );
    }

    /// 全是 system 时不能把一切都钉住，否则永远压不动。
    #[test]
    fn all_system_messages_still_leave_something_compressible() {
        let body = serde_json::json!({
            "messages": [
                { "role": "system", "content": "a" },
                { "role": "system", "content": "b" },
            ]
        });
        let (pinned, msgs) = compression_plan_input(&body);
        assert_eq!(pinned, 1);
        assert_eq!(msgs.len(), 1);
    }

    /// 写回必须**无损**：tool_calls / tool_call_id / name 全部原样保留。
    ///
    /// 这是压缩之前不能上线的头号原因。上一版拿 Msg 重建 `{role, content}`，写回之后
    /// 数组里会出现没有 tool_call_id 的 `{"role":"tool"}` 消息，上游直接拒收 ——
    /// 也就是 agent 模式一压缩就整个坏掉。
    #[test]
    fn write_back_preserves_tool_call_structure() {
        let mut body = serde_json::json!({
            "michael_compression": "5m",
            "messages": [
                { "role": "system", "content": "系统提示词" },
                { "role": "user", "content": "老消息" },
                { "role": "assistant", "content": serde_json::Value::Null, "tool_calls": [
                    { "id": "call_1", "type": "function", "function": { "name": "read_file", "arguments": "{}" } }
                ]},
                { "role": "tool", "tool_call_id": "call_1", "name": "read_file", "content": "文件内容" },
                { "role": "user", "content": "新问题" },
            ]
        });
        // 压掉前两条（索引相对 pinned 之后：0=老消息, 1=assistant），逐字从 2 起。
        compression_write_back(&mut body, 1, 2, &["早期摘要".to_string()], None);
        let arr = body["messages"].as_array().expect("messages 必须还在");

        assert_eq!(arr[0]["role"], "system");
        assert_eq!(
            arr[0]["content"], "系统提示词",
            "钉住的系统提示词必须原样保留"
        );
        assert_eq!(arr[1]["role"], "system");
        assert!(
            arr[1]["content"].as_str().unwrap().contains("早期摘要"),
            "摘要作为一条新的 system 注入"
        );
        // 逐字尾部必须是**原始对象**，结构字段一个不少。
        assert_eq!(arr[2]["role"], "tool");
        assert_eq!(arr[2]["tool_call_id"], "call_1", "tool_call_id 不能丢");
        assert_eq!(arr[2]["name"], "read_file", "name 不能丢");
        assert_eq!(arr[3]["content"], "新问题");
        assert_eq!(arr.len(), 4);
    }

    #[test]
    fn exact_history_is_injected_between_summary_and_recent_tail() {
        let mut body = serde_json::json!({
            "messages": [
                { "role": "system", "content": "规则" },
                { "role": "user", "content": "旧问题" },
                { "role": "assistant", "content": "旧回答" },
                { "role": "user", "content": "当前问题" }
            ]
        });
        let evidence = "<history-evidence>src/auth.rs:42 JWT_TTL=3600</history-evidence>";
        compression_write_back(
            &mut body,
            1,
            2,
            &["认证模块曾修改".to_string()],
            Some(evidence),
        );
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0]["content"], "规则");
        assert!(messages[1]["content"]
            .as_str()
            .unwrap()
            .contains("认证模块曾修改"));
        assert_eq!(messages[2]["content"], evidence);
        assert_eq!(messages[3]["content"], "当前问题");
    }

    #[test]
    fn fixed_overhead_counts_pinned_prompts_and_tool_schemas() {
        let body = serde_json::json!({
            "messages": [
                { "role": "system", "content": "x".repeat(4000) },
                { "role": "user", "content": "hi" }
            ],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "write_file",
                    "description": "y".repeat(4000),
                    "parameters": {"type":"object"}
                }
            }]
        });
        let overhead = compression_fixed_overhead_tokens(&body, 1);
        assert!(overhead > 3_500, "固定开销不能只留一个常量余量: {overhead}");
    }

    #[test]
    fn retrieval_query_never_drops_the_latest_user_request() {
        let marker = "LATEST_USER_INVOICE_771923";
        let messages = vec![
            crate::compression::Msg::new("user", marker),
            crate::compression::Msg::new("assistant", "x".repeat(30_000)),
            crate::compression::Msg::new("tool", "y".repeat(30_000)),
            crate::compression::Msg::new("assistant", "z".repeat(30_000)),
        ];
        let query = compression_retrieval_query(&messages);
        assert!(query.contains(marker));
        assert!(query.chars().count() <= 16_001);
    }

    #[test]
    fn archive_segment_keeps_original_json_and_searchable_tool_arguments() {
        let body = serde_json::json!({
            "messages": [
                { "role": "system", "content": "规则" },
                { "role": "assistant", "content": null, "tool_calls": [{
                    "id": "call_archive",
                    "type": "function",
                    "function": {
                        "name": "write_file",
                        "arguments": "{\"path\":\"src/archive.rs\",\"content\":\"const LIMIT: u32 = 5000000;\"}"
                    }
                }]}
            ]
        });
        let (pinned, msgs) = compression_plan_input(&body);
        let segment = crate::compression::Segment {
            start: 0,
            end: 1,
            tokens: msgs[0].tokens,
        };
        let (_, archive, index) =
            compression_archive_segment(&body, pinned, &msgs, &segment).expect("archive");
        assert_eq!(
            archive.messages[0].original["tool_calls"][0]["id"],
            "call_archive"
        );
        assert!(archive.messages[0].text.contains("src/archive.rs"));
        assert!(index.terms.iter().any(|term| term == "5000000"));
    }

    /// 协议字段绝不能透传给上游 —— 包括**不压缩**的那些路径。
    #[test]
    fn protocol_fields_are_stripped_even_without_compressing() {
        let mut body = serde_json::json!({
            "michael_compression": "2m",
            "mc_prefix": "tok",
            "messages": [{ "role": "user", "content": "hi" }],
        });
        compression_strip_protocol_fields(&mut body);
        assert!(body.get("michael_compression").is_none());
        assert!(body.get("mc_prefix").is_none());
        assert!(
            body.get("messages").is_some(),
            "只清协议字段，不动 messages"
        );
    }

    #[test]
    fn missing_messages_is_not_a_panic() {
        assert!(compression_plan_input(&serde_json::json!({})).1.is_empty());
        assert!(
            compression_plan_input(&serde_json::json!({ "messages": "nope" }))
                .1
                .is_empty()
        );
    }
}

#[cfg(test)]
mod upstream_timeout_tests {
    use super::*;

    /// The gateway must answer before the IDE stops waiting.
    ///
    /// When it didn't, the IDE hit its response-header timeout, fast-retried, and
    /// every retry opened a fresh gateway request with its own set of upstream calls —
    /// the user saw "已等待 47s；仍在等待有效输出" while `/v1/messages` requests kept
    /// piling up at the provider. Keep a real margin so a slow answer still beats the
    /// client's deadline.
    #[test]
    fn route_budget_fits_inside_the_client_header_timeout() {
        for deep in [false, true] {
            let budget = route_budget_for(deep);
            assert!(
                budget < CLIENT_HEADER_TIMEOUT,
                "route budget {:?} must be under the client's {:?} header timeout (deep={deep})",
                budget,
                CLIENT_HEADER_TIMEOUT
            );
            assert!(
                CLIENT_HEADER_TIMEOUT - budget >= Duration::from_secs(2),
                "leave >=2s of margin; budget {:?} vs client {:?} (deep={deep})",
                budget,
                CLIENT_HEADER_TIMEOUT
            );
        }
    }

    /// Thinking effort must never widen a broken transport's response-header window.
    #[test]
    /// Regression (2026-08-01): the deep-thinking budget must recognise EVERY wire shape
    /// that turns thinking on. It used to key off `budget_tokens > 0` alone, so when the
    /// gateway switched modern Claude to `{"type":"adaptive"}` — which has no budget field —
    /// thinking requests silently fell back to the standard 7s header / 180s idle budget and
    /// died as 504s. The bug was invisible: nothing errored, the deadline was just wrong.
    #[test]
    fn adaptive_thinking_still_counts_as_deep_thinking() {
        // The shape modern Claude requires — no budget_tokens anywhere.
        assert!(request_is_deep_thinking(
            &json!({"thinking": {"type": "adaptive"}})
        ));
        // Legacy explicit-budget shape (3.7 / 4.6) must keep working.
        assert!(request_is_deep_thinking(
            &json!({"thinking": {"type": "enabled", "budget_tokens": 12000}})
        ));
        // Deepest OpenAI-shaped dials.
        assert!(request_is_deep_thinking(&json!({"reasoning_effort": "max"})));
        assert!(request_is_deep_thinking(&json!({"reasoning_effort": "xhigh"})));

        // ...and a request with no thinking at all must NOT get the deep budget, or every
        // ordinary chat inherits a 600s idle window and a hung route stops looking hung.
        assert!(!request_is_deep_thinking(&json!({"messages": []})));
        assert!(!request_is_deep_thinking(&json!({"reasoning_effort": "low"})));
        assert!(!request_is_deep_thinking(
            &json!({"thinking": {"type": "disabled"}})
        ));
    }

    #[test]
    fn deep_thinking_uses_the_same_transport_budget() {
        assert_eq!(route_budget_for(true), route_budget_for(false));
    }

    /// A single stalled route must not be allowed to consume a whole budget on its own
    /// when the budget is the smaller of the two.
    #[test]
    fn per_attempt_header_wait_is_request_aware_and_capped() {
        // 7s 而不是 5s：实测小聊天最慢 3.9s，5s 只剩 1.1s 余量，一次偏慢的 prefill
        // 就会被当成传输故障掐掉。总时长仍由 ROUTE_BUDGET 兜住。
        assert_eq!(
            max_header_wait_for_request(false, false),
            Duration::from_secs(7)
        );
        assert_eq!(
            max_header_wait_for_request(false, true),
            Duration::from_secs(8)
        );
        assert_eq!(
            max_header_wait_for_request(true, true),
            Duration::from_secs(10)
        );
        assert!(DEEP_MAX_HEADER_WAIT < ROUTE_BUDGET);
    }

    #[test]
    fn absolute_client_deadline_caps_every_gateway_retry() {
        let now_ms = 1_000_000;
        assert_eq!(
            route_budget_with_client_deadline(false, Some(now_ms + 4_000), now_ms),
            Duration::from_millis(3_250),
        );
        assert_eq!(
            route_budget_with_client_deadline(true, Some(now_ms - 1), now_ms),
            Duration::ZERO,
            "an already-expired desktop request must not open an upstream call"
        );
        assert_eq!(
            route_budget_with_client_deadline(false, None, now_ms),
            ROUTE_BUDGET,
            "older/BYOK clients use the bounded gateway fallback"
        );
    }
}

#[cfg(test)]
mod audit_regression_tests {
    use super::*;
    use serde_json::json;

    /// A client must not be able to suppress the usage frame. Before the fix the
    /// gateway used `entry().or_insert_with()`, so `include_usage: false` survived,
    /// the upstream never reported usage, and `compute_cost` billed 0 — unlimited
    /// free inference for anyone with a valid key.
    fn apply_stream_options(body: &mut serde_json::Value) {
        if let Some(obj) = body.as_object_mut() {
            let opts = obj
                .entry("stream_options")
                .or_insert_with(|| serde_json::json!({}));
            if !opts.is_object() {
                *opts = serde_json::json!({});
            }
            if let Some(opts) = opts.as_object_mut() {
                opts.insert("include_usage".into(), serde_json::Value::Bool(true));
            }
        }
    }

    #[test]
    fn client_cannot_disable_include_usage() {
        let mut body = json!({"stream": true, "stream_options": {"include_usage": false}});
        apply_stream_options(&mut body);
        assert_eq!(body["stream_options"]["include_usage"], json!(true));
    }

    #[test]
    fn empty_or_bogus_stream_options_still_get_include_usage() {
        for given in [json!({}), json!("nope"), json!(7), json!(null)] {
            let mut body = json!({ "stream": true, "stream_options": given });
            apply_stream_options(&mut body);
            assert_eq!(body["stream_options"]["include_usage"], json!(true));
        }
    }

    #[test]
    fn unrelated_stream_options_keys_survive() {
        let mut body = json!({"stream": true, "stream_options": {"foo": "bar"}});
        apply_stream_options(&mut body);
        assert_eq!(body["stream_options"]["include_usage"], json!(true));
        assert_eq!(body["stream_options"]["foo"], json!("bar"));
    }

    /// I18N_PACK_CACHE is one process-global, and the test harness runs tests on
    /// parallel threads — so the flood test below can evict another test's entry
    /// between its put and its get. That was a real 1-in-~75 flake (caught twice in a
    /// 150-run hunt): `round_trips_a_fresh_entry` observed None for a key it had just
    /// inserted, because `is_bounded` was mid-flood on another thread. Every test that
    /// touches the cache takes this lock; `into_inner` on poison keeps one failing
    /// test from cascading into the others.
    static I18N_CACHE_TEST_LOCK: Mutex<()> = Mutex::new(());

    /// The pack cache holds ~630KB per entry and its key is a hash of the request, so
    /// a caller varying one character misses every time. Unbounded, that OOMs the
    /// gateway before the upstream bill even becomes the bigger problem.
    #[test]
    fn i18n_pack_cache_is_bounded() {
        let _serial = I18N_CACHE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        for i in 0..(I18N_PACK_CACHE_MAX_ENTRIES * 3) {
            i18n_pack_cache_put(format!("k{i}"), json!({ "n": i }));
        }
        let len = I18N_PACK_CACHE.lock().expect("cache").len();
        assert!(
            len <= I18N_PACK_CACHE_MAX_ENTRIES,
            "cache grew to {len}, cap is {I18N_PACK_CACHE_MAX_ENTRIES}"
        );
    }

    #[test]
    fn i18n_pack_cache_round_trips_a_fresh_entry() {
        let _serial = I18N_CACHE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        i18n_pack_cache_put("fresh-key".into(), json!({ "ok": true }));
        assert_eq!(
            i18n_pack_cache_get("fresh-key"),
            Some(json!({ "ok": true }))
        );
        assert_eq!(i18n_pack_cache_get("never-inserted"), None);
    }

    /// Cache misses are what cost money, so they are budgeted per user.
    #[test]
    fn i18n_pack_budget_stops_a_runaway_caller() {
        let uid = uuid::Uuid::new_v4();
        for _ in 0..I18N_PACK_BUDGET_PER_WINDOW {
            assert!(i18n_pack_charge_budget(uid).is_ok());
        }
        let err = i18n_pack_charge_budget(uid).expect_err("budget must stop the caller");
        assert_eq!(err.status, StatusCode::TOO_MANY_REQUESTS);
        // Budgets are per user, so one runaway client can't lock everyone else out.
        assert!(i18n_pack_charge_budget(uuid::Uuid::new_v4()).is_ok());
    }

    #[test]
    fn asset_generation_budget_is_per_user() {
        let uid = uuid::Uuid::new_v4();
        for _ in 0..ASSET_GEN_PER_WINDOW {
            assert!(asset_gen_charge_budget(uid).is_ok());
        }
        assert_eq!(
            asset_gen_charge_budget(uid)
                .expect_err("budget must stop the caller")
                .status,
            StatusCode::TOO_MANY_REQUESTS
        );
        assert!(asset_gen_charge_budget(uuid::Uuid::new_v4()).is_ok());
    }
}

// ============ michael-compression 接线层 ============
//
// 纯规划与缓存键逻辑在 `crate::compression`（无 I/O、可单测）；这里只负责它够不到的
// 东西：读请求、查 Redis、挑压缩模型、打上游、把结果写回 body。

/// 从请求里解析 michael-compression 档位。
///
/// 支持两种写法：请求头 `x-michael-compression: 2m`，或 body 里的 `michael_compression`
/// 字段（给不方便加头的 OpenAI 兼容客户端）。都没有就返回 None —— **不启用**。
fn compression_tier_from(
    headers: &HeaderMap,
    body: &serde_json::Value,
) -> Option<crate::compression::Tier> {
    if let Some(raw) = headers
        .get("x-michael-compression")
        .and_then(|v| v.to_str().ok())
    {
        return crate::compression::Tier::parse(raw);
    }
    body.get("michael_compression")
        .and_then(|v| v.as_str())
        .and_then(crate::compression::Tier::parse)
}

/// 把 OpenAI 形状的 messages 读成压缩层用的结构。
///
/// 只取纯文本内容：带图片等多模态块的消息按其文本部分参与规划。真正落入压缩区之前，
/// `compression_message_has_nontext_content` 会 fail-closed；否则整条原消息被摘要替换时，
/// 图片会悄悄消失，而 PrefixRecord 下一轮又会让客户端省略原消息，造成永久数据丢失。
/// 规划用的消息视图。
///
/// 返回 `(pinned, msgs)`：`pinned` 是开头那一串**必须逐字保留**的 system 消息条数，
/// `msgs` 是其后可参与压缩的部分（索引从 0 起，与 `pinned` 无关）。
///
/// 为什么要把开头的 system 钉住：`prompts::assemble_into` 会把服务端组装的 L0 系统
/// 提示词放在 messages[0]，而 `plan()` 的逐字尾部是**从末尾往前**取的 —— 压缩一旦
/// 触发，verbatim_from 必然 >= 1，系统提示词就落进被压前缀，被最便宜的模型写的约
/// 600 token 摘要替换掉。整套行为准则就这么没了。
fn compression_plan_input(body: &serde_json::Value) -> (usize, Vec<crate::compression::Msg>) {
    let Some(arr) = body.get("messages").and_then(|v| v.as_array()) else {
        return (0, Vec::new());
    };
    let role_of = |m: &serde_json::Value| {
        m.get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("user")
            .to_string()
    };
    let pinned = arr
        .iter()
        .take_while(|m| role_of(m) == "system")
        .count()
        // 全是 system 的极端情况下别把一切都钉住，否则永远压不动。
        .min(arr.len().saturating_sub(1));

    let msgs = arr[pinned..]
        .iter()
        .map(|m| crate::compression::Msg::new(role_of(m), compression_countable_text(m)))
        .collect();
    (pinned, msgs)
}

/// 规划时用来估算体积的文本。
///
/// 必须把 `tool_calls[].function.arguments` 也算进去：agent 模式下
/// write_file / multi_edit 的**整个文件内容都在 arguments 里**，而 `content` 是 null。
/// 只数 content 的话，最大的那些消息全被估成 0 token，规划器于是认为"没超窗口"、
/// 什么都不压 —— 压缩在最需要它的场景下恰好不工作。
fn compression_countable_text(m: &serde_json::Value) -> String {
    let mut out = oai_content_text(m.get("content"));
    if let Some(calls) = m.get("tool_calls").and_then(|v| v.as_array()) {
        for c in calls {
            if let Some(name) = c.pointer("/function/name").and_then(|v| v.as_str()) {
                out.push('\n');
                out.push_str(name);
            }
            if let Some(args) = c.pointer("/function/arguments").and_then(|v| v.as_str()) {
                out.push('\n');
                out.push_str(args);
            }
        }
    }
    out
}

/// 消息是否含不能被纯文本摘要忠实保存的内容块。
///
/// OpenAI/Anthropic 兼容线路会出现 `image_url`、`input_image`、音频或文件块。这里只放行
/// 明确的文本块；未知类型同样拒绝，避免供应商新增一种媒体类型后被我们静默吞掉。
fn compression_message_has_nontext_content(m: &serde_json::Value) -> bool {
    m.get("content")
        .and_then(|content| content.as_array())
        .is_some_and(|parts| {
            parts.iter().any(|part| {
                !matches!(
                    part.get("type").and_then(|value| value.as_str()),
                    Some("text" | "input_text")
                )
            })
        })
}

/// 挑一个用来做压缩的便宜模型：按官方单价升序取第一个可用连接。
///
/// 压缩是机械活，用旗舰模型压是纯烧钱——这正是客户端 `_pickCheapModel` 曾经犯的错。
async fn compression_pick_compressors(state: &AppState) -> Vec<(Model, String)> {
    let Ok(models) = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE active = true AND api_key <> '' ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await
    else {
        return Vec::new();
    };
    let mut ranked: Vec<(f64, Model, String)> = Vec::new();
    for m in models {
        for id in allowed_ids(&m) {
            let price = official_price(&id).map(|(i, o)| i + o).unwrap_or(f64::MAX);
            ranked.push((price, m.clone(), id));
        }
    }
    ranked.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // **每个上游连接只留最便宜的那一个模型**，然后按价格排出候选序列。
    //
    // 为什么要跨连接备选：实测踩到过一次 —— 最便宜的模型正好在一条挂掉的上游线路上，
    // 于是每一段摘要都在同一毫秒被拒绝，压缩整体降级为不压缩，而原始的 1.2MB 历史
    // 直接发给了目标模型并把它打成 504。压缩不能被单一供应商绑死。
    //
    // 同一连接内只留一个候选：同一条线路挂了，换它自己的另一个模型也是白搭。
    let mut seen_conn = std::collections::HashSet::new();
    ranked
        .into_iter()
        .filter(|(_, m, _)| seen_conn.insert(m.id))
        .map(|(_, m, id)| (m, id))
        .collect()
}

/// 压一个段。失败返回 None —— 调用方降级为「这段不压」，绝不让压缩失败拖垮聊天。
/// 一次段压缩的结果：摘要正文 + 上游报告的 usage（用于计费）。
struct CompressionCall {
    summary: String,
    usage: Option<serde_json::Value>,
}

async fn compression_summarize(
    conn: &Model,
    model_id: &str,
    text: &str,
) -> Option<CompressionCall> {
    let payload = json!({
        "model": model_id,
        "temperature": 0.1,
        "max_tokens": crate::compression::SEGMENT_SUMMARY_TOKENS,
        "messages": [
            { "role": "system", "content": crate::compression::segment_compress_prompt(crate::compression::SEGMENT_SUMMARY_TOKENS) },
            { "role": "user", "content": text },
        ],
    });
    let resp = GW_HTTP
        .post(format!("{}/chat/completions", api_base(&conn.base_url)))
        .header("Authorization", format!("Bearer {}", conn.api_key))
        .json(&payload)
        .timeout(Duration::from_secs(120))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let data: serde_json::Value = resp.json().await.ok()?;
    let out = data
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if out.is_empty() {
        return None;
    }
    Some(CompressionCall {
        summary: out,
        usage: data.get("usage").cloned(),
    })
}

#[derive(Default)]
struct CompressionPrefixContext {
    summaries: Vec<String>,
    summary_keys: Vec<String>,
    raw_keys: Vec<String>,
    search_indexes: Vec<crate::compression::SegmentSearchIndex>,
    covered_msgs: usize,
    raw_tokens: usize,
}

fn compression_prefix_invalid_error() -> AppError {
    AppError {
        status: StatusCode::CONFLICT,
        msg: "[mc-prefix-invalid] michael-compression 前缀已失效，请清除前缀并用完整历史重试"
            .into(),
    }
}

/// 取出并校验请求带来的前缀引用。
///
/// 返回 (摘要, 段键, 覆盖的消息数, 覆盖部分的原始 token 数)。没有引用返回 `Ok(None)`；
/// 请求明确带了引用但它不存在、越权、口径不匹配或有段过期时返回带机器标记的 409。
/// 客户端据此清掉本地引用并用完整 transcript 自动重试。**宁可多传一次，也不能静默丢掉
/// 一段历史**：那会让模型在请求正常计费的同时莫名其妙地失忆。
async fn compression_take_prefix(
    state: &AppState,
    body: &mut serde_json::Value,
    uid: uuid::Uuid,
) -> Result<Option<CompressionPrefixContext>, AppError> {
    use crate::compression as mc;

    let token = body
        .get("mc_prefix")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string);
    let Some(token) = token else {
        return Ok(None);
    };
    let claimed_covered = body
        .get("mc_prefix_covered")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);
    // 这是我们自己的协议字段，绝不能透传给上游。
    if let Some(obj) = body.as_object_mut() {
        obj.remove("mc_prefix");
        obj.remove("mc_prefix_covered");
    }

    let invalid = compression_prefix_invalid_error;
    let claimed_covered = claimed_covered
        .filter(|covered| *covered > 0)
        .ok_or_else(invalid)?;

    let mut redis = state.redis.clone();
    let cached: Option<String> = redis::cmd("GET")
        .arg(mc::prefix_redis_key(&token))
        .query_async(&mut redis)
        .await
        .ok()
        .flatten();
    let raw = match cached {
        Some(raw) => raw,
        None => {
            let record_json = sqlx::query_scalar::<_, serde_json::Value>(
                "SELECT record
                 FROM michael_context_prefixes
                 WHERE token = $1 AND user_id = $2 AND expires_at > now()",
            )
            .bind(&token)
            .bind(uid)
            .fetch_optional(&state.db)
            .await
            .map_err(|error| AppError {
                status: StatusCode::SERVICE_UNAVAILABLE,
                msg: format!("michael-compression: 读取持久上下文前缀失败: {error}"),
            })?
            .ok_or_else(invalid)?;
            let raw = serde_json::to_string(&record_json).map_err(|_| invalid())?;
            let _: Result<(), redis::RedisError> = redis::cmd("SET")
                .arg(mc::prefix_redis_key(&token))
                .arg(&raw)
                .arg("EX")
                .arg(mc::PREFIX_TTL_SECS)
                .query_async(&mut redis)
                .await;
            raw
        }
    };
    let record: mc::PrefixRecord = serde_json::from_str(&raw).map_err(|_| invalid())?;

    if !mc::prefix_belongs_to(&record, &uid.to_string()) {
        tracing::warn!(%uid, "michael-compression: 前缀引用不属于该用户，已拒绝");
        return Err(invalid());
    }
    if claimed_covered != record.covered_msgs {
        tracing::warn!(
            %uid,
            claimed_covered,
            record_covered = record.covered_msgs,
            "michael-compression: 客户端前缀覆盖条数不匹配，已拒绝该引用"
        );
        return Err(invalid());
    }

    if record.raw_segment_keys.len() != record.segment_keys.len()
        || record.raw_segment_keys.is_empty()
    {
        tracing::info!(
            %uid,
            summaries = record.segment_keys.len(),
            raw_archives = record.raw_segment_keys.len(),
            "michael-compression: 旧版或不完整前缀缺少无损原文归档，要求客户端重建"
        );
        return Err(invalid());
    }

    let (summaries, search_indexes) = compression_load_prefix_segments(
        state,
        uid,
        &record.segment_keys,
        &record.raw_segment_keys,
    )
    .await
    .ok_or_else(|| {
        tracing::info!(
            %uid,
            "michael-compression: 持久上下文段不存在或已损坏，要求客户端重发完整历史"
        );
        invalid()
    })?;
    // 活跃会话滑动续期。前缀和组成它的摘要必须一起续，否则其中任一先过期都会形成一个
    // 看似有效、实际有缺口的引用。EXPIRE 失败不影响本轮已经读到的完整数据。
    mc::renew_context_cache(
        &mut redis,
        &token,
        &record.segment_keys,
        &record.raw_segment_keys,
    )
    .await;
    let _ = sqlx::query(
        "UPDATE michael_context_prefixes
         SET expires_at = now() + interval '90 days', updated_at = now()
         WHERE token = $1 AND user_id = $2",
    )
    .bind(&token)
    .bind(uid)
    .execute(&state.db)
    .await;
    Ok(Some(CompressionPrefixContext {
        summaries,
        summary_keys: record.segment_keys,
        raw_keys: record.raw_segment_keys,
        search_indexes,
        covered_msgs: record.covered_msgs,
        raw_tokens: record.raw_tokens,
    }))
}

/// 把压缩结果写回 body。
///
/// 只做**拼接**，不重建消息：钉住的 system 原样保留 → 摘要作为一条新的 system 注入
/// → 逐字尾部直接克隆**原始 JSON 对象**。
///
/// 上一版是拿 `Msg` 重建 `{role, content}`，把 `tool_calls`、`tool_call_id`、`name`、
/// 图片块全部丢掉。而 agent 模式发的正是这些：write_back 之后数组里会出现
/// `{"role":"tool","content":"..."}` 这种没有 tool_call_id 的消息，上游直接拒收。
/// `Msg` 只能用来规划，绝不能用来生成线路内容。
///
/// `verbatim_from` 是**相对于 pinned 之后那段**的索引，与 `compression_plan_input`
/// 的返回值口径一致。
fn compression_write_back(
    body: &mut serde_json::Value,
    pinned: usize,
    verbatim_from: usize,
    summaries: &[String],
    retrieved_history: Option<&str>,
) {
    let Some(arr) = body.get("messages").and_then(|v| v.as_array()) else {
        return;
    };
    let mut out: Vec<serde_json::Value> = Vec::with_capacity(arr.len() + 1);
    out.extend(arr.iter().take(pinned).cloned());
    if let Some(text) = crate::compression::summary_system_text(summaries) {
        out.push(json!({ "role": "system", "content": text }));
    }
    if let Some(text) = retrieved_history.filter(|text| !text.trim().is_empty()) {
        out.push(json!({ "role": "system", "content": text }));
    }
    let tail_start = pinned.saturating_add(verbatim_from).min(arr.len());
    out.extend(arr[tail_start..].iter().cloned());
    if let Some(slot) = body.get_mut("messages") {
        *slot = serde_json::Value::Array(out);
    }
}

/// 清掉我们自己的协议字段。**必须在任何 early return 之前调用**。
///
/// 上一版只在 `compression_write_back` 里清，而那个函数在每一条提前返回的路径上都
/// 不会被执行 —— 包括最常见的"没超窗口、不压缩"。于是用 body 字段开启压缩的请求，
/// 每一次不压缩时都把 `michael_compression` 原样透传给了上游供应商。
fn compression_strip_protocol_fields(body: &mut serde_json::Value) {
    if let Some(obj) = body.as_object_mut() {
        obj.remove("michael_compression");
        obj.remove("mc_prefix");
        obj.remove("mc_prefix_covered");
    }
}

/// 签发一个前缀引用，供客户端下一轮续传。
async fn compression_issue_prefix(
    state: &AppState,
    uid: uuid::Uuid,
    segment_keys: Vec<String>,
    raw_segment_keys: Vec<String>,
    covered_msgs: usize,
    raw_tokens: usize,
) -> Option<String> {
    use crate::compression as mc;
    if segment_keys.is_empty() || segment_keys.len() != raw_segment_keys.len() {
        return None;
    }
    let record = mc::PrefixRecord {
        uid: uid.to_string(),
        segment_keys,
        raw_segment_keys,
        covered_msgs,
        raw_tokens,
    };
    let token = mc::new_prefix_token();
    let record_json = serde_json::to_value(&record).ok()?;
    let payload = serde_json::to_string(&record).ok()?;
    let stored = sqlx::query(
        "INSERT INTO michael_context_prefixes (token, user_id, record, expires_at)
         VALUES ($1, $2, $3, now() + interval '90 days')",
    )
    .bind(&token)
    .bind(uid)
    .bind(record_json)
    .execute(&state.db)
    .await
    .ok()?;
    if stored.rows_affected() != 1 {
        return None;
    }
    let mut redis = state.redis.clone();
    let _: Result<(), redis::RedisError> = redis::cmd("SET")
        .arg(mc::prefix_redis_key(&token))
        .arg(payload)
        .arg("EX")
        .arg(mc::PREFIX_TTL_SECS)
        .query_async(&mut redis)
        .await;
    // Opportunistic bounded cleanup keeps abandoned per-turn handles from accumulating forever.
    let _ = sqlx::query(
        "DELETE FROM michael_context_prefixes
         WHERE token IN (
             SELECT token FROM michael_context_prefixes
             WHERE expires_at <= now()
             ORDER BY expires_at
             LIMIT 500
         )",
    )
    .execute(&state.db)
    .await;
    let _ = sqlx::query(
        "DELETE FROM michael_context_archives
         WHERE (user_id, archive_key) IN (
             SELECT user_id, archive_key FROM michael_context_archives
             WHERE last_accessed_at <= now() - interval '90 days'
             ORDER BY last_accessed_at
             LIMIT 500
         )",
    )
    .execute(&state.db)
    .await;
    Some(token)
}

fn compression_fixed_overhead_tokens(body: &serde_json::Value, pinned: usize) -> usize {
    let pinned_tokens = body
        .get("messages")
        .and_then(|messages| messages.as_array())
        .map(|messages| {
            messages
                .iter()
                .take(pinned)
                .map(|message| {
                    crate::compression::estimate_tokens(&compression_countable_text(message))
                })
                .sum::<usize>()
        })
        .unwrap_or(0);
    let tools_tokens = body
        .get("tools")
        .and_then(|tools| serde_json::to_string(tools).ok())
        .map(|tools| crate::compression::estimate_tokens(&tools))
        .unwrap_or(0);
    // Roles, JSON framing and provider-specific wrappers still consume tokens. The main window
    // safety factor is the broad guard; this fixed reserve prevents a large tool catalog from
    // stealing the exact-retrieval slot unnoticed.
    pinned_tokens
        .saturating_add(tools_tokens)
        .saturating_add(2_048)
}

fn compression_retrieval_query(msgs: &[crate::compression::Msg]) -> String {
    let mut context_parts = Vec::new();
    let latest_user_index = msgs.iter().rposition(|message| message.role == "user");
    let recent_from = msgs.len().saturating_sub(4);
    for (index, message) in msgs.iter().enumerate().skip(recent_from) {
        if message.role != "system" && Some(index) != latest_user_index {
            context_parts.push(message.text.as_str());
        }
    }
    let latest_user = latest_user_index
        .map(|index| msgs[index].text.as_str())
        .unwrap_or("");
    let latest_user_tail = latest_user
        .chars()
        .rev()
        .take(12_000)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    let context_tail = context_parts
        .join("\n")
        .chars()
        .rev()
        .take(4_000)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    if context_tail.is_empty() {
        latest_user_tail
    } else if latest_user_tail.is_empty() {
        context_tail
    } else {
        format!("{latest_user_tail}\n{context_tail}")
    }
}

fn compression_archive_segment(
    body: &serde_json::Value,
    pinned: usize,
    msgs: &[crate::compression::Msg],
    segment: &crate::compression::Segment,
) -> Option<(
    String,
    crate::compression::RawSegmentArchive,
    crate::compression::SegmentSearchIndex,
)> {
    use crate::compression as mc;
    let original = body
        .get("messages")?
        .as_array()?
        .get(pinned + segment.start..pinned + segment.end)?
        .to_vec();
    let planned = msgs.get(segment.start..segment.end)?;
    if original.len() != planned.len() || original.is_empty() {
        return None;
    }
    let messages = original
        .iter()
        .cloned()
        .zip(planned.iter())
        .map(|(original, message)| mc::ArchivedMessage {
            role: message.role.clone(),
            text: message.text.clone(),
            tokens: message.tokens,
            original,
        })
        .collect::<Vec<_>>();
    let archive = mc::RawSegmentArchive {
        version: mc::RawSegmentArchive::VERSION,
        messages,
    };
    let index = mc::build_search_index(&archive.messages);
    let key = mc::raw_segment_cache_key(&original);
    Some((key, archive, index))
}

async fn compression_persist_archives(
    state: &AppState,
    uid: uuid::Uuid,
    archives: &[(
        String,
        crate::compression::RawSegmentArchive,
        crate::compression::SegmentSearchIndex,
    )],
    summaries: &[String],
) -> bool {
    use crate::compression as mc;
    if archives.is_empty() {
        return true;
    }
    if archives.len() != summaries.len() {
        return false;
    }
    let mut rows = Vec::with_capacity(archives.len());
    for ((key, archive, index), summary) in archives.iter().zip(summaries) {
        let Some(payload) = mc::encode_raw_archive(archive) else {
            return false;
        };
        let Ok(search_index) = serde_json::to_value(index) else {
            return false;
        };
        let raw_tokens = archive
            .messages
            .iter()
            .map(|message| message.tokens)
            .sum::<usize>()
            .min(i64::MAX as usize) as i64;
        rows.push((
            key.clone(),
            payload,
            search_index,
            summary.clone(),
            raw_tokens,
        ));
    }

    let Ok(mut tx) = state.db.begin().await else {
        return false;
    };
    for (key, payload, search_index, summary, raw_tokens) in &rows {
        let result = sqlx::query(
            "INSERT INTO michael_context_archives
                (user_id, archive_key, payload, search_index, summary, raw_tokens)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (user_id, archive_key) DO UPDATE SET
                payload = EXCLUDED.payload,
                search_index = EXCLUDED.search_index,
                summary = EXCLUDED.summary,
                raw_tokens = EXCLUDED.raw_tokens,
                last_accessed_at = now()",
        )
        .bind(uid)
        .bind(key)
        .bind(payload)
        .bind(search_index)
        .bind(summary)
        .bind(raw_tokens)
        .execute(&mut *tx)
        .await;
        if result.is_err() {
            let _ = tx.rollback().await;
            return false;
        }
    }
    if tx.commit().await.is_err() {
        return false;
    }

    // PostgreSQL is the source of truth. Redis is a best-effort hot cache, so a cache write
    // failure must not make a durable archive unusable or prevent prefix issuance.
    let mut redis = state.redis.clone();
    for (key, archive, index) in archives {
        let _ = mc::store_raw_archive(&mut redis, key, archive, index).await;
    }
    true
}

async fn compression_load_prefix_segments(
    state: &AppState,
    uid: uuid::Uuid,
    summary_keys: &[String],
    raw_keys: &[String],
) -> Option<(Vec<String>, Vec<crate::compression::SegmentSearchIndex>)> {
    use crate::compression as mc;
    if summary_keys.len() != raw_keys.len() {
        return None;
    }
    let mut redis = state.redis.clone();
    let mut summaries = mc::cached_summaries(&mut redis, summary_keys).await;
    let mut indexes = mc::cached_search_indexes(&mut redis, raw_keys).await;
    let mut missing = Vec::new();
    for (position, raw_key) in raw_keys.iter().enumerate() {
        if summaries[position].is_none() || indexes[position].is_none() {
            missing.push(raw_key.clone());
        }
    }

    if !missing.is_empty() {
        let rows = sqlx::query_as::<_, (String, serde_json::Value, String)>(
            "SELECT archive_key, search_index, summary
             FROM michael_context_archives
             WHERE user_id = $1 AND archive_key = ANY($2)",
        )
        .bind(uid)
        .bind(&missing)
        .fetch_all(&state.db)
        .await
        .ok()?;
        let from_db = rows
            .into_iter()
            .map(|(key, index, summary)| (key, (index, summary)))
            .collect::<HashMap<_, _>>();
        for (position, raw_key) in raw_keys.iter().enumerate() {
            if summaries[position].is_some() && indexes[position].is_some() {
                continue;
            }
            let (value, summary) = from_db.get(raw_key)?;
            if summaries[position].is_none() {
                mc::store_summary(&mut redis, &summary_keys[position], summary).await;
                summaries[position] = Some(summary.clone());
            }
            if indexes[position].is_none() {
                let index: mc::SegmentSearchIndex = serde_json::from_value(value.clone()).ok()?;
                if !index.is_valid() {
                    return None;
                }
                mc::store_search_index(&mut redis, raw_key, &index).await;
                indexes[position] = Some(index);
            }
        }
    }

    let _ = sqlx::query(
        "UPDATE michael_context_archives
         SET last_accessed_at = now()
         WHERE user_id = $1 AND archive_key = ANY($2)",
    )
    .bind(uid)
    .bind(raw_keys)
    .execute(&state.db)
    .await;
    Some((
        summaries.into_iter().collect::<Option<Vec<_>>>()?,
        indexes.into_iter().collect::<Option<Vec<_>>>()?,
    ))
}

async fn compression_load_raw_archive(
    state: &AppState,
    uid: uuid::Uuid,
    raw_key: &str,
) -> Option<crate::compression::RawSegmentArchive> {
    use crate::compression as mc;
    let mut redis = state.redis.clone();
    if let Some(archive) = mc::cached_raw_archive(&mut redis, raw_key).await {
        return Some(archive);
    }
    let (payload, search_index) = sqlx::query_as::<_, (Vec<u8>, serde_json::Value)>(
        "SELECT payload, search_index
         FROM michael_context_archives
         WHERE user_id = $1 AND archive_key = $2",
    )
    .bind(uid)
    .bind(raw_key)
    .fetch_optional(&state.db)
    .await
    .ok()??;
    let archive = mc::decode_raw_archive(&payload)?;
    let index: mc::SegmentSearchIndex = serde_json::from_value(search_index).ok()?;
    if !index.is_valid() {
        return None;
    }
    let _ = mc::store_raw_archive(&mut redis, raw_key, &archive, &index).await;
    let _ = sqlx::query(
        "UPDATE michael_context_archives
         SET last_accessed_at = now()
         WHERE user_id = $1 AND archive_key = $2",
    )
    .bind(uid)
    .bind(raw_key)
    .execute(&state.db)
    .await;
    Some(archive)
}

struct RetrievedCompressionHistory {
    text: Option<String>,
    tokens: usize,
    segment_count: usize,
    excerpt_count: usize,
}

struct CompressionRetrievalRequest<'a> {
    query: &'a str,
    summaries: &'a [String],
    indexes: &'a [crate::compression::SegmentSearchIndex],
    raw_keys: &'a [String],
    in_memory: &'a HashMap<usize, crate::compression::RawSegmentArchive>,
    budget_tokens: usize,
}

async fn compression_retrieve_history(
    state: &AppState,
    uid: uuid::Uuid,
    request: CompressionRetrievalRequest<'_>,
) -> Result<RetrievedCompressionHistory, AppError> {
    use crate::compression as mc;
    if request.query.trim().is_empty()
        || request.budget_tokens < 256
        || request.summaries.len() != request.indexes.len()
        || request.summaries.len() != request.raw_keys.len()
    {
        return Ok(RetrievedCompressionHistory {
            text: None,
            tokens: 0,
            segment_count: 0,
            excerpt_count: 0,
        });
    }
    let selected =
        mc::rank_retrieval_segments(request.query, request.summaries, request.indexes, 6);
    if selected.is_empty() {
        return Ok(RetrievedCompressionHistory {
            text: None,
            tokens: 0,
            segment_count: 0,
            excerpt_count: 0,
        });
    }
    let mut archives = Vec::with_capacity(selected.len());
    for index in selected {
        let archive = match request.in_memory.get(&index) {
            Some(archive) => archive.clone(),
            None => compression_load_raw_archive(state, uid, &request.raw_keys[index])
                .await
                .ok_or_else(compression_prefix_invalid_error)?,
        };
        archives.push((index, archive));
    }
    let excerpts = mc::select_retrieval_excerpts(request.query, &archives, request.budget_tokens);
    let text = mc::retrieval_system_text(&excerpts);
    let tokens = text.as_deref().map(mc::estimate_tokens).unwrap_or_default();
    Ok(RetrievedCompressionHistory {
        text,
        tokens,
        segment_count: archives.len(),
        excerpt_count: excerpts.len(),
    })
}

/// 就地把 body.messages 换成压缩后的序列。
///
/// 全程 best-effort：任何一步失败都保持 body 原样（这一轮上下文短一点，但聊天照常可用）。
///
/// 返回本轮签发的新前缀引用（若有），供响应头回传给客户端做下一轮续传。
/// 内联路径**只查缓存**，绝不在请求链路上现算摘要。
///
/// 这是实测逼出来的结论，不是保守设计。同一个 20k 段在同一家供应商上：一次 5.1s、
/// 一次 39s、一次 7.0s；另一家在 6KB 和 20KB 上返回瞬时 503，却在 61KB 上成功 ——
/// 也就是延迟和成功率都不可预测。而客户端等响应头只等 15s。任何"在请求里现算"的
/// 预算都是错的：设小了段全失败并降级为不压缩（原始历史直接怼给目标模型，反而把
/// 本来可能成功的请求变成必然 504），设大了客户端先放弃、重试，每次重试再触发一轮
/// 同样的压缩。
///
/// 所以：请求里只用已经算好的摘要（Redis 查询，毫秒级、延迟确定）；缺的段交给**后台**
/// 预热，下一轮就能命中。代价是第一次长对话那一轮不压缩，换来的是延迟可预测。
const COMPRESSION_WARM_SEGMENT_TIMEOUT: Duration = Duration::from_secs(90);
/// 同时预热的段数。并发过小会让 5M 冷启动等待数分钟；过大又会瞬间打满便宜线路限流。
const COMPRESSION_WARM_CONCURRENCY: usize = 6;
/// 后台预热一轮最多现算多少段。
///
/// 8 太小：实测一个 400k token 的对话有 17 段，一轮只预热 8 段的话要三轮才能压到窗口
/// 以内 —— 这三轮里每一轮都在降级为不压缩，也就是"5M 档看着开了却一直不生效"。
/// 后台没有延迟压力，上限的唯一意义是别把便宜模型的限流打满，所以给到能一轮覆盖
/// 常见长对话的量级。真正的兜底是"一段都压不出来就整体放弃"那条。
const COMPRESSION_WARM_MAX_SEGMENTS: usize = 128;
/// 段摘要缓存的命名空间。刻意与具体压缩模型无关（见调用点注释）。
const COMPRESSION_CACHE_NAMESPACE: &str = "mc-any-v1";
/// 还没撞窗口也提前准备摘要/前缀。这样 1M 原生窗口的模型不会等请求体逼近 3.5MB 才启动。
const COMPRESSION_PREFIX_TRIGGER_MAX_TOKENS: usize = 400_000;

async fn apply_michael_compression(
    state: &AppState,
    body: &mut serde_json::Value,
    model_id: &str,
    tier: crate::compression::Tier,
    uid: uuid::Uuid,
) -> Result<Option<(String, usize)>, AppError> {
    use crate::compression as mc;

    let started = std::time::Instant::now();

    // 前缀续传：客户端只发了未覆盖的消息，历史摘要从 Redis 取回。
    let carried = compression_take_prefix(state, body, uid)
        .await?
        .unwrap_or_default();

    // 前缀一旦被取用，客户端手上就只有"未覆盖"的那截消息了。此后任何一条返回路径都
    // **必须**把摘要拼回去，否则请求会带着一段被静默截断的对话发往上游 —— 而且照常
    // 计费。这个闭包就是那条唯一的退出通道。
    let (pinned, msgs) = compression_plan_input(body);
    if msgs.is_empty() {
        if !carried.summaries.is_empty() {
            compression_write_back(body, pinned, 0, &carried.summaries, None);
        }
        return Ok(None);
    }

    let native = official_context(model_id).unwrap_or(128_000).max(1) as usize;
    let window_budget = mc::window_budget(native);
    let fixed_overhead = compression_fixed_overhead_tokens(body, pinned);
    let budget = window_budget.saturating_sub(fixed_overhead);
    if budget <= mc::VERBATIM_TAIL_TOKENS + mc::RETRIEVAL_BUDGET_MIN_TOKENS {
        return Err(AppError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            msg: "michael-compression: 系统提示词和工具 schema 已占满目标模型窗口".into(),
        });
    }
    let retrieval_reserve = mc::retrieval_budget(native).min(budget / 3);
    let carried_budget = mc::actual_summary_tokens(&carried.summaries);
    let remaining_budget = budget.saturating_sub(carried_budget);
    let segment_tokens = mc::segment_tokens_for_budget(tier, budget, retrieval_reserve);
    let mut plan = mc::plan_to_budget(
        &msgs,
        remaining_budget,
        mc::VERBATIM_TAIL_TOKENS,
        segment_tokens,
    );

    // 提前切出旧段：即使原文暂时还塞得进窗口，也要在请求体逼近 3.5MB 前完成预热并签发
    // 前缀。普通增长型会话因此不会在跨过原生窗口的那一轮突然冷启动。
    let prefix_trigger = ((budget * 2) / 3)
        .min(COMPRESSION_PREFIX_TRIGGER_MAX_TOKENS)
        .max(mc::VERBATIM_TAIL_TOKENS + segment_tokens);
    if carried.summaries.is_empty() && plan.compress.is_empty() && plan.raw_tokens >= prefix_trigger
    {
        plan = mc::plan_for_prefix(&msgs, mc::VERBATIM_TAIL_TOKENS, segment_tokens);
    }

    // 压缩器目前只读文本。一旦把含图片/音频的整条原消息换成文本摘要，下一轮前缀续传
    // 又会让客户端彻底省略它，媒体就永久丢了。先明确拒绝，不能以“请求成功”为代价失忆。
    let compress_through = plan.compress.last().map(|segment| segment.end).unwrap_or(0);
    if compress_through > 0
        && body
            .get("messages")
            .and_then(|messages| messages.as_array())
            .is_some_and(|messages| {
                messages
                    .iter()
                    .skip(pinned)
                    .take(compress_through)
                    .any(compression_message_has_nontext_content)
            })
    {
        return Err(AppError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            msg: "[mc-nontext-history] michael-compression 暂不能压缩包含图片、音频或文件块的早期消息；请保留该媒体在近期原文或开启新会话"
                .into(),
        });
    }

    let total_raw = carried.raw_tokens + plan.raw_tokens;
    if total_raw > tier.max_input_tokens() {
        return Err(AppError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            msg: format!(
                "michael-compression: {} 档最多接受 {} token，当前累计约 {} token",
                tier.as_str(),
                tier.max_input_tokens(),
                total_raw
            ),
        });
    }

    if plan.compress.is_empty() {
        // 没有新段要压。带了前缀就必须把摘要拼回去，否则这一轮历史凭空消失。
        if carried.summaries.is_empty() {
            return Ok(None); // 没超窗口，一分钱不花，body 未被改动
        }
        let base_projected = carried_budget + plan.raw_tokens;
        if base_projected > budget {
            return Err(AppError {
                status: StatusCode::PAYLOAD_TOO_LARGE,
                msg:
                    "michael-compression: 最新单条消息超过目标模型可用窗口，无法通过压缩旧历史解决"
                        .into(),
            });
        }
        let query = compression_retrieval_query(&msgs);
        let retrieved = compression_retrieve_history(
            state,
            uid,
            CompressionRetrievalRequest {
                query: &query,
                summaries: &carried.summaries,
                indexes: &carried.search_indexes,
                raw_keys: &carried.raw_keys,
                in_memory: &HashMap::new(),
                budget_tokens: retrieval_reserve.min(budget.saturating_sub(base_projected)),
            },
        )
        .await?;
        compression_write_back(
            body,
            pinned,
            0,
            &carried.summaries,
            retrieved.text.as_deref(),
        );
        tracing::info!(
            %uid,
            model = %model_id,
            tier = tier.as_str(),
            fixed_overhead,
            base_projected,
            retrieval_tokens = retrieved.tokens,
            retrieval_segments = retrieved.segment_count,
            retrieval_excerpts = retrieved.excerpt_count,
            "michael-compression reused prefix with exact-history retrieval"
        );
        return Ok(None); // 前缀没变长，沿用客户端手上那个引用
    }

    let mut redis = state.redis.clone();
    let mut summaries: Vec<String> = carried.summaries.clone();
    let mut new_keys: Vec<String> = Vec::with_capacity(plan.compress.len());
    let mut cached = 0usize;
    let mut pending: Vec<(String, String)> = Vec::new();
    let mut planned_archives = Vec::with_capacity(plan.compress.len());

    // 只查缓存。命中的段不花钱、延迟确定；没命中的交给后台预热。
    //
    // 缓存命中必须是**前缀连续**的：段摘要按顺序拼成历史，中间缺一段就等于历史错位。
    // 所以第一个未命中之后就停止采用（即使后面的段碰巧有缓存），但仍然把它们都记进
    // pending 交给后台，下一轮才能连成一片。
    let mut broke = false;
    for seg in plan.compress.iter() {
        let text = mc::segment_text(&msgs, seg);
        let archive =
            compression_archive_segment(body, pinned, &msgs, seg).ok_or_else(|| AppError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                msg: "michael-compression: 无法建立无损历史归档".into(),
            })?;
        planned_archives.push(archive);
        // 缓存键**不绑定压缩模型**：后台预热会在供应商之间备选，算这一段的可能不是
        // 内联时会挑中的那家。键一旦绑定模型，内联查询就永远错过后台刚写好的结果，
        // 缓存被切成碎片、每轮都当冷启动。摘要是文本的语义产物，不是模型的产物。
        let key = mc::segment_cache_key(
            &text,
            COMPRESSION_CACHE_NAMESPACE,
            mc::SEGMENT_SUMMARY_TOKENS,
        );
        match mc::cached_summary(&mut redis, &key).await {
            Some(hit) if !broke => {
                cached += 1;
                summaries.push(hit);
                new_keys.push(key);
            }
            Some(_) => {}
            None => {
                broke = true;
                pending.push((key, text));
            }
        }
    }

    // 缺的段不在请求链路上现算，交给后台预热；本轮就用手上已有的缓存。
    if !pending.is_empty() {
        compression_spawn_warm(state, uid, pending.clone());
    }

    let actually_compressed = new_keys.len();
    if actually_compressed == 0 {
        let raw_projected = carried_budget + plan.raw_tokens;
        if raw_projected <= budget {
            // 提前预热阶段：原文仍安全，当前请求不必等待后台摘要。
            if !carried.summaries.is_empty() {
                let query = compression_retrieval_query(&msgs);
                let retrieved = compression_retrieve_history(
                    state,
                    uid,
                    CompressionRetrievalRequest {
                        query: &query,
                        summaries: &carried.summaries,
                        indexes: &carried.search_indexes,
                        raw_keys: &carried.raw_keys,
                        in_memory: &HashMap::new(),
                        budget_tokens: retrieval_reserve.min(budget.saturating_sub(raw_projected)),
                    },
                )
                .await?;
                compression_write_back(
                    body,
                    pinned,
                    0,
                    &carried.summaries,
                    retrieved.text.as_deref(),
                );
            }
            return Ok(None);
        }
        // 真正超窗时绝不能再把原文直接送上游。503 会被桌面端现有的、可取消的预流
        // 无限重试接住；每次重试只查 Redis，后台完成后立即进入正常模型请求。
        return Err(AppError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            msg: format!(
                "michael-compression warming: 正在准备 {} 个上下文段，请保持本轮运行",
                pending.len()
            ),
        });
    }
    let verbatim_from = plan
        .compress
        .get(actually_compressed.saturating_sub(1))
        .map(|s| s.end)
        .unwrap_or(0);

    // 摘要命中只代表“可以压缩”，不代表“可以丢掉原文”。在签发覆盖前缀之前，先把每个
    // 被覆盖段的完整 JSON、逐字文本和检索索引持久化。任何一个 SET 失败都不签发新前缀；
    // 客户端下轮仍会从旧边界重发这段，数据不会静默消失。
    let accepted_archives = planned_archives
        .into_iter()
        .take(actually_compressed)
        .collect::<Vec<_>>();
    let mut new_raw_keys = Vec::with_capacity(accepted_archives.len());
    let mut new_indexes = Vec::with_capacity(accepted_archives.len());
    let mut in_memory_archives = HashMap::new();
    let carried_count = carried.raw_keys.len();
    let new_summaries = &summaries[carried.summaries.len()..];
    let raw_storage_complete =
        compression_persist_archives(state, uid, &accepted_archives, new_summaries).await;
    for (offset, (raw_key, archive, index)) in accepted_archives.into_iter().enumerate() {
        in_memory_archives.insert(carried_count + offset, archive);
        new_raw_keys.push(raw_key);
        new_indexes.push(index);
    }

    let mut all_raw_keys = carried.raw_keys.clone();
    all_raw_keys.extend(new_raw_keys);
    let mut all_indexes = carried.search_indexes.clone();
    all_indexes.extend(new_indexes);

    // **校验结果真的塞得进窗口。** 此前没有任何环节做这件事：撞到上限就 break、剩下的
    // 段留在原文里，于是第一轮压缩照样发出一个远超窗口的请求 —— 而客户端因为看到档位
    // 已经关掉了自己的裁剪，没有兜底。
    let tail_tokens: usize = msgs[verbatim_from..]
        .iter()
        .map(|message| message.tokens)
        .sum();
    let base_projected = mc::actual_summary_tokens(&summaries).saturating_add(tail_tokens);
    if base_projected > budget {
        tracing::warn!(
            model = %model_id, tier = tier.as_str(), base_projected, budget,
            elapsed_ms = started.elapsed().as_millis(),
            "michael-compression: 连续缓存段尚不足以装入窗口，等待后台预热"
        );
        return Err(AppError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            msg: format!(
                "michael-compression warming: 已有 {} 段，仍在准备后续 {} 段",
                actually_compressed,
                pending.len()
            ),
        });
    }

    let query = compression_retrieval_query(&msgs);
    let retrieved = compression_retrieve_history(
        state,
        uid,
        CompressionRetrievalRequest {
            query: &query,
            summaries: &summaries,
            indexes: &all_indexes,
            raw_keys: &all_raw_keys,
            in_memory: &in_memory_archives,
            budget_tokens: retrieval_reserve.min(budget.saturating_sub(base_projected)),
        },
    )
    .await?;
    let projected = base_projected.saturating_add(retrieved.tokens);
    if projected > budget {
        return Err(AppError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            msg: "michael-compression warming: 精确历史回注仍在重新规划窗口".into(),
        });
    }

    compression_write_back(
        body,
        pinned,
        verbatim_from,
        &summaries,
        retrieved.text.as_deref(),
    );

    let mut all_keys = carried.summary_keys.clone();
    all_keys.extend(new_keys);
    let covered = carried.covered_msgs + verbatim_from;
    // PrefixRecord.raw_tokens 的口径是“已覆盖部分”，不能把仍在逐字尾部、下一轮还会重发的
    // token 记进去，否则每轮都会重复累计同一段尾部。
    let newly_covered_raw: usize = msgs[..verbatim_from].iter().map(|m| m.tokens).sum();
    let covered_raw = carried.raw_tokens + newly_covered_raw;
    let issued = if raw_storage_complete {
        compression_issue_prefix(state, uid, all_keys, all_raw_keys, covered, covered_raw)
            .await
            .map(|tok| (tok, covered))
    } else {
        tracing::warn!(
            %uid,
            "michael-compression: 无损原文归档写入不完整，本轮不签发扩展前缀"
        );
        None
    };

    tracing::info!(
        %uid, model = %model_id, tier = tier.as_str(),
        carried_msgs = carried.covered_msgs, carried_raw = carried.raw_tokens,
        raw_tokens = plan.raw_tokens, total_raw,
        pinned, fixed_overhead, verbatim_from, base_projected, projected, budget,
        retrieval_tokens = retrieved.tokens,
        retrieval_segments = retrieved.segment_count,
        retrieval_excerpts = retrieved.excerpt_count,
        segments_cached = cached, segments_pending = pending.len(),
        raw_storage_complete,
        elapsed_ms = started.elapsed().as_millis(),
        issued_prefix = issued.is_some(),
        "michael-compression applied"
    );
    Ok(issued)
}

/// 后台预热：把缺的段算出来写进缓存，下一轮请求就能命中。
///
/// 为什么必须在后台：实测同一个 20k 段在同一家供应商上一次 5.1s、一次 39s、一次 7.0s，
/// 另一家在 6KB 和 20KB 上返回瞬时 503 却在 61KB 上成功 —— 延迟和成功率都不可预测，
/// 而客户端等响应头只等 15s。放在请求链路里，无论预算设多少都是错的。
///
/// 这里可以从容：90s 单段超时、跨供应商逐个重试。代价只是"第一次长对话那一轮不压缩"。
async fn compression_warm_one(
    state: AppState,
    uid: uuid::Uuid,
    candidates: std::sync::Arc<Vec<(Model, String)>>,
    key: String,
    text: String,
) -> (usize, usize) {
    use crate::compression as mc;
    let mut redis = state.redis.clone();
    if mc::cached_summary(&mut redis, &key).await.is_some() {
        return (0, 0);
    }

    // Redis 分布式单飞：多个 IDE 重试或多个网关实例同时看到同一个冷段时，只有一个任务
    // 真正调用压缩模型，其余等待缓存出现，避免重复扣费和供应商限流风暴。
    let lock_key = format!("mc:warm:{}", key.trim_start_matches("mc:"));
    let lock_token = uuid::Uuid::new_v4().simple().to_string();
    // 一段会依次尝试多个供应商；锁必须覆盖“候选数 × 单供应商超时”的最坏路径，固定
    // 300 秒在候选较多时会中途过期，另一个重试任务随即重复压缩并重复扣费。
    let lock_ttl = COMPRESSION_WARM_SEGMENT_TIMEOUT
        .as_secs()
        .saturating_mul(candidates.len().max(1) as u64)
        .saturating_add(60)
        .clamp(300, 7_200);
    let acquired: Option<String> = redis::cmd("SET")
        .arg(&lock_key)
        .arg(&lock_token)
        .arg("NX")
        .arg("EX")
        .arg(lock_ttl)
        .query_async(&mut redis)
        .await
        .ok()
        .flatten();
    if acquired.is_none() {
        return (0, 0);
    }

    let mut ok = false;
    for (conn, id) in candidates.iter() {
        match tokio::time::timeout(
            COMPRESSION_WARM_SEGMENT_TIMEOUT,
            compression_summarize(conn, id, &text),
        )
        .await
        {
            Ok(Some(call)) => {
                mc::store_summary(&mut redis, &key, &call.summary).await;
                bill_compression_call(&state, uid, conn, id, call.usage.as_ref()).await;
                ok = true;
                break;
            }
            other => {
                tracing::warn!(
                    compressor = %id, base_url = %conn.base_url,
                    timed_out = other.is_err(),
                    "michael-compression: 预热段失败，换下一个供应商"
                );
            }
        }
    }

    // 只释放自己持有的锁；TTL 到期后若另一个任务已接管，不能误删对方的新锁。
    let _: Result<i32, redis::RedisError> = redis::cmd("EVAL")
        .arg("if redis.call('get', KEYS[1]) == ARGV[1] then return redis.call('del', KEYS[1]) else return 0 end")
        .arg(1)
        .arg(&lock_key)
        .arg(&lock_token)
        .query_async(&mut redis)
        .await;

    if ok {
        (1, 0)
    } else {
        (0, 1)
    }
}

fn compression_spawn_warm(state: &AppState, uid: uuid::Uuid, pending: Vec<(String, String)>) {
    use futures_util::StreamExt;
    let Some(first_key) = pending.first().map(|(key, _)| key.clone()) else {
        return;
    };
    let state = state.clone();
    tokio::spawn(async move {
        // 同一个连续缺口只允许一个批任务。IDE 在预热期间会每 1.2s 重试一次；没有这层锁，
        // 每次重试都会重新查模型目录并创建 100 多个子任务，即使段锁最终挡住了真实调用。
        let batch_lock_key = format!("mc:warm-batch:{}", first_key.trim_start_matches("mc:"));
        let batch_lock_token = uuid::Uuid::new_v4().simple().to_string();
        let mut lock_redis = state.redis.clone();
        let acquired: Option<String> = redis::cmd("SET")
            .arg(&batch_lock_key)
            .arg(&batch_lock_token)
            .arg("NX")
            .arg("EX")
            .arg(1_800u64)
            .query_async(&mut lock_redis)
            .await
            .ok()
            .flatten();
        if acquired.is_none() {
            return;
        }
        let candidates = compression_pick_compressors(&state).await;
        if candidates.is_empty() {
            tracing::warn!("michael-compression: 后台预热没有可用的压缩模型");
            let _: Result<i32, redis::RedisError> = redis::cmd("EVAL")
                .arg("if redis.call('get', KEYS[1]) == ARGV[1] then return redis.call('del', KEYS[1]) else return 0 end")
                .arg(1)
                .arg(&batch_lock_key)
                .arg(&batch_lock_token)
                .query_async(&mut lock_redis)
                .await;
            return;
        }
        let candidates = std::sync::Arc::new(candidates);
        let results = futures_util::stream::iter(
            pending
                .into_iter()
                .take(COMPRESSION_WARM_MAX_SEGMENTS)
                .map(|(key, text)| {
                    compression_warm_one(state.clone(), uid, candidates.clone(), key, text)
                }),
        )
        .buffer_unordered(COMPRESSION_WARM_CONCURRENCY)
        .collect::<Vec<_>>()
        .await;
        let warmed: usize = results.iter().map(|(w, _)| *w).sum();
        let failed: usize = results.iter().map(|(_, f)| *f).sum();
        let _: Result<i32, redis::RedisError> = redis::cmd("EVAL")
            .arg("if redis.call('get', KEYS[1]) == ARGV[1] then return redis.call('del', KEYS[1]) else return 0 end")
            .arg(1)
            .arg(&batch_lock_key)
            .arg(&batch_lock_token)
            .query_async(&mut lock_redis)
            .await;
        tracing::info!(%uid, warmed, failed, "michael-compression: 后台预热结束");
    });
}

/// 给一次段压缩记账，走和聊天完全相同的 `bill()` 路径。
///
/// 压缩调用花的是运营方的上游余额，如果不记账，`model_usage` 就对不上真实支出——
/// 这正是审计在 `/api/i18n/pack` 上查到的问题（匿名、不计费、不可归因），不能在新特性
/// 上重犯。用量拿不到时按 0 计，但**仍然写一行 model_usage**，保证调用可归因。
async fn bill_compression_call(
    state: &AppState,
    uid: uuid::Uuid,
    conn: &Model,
    compressor_model: &str,
    usage: Option<&serde_json::Value>,
) {
    let reported = usage_is_authoritative(usage);
    let (model_in, model_out) = model_price_override(&conn.model_prices, compressor_model);
    let cost = resolve_cost(
        &conn.billing_mode,
        conn.per_call_cents,
        usage.filter(|_| reported),
        compressor_model,
        conn.rate,
        conn.input_price,
        conn.output_price,
        conn.cache_read_price,
        conn.cache_create_price,
        model_in,
        model_out,
    );
    let mut tokens = extract_bill_tokens(
        usage.filter(|_| reported),
        // 在用量表里单独标记，便于把压缩成本和聊天成本分开对账。
        &format!("michael-compression/{compressor_model}"),
        !reported,
    );
    tokens.request_id = None;
    // use_quota=true：压缩是**套餐内含的能力**（档位就是按套餐分的），所以走会员的
    // 时段额度，而不是钱包余额。
    //
    // 上一版是 false，理由是"别让用户觉得什么都没做额度就少了"。但那会把纯订阅、
    // 零余额的用户扣成负数 —— 他被自己套餐包含的功能扣出了债。压缩省下来的输入
    // token 远多于摘要本身的花费，走额度对用户是净赚；而"额度少了一截"这件事，
    // 正确的解法是在用量页面把压缩单独列出来，不是把账记到钱包上。
    bill(state, uid, conn.id, cost, true, &tokens, false, 0).await;
}

#[cfg(test)]
mod cache_key_tests {
    use super::gw_cache_key;
    use serde_json::json;

    fn body() -> serde_json::Value {
        json!({ "model": "gpt-5.5", "messages": [{ "role": "user", "content": "hi" }] })
    }

    #[test]
    fn same_user_same_body_hits() {
        let u = uuid::Uuid::nil();
        assert_eq!(gw_cache_key(u, &body()), gw_cache_key(u, &body()));
    }

    #[test]
    fn different_users_never_share_an_entry() {
        // The key used to be global, so one account's completion could be served to
        // another. Scoping to the caller is what makes that impossible.
        let a = uuid::Uuid::from_u128(1);
        let b = uuid::Uuid::from_u128(2);
        assert_ne!(gw_cache_key(a, &body()), gw_cache_key(b, &body()));
    }

    #[test]
    fn different_body_different_key() {
        let u = uuid::Uuid::nil();
        let mut other = body();
        other["messages"][0]["content"] = json!("bye");
        assert_ne!(gw_cache_key(u, &body()), gw_cache_key(u, &other));
    }
}

#[cfg(test)]
mod step_kind_tests {
    use super::{step_emitted_tool, step_is_tool_turn, step_mode};
    use axum::http::HeaderMap;
    use serde_json::json;

    #[test]
    fn mode_comes_from_the_ide_header() {
        let mut h = HeaderMap::new();
        h.insert("x-ide-mode", "Agent".parse().unwrap());
        assert_eq!(step_mode(&h).as_deref(), Some("agent"));
        assert_eq!(step_mode(&HeaderMap::new()), None);
    }

    #[test]
    fn tool_turn_detects_agent_loop_continuations() {
        // last message is a tool result => this is a loop continuation, not a human turn
        let cont = json!({"messages":[{"role":"user","content":"hi"},{"role":"tool","content":"{}"}]});
        assert_eq!(step_is_tool_turn(&cont), Some(true));
        let fresh = json!({"messages":[{"role":"user","content":"hi"}]});
        assert_eq!(step_is_tool_turn(&fresh), Some(false));
        assert_eq!(step_is_tool_turn(&json!({})), None);
    }

    #[test]
    fn emitted_tool_reads_streaming_and_nonstreaming_shapes() {
        let streaming = r#"data: {"choices":[{"delta":{"tool_calls":[{"function":{"name":"read_file","arguments":""}}]}}]}"#;
        assert_eq!(step_emitted_tool(streaming).as_deref(), Some("read_file"));
        let non_streaming = r#"{"choices":[{"message":{"tool_calls":[{"function":{"name":"run_cmd"}}]}}]}"#;
        assert_eq!(step_emitted_tool(non_streaming).as_deref(), Some("run_cmd"));
    }

    #[test]
    fn prose_replies_have_no_emitted_tool() {
        let prose = r#"data: {"choices":[{"delta":{"content":"这里是一段普通回答，没有工具调用。"}}]}"#;
        assert_eq!(step_emitted_tool(prose), None);
        assert_eq!(step_emitted_tool(""), None);
    }

    #[test]
    fn classifier_never_panics_on_hostile_input() {
        // runs over untrusted upstream text; must be total, not merely usually-correct
        for s in ["\"function\"", "\"function\"{\"name\":", "\"function\"{\"name\":\"", "{}", "\"function\"{\"name\":\"\"}"] {
            let _ = step_emitted_tool(s);
        }
        let long = format!("\"function\"{{\"name\":\"{}\"}}", "x".repeat(500));
        assert_eq!(step_emitted_tool(&long), None, "over-long names are rejected");
    }
}

#[cfg(test)]
mod adaptive_header_wait_tests {
    use super::*;

    fn fresh_route() -> uuid::Uuid {
        let id = uuid::Uuid::new_v4();
        if let Ok(mut m) = ROUTE_HEADER_STATS.lock() {
            m.remove(&id);
        }
        id
    }

    /// An unknown route must behave exactly as before — no measurement, no opinion.
    #[test]
    fn unknown_route_keeps_the_fixed_fallback() {
        let id = fresh_route();
        assert_eq!(
            adaptive_first_attempt_wait(id, FIRST_ATTEMPT_HEADER_WAIT),
            FIRST_ATTEMPT_HEADER_WAIT
        );
        // Two samples is still not a baseline worth trusting.
        record_header_success(id, 300);
        record_header_success(id, 300);
        assert_eq!(
            adaptive_first_attempt_wait(id, FIRST_ATTEMPT_HEADER_WAIT),
            FIRST_ATTEMPT_HEADER_WAIT
        );
    }

    /// A fast route gets a TIGHT leash — the old flat 4s was pure tax on these.
    #[test]
    fn fast_route_cuts_over_far_sooner_than_the_flat_wait() {
        let id = fresh_route();
        for _ in 0..6 {
            record_header_success(id, 200);
        }
        let wait = adaptive_first_attempt_wait(id, FIRST_ATTEMPT_HEADER_WAIT);
        assert!(
            wait < FIRST_ATTEMPT_HEADER_WAIT,
            "a 200ms route must not be given 4s: got {wait:?}"
        );
        // …but never so tight that ordinary jitter kills it.
        assert!(wait >= Duration::from_millis(900), "floor must hold: {wait:?}");
    }

    /// A slow-but-HONEST route must stop being killed at 4s — this is the regression
    /// that made the gateway abandon working upstreams and pay the cost twice.
    #[test]
    fn slow_but_healthy_route_is_given_more_than_the_flat_wait() {
        let id = fresh_route();
        for _ in 0..6 {
            record_header_success(id, 3_000);
        }
        let wait = adaptive_first_attempt_wait(id, FIRST_ATTEMPT_HEADER_WAIT);
        assert!(
            wait > FIRST_ATTEMPT_HEADER_WAIT,
            "a consistently-3s route needs room to answer: got {wait:?}"
        );
        assert!(wait <= Duration::from_secs(9), "ceiling must hold: {wait:?}");
    }

    /// Consecutive stalls shorten the leash: on these relays a FRESH CONNECTION is what
    /// actually works, so reaching it sooner is the whole point.
    #[test]
    fn repeated_stalls_shorten_the_leash_and_success_restores_it() {
        let id = fresh_route();
        for _ in 0..6 {
            record_header_success(id, 2_000);
        }
        let calm = adaptive_first_attempt_wait(id, FIRST_ATTEMPT_HEADER_WAIT);
        record_header_stall(id);
        record_header_stall(id);
        let stalling = adaptive_first_attempt_wait(id, FIRST_ATTEMPT_HEADER_WAIT);
        assert!(
            stalling < calm,
            "a habitual staller must cut over sooner: {stalling:?} !< {calm:?}"
        );
        // One success clears the streak — a transient blip must not permanently bias it.
        record_header_success(id, 2_000);
        assert!(
            adaptive_first_attempt_wait(id, FIRST_ATTEMPT_HEADER_WAIT) > stalling,
            "the shorter leash must relax once the route answers again"
        );
    }

    /// The EWMA has to actually track a provider that degrades, within a few requests.
    #[test]
    fn baseline_follows_a_degrading_provider() {
        let id = fresh_route();
        for _ in 0..6 {
            record_header_success(id, 200);
        }
        let before = adaptive_first_attempt_wait(id, FIRST_ATTEMPT_HEADER_WAIT);
        for _ in 0..6 {
            record_header_success(id, 4_000);
        }
        let after = adaptive_first_attempt_wait(id, FIRST_ATTEMPT_HEADER_WAIT);
        assert!(after > before, "the baseline must follow reality: {after:?} !> {before:?}");
    }
}
