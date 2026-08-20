use axum::body::Body;
use crate::auth::QUOTA_WINDOW_REFRESH;
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
use crate::route_health;
use crate::AppState;

/// Shared, pooled HTTP client for upstream model calls. Building a fresh
/// `reqwest::Client` per request (the old behaviour) forced a brand-new TCP+TLS
/// handshake to the provider on every call — a large chunk of the "feels slow"
/// latency, and it compounds badly for an agent firing many sequential requests.
/// One pooled client keeps connections warm (keep-alive), so only the first call
/// to a host pays the handshake. No global timeout: streamed chat responses are
/// open-ended; only the connect phase is bounded (per-request timeouts are added
/// for the non-streaming calls that need them).
/// 发给上游的 `User-Agent`。
///
/// reqwest 不配置就一个字节都不发，而"没有 User-Agent 的 POST"正是各家 WAF / CDN
/// 最先挑出来限速或挂起的特征之一。上游是转卖商，前面挂什么中间层不由我们决定，
/// 所以这里给一个稳定、可识别、带版本的标识——出问题时对方也能在自己日志里找到我们。
const GATEWAY_USER_AGENT: &str = concat!("MichaelIDE-Gateway/", env!("CARGO_PKG_VERSION"));

static GW_HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(GATEWAY_USER_AGENT)
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
        .user_agent(GATEWAY_USER_AGENT)
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

const CHAT_UPSTREAM_MAX_ATTEMPTS_PER_ROUTE: u32 = 1;
/// 上游**明确回了一个错误响应**时，允许再换一条同模型线路。
///
/// 「一次请求只发一次」这条规矩是对的，理由写在下面循环里：传输层失败也可能发生在上游
/// 已经收下 body 之后，重发会重复跑模型、重复计费。但它此前被套用在了两种性质完全不同的
/// 失败上：
///
///   · **表头卡死 / 发送出错** —— 什么都没回来，上游可能正在跑。不能重发，维持原样。
///   · **完整的错误响应**（502/503/401/…）—— 上游用自己的话说了「我失败了」。它没跑模型，
///     也不会为此计费。这时候换一条线路既安全，又正是用户要的。
///
/// 线上代价是可量的：40 小时里 48 次 GPT 502 全部写着 `route_count=2 attempted_sends=1`
/// ——旁边那条同模型线路一次都没试过。那把失效的 key（`invalid_api_key` → 424）也是同一回事：
/// 落到它上面的请求直接判死，而循环里那句注释写着「401/403 仍然会换线，那是每条线路各自的
/// 凭据」——在 MAX_ROUTES=1 之下，这句话从来没成立过。
const CHAT_UPSTREAM_MAX_ROUTES_WHEN_ANSWERED: usize = 2;
const CHAT_UPSTREAM_ROUTE_COOLDOWN: Duration = Duration::from_secs(20);

/// What the IDE waits for response headers before it gives up. The upstream relay
/// holds the HTTP response until its first SSE event, so this includes provider
/// prefill time. After headers open, the stream has its own long idle deadline.
/// Only read by the test that enforces the coupling — the value's job is to make the
/// client's deadline visible here so nobody widens the gateway budget past it.
#[cfg_attr(not(test), allow(dead_code))]
const CLIENT_HEADER_TIMEOUT: Duration = Duration::from_secs(60);
/// This supplier does not flush HTTP headers until its first SSE event, so response-header
/// latency includes model prefill. Production logs show healthy Claude headers beyond 8s
/// (p95 ~8.2s, max ~8.5s on the current route). The old 8/10/11s ceilings sat inside the
/// normal latency tail and generated self-inflicted 504s before the provider had failed.
const STANDARD_MAX_HEADER_WAIT: Duration = Duration::from_secs(57);
const AGENT_MAX_HEADER_WAIT: Duration = Duration::from_secs(57);
const DEEP_MAX_HEADER_WAIT: Duration = Duration::from_secs(57);
const MAX_ERROR_BODY_WAIT: Duration = Duration::from_secs(2);
const ROUTE_BUDGET: Duration = Duration::from_secs(58);
const CLIENT_DEADLINE_MARGIN: Duration = Duration::from_millis(750);
const RESPONSE_DEADLINE_HEADER: &str = "x-ide-response-deadline-ms";
/// 同一件事的**相对**说法："从我发出这一刻算，我还等这么多毫秒"。
///
/// 和上面那个绝对时间戳并存，因为两者的失效模式完全不同：绝对时间戳天然把上传耗时
/// 算了进去，但它是**客户端墙上时钟**的时间戳，必须和服务端墙上时钟相减；相对预算
/// 不牵涉任何时钟比对，只要客户端自己的定时器是对的，它就是对的。
const RESPONSE_BUDGET_HEADER: &str = "x-ide-response-budget-ms";
/// 两个头都在时，允许的时钟分歧。超过这个数就认定客户端时钟不可信，只采信相对预算。
///
/// 正常情况下这个差值就是「上传+排队耗时」的负值（几百毫秒到几秒），5 秒足够覆盖；
/// 真正的时钟偏差通常是几十秒到几分钟量级，不会落在这个窗口里。
const MAX_TRUSTED_CLOCK_SKEW: Duration = Duration::from_secs(5);
/// 只有绝对时间戳（老客户端）时，低于这个剩余量就判定这个头不可信。
///
/// 客户端自己的耐心是 CLIENT_HEADER_TIMEOUT（60s）。请求还没被处理就已经烧掉一半
/// 以上的耐心，"上传花了 30 秒"和"这台机器的时钟不准"这两种解释里，后者常见得多——
/// 而真的已经超时的客户端会 abort 连接，我们根本收不到它。两种解释都指向同一个动作：
/// 把这个头当不存在，退回网关自己的预算。
const MIN_TRUSTED_ABSOLUTE_REMAINING: Duration = Duration::from_secs(30);

/// Total time the gateway may spend hunting for a working upstream route before it
/// must answer the client.
///
/// This has to stay comfortably under the client's own header timeout. When it
/// didn't, the client gave up first and fast-retried, and each retry opened a fresh
/// gateway request with its own set of upstream calls — a multiplying storm of
/// `/v1/messages` requests rather than one failure the user could read.
/// 线路总预算。**刻意不按尝试次数放大**：这是运输层健康的判定窗口，由客户端的
/// 耐心决定（CLIENT_HEADER_TIMEOUT = 60s，镜像自 IDE 的
/// `_AI_RESPONSE_HEADERS_DEADLINE_MS`）。深思考请求在这个总预算内拿到更长的单次表头
/// 上限，响应打开之后再由流空闲窗口接管。
///
/// 把它改成按 "尝试次数 × 表头上限" 放大是错的 —— 两次完整窗口会超过客户端 60s 的
/// 耐心，网关等得再久，用户那边也早就断了，只会把一个有错误信息的 504 换成一个
/// 什么都没有的客户端超时。`route_budget_fits_inside_the_client_header_timeout`
/// 就是钉这件事的。
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

/// 客户端在两个头里表达的"我还能等多久"。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ClientPatience {
    /// `x-ide-response-budget-ms` —— 相对，不牵涉时钟。
    budget_ms: Option<u64>,
    /// `x-ide-response-deadline-ms` —— 绝对，是客户端墙上时钟的时间戳。
    deadline_ms: Option<u64>,
}

/// 预算是怎么定下来的。只用于日志：把"这台机器的时钟不对"变成看得见的东西，
/// 而不是变成"就他一个人用不了，日志里什么都没有"。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClientPatienceVerdict {
    /// 两个头都没有（BYOK、网页调试、更老的客户端）。
    Absent,
    /// 只有相对预算。
    RelativeOnly,
    /// 两个都有且互相印证：取更紧的那个（绝对那个把上传耗时也算了进去）。
    ClocksAgree { skew_ms: i64 },
    /// 两个都有但对不上：丢掉绝对时间戳，只用相对预算。
    ClockSkewed { skew_ms: i64 },
    /// 只有绝对时间戳，落在合理范围内。
    AbsoluteOnly,
    /// 只有绝对时间戳，且算出来的剩余量荒谬 —— 当作没有这个头。
    AbsoluteUntrusted { remaining_ms: u64 },
}

/// 这一轮允许花在"找一条能用的线路"上的总时间。
///
/// 绝对截止时间戳曾经是唯一判据，而它有一个静默的致命失效模式：用户机器的时钟慢上
/// 一两分钟（NTP 被挡、虚拟机休眠唤醒、装机时没对时），`deadline_ms` 就永远小于
/// `now_ms`，预算恒为零 —— 那台机器上**每一次**请求都在开出上游调用之前就判死，而且
/// 永远如此。服务端只看得到"这个人什么都发不出去"，看不出为什么。
///
/// 现在的判据分三层，每一层的理由不同：
///   * 相对预算不牵涉时钟比对，所以它是**上限**，永远采信；
///   * 绝对时间戳只用来**收紧**上限，且只在两个时钟对得上时才收紧（它的价值是把上传
///     耗时算了进去，这一点相对预算做不到）；
///   * 只有绝对时间戳的老客户端无法验证时钟，于是做合理性检查：算出来的剩余量少于
///     客户端总耐心的一半，就当这个头不存在。
///
/// 判不准时宁可**多开**一次上游调用：客户端断开会 drop 掉这个 future，调用随即取消
/// （见表头等待那一段的注释），代价是一次可能被放弃的转发；而判死的代价是一台机器
/// 永久不可用。
fn route_budget_with_client_patience(
    deep_thinking: bool,
    patience: ClientPatience,
    now_ms: u64,
) -> (Duration, ClientPatienceVerdict) {
    let fallback = route_budget_for(deep_thinking);
    let tighten = |limit: Duration| fallback.min(limit.saturating_sub(CLIENT_DEADLINE_MARGIN));

    match (patience.budget_ms, patience.deadline_ms) {
        (Some(budget_ms), Some(deadline_ms)) => {
            let derived_ms = deadline_ms.saturating_sub(now_ms);
            // 正常情况下这个差值就是上传耗时的负值；真正的时钟偏差要大一个量级。
            let skew_ms = derived_ms as i64 - budget_ms as i64;
            let relative = Duration::from_millis(budget_ms);
            if skew_ms.unsigned_abs() <= MAX_TRUSTED_CLOCK_SKEW.as_millis() as u64 {
                let absolute = Duration::from_millis(derived_ms);
                (
                    tighten(relative.min(absolute)),
                    ClientPatienceVerdict::ClocksAgree { skew_ms },
                )
            } else {
                (
                    tighten(relative),
                    ClientPatienceVerdict::ClockSkewed { skew_ms },
                )
            }
        }
        (Some(budget_ms), None) => (
            tighten(Duration::from_millis(budget_ms)),
            ClientPatienceVerdict::RelativeOnly,
        ),
        (None, Some(deadline_ms)) => {
            let remaining = Duration::from_millis(deadline_ms.saturating_sub(now_ms));
            if remaining < MIN_TRUSTED_ABSOLUTE_REMAINING {
                (
                    fallback,
                    ClientPatienceVerdict::AbsoluteUntrusted {
                        remaining_ms: remaining.as_millis().min(u64::MAX as u128) as u64,
                    },
                )
            } else {
                (tighten(remaining), ClientPatienceVerdict::AbsoluteOnly)
            }
        }
        (None, None) => (fallback, ClientPatienceVerdict::Absent),
    }
}

fn client_patience_from_headers(headers: &HeaderMap) -> ClientPatience {
    let read = |name: &str| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<u64>().ok())
    };
    ClientPatience {
        budget_ms: read(RESPONSE_BUDGET_HEADER),
        deadline_ms: read(RESPONSE_DEADLINE_HEADER),
    }
}

fn route_budget_for_headers(headers: &HeaderMap, deep_thinking: bool) -> Duration {
    let patience = client_patience_from_headers(headers);
    let (budget, verdict) =
        route_budget_with_client_patience(deep_thinking, patience, unix_time_ms());
    match verdict {
        // 这两条是"这台机器的时钟不对"的唯一可见证据，必须留痕：否则它只会表现为
        // 某个用户莫名其妙什么都发不出去。
        ClientPatienceVerdict::ClockSkewed { skew_ms } => tracing::warn!(
            skew_ms,
            budget_secs = budget.as_secs(),
            "客户端时钟与服务端相差过大，已忽略绝对截止时间戳，改用相对预算"
        ),
        ClientPatienceVerdict::AbsoluteUntrusted { remaining_ms } => tracing::warn!(
            remaining_ms,
            budget_secs = budget.as_secs(),
            "绝对截止时间戳算出的剩余量不可信（多半是客户端时钟不准），已退回网关预算"
        ),
        _ => {}
    }
    budget
}

/// Does this request ask the model to think before answering?
///
/// Thinking moves work into prefill, and this supplier withholds HTTP headers until its
/// first SSE event, so a thinking request legitimately takes longer to produce headers
/// than a plain one. That is what the deep budget (10s headers / 600s idle) exists for.
///
/// All three wire shapes have to be recognised, because they are not interchangeable
/// across models and the gateway emits different ones for different families:
///   * `reasoning_effort: low+`        — OpenAI-shaped request with thinking enabled
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
        .is_some_and(|e| {
            !matches!(e.to_ascii_lowercase().as_str(), "" | "off" | "none" | "disabled")
        });
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

/// Return only a stable category for telemetry. Never log a caller-provided value
/// directly: the field is meant to be an enum, but an untrusted client can send
/// arbitrary JSON.
fn telemetry_reasoning_effort(body: &serde_json::Value) -> &'static str {
    match body
        .get("reasoning_effort")
        .and_then(|value| value.as_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        None => "absent",
        Some("") | Some("off") | Some("none") | Some("disabled") => "off",
        Some("low") => "low",
        Some("medium") => "medium",
        Some("high") => "high",
        Some("xhigh") => "xhigh",
        Some("max") => "max",
        Some(_) => "other",
    }
}

/// As above, preserve only the known wire-shape category rather than arbitrary
/// request content. This keeps the diagnostic useful without retaining prompts.
fn telemetry_thinking_type(body: &serde_json::Value) -> &'static str {
    match body
        .pointer("/thinking/type")
        .and_then(|value| value.as_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        None => "absent",
        Some("adaptive") => "adaptive",
        Some("enabled") => "enabled",
        Some("disabled") => "disabled",
        Some(_) => "other",
    }
}

fn telemetry_output_config_effort(body: &serde_json::Value) -> &'static str {
    match body
        .pointer("/output_config/effort")
        .and_then(|value| value.as_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        None => "absent",
        Some("low") => "low",
        Some("medium") => "medium",
        Some("high") => "high",
        Some(_) => "other",
    }
}

const ANTHROPIC_CONTEXT_1M_BETA: &str = "context-1m-2025-08-07";
const ANTHROPIC_INTERLEAVED_THINKING_BETA: &str = "interleaved-thinking-2025-05-14";
const ANTHROPIC_EFFORT_BETA: &str = "effort-2025-11-24";

/// Build the single Anthropic capability header sent to native `/v1/messages` routes.
///
/// Sub2API treats this as a comma-separated capability set. Sending `context-1m` in a
/// standalone header used to omit the two capabilities its API-key path needs in order
/// to preserve adaptive/interleaved thinking and `output_config.effort`. Never include
/// `redact-thinking`: that capability intentionally removes visible thinking text.
/// 不带 `context-1m` 时上游实际允许的输入上限。
///
/// **不从目录取。** 目录（`official_contexts`）描述的是 Anthropic 自己的行为——它对
/// Opus 4.6+/Sonnet 4.6+/Fable 原生就给 1M，所以那边的条目是「1M，不需要 beta」。而中转商
/// 把 1M 挡在和 Sonnet 4/4.5 同一个 beta 后面（实测原文：`400 {"error":"1m context is
/// fully available; please enable 1m context and retry"}`）。也就是说：不发这个 flag 时，
/// 我们实际能用的是 Anthropic 的经典窗口，不是目录上那个数。
const ANTHROPIC_CONTEXT_WITHOUT_1M_BETA_TOKENS: usize = 200_000;

/// 触发 `context-1m` 的正文字节阈值。
///
/// 任何 BPE 分词器在 UTF-8 上都满足 **token 数 ≤ 字符数 ≤ 字节数**，所以「正文字节数 < N」
/// 可以*证明* token 数 < N —— 是硬上界，不是经验估计。取 150k 而不是 200k，是给工具
/// schema、系统提示词模板这类不在正文字符串里、但会进 token 账的部分留 25% 余量。
///
/// 方向是刻意选的：多发一次这个 flag 只是回到改动前的行为，而少发一次会换来一个硬 400
/// （而且 400 不会 failover），所以一切模糊地带都往「发」的方向倒。图片正是这样一个
/// 模糊地带——base64 字节巨大、token 很少，于是它天然把我们推向多发，正合适。
const ANTHROPIC_1M_BETA_TEXT_BYTES: usize = 150_000;

/// 请求体里所有字符串内容的字节数之和。
///
/// 只数字符串值：JSON 的键名和结构字符不进模型，不该算进 token 账。深度由 serde_json
/// 解析时的递归上限兜住（默认 128 层），所以这里的递归是有界的。
fn body_text_bytes(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::String(text) => text.len(),
        serde_json::Value::Array(items) => items.iter().map(body_text_bytes).sum(),
        serde_json::Value::Object(fields) => fields.values().map(body_text_bytes).sum(),
        _ => 0,
    }
}

/// 这一次请求要不要发 `context-1m`。
///
/// 以前的判据只有「这个模型支不支持 1M」，于是**每一个** Claude 请求都带着它——包括一个
/// 354 token 的请求。实测（近 7 天 11,990 次 Claude 调用）真正超过 20 万 token 的只有 93 次，
/// 占 0.78%：另外 99.2% 都在把一个溢价通道标记发给一个用不上它的请求。
///
/// 现在加一道体积判据：模型支持 1M **且** 这一次的正文大到可能超出标准窗口，才发。
fn wants_1m_context(model_id: &str, upstream_body: &serde_json::Value) -> bool {
    let model_supports_1m = official_contexts(model_id)
        .iter()
        .any(|(tokens, _)| *tokens >= 1_000_000);
    if !model_supports_1m {
        return false;
    }
    debug_assert!(ANTHROPIC_1M_BETA_TEXT_BYTES < ANTHROPIC_CONTEXT_WITHOUT_1M_BETA_TOKENS);
    body_text_bytes(upstream_body) >= ANTHROPIC_1M_BETA_TEXT_BYTES
}

fn anthropic_beta_header(body: &serde_json::Value, wants_1m: bool) -> Option<String> {
    let mut betas = Vec::with_capacity(3);
    if wants_1m {
        betas.push(ANTHROPIC_CONTEXT_1M_BETA);
    }
    if body.pointer("/thinking/type").and_then(|value| value.as_str()) == Some("adaptive") {
        betas.push(ANTHROPIC_INTERLEAVED_THINKING_BETA);
        betas.push(ANTHROPIC_EFFORT_BETA);
    }
    (!betas.is_empty()).then(|| betas.join(","))
}

/// Collapse an untrusted native event type into a fixed telemetry enum.
fn telemetry_anthropic_event_kind(event_type: Option<&str>) -> &'static str {
    match event_type {
        Some("message_start") => "message_start",
        Some("content_block_start") => "content_block_start",
        Some("content_block_delta") => "content_block_delta",
        Some("content_block_stop") => "content_block_stop",
        Some("message_delta") => "message_delta",
        Some("message_stop") => "message_stop",
        Some("ping") => "ping",
        Some("error") => "error",
        Some(_) => "other",
        None => "absent",
    }
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

/// 「这条线路最近把表头预算整整耗满才失败」的记号。
///
/// 和上面那张冷却表分开记，因为它们回答的是两个不同的问题：
///   * 冷却表：这一轮**该不该优先绕开**它。只有在还有别的同模型线路时才有意义
///     —— `route_count > 1` 那道判据就是这个意思。
///   * 这张表：**绕不开的时候**（这个模型只有这一条线，或者用户点了强力版把候选
///     压成了一条）该给它多少耐心。
///
/// 少了这一层，一条只会挂着不回话的线路，会让每一个落到它上面的请求都垫满 57 秒。
/// 而客户端自己的耐心是 60 秒 —— 一次就烧光了，它那套 4 次重试一次都轮不上，用户
/// 等一分钟只换回一条错误。记下来之后同一条线路改用短探测预算：仍然每次都试
/// （所以上游一恢复就自动恢复，不需要任何人去后台改配置），但失败得起，客户端
/// 的重试预算还剩得下。
static CHAT_UPSTREAM_ROUTE_STALLS: LazyLock<Mutex<HashMap<uuid::Uuid, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 记号的有效期。比冷却长得多：冷却是"这一轮换条线走"，而这个是"这条线的脾气"，
/// 需要跨越好几轮请求才看得出来。
const CHAT_UPSTREAM_STALL_MEMORY: Duration = Duration::from_secs(120);

/// 对一条最近卡满过的线路，单次表头等待的上限。
///
/// 这个数是两个方向夹出来的，两边都用实测表头延迟分布定标（2026-08-19，786 个成功样本：
/// p50 9.4s、p90 21.7s、p95 31.9s）：
///
///   * **下限**——必须高于健康响应的 p90，否则一条只是慢的线路会被这条规则routinely
///     误伤：它每被截断一次就又记一次卡死，自己把自己按死在短预算上。25s > 21.7s，
///     大约 8% 的健康响应会被切掉；而一次成功就撤销记号（`clear_route_stall`），
///     所以真正只是慢的线路一两个请求内就自己恢复完整预算。
///   * **上限**——必须显著低于完整预算才有意义。客户端最多重试 4 次、退避 2/4/8/16s，
///     所以一轮彻底失败的总耗时从 57×5+30 ≈ 315 秒（5 分 15 秒）降到 25×5+30 ≈ 155 秒
///     （2 分 35 秒）。
///
/// 注意**不是**「让客户端的重试得以发生」——客户端那个 60 秒是每次尝试各自一份，不是
/// 整轮共用一份，57 秒的回答本来就不会吃掉后续重试。这里买到的是等待时间腰斩，不是
/// 重试次数。
const CHAT_UPSTREAM_STALLED_PROBE_WAIT: Duration = Duration::from_secs(25);
// 中转丢块自愈：jgy 等聚合中转在深思考超过 ~7.5K token 后会丢掉后面的 text/tool_use
// 块并谎报 end_turn（对照实验：budget 6000 → thinking+text+tool_use 正常；budget 24000
// → 只回 thinking 就 end_turn；官方 API 绝不会思考完直接收尾）。检出签名后该线路记
// 30 分钟"思考钳位"，期间 budget_tokens 压到实测安全值；健康线路不受影响，到期自动解除。
/// 这次流是不是"中转把后半段掐了"。
///
/// 判据必须覆盖协议校验器**实际会吐出的每一种截断错误**。原本这里只认两个字符串，
/// 而校验器一共会吐出七种：流在 message_stop 之前结束、tool_use 没收尾、SSE 帧不完整、
/// 没有终止 [DONE]、流卡死、工具名缺失——这五种一个都不匹配。于是自愈在最高频的那几种
/// 截断上根本不触发：线路不被钳位，客户端把同一个注定失败的请求原样重掷，最多 10 次。
///
/// 用户看到的就是：内容已经出来一半，然后长时间干等——因为每一次重试都会再被掐一次。
///
/// 下面的 `relay_truncation_signatures_stay_in_sync` 钉住它不再漂：它直接扫本文件里所有
/// 截断类错误文案，少认一个就红。写这条守卫时它当场抓出我自己漏掉的一个，正是它的用处。
fn looks_like_relay_truncation(err: &str) -> bool {
    // 用尽量短、尽量不含可变措辞的片段：文案改写（"ended before protocol completion"
    // → "ended before message_stop"）正是上一次让这套自愈静默失效的原因。
    const SIGNATURES: &[&str] = &[
        "incomplete arguments JSON",
        "incomplete SSE frame",
        // 线上实测最高频的那一种：中转丢块之后，tool_use 的 input 是残的，于是被
        // 必填参数校验拦下——文案里没有任何"截断/incomplete"字样，纯靠这条认。
        // 268 次请求里协议校验失败 4 次，其中 3 次是它，而钳位一次都没触发。
        "is missing required arguments",
        "ended before protocol completion",
        "ended before message_stop",
        "ended before tool_use",
        "ended without terminal data",
        "stream stalled for",
        "ended without function.name",
    ];
    SIGNATURES.iter().any(|sig| err.contains(sig))
}

const THINKING_CLIP_COOLDOWN: Duration = Duration::from_secs(30 * 60);
const THINKING_CLIP_SAFE_BUDGET: i64 = 6000;
static THINKING_CLIP_ROUTES: LazyLock<Mutex<HashMap<uuid::Uuid, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 「要了思考，一个字都没回」的线路。
///
/// 这件事**早就检测出来了**（见 thinking_requested_but_none_returned 那条 warn），但检测完
/// 只做了两件事：打一条日志、不进缓存。选路完全不知道有这回事，于是下一次请求照样落到
/// 同一条线路上，用户照样看不到「已思考」。实测：claude-opus-5 的三条同模型线路里，
/// 排头那条（label "Claude"）稳定吞掉思考，而用户每次都先撞上它——他的原话是
/// 「问问题他不会去思考」。
///
/// 有别的同模型线路可走时，把它排到后面。**不是拉黑**：到期自动再探一次，
/// 上游哪天恢复了第一个成功返回思考的请求就把记号撤掉（见 clear_thinking_mute），
/// 不需要任何人去后台改配置。
static THINKING_MUTE_ROUTES: LazyLock<Mutex<HashMap<uuid::Uuid, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 记号有效期。取 30 分钟，和思考钳位同一档：这是「这条线路的脾气」，
/// 要跨越好几轮请求才看得出来，比一轮换线的冷却长得多。
const THINKING_MUTE_MEMORY: Duration = Duration::from_secs(30 * 60);

fn mark_thinking_mute(id: uuid::Uuid) {
    if let Ok(mut guard) = THINKING_MUTE_ROUTES.lock() {
        guard.insert(id, Instant::now() + THINKING_MUTE_MEMORY);
    }
}

/// 这条线路回过思考了 —— 撤掉记号。这是自愈的全部机制：没有它，记号只会越积越多，
/// 一条只是偶尔抽风的线路会被永久排到后面。
fn clear_thinking_mute(id: uuid::Uuid) {
    if let Ok(mut guard) = THINKING_MUTE_ROUTES.lock() {
        guard.remove(&id);
    }
}

fn route_mutes_thinking(id: uuid::Uuid, now: Instant) -> bool {
    let Ok(mut guard) = THINKING_MUTE_ROUTES.lock() else {
        return false;
    };
    match guard.get(&id).copied() {
        Some(until) if until > now => true,
        Some(_) => {
            guard.remove(&id);
            false
        }
        None => false,
    }
}
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
    // 只延长不缩短：否则一条正处于 5 分钟鉴权冷却里的坏线路，被一次瞬时故障
    // 覆盖成 20 秒，又提前回到轮换里继续 401。
    if let Ok(mut guard) = CHAT_UPSTREAM_ROUTE_COOLDOWNS.lock() {
        let until = Instant::now() + CHAT_UPSTREAM_ROUTE_COOLDOWN;
        let e = guard.entry(id).or_insert(until);
        if until > *e {
            *e = until;
        }
    }
}

/// 鉴权失败（401/403、invalid key）后的冷却，比瞬时故障长得多。
///
/// 坏 key 不会在几十秒里变好，所以 20 秒的瞬时冷却在这里没用——到期它又回轮换、又
/// 401。5 分钟意味着：坏了之后基本不再被挑中（同模型的好线路接管），而运维在后台把
/// key 修好后，最迟 5 分钟这条线路自动回归，不需要重启。用 max 避免把一条已经在更长
/// 冷却里的线路缩短。
const CHAT_UPSTREAM_AUTH_COOLDOWN: Duration = Duration::from_secs(5 * 60);
fn mark_route_cooldown_auth(id: uuid::Uuid) {
    if let Ok(mut guard) = CHAT_UPSTREAM_ROUTE_COOLDOWNS.lock() {
        let until = Instant::now() + CHAT_UPSTREAM_AUTH_COOLDOWN;
        let e = guard.entry(id).or_insert(until);
        if until > *e {
            *e = until;
        }
    }
}

/// 这条线路在记忆窗口内卡满过表头预算吗。
fn route_recently_stalled(id: uuid::Uuid, now: Instant) -> bool {
    let Ok(mut guard) = CHAT_UPSTREAM_ROUTE_STALLS.lock() else {
        return false;
    };
    match guard.get(&id).copied() {
        Some(until) if until > now => true,
        Some(_) => {
            guard.remove(&id);
            false
        }
        None => false,
    }
}

fn mark_route_stall(id: uuid::Uuid) {
    if let Ok(mut guard) = CHAT_UPSTREAM_ROUTE_STALLS.lock() {
        guard.insert(id, Instant::now() + CHAT_UPSTREAM_STALL_MEMORY);
    }
}

/// 拿到表头就立刻清记号。
///
/// 这一条是"自愈"的全部机制：上游一旦恢复，第一个成功的请求就把短探测预算撤掉，
/// 后面的请求拿回完整的 57 秒。没有它，短探测会自我延续 —— 被 25 秒截断的失败又写一
/// 次记号，一条只是慢的线路会被永远按在 25 秒上。
fn clear_route_stall(id: uuid::Uuid) {
    if let Ok(mut guard) = CHAT_UPSTREAM_ROUTE_STALLS.lock() {
        guard.remove(&id);
    }
}

/// 这一次给这条线路多少表头耐心。
///
/// 正常情况就是按请求形态算出来的上限；只有当这条线路最近整整卡满过一次时，才压到
/// 短探测预算。注意这是**减少**耐心，不是跳过 —— 请求照发，上游恢复了就照常拿到结果。
fn header_wait_for_route(base: Duration, route_id: uuid::Uuid, now: Instant) -> Duration {
    if route_recently_stalled(route_id, now) {
        base.min(CHAT_UPSTREAM_STALLED_PROBE_WAIT)
    } else {
        base
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

/// 失败信息尾巴。`power_route` 是这一轮有没有带 `x-ide-power-route`。
///
/// 分出这一支是因为"只有 1 条同模型线路"在两种情况下含义完全不同：后台确实只配了一条，
/// 用户无能为力；而带了强力版开关时是**这个开关自己**把候选压成了一条 —— 关掉它立刻就有
/// 别的线路可走。不说清楚的话，用户看到的是一条自己没法处理的报错，而实际上出口就在
/// 他刚点亮的那个图标上。
fn chat_upstream_attempt_suffix(
    route_count: usize,
    attempts: u32,
    last_status: u16,
    power_route: bool,
) -> String {
    if power_route && route_count <= 1 {
        format!(
            "（已请求 {attempts} 次；「强力版」把这一轮限定在这 1 条线路上，关掉它可改走其它同模型线路；最后状态 {last_status}）"
        )
    } else if route_count <= 1 {
        format!("（已请求 {attempts} 次；当前只有 1 条同模型线路；最后状态 {last_status}）")
    } else if (attempts as usize) < route_count {
        // 「已请求 1 次 / 2 条同模型线路」读起来是"两条都试过了、都不行"，而实际上另一条
        // 健康线路一次都没碰过——一个 inbound 请求只发一次上游（CHAT_UPSTREAM_MAX_ROUTES_
        // PER_REQUEST = 1），换线是**跨请求**发生的：这次失败会给这条线路记冷却，下一次
        // 发送就自动排到别的线路上。用户实拍到的正是这个误读：他以为线路全废了，其实
        // 重发一次就好。把没试过的那几条说出来，并把"重发一次"这个出口讲明白。
        let untried = route_count - attempts as usize;
        format!(
            "（本次只试了 1 条线路，同模型另有 {untried} 条没试过；这条已被记下冷却，直接重发一次就会自动改走其它线路；最后状态 {last_status}）"
        )
    } else {
        format!("（已请求 {attempts} 次 / {route_count} 条同模型线路；最后状态 {last_status}）")
    }
}

/// 把上游错误映射成对用户有用的中文。模块级函数，测试可直接调用。
fn upstream_friendly_message(status: u16, low: &str) -> String {
    // 余额不足。**中英文都要认**：这里原来只匹配 insufficient_balance /
    // insufficient account balance 两个英文串，而国内中转普遍用中文报这件事。
    //
    // 线上实测（2026-08-05，claude-sonnet-5 走 changhuai.ai）：上游返回
    //   {"error":{"type":"new_api_error","message":
    //    "预扣费额度失败, 用户剩余额度: ＄0.055828, 需要预扣费额度: ＄0.134302"}}
    // 一个字都没命中上面两个英文串，于是一路落到最后那句"上游暂时不可用，请换个
    // 模型或稍后再试" —— 用户看到的是"线路坏了"，真实原因是账户只剩五分钱。
    // 上游把余额、需要多少、请求 id 全说清楚了，全被这层映射丢掉。
    //
    // 顺带把上游原话带上：余额这种事，"还剩多少、需要多少"比任何转述都有用。
    if low.contains("insufficient_balance")
        || low.contains("insufficient account balance")
        || low.contains("余额不足")
        || low.contains("额度不足")
        || low.contains("预扣费")
        || low.contains("剩余额度")
        || low.contains("quota exceeded")
    {
        let detail = safe_upstream_error_excerpt(low);
        if detail.is_empty() {
            "上游供应商账户余额不足。请在后台为该模型线路充值，或切换到其他可用线路。".into()
        } else {
            format!(
                "上游供应商账户余额不足，请为该模型线路充值或切换线路。上游原话：{detail}"
            )
        }
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
        // 「模型系统」这个页面早就没了——控制台左侧现在是「模型线路 → 线路」
        // （admin-ui 的 NAV，group "routing"）。更要紧的是：这句话会原样发给**每一个**用户，
        // 而控制台要求 role=admin、nginx 还有一层 auth_request，普通用户点进去只会看到 404。
        // 把运维指令当成用户指引群发出去，等于告诉大部分人"去一个你打不开的页面"。
        // 所以两句话都给：管理员知道去哪改，普通用户知道自己现在能做什么。
        "上游密钥无效（这条线路的 key 不对，重发多少次都一样）。换个模型可以继续用；管理员请到控制台「模型线路 → 线路」更新该连接的 API Key。"
            .into()
    } else if status == 400 {
        let detail = safe_upstream_error_excerpt(low);
        if detail.is_empty() {
            "上游拒绝了请求（400），但没有返回更细原因。".into()
        } else {
            format!("上游拒绝了请求（400）：{detail}")
        }
    } else {
        // 兜底分支**必须带上上游原话**。
        //
        // 原来这里是一句光秃秃的"上游暂时不可用，请换个模型或稍后再试"。任何没被
        // 上面分支认出来的错误，都会被压成这一句 —— 上游说了什么全部丢掉。余额那次
        // 就是这么被埋掉的：上游明明写着"剩余 ＄0.0558，需要 ＄0.1343"，用户看到的
        // 却是"线路坏了，换个模型"，于是去查 IDE、查网络、查线路，唯独查不到真因。
        //
        // 加一条分支只能修一种已知错误；把原话带出来，才是让**下一种**没见过的
        // 上游错误也能被看懂。excerpt 已经做了 key 脱敏和 220 字截断。
        let detail = safe_upstream_error_excerpt(low);
        if detail.is_empty() {
            format!("上游暂时不可用（HTTP {status}），且没有返回原因。请换个模型或稍后再试。")
        } else {
            format!("上游暂时不可用（HTTP {status}）：{detail}")
        }
    }
}

#[cfg(test)]
fn friendly_upstream_for_test(status: u16, raw: &str) -> String {
    upstream_friendly_message(status, &raw.to_lowercase())
}

/// 把上游报错整理成可以给用户看的一句话。
///
/// 三件事都要做，少一件就漏：
///
/// 1. **URL**。`reqwest` 的 `Display` 会在末尾追加 ` for url (https://上游主机/…)`，于是
///    「路由不可用」这类 502 会把上游是谁一起告诉任何一个登录用户。health.rs 专门写了
///    base_url 不该出现在登录用户能打开的页面，还配了断言测试 —— 而这条路把它绕过去了。
/// 2. **密钥**。原来只剥三种 `sk-` 前缀，且**只替换第一处**：一句话里出现两个 key，第二个
///    照样发出去；`AIza…`（Google）、`Bearer …`、以及各家的长十六进制 token 一概不管。
/// 3. **循环到没有匹配**，不是替换一次就收工。
fn safe_upstream_error_excerpt(low: &str) -> String {
    let mut text = low
        .replace(['\r', '\n', '\t'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // URL 整段拿掉 —— 主机名本身就是要藏的东西，留下 scheme 之外的任何部分都没意义。
    loop {
        let Some(pos) = text.find("http://").or_else(|| text.find("https://")) else {
            break;
        };
        // 到第一个空白或右括号为止：reqwest 的格式是 `for url (https://…/path)`。
        let end = text[pos..]
            .find(|c: char| c.is_whitespace() || c == ')')
            .map(|off| pos + off)
            .unwrap_or(text.len());
        text.replace_range(pos..end, "[redacted-url]");
    }

    // 密钥形态。每一种都循环替换到没有匹配为止。
    for marker in ["sk-proj-", "sk_live_", "sk-", "aiza", "bearer ", "api-key "] {
        while let Some(pos) = text.find(marker) {
            let end = text[pos..]
                .char_indices()
                .find(|(i, c)| *i > marker.len() && !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_'))
                .map(|(i, _)| pos + i)
                .unwrap_or(text.len());
            // 至少吃掉 marker 本身，否则找到的还是同一处，会死循环。
            let end = end.max(pos + marker.len()).min(text.len());
            text.replace_range(pos..end, "[redacted-key]");
        }
    }

    // 兜底：剩下的长连续串（20 位以上的十六进制/base64 形态）当作凭据处理。上面几种前缀
    // 覆盖不到没有前缀的 token，而那种恰恰最难事后发现。
    let mut out = String::with_capacity(text.len());
    for word in text.split(' ') {
        let looks_like_secret = word.len() >= 20
            && word
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
            && word.chars().filter(|c| c.is_ascii_digit()).count() >= 4;
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(if looks_like_secret { "[redacted]" } else { word });
    }

    out.chars().take(220).collect()
}

/// 上游在说「我现在没有产能，等会儿再来」——不管它把这句话套在哪个状态码里。
///
/// 实测原文（deepseek-v4-pro，2026-08-19）：
///   `400 {"error":{"message":"请稍后重试，暂无可用渠道，或切换模型 (request id: …)",
///                  "type":"invalid_request_error"}}`
///
/// 它自己都在说「请稍后重试」，可外面套着 `invalid_request_error`，于是被判成「请求写错了」：
/// 网关据此 `break 'routes` 不再换线，客户端的 `_isRetryableAiError` 也不认 400。一个几秒后
/// 就会好的容量问题，两边同时把它变成了用户面前的死路。
///
/// 注意和「没有可用账号」（access_failure，→ 424）的区别：那是**账号/配置**坏了，重试无用；
/// 这里是**产能**暂时不足，正是重试和换线该处理的情况。
fn upstream_capacity_wording(low: &str) -> bool {
    low.contains("暂无可用")
        || low.contains("no available channel")
        || low.contains("请稍后重试")
        || low.contains("请稍后再试")
        || low.contains("try again later")
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
            //
            // 唯一的例外：上游把**容量**错误包在 400 里发出来（见 upstream_capacity_wording）。
            // 那是暂时的，照 400 发下去等于告诉客户端「别再试了」——正好和上游的原话相反。
            400 if upstream_capacity_wording(low) => StatusCode::SERVICE_UNAVAILABLE,
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
pub(crate) fn api_base(base: &str) -> String {
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
    /// 每线路开关：true = 关闭缓存计费，缓存读/写都不收钱（输入输出照常）。灰产/便宜渠道用。
    pub cache_disabled: bool,
    /// Optional admin blurb shown in the IDE picker's hover card.
    pub description: String,
    pub active: bool,
    pub sort: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub enabled_models: Vec<String>,
    /// Show this route's models under another route's label in the IDE picker.
    ///
    /// Display only: it feeds the `group` field of `/api/models` and nothing else. Requests
    /// resolve by model id (chat_completions), which never reads this column — so keys,
    /// base_url, billing mode, per-model prices and usage attribution all stay with this
    /// route. See migration 20260825.
    pub group_into: Option<uuid::Uuid>,
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
    /// 后台按模型手填的能力兜底：{ "model-id": { "contexts": [...], "max_output": n } }。
    /// **只在实时目录没有这个模型时生效**——目录是权威，这里是运维给目录漏网之鱼补的。
    pub model_caps: serde_json::Value,
    /// Per-model billing override, same shape as `model_prices`:
    ///   { "<model_id>": { "mode": "rate"|"per_call"|"free", "per_call_cents": N } }
    /// A `models` row is a CONNECTION holding many `enabled_models`, so billing_mode /
    /// per_call_cents alone could only switch a whole channel. This overrides per model.
    pub model_billing: serde_json::Value,
    /// Upstream wire protocol: "anthropic" (native /v1/messages) or "openai" (/chat/completions
    /// compat). When "anthropic", the gateway translates the OpenAI request/response ⇄ Anthropic.
    pub protocol: String,
    /// 把客户端拨的思考档位**原样**发给上游（含 `xhigh` / `max`），还是封顶在 `high`。
    ///
    /// 默认 false = 保持旧行为。见 `anthropic_effort_word` 里那段说明：封顶的理由是一条
    /// 从未被验证过的推断，所以做成每条线路可配，而不是继续写死在 match 里。
    #[sqlx(default)]
    pub effort_passthrough: bool,
    /// 这条线路是不是「Claude 强力版」承载线路。IDE 打开强力版开关的那一轮，
    /// 路由只在勾了这个标记的线路里挑。
    pub power_route: bool,
}

/// 后台按模型手填的能力兜底（contexts 升序去重、最多 5 档；max_output）。
///
/// 实时目录没收录时才轮到它。空 = 运维也没填，那就真的是"不知道"——
/// 这比代码里编一个数诚实，也比编一个数好查。
fn model_caps_override(model_caps: &serde_json::Value, model_id: &str) -> (Vec<i64>, Option<i64>) {
    let Some(entry) = model_caps.get(model_id) else {
        return (Vec::new(), None);
    };
    let mut contexts: Vec<i64> = entry
        .get("contexts")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_i64()).filter(|n| *n > 0).collect())
        .unwrap_or_default();
    contexts.sort_unstable();
    contexts.dedup();
    contexts.truncate(5); // 和实时侧同一个上限，UI 上不会因为来源不同而突然冒出七八档
    let max_output = entry
        .get("max_output")
        .and_then(|v| v.as_i64())
        .filter(|n| *n > 0);
    (contexts, max_output)
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
    // 实时优先：目录里的 output_modalities 含 image 就是画图模型，不用从名字猜。
    // 名字表会把 `claude-3-image-analysis` 这类"看图但不画图"的误判成画图模型
    // （它含 `-image`），而真正的画图模型只要命名里不带这几个词就漏判。
    if let Some(generates) = crate::model_catalog::generates_image(model_id) {
        return generates;
    }
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
    cache_disabled: bool,
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
        cache_disabled,
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

/// 落库加密的 context（= 列身份，绑进 AAD）。见 field_crypto.rs。
const MODEL_KEY_CTX: &str = "models.api_key";

/// 取出一条线路的上游 api_key 明文。存的是密文（`fc1:...`）或遗留明文，这里统一解开。
///
/// 解不开（密钥没配却是密文、或密钥不对）返回空串：空 Bearer 会让上游干净地回 401，
/// 好过把一段 `fc1:...` 当令牌发出去，也好过 panic 掉一条不相关的请求。
pub(crate) fn model_key(stored: &str) -> String {
    crate::field_crypto::decrypt(stored, MODEL_KEY_CTX).unwrap_or_default()
}

pub(crate) fn allowed_ids(m: &Model) -> Vec<String> {
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

/// 把上游返回的 body 解成 `(文本, usage)`——SSE 和普通 JSON 都认。
///
/// 为什么这里也要改流式：中转（Sub2API 这类）对**同步**请求是整段生成完才回，用户控制台
/// 里这些请求的类型写着"同步"，而本网关日志侧量到的 upstream_header_ms 是 8~40 秒、
/// first_upstream_chunk_after_headers_ms 恒为 0——正是那个形状。
///
/// usage 必须一起捞回来：视觉那条路径要靠它计费，丢了就是**按 0 结账**（本文件里另有一段
/// 注释记着这条路曾经被白嫖过）。SSE 的 usage 在最后一帧，所以请求侧要带
/// `stream_options.include_usage = true`，这里取最后见到的那个。
fn text_and_usage_from_body(body: &str) -> (String, Option<serde_json::Value>) {
    let mut text = String::new();
    let mut usage: Option<serde_json::Value> = None;
    let mut saw_frame = false;
    for line in body.lines() {
        let Some(data) = line.trim().strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data == "[DONE]" {
            saw_frame = true;
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(data) else {
            continue;
        };
        saw_frame = true;
        if let Some(t) = v["choices"][0]["delta"]["content"].as_str() {
            text.push_str(t);
        } else if v["type"] == "content_block_delta" {
            if let Some(t) = v["delta"]["text"].as_str() {
                text.push_str(t);
            }
        }
        if v.get("usage").map(|u| !u.is_null()).unwrap_or(false) {
            usage = v.get("usage").cloned();
        }
    }
    if saw_frame {
        return (text, usage);
    }
    // 中转无视了 stream:true，回的是普通 JSON。没有这条兜底，那些线路会整个失效。
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        let t = v.pointer("/choices/0/message/content")
            .and_then(|x| x.as_str())
            .or_else(|| v.pointer("/content/0/text").and_then(|x| x.as_str()))
            .unwrap_or("")
            .to_string();
        return (t, v.get("usage").cloned());
    }
    (String::new(), None)
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
        // SSE：同步请求在中转那边是整段生成完才回，见 text_and_usage_from_body 上的注释。
        "stream": true,
        "stream_options": { "include_usage": true },
        "messages": [
            {
                "role": "system",
                "content": "You are a professional software UI localization engine. Return ONLY valid JSON. Translate UI strings accurately and naturally. Preserve placeholders like {name}, {count}, {path}, punctuation that belongs to variables, product names (Mr. Day One, Git, MCP, Skills), code identifiers, file paths, shortcuts, and HTML/Markdown markers. Keep keys unchanged. Do not add explanations."
            },
            {
                "role": "user",
                "content": format!(
                    "Translate this Mr. Day One UI language pack from {} to locale {}. Return JSON exactly as {{\"translations\":{{\"key\":\"translated text\"}}}}. Entries JSON:\n{}",
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
        .header("Authorization", format!("Bearer {}", model_key(&m.api_key)))
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
    let (content, _usage) = text_and_usage_from_body(&text);
    if content.trim().is_empty() {
        return Err(format!("{} / {} 返回空内容", m.label, model_id));
    }
    let parsed = json_object_from_model_text(&content)
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
    // 总字节封顶。逐项限制（700 × 900）合起来仍有约 630KB 会被原样送进上游 —— 每次
    // 缓存未命中都是一次这么大的输入，而这条路不计费。UI 文案实际远小于此；超了就拒，
    // 而不是照单发给上游。
    const MAX_ENTRIES_BYTES: usize = 128 * 1024;
    let total_bytes: usize = entries.iter().map(|(k, v)| k.len() + v.len()).sum();
    if total_bytes > MAX_ENTRIES_BYTES {
        return Err(AppError::bad("entries 总量过大"));
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

    // 扇出封顶：最多试 2 条线路（按管理台的 sort 顺序，即运营方自己排在最前面的两条）。
    //
    // 失败时这里会遍历**每一条**线路 × 每一个 model_id，逐个打上游（用运营方的 key，
    // 不计费）。而失败是可以稳定构造的 —— 700 项的翻译输出超过任何上游的输出上限，
    // 回包必然截断、解析失败。于是「每小时 40 次 miss 配额」被放大成「40 × 线路数 ×
    // 模型数」次真实上游调用。翻译是机械活，靠前的线路试不出来，再沿目录往下试只是
    // 烧钱；试不出就直接落到 public_translate 兜底。
    const MAX_ROUTES_TRIED: usize = 2;
    let mut failures: Vec<String> = Vec::new();
    for m in models.iter().take(MAX_ROUTES_TRIED) {
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
#[derive(serde::Deserialize)]
pub struct GroupReq {
    /// The route to file this one under, or null to ungroup.
    pub group_into: Option<uuid::Uuid>,
}

/// `POST /api/admin/models/:id/group` — show this route's models under another's name.
///
/// Display only. Nothing is copied or moved: the route keeps its own key, base_url,
/// billing mode and per-model prices, and requests keep resolving by model id exactly as
/// before (chat_completions never reads this column). Ungrouping is the same call with
/// null, and it restores the previous display exactly because nothing else was changed.
pub async fn admin_group(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<GroupReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;

    if let Some(target) = req.group_into {
        if target == id {
            return Err(AppError::bad("不能把一条线路分组到它自己"));
        }
        let exists: Option<uuid::Uuid> = sqlx::query_scalar("SELECT id FROM models WHERE id = $1")
            .bind(target)
            .fetch_optional(&state.db)
            .await?;
        if exists.is_none() {
            return Err(AppError::bad("目标线路不存在"));
        }
        // 目标本身已经被分组到别处时拒绝：客户端只解析一跳，允许链式只会让人以为
        // A 会显示在 C 下面，而实际显示在 B 下面。
        let target_grouped: Option<Option<uuid::Uuid>> =
            sqlx::query_scalar("SELECT group_into FROM models WHERE id = $1")
                .bind(target)
                .fetch_optional(&state.db)
                .await?;
        if matches!(target_grouped, Some(Some(_))) {
            return Err(AppError::bad("目标线路自己已经分到别的组里了，先把它取消分组"));
        }
        // 反过来也一样：把 A 分到 B，而 B 已经分到 A，就成了环。
        let has_children: i64 = sqlx::query_scalar(
            "SELECT count(*)::bigint FROM models WHERE group_into = $1",
        )
        .bind(id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
        if has_children > 0 {
            return Err(AppError::bad("这条线路下面还挂着别的线路，先把它们取消分组"));
        }
    }

    let done = sqlx::query("UPDATE models SET group_into = $2 WHERE id = $1")
        .bind(id)
        .bind(req.group_into)
        .execute(&state.db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(AppError::bad("线路不存在"));
    }

    Ok(Json(json!({ "ok": true, "group_into": req.group_into })))
}

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
                "model_caps": m.model_caps,
                "power_route": m.power_route,
                "model_billing": m.model_billing,
                "protocol": m.protocol,
                "effort_passthrough": m.effort_passthrough,
                // Display grouping only — see `group_into` on the struct.
                "group_into": m.group_into,
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

/// 面值分母：卖出的 $1.00 额度对应多少上游真实成本美元（原先硬编码的 6.63）。
/// 现在唯一定义在 app_settings 表里，见 `settings.rs`——管理台改一次，服务端测算、
/// 两个管理页和 IDE 客户端同时跟着变，不会各说各话。
fn user_quota_raw_usd_per_visible_usd() -> f64 {
    crate::settings::raw_usd_per_visible_usd()
}
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
    let quota_raw_usd = visible_quota_usd * user_quota_raw_usd_per_visible_usd();
    let provider_usd_capacity = quota_raw_usd / multiplier;
    let channel_cost_cny = provider_usd_capacity / usd_per_cny;
    let profit_cny = sales_cny - channel_cost_cny;
    let margin_percent = profit_cny / sales_cny * 100.0;
    let break_even_multiplier = quota_raw_usd / (sales_cny * usd_per_cny);
    let target_cost_ratio = 1.0 - target_margin_percent / 100.0;
    let target_multiplier = break_even_multiplier / target_cost_ratio;
    let target_sales_cny = channel_cost_cny / target_cost_ratio;
    let safe_visible_quota_usd = sales_cny * usd_per_cny * target_cost_ratio * multiplier
        / user_quota_raw_usd_per_visible_usd();
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

    // 缓存价三级：管理员手填 > 实时目录的真实价 > 按输入价推算。
    //
    // 推算（×0.1 / ×1.25）以前是唯一来源，实测偏得很远：deepseek-v4-flash 缓存读真实
    // 0.0123 而推算 0.0061、glm-5 真实 0.12 而推算 0.06——都少算一半。少算缓存读价意味着
    // 按更便宜的价估成本、实际多付，而且账面上完全看不出来。
    let (cache_read_price, cache_creation_price) =
        cache_prices_for(&model, model_id, input_price);
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
        model.cache_disabled,
    );
    let calls = req.calls as f64;
    let provider_usd_total = provider_usd_per_call * calls;
    let channel_cost_cny = provider_usd_total / usd_per_cny;
    let billed_raw_usd = billed_cents_per_call as f64 / 100.0 * calls;
    let visible_quota_usd = billed_raw_usd / user_quota_raw_usd_per_visible_usd();
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
        "quota_raw_usd_per_visible_usd": user_quota_raw_usd_per_visible_usd(),
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
        "quota_raw_usd_per_visible_usd": user_quota_raw_usd_per_visible_usd(),
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
    #[serde(default)]
    pub cache_disabled: Option<bool>,
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
    // per_call_micro_usd 必须一起写进去。这一列是 20260806_conn_per_call_micro 后加的：
    // ModelReq 加了字段、admin_update 也读了，唯独这条 INSERT 漏掉，于是新建连接时填的
    // 每次调用费被**静默丢弃**，落库永远是 0（clippy 报的 "field is never read" 就是它）。
    //
    // 单独看不会立刻漏账：新建的连接 enabled_models 是空的，还serve不了流量，而后续
    // 启用模型要走 admin_update，那条路上有零费率闸门。但运营填了价、保存成功、价没了，
    // 下一次编辑还得重填一遍——而且一旦没注意到，闸门看到的就是 0。
    let id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO models (label, provider, base_url, model_id, api_key, rate, input_price, output_price, description, sort, billing_mode, per_call_cents, cache_read_price, cache_create_price, per_call_micro_usd, cache_disabled) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16) RETURNING id",
    )
    .bind(req.label.trim())
    .bind(req.provider.unwrap_or_default())
    .bind(req.base_url.trim().trim_end_matches('/'))
    .bind(req.model_id.unwrap_or_default().trim())
    .bind(crate::field_crypto::encrypt(req.api_key.trim(), MODEL_KEY_CTX))
    .bind(req.rate.unwrap_or(1.0).max(0.0))
    .bind(req.input_price.unwrap_or(0.0).max(0.0))
    .bind(req.output_price.unwrap_or(0.0).max(0.0))
    .bind(req.description.unwrap_or_default().trim())
    .bind(req.sort.unwrap_or(0))
    .bind(bmode)
    .bind(req.per_call_cents.unwrap_or(0).max(0))
    .bind(req.cache_read_price.unwrap_or(0.0).max(0.0))
    .bind(req.cache_create_price.unwrap_or(0.0).max(0.0))
    .bind(req.per_call_micro_usd.unwrap_or(0).max(0))
    .bind(req.cache_disabled.unwrap_or(false))
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
        .header("Authorization", format!("Bearer {}", model_key(&m.api_key)))
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
    // 每个模型的**实时能力**，和 id 一起回给后台。
    //
    // 以前这里只回 id，于是后台配一条线路时，上下文、价格、缓存价、思考档位全靠管理员
    // 自己查文档手填——填错了没人知道，填漏了就掉到"连接兜底价"。而这些值网关这边已经
    // 实时抓着了（model_catalog），不给后台看纯粹是浪费。
    //
    // `source`: "live" = 实时目录里有这一款；"static" = 目录没收录，仍走硬编码兜底，
    // 后台据此提示管理员"这一款需要你自己填价"。
    let capabilities: serde_json::Map<String, serde_json::Value> = ids
        .iter()
        .map(|id| {
            let entry = crate::model_catalog::lookup(id);
            let value = match &entry {
                Some(e) => json!({
                    "source": "live",
                    "contexts": e.contexts,
                    "max_output": e.max_output,
                    "efforts": e.efforts,
                    "default_effort": e.default_effort,
                    "input_price": e.input_price,
                    "output_price": e.output_price,
                    "cache_read_price": e.cache_read_price,
                    "cache_write_price": e.cache_write_price,
                    "accepts_image": crate::model_catalog::accepts_image(id),
                    "generates_image": crate::model_catalog::generates_image(id),
                }),
                None => json!({ "source": "static" }),
            };
            (id.clone(), value)
        })
        .collect();
    Ok(Json(json!({
        "models": ids,
        "enabled": m.enabled_models,
        "capabilities": capabilities,
        // 目录整体抓到了多少条。0 = 这台机器还没抓到过（刚启动/目录源不可达），
        // 后台该显示"能力数据暂不可用"而不是让管理员以为这些模型都没有能力信息。
        "catalog_size": crate::model_catalog::len(),
    })))
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
    pub cache_disabled: Option<bool>,
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
    /// { raw_model_id → {"contexts":[...],"max_output":n} }：目录没收录时的手填兜底。
    pub model_caps: Option<serde_json::Value>,
    pub power_route: Option<bool>,
    pub model_billing: Option<serde_json::Value>,
    /// "anthropic" | "openai" — upstream wire protocol for this connection.
    pub protocol: Option<String>,
    /// 思考档位直通：开启后 `xhigh` / `max` 原样发给上游，关闭时封顶在 `high`。
    /// 见 `anthropic_effort_word`——封顶的理由从来没被真实探测验证过，所以做成开关。
    pub effort_passthrough: Option<bool>,
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
        _ => m.api_key, // 没传就沿用原值（此时已是密文；encrypt 对已加密值幂等）
    };
    // 新传的是明文 → 加密；沿用的旧值已是密文 → 原样透过。见 field_crypto::encrypt。
    let api_key = crate::field_crypto::encrypt(&api_key, MODEL_KEY_CTX);
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
    // ...but only for a route that will actually SERVE traffic. This guard used to run on every
    // update, including a bare {"active": false}, so a per-call connection with unpriced models
    // could not be disabled — the operator was told to go price the models first, at the exact
    // moment the route was misbehaving and needed to come out of rotation. A disabled route bills
    // nothing, so unpriced models on it cannot cause an unbilled call; the guard has nothing to
    // protect. Deleting it was the only remaining escape, and that destroys the api key, the
    // enabled-model set, display names and every per-model price with it.
    if active && billing_mode == "per_call" && per_call_cents == 0 && per_call_micro_usd == 0 {
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
    // 没传就保持原值——和上面 protocol 一样的语义，别让一次只改价格的保存把开关关掉。
    let effort_passthrough = req.effort_passthrough.unwrap_or(m.effort_passthrough);
    sqlx::query("UPDATE models SET label=$1, provider=$2, base_url=$3, api_key=$4, rate=$5, active=$6, sort=$7, enabled_models=$8, input_price=$9, output_price=$10, description=$11, billing_mode=$12, per_call_cents=$13, model_names=$14, cache_read_price=$15, cache_create_price=$16, model_prices=$17, protocol=$18, model_billing=$20, per_call_micro_usd=$21, effort_passthrough=$22, model_caps=$23, power_route=$24, cache_disabled=$25 WHERE id=$19")
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
        .bind(effort_passthrough)
        .bind(req.model_caps.clone().unwrap_or_else(|| m.model_caps.clone()))
        .bind(req.power_route.unwrap_or(m.power_route))
        .bind(req.cache_disabled.unwrap_or(m.cache_disabled))
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

// ---------- IDE-facing: list active models (safe fields, no secrets) ----------
/// GET /api/models — active models for the IDE (no api_key / base_url leaked).
/// 缓存读 / 缓存写的单价，三级：管理员手填 > 实时目录的真实价 > 按输入价推算。
///
/// 抽成函数是因为**估价和展示必须读同一条规则**。这条规则本来只长在报价接口里，
/// 卡片要显示缓存价时若各写一遍，两处迟早会分叉——而分叉的表现是卡片上写一个价、
/// 账单上按另一个价扣，用户对不上账还查不出原因。
fn cache_prices_for(model: &Model, model_id: &str, input_price: f64) -> (f64, f64) {
    // 必须和 compute_cost 的三级完全同口径，否则这个预览/展示会显示一个和真实扣费不一样
    // 的缓存价——用户拿它核对账单时反而会以为扣错了。手填 > 目录真实倍率×计费输入价 > 推算。
    let live = crate::model_catalog::lookup(model_id);
    let live_in = live.as_ref().and_then(|e| e.input_price).filter(|p| *p > 0.0);
    let ratio = |cache: Option<f64>| match (cache, live_in) {
        (Some(c), Some(ci)) => Some(c / ci),
        _ => None,
    };
    let read = if model.cache_read_price > 0.0 {
        model.cache_read_price
    } else if let Some(r) = ratio(live.as_ref().and_then(|e| e.cache_read_price)) {
        input_price * r
    } else {
        input_price * CACHE_READ_FACTOR
    };
    let write = if model.cache_create_price > 0.0 {
        model.cache_create_price
    } else if let Some(r) = ratio(live.as_ref().and_then(|e| e.cache_write_price)) {
        input_price * r
    } else {
        input_price * CACHE_WRITE_FACTOR
    };
    (read, write)
}

pub async fn list_for_client(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE active = true ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await?;
    /*
     * Resolve the heading each route's models are filed under.
     *
     * One hop only, deliberately not a chain: A grouped into B grouped into C shows A
     * under B, not under C. Following the chain would need cycle detection, and "grouped
     * into a route that is itself grouped" is a configuration mistake worth leaving
     * visible rather than silently flattening.
     *
     * A dangling or self-referential target falls back to the route's own label, so a
     * half-configured grouping can never make a model disappear from the picker.
     */
    let label_of: std::collections::HashMap<uuid::Uuid, &str> =
        rows.iter().map(|m| (m.id, m.label.as_str())).collect();

    /*
     * 「Claude 强力版」是**按钮**，不是分组。
     *
     * 运维把一条线路勾成强力版之后，它原本会照常在选择器里多出一个以自己 label 为标题
     * 的分组，里面是一批和普通分组重名的模型 —— 用户看到的是"同一个模型出现两次"，
     * 而强力版本来只该是悬浮卡片右上角那个开关。
     *
     * 所以强力线路提供的 model id，只要**任何一条普通线路也提供**，就不再往列表里推。
     * 反过来，某个 id 只有强力线路提供时照常推 —— 上面那段注释说的"配错的分组绝不能让
     * 模型从选择器里消失"，对这里同样成立。
     *
     * 刻意**不做**全局按 model_id 去重：同一个模型挂在两条普通线路下、以不同价格出售，
     * 是运维在卖的东西（线上"特价开业福利"和"Claude"就是这么配的，sonnet 一个 10 一个 5）。
     * 去重会把那份价格选择一起铲掉。
     */
    // 运维指定的开箱默认模型，整批只读一次。
    let default_model_id = crate::settings::default_model();
    let power_ids: std::collections::HashSet<String> = rows
        .iter()
        .filter(|m| m.power_route)
        .flat_map(|m| allowed_ids(m))
        .collect();
    let plain_ids: std::collections::HashSet<String> = rows
        .iter()
        .filter(|m| !m.power_route)
        .flat_map(|m| allowed_ids(m))
        .collect();

    let mut list = Vec::new();
    for m in &rows {
        let group = m
            .group_into
            .filter(|target| *target != m.id)
            .and_then(|target| label_of.get(&target).copied())
            .unwrap_or(m.label.as_str());

        for mid in allowed_ids(m) {
            // 强力线路的条目：普通线路也有这个 id 就不推（用按钮去它那儿），
            // 只有它有才推（否则这个模型就再也选不到了）。
            if m.power_route && plain_ids.contains(&mid) {
                continue;
            }
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
            let (cache_read, cache_write) = cache_prices_for(m, &mid, input_price);
            // 上下文档位：实时目录 → 后台手填 → 空。在 json! 外面算好——宏里放不下块表达式。
            let context_windows: Vec<serde_json::Value> = {
                let mut tiers = official_contexts(&mid);
                if tiers.is_empty() {
                    // 目录漏网的模型（glm-5.3 这类，OpenRouter 里只有 5.1/5.2/5-turbo）
                    // 走运维在后台填的那份兜底。
                    tiers = model_caps_override(&m.model_caps, &mid)
                        .0
                        .into_iter()
                        .map(|t| (t, context_beta_header(&mid, t)))
                        .collect();
                }
                tiers
                    .into_iter()
                    .map(|(tokens, beta)| json!({ "tokens": tokens, "beta": beta }))
                    .collect()
            };
            list.push(json!({
                // Which route this model came from. Requests are resolved by model id
                // (chat_completions), not by this — it is here so a caller can tell two
                // routes exposing the same id apart.
                "conn_id": m.id,
                // Only the heading in the picker. Grouping changes this and nothing else.
                "group": group,
                // 这个 model id 有没有一条勾了强力版的线路。客户端据此决定要不要显示那个
                // 闪电按钮 —— 没有强力线路却把按钮画出来，用户点了只会撞上一句报错。
                //
                // 算的是**全局并集**，不是"当前这条线路是不是强力线路"：客户端拿到的是
                // 按 model id 索引的目录，同一个 id 可能挂在好几条线路下，逐条判断会得出
                // 一个取决于排序的随机答案。这个式子和派单那边的筛选条件必须是同一个。
                "power_route_available": power_ids.contains(&mid),
                // 新装客户端开箱选谁。运维在设置里指定（app_settings.default_model），
                // 没指定就一个都不标、客户端沿用「取列表第一个」的旧行为。
                //
                // 为什么要有这一位：客户端原来取的就是列表第一个，而那个顺序是路线的
                // enabled_models 按字母排出来的——于是每个新用户开箱都落在 claude-fable-5 上，
                // 而它是在售模型里硬失败率最高的一档（2026-08-19 实测 18.8%，对照
                // claude-opus-5 的 3.6%、glm-5.3 的 0%）。「模型老是用不了」对新用户来说
                // 是开箱即得的。
                //
                // 走配置而不是在客户端写死模型名：这张目录里用过的名字已经有 52 个，
                // 写死意味着每换一次默认都要发一版桌面端。
                "default": !default_model_id.is_empty() && mid == default_model_id,
                "provider": m.provider,
                "model_id": mid.clone(),
                "name": name,
                "price_cents": m.price_cents,
                // Expose the display price the admin configured for the IDE picker.
                // No api_key/base_url is leaked; just the model's visible input/output
                // price so the client can show exactly what the backend is using.
                "input_price": input_price,
                "output_price": output_price,
                // 缓存读 / 缓存写的单价。走的是 cache_prices_for —— 和报价接口**同一条**
                // 规则，卡片上写的价和账单上扣的价因此不可能分叉。
                "cache_read_price": cache_read,
                "cache_write_price": cache_write,
                "price_source": price_source,
                // `rate` **不下发**。它是运营方的加价倍率，本文件对它的定义原文就是
                // "the operator's margin, hidden from users"——而这个接口没有任何鉴权
                // （main.rs 的路由上没有 Claims 提取器，nginx 的 location / 也不拦），
                // 于是一条 curl 就能把它和加价前的 input_price/output_price 一起取走，
                // 两者相除即毛利率，还能顺带枚举 conn_id。
                //
                // 客户端拿它没用：13182 行只是把它读进定价对象，全仓库没有任何一处把它
                // 渲染出来。删掉对界面零影响。
                "description": m.description,
                // 每模型真实上下文窗口（tokens）：客户端上下文表和棘轮压缩阈值都靠它，
                // 不下发就只能靠客户端猜（GPT-5 曾被猜成 128K，白扔 3/4 窗口）。
                // 实时目录 → 后台手填 → 空。三级都拿不到就是明确的"不知道"，
                // 客户端按未知处理，绝不由网关编一个数。
                "context_window": official_context(&mid)
                    .or_else(|| model_caps_override(&m.model_caps, &mid).0.first().copied()),
                // Full native list so the client can show every window a model really offers,
                // instead of collapsing a genuine choice down to the default.
                // Output is the second half of a model's shape and was never sent, so the client
                // had no ceiling to budget against and the gateway clamped every model to one
                // number. null means unknown for this route — the client must not invent one.
                "max_output_tokens": official_max_output(&mid)
                    .or_else(|| model_caps_override(&m.model_caps, &mid).1),
                "context_windows": context_windows,
                // 这个模型真正支持的推理档位，实时抓的。
                //
                // **空数组是有意义的答案，不是"没查到"**：实测 glm-5 根本不吃档位这个概念，
                // deepseek-v4-flash 只支持 xhigh/high。客户端据此决定给不给这个模型显示档位
                // 选择器——以前它对所有模型一律显示 low/medium/high/max，于是用户选中一个
                // 该模型不支持的档位，上游要么拒要么静默降级，两种都没有任何提示。
                //
                // 目录里没有这个模型（中转商私有命名）时同样是空数组，客户端按"未知"处理，
                // 保持它原来的档位 UI，不要凭空断言这个模型不支持推理。
                "supported_efforts": crate::model_catalog::lookup(&mid)
                    .map(|e| e.efforts)
                    .unwrap_or_default(),
                "default_effort": crate::model_catalog::lookup(&mid)
                    .and_then(|e| e.default_effort),
                // 这一条能力信息是不是实时抓来的。false = 走了硬编码兜底，客户端和运维
                // 都该看得见这个区别，否则"实时化"到底生没生效永远说不清。
                // 这条能力信息从哪来，后台和客户端都该看得见：
                // live = 实时目录；admin = 运维在后台手填的兜底；unknown = 都没有。
                "capability_source": if crate::model_catalog::lookup(&mid).is_some() {
                    "live"
                } else if !model_caps_override(&m.model_caps, &mid).0.is_empty() {
                    "admin"
                } else {
                    "unknown"
                },
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

    /*
     * 并发闸。这条路由此前是**唯一**没有的。
     *
     * `/v1/chat/completions`、`/v1/responses`、`/v1/images/generations` 三条都拿了
     * InFlightGuard，只有这一条漏了 —— 而它恰恰是会替用户发起 gpt-5.5 视觉调用的那一条
     * （见 vision_preprocess）。没有闸意味着一个账号可以同时挂起任意多个 90 秒的上游
     * 请求：钱最终会扣，但扣之前运营方已经先垫付了全部并发量，而且 upstream 那边的
     * 速率配额是共享的，一个人就能把所有人卡住。
     *
     * 加密对这件事一点用都没有 —— 自己写脚本的人本来就绕开了浏览器。能拦的只有这里。
     */
    let _inflight_guard = InFlightGuard::acquire(&state, uid).await?;

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
    let (_pre_mode, _pre_percall, pre_free, _pre_micro) = effective_billing_micro(&model, &_pre_mid);
    // 这条路由此前**用订阅额度放行、却用钱包结算**：admit_billing 收到 quota_ok=true 就放行，
    // 而下面 bill(..., use_quota=false, ...) 只扣钱包。只有会员额度、钱包是 0 的用户，
    // 每一次调用都在把钱包记成负数——声明里的"扣订阅额度"在这条路由上从没发生过。
    let mut use_quota = false;
    if pre_free {
        // 免费池空了不再直接拒绝：改用会员额度/钱包继续。这道门要和另外两个准入口
        // 同一条规则，否则又会出现"同一个免费模型，从这个接口能用、从那个接口说没额度"。
        //
        // 判据必须是"这一次付得起吗"，不是"还剩不剩一点"：结算全额扣或一点不扣，
        // 余数永远清不空，`<= 0` 当天就再也为真不了（和另外两个入口同一个坑）。
        if !free_pool_covers_call(free_points_balance(&state, uid).await, _pre_micro) {
            let (plan, plan_exp, q_total, q_window, q_weekly_cap, q_week_used, credits): (
                String, Option<chrono::DateTime<chrono::Utc>>, i64, i64, i64, i64, i64,
            ) = sqlx::query_as(
                "SELECT plan, plan_expires_at, quota_total_cents, quota_window_cents, \
                        quota_weekly_cap_cents, quota_week_used_cents, credits_cents \
                 FROM users WHERE id = $1",
            )
            .bind(uid)
            .fetch_one(&state.db)
            .await?;
            let plan_active = plan != "none" && plan_exp.is_none_or(|e| e > chrono::Utc::now());
            let quota_ok = plan_active
                && q_total > 0
                && q_window > 0
                && (q_weekly_cap == 0 || q_week_used < q_weekly_cap);
            admit_billing(
                free_fallback_to_paid(), true, false, quota_ok, credits,
                plan_active, q_total, q_window, q_weekly_cap, q_week_used,
            )?;
            // 放行靠的是哪个池子，结算就得扣哪个。
            use_quota = quota_ok;
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
        vision_preprocess(&state, uid, &mut body).await;
    }

    let url = format!("{}/chat/completions", api_base(&model.base_url));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| AppError::internal(e.to_string()))?;
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", model_key(&model.api_key)))
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
        // 上游报错**不能原样透传**给用户。`data` 是上游的完整 JSON：里面可能有中转商的
        // 主机名、请求 URL，部分中转商还会把 Authorization 原样回显。同一份代码别处早就
        // 走了 safe_upstream_error_excerpt（剥 URL、剥各家 key 形态、循环剥到没有匹配，
        // 并且配了断言测试），只有这条路绕过去——而它对**任何登录用户**开放。
        // 502 在 error.rs 里是刻意不做统一脱敏的，所以必须在这里脱。
        let raw = data.to_string();
        return Err(AppError {
            status: axum::http::StatusCode::BAD_GATEWAY,
            msg: format!(
                "模型供应商错误 {}: {}",
                status.as_u16(),
                safe_upstream_error_excerpt(&raw.to_lowercase())
            ),
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
        model.cache_disabled,
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
    bill(&state, uid, model.id, cost, use_quota, &tokens, free_pool, free_micro).await;
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
    // 三列一起写：摘要（鉴权索引）、密文（回显）、明文（默认 None，见 api_key_store）。
    // 明文只在 API_KEY_KEEP_PLAINTEXT=1 时才有值——那个开关是留给"需要回滚到旧二进制"
    // 的极端情况的，默认新 key 从一开始就不落明文。
    let (digest, enc, plain) = crate::api_key_store::columns_for_new(&key);
    sqlx::query(
        "INSERT INTO api_keys (user_id, api_key, api_key_sha256, api_key_enc, label) \
         VALUES ($1,$2,$3,$4,$5)",
    )
        .bind(uid)
        .bind(plain)
        .bind(&digest)
        .bind(&enc)
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
        // 取密文列优先。明文清除之后 k.api_key 会是 NULL，直接选它会在解码时报错
        // （ApiKeyRow.api_key 是 String）——列名保持 api_key 以便沿用同一个行类型。
        "SELECT k.id, k.label, COALESCE(k.api_key_enc, k.api_key, '') AS api_key, \
                u.email, k.created_at, k.last_used_at \
         FROM api_keys k LEFT JOIN users u ON u.id = k.user_id ORDER BY k.created_at DESC LIMIT 200",
    )
    .fetch_all(&state.db)
    .await?;
    let list: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| json!({ "id": r.id, "label": r.label, "email": r.email, "key_masked": mask_key(&crate::field_crypto::decrypt_or_raw(&r.api_key, crate::api_key_store::API_KEY_CTX)), "created_at": r.created_at, "last_used_at": r.last_used_at }))
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
    // 这个接口必须把**同一把 key 原样还回去**（IDE 自动配置，跨设备跨会话要稳定），
    // 所以取的是密文列再解密，而不是哈希——哈希是单向的。
    // COALESCE：过渡期里旧行可能还只有明文，新行则只有密文。
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT COALESCE(api_key_enc, api_key) FROM api_keys \
         WHERE user_id = $1 AND label = 'ide-auto' AND COALESCE(api_key_enc, api_key) IS NOT NULL \
         ORDER BY created_at LIMIT 1",
    )
        .bind(uid)
        .fetch_optional(&state.db)
        .await?;
    let key = match existing {
        // decrypt_or_raw：存的是密文就解开，是过渡期遗留的明文就原样用。
        Some(stored) => {
            crate::field_crypto::decrypt_or_raw(&stored, crate::api_key_store::API_KEY_CTX)
        }
        None => {
            let k = gen_api_key();
            let (digest, enc, plain) = crate::api_key_store::columns_for_new(&k);
            sqlx::query(
                "INSERT INTO api_keys (user_id, api_key, api_key_sha256, api_key_enc, label) \
                 VALUES ($1, $2, $3, $4, 'ide-auto')",
            )
            .bind(uid)
            .bind(plain)
            .bind(&digest)
            .bind(&enc)
            .execute(&state.db)
            .await?;
            k
        }
    };
    Ok(Json(json!({ "api_key": key })))
}

/// A model id whose vision is weak/absent → route images through gpt-5.5 first.
fn needs_vision_help(model_id: &str) -> bool {
    // 实时优先：目录直接说了这个模型接不接受 image 输入，不用从名字猜。
    //
    // 判错的代价是真金白银：判成"不能看图"就要多走一次代看图（下面那段用 gpt-5.5 描述
    // 图片再转成文本，按 $5/M 输入计价），而且拿到的是二手描述、质量不如模型自己看。
    // 实测 qwen3.8-max 和 kimi-k3 都真的能看图，可它们名字里 gpt/claude/vision/-vl
    // 一个都没有——按下面这张名字表判，两款全判错。
    if let Some(accepts_image) = crate::model_catalog::accepts_image(model_id) {
        return !accepts_image;
    }
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
/// 一次代看图最多带几张。
///
/// 之前不限：请求体上限 12 MB，全部图片打包进**一次** gpt-5.5 调用，按 $5/M 输入计价。
/// 也就是说单个请求就能构造出一次很贵的上游调用。截断而不是拒绝 —— 正常人不会一次发
/// 八张以上，而超出的那部分对"让文本模型看懂这张图"这个目的也没有边际价值。
const MAX_VISION_IMAGES: usize = 8;
/// 每个账号每小时能触发多少次代看图。
///
/// 这是钱包之外的第二道闸。钱包只保证"最终会扣到他头上"，但运营方是**先垫付**的：
/// 上游那边的速率配额是所有用户共享的，一个账号狂刷就能把别人卡住，而且退款是事后的事。
const VISION_CALLS_PER_HOUR: i64 = 60;

async fn vision_preprocess(state: &AppState, uid: uuid::Uuid, body: &mut serde_json::Value) {
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
    let dropped = images.len().saturating_sub(MAX_VISION_IMAGES);
    images.truncate(MAX_VISION_IMAGES);

    // 每小时配额。超了就跳过识别、照常把图片剥成文本 —— 这条路径本来就是 best-effort，
    // 让整个对话失败比少一段图片描述糟糕得多。理由会写进注入的文本里，用户看得到。
    let over_budget = !vision_budget_ok(state, uid).await;
    // best-effort: have gpt-5.5 describe the images (may fail → we still strip them)
    let mut desc: Option<String> = None;
    let conns = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE active = true ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let vconn = if over_budget {
        None
    } else {
        conns.into_iter().find(|m| {
            allowed_ids(m)
                .iter()
                .any(|id| id.eq_ignore_ascii_case("gpt-5.5"))
        })
    };
    if let Some(vconn) = vconn {
        let mut vcontent = vec![json!({
            "type": "text",
            "text": "请详细、客观地描述这些图片的全部内容（文字、数据、图表、代码、界面元素、布局、配色等），让一个无法读图的模型也能据此完成工作。只输出描述本身。"
        })];
        vcontent.extend(images.clone());
        // SSE + include_usage：usage 必须一起回来，这条路径要靠它计费（丢了就是按 0 结账）。
        let payload = json!({
            "model": "gpt-5.5",
            "messages": [{ "role": "user", "content": vcontent }],
            "stream": true,
            "stream_options": { "include_usage": true }
        });
        if let Ok(client) = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()
        {
            let url = format!("{}/chat/completions", api_base(&vconn.base_url));
            if let Ok(r) = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", model_key(&vconn.api_key)))
                .json(&payload)
                .send()
                .await
            {
                if let Ok(_body) = r.text().await {
                    let (_vtext, _vusage) = text_and_usage_from_body(&_body);
                    let d = json!({
                        "choices": [{ "message": { "content": _vtext } }],
                        "usage": _vusage,
                    });
                    /*
                     * 立刻结账，**在返回之前**。
                     *
                     * 这一次调用花的是运营方自己的 key，此前完全不计费：调用方只要
                     * 挑一个非原生视觉的模型（deepseek-*、glm-*、grok-*、kimi、qwen
                     * 都算，见 needs_vision_help），随请求塞满图片，服务端就替他打一
                     * 次 gpt-5.5（$5/M 输入），而账单上什么都不会出现。
                     *
                     * 更糟的是顺序：这一步跑在下游请求**之前**，而下游一旦非 2xx，
                     * 外面那个 handler 会直接 return Err —— 那是在 bill() 之前。
                     * 于是「故意让下游报错」就成了一个稳定的白嫖姿势，而且这条路由
                     * 上没有 InFlightGuard，可以无限并发。
                     *
                     * 所以在这里就把账结掉，不依赖调用方后面还会不会走到计费点。
                     * 记账口径和 bill_compression_call 一致，单独打标便于对账。
                     */
                    bill_vision_call(state, uid, &vconn, d.get("usage")).await;
                    if let Some(s) = d["choices"][0]["message"]["content"].as_str() {
                        if !s.trim().is_empty() {
                            desc = Some(s.to_string());
                        }
                    }
                }
            }
        }
    }
    // 说清楚为什么少了东西。静默降级会让人以为模型看不懂图，转头去反复重试 ——
    // 那正好是配额已经吃紧时最不该发生的事。
    let note = match (&desc, over_budget) {
        (Some(d), _) if dropped > 0 => format!(
            "【图片内容（由 GPT-5.5 视觉识别，仅前 {} 张）】：\n{}\n（另有 {} 张未识别）",
            MAX_VISION_IMAGES, d, dropped
        ),
        (Some(d), _) => format!("【图片内容（由 GPT-5.5 视觉识别）】：\n{}", d),
        (None, true) => {
            "【图片】（本小时的图片识别次数已用完，未读取图片内容；稍后再试）".to_string()
        }
        (None, false) => "【图片】（视觉识别暂不可用，无法读取图片内容）".to_string(),
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
    // **全部来自实时目录，没有硬编码兜底。**
    //
    // 原来这里挂着一张按模型名字符串匹配的表，注释自己写着 "Keep in sync with provider docs"
    // ——也就是靠人记得同步。2026-08-16 拿在售的 13 款逐个对账，**6 款是错的**，
    // 最离谱的 deepseek-v4-flash 写 128K 而真实 1.05M（少 88%）。它不是安全网，是负资产：
    // 它会在实时数据缺席时**自信地给出一个错的数**，而错的数比没有数更难发现。
    //
    // 现在的降级链只剩"真实数据的不同新鲜度"：内存缓存 → 库里上次抓到的值 → 空。
    // 空 = 明确的"不知道"，由调用方和后台处理（管理员可在模型线路里手填），
    // 而不是拿一个编出来的数糊过去。
    match crate::model_catalog::lookup(model_id) {
        Some(entry) => entry
            .contexts
            .iter()
            .map(|&tokens| (tokens, context_beta_header(model_id, tokens)))
            .collect(),
        None => Vec::new(),
    }
}

/// 某个窗口要带哪个 beta header 才拿得到。
///
/// **这不属于"能力数据"，所以它不跟着上面一起删**：目录只说"这个窗口存在"，不说
/// "要带哪个头"。这是协议细节，只有 Anthropic 那一两个，且几乎不动。
fn context_beta_header(model_id: &str, tokens: i64) -> Option<&'static str> {
    let m = model_id.to_lowercase();
    // Sonnet 4 / 4.5：200K 默认，1M 在 beta 头后面。4.6 起 1M 是默认，不需要头。
    if tokens >= 1_000_000
        && m.contains("sonnet-4")
        && !m.contains("sonnet-4-6")
        && !m.contains("sonnet-4.6")
    {
        return Some("context-1m-2025-08-07");
    }
    None
}

/// The most output tokens a model will produce in one response.
///
/// The catalogue carried a context window and nothing else, so every part of the pipeline guessed:
/// a flat 128000 clamp with no model in scope (Haiku 4.5 caps at 64,000 and rejects it) and an
/// invented 8192 default. Context and output are different kinds of number — one is a budget
/// denominator, the other a wire parameter — and conflating them is what let both guesses stand.
///
/// None means "not known for this route", and every caller must fall back rather than invent one.
fn official_max_output(model_id: &str) -> Option<i64> {
    // 同 official_contexts：实时优先，静态兜底。输出上限和上下文是一个模型形状的两半，
    // 只实时化一半会让两个数来自不同年代的事实。
    // 纯实时，无硬编码兜底（同 official_contexts）。None = 不知道，调用方自己决定怎么办。
    crate::model_catalog::lookup(model_id).and_then(|e| e.max_output)
}

/// The DEFAULT native window — the first entry of official_contexts. Kept as the single number
/// that budgeting and michael-compression plan against, so adding a beta-gated larger option
/// never silently inflates anyone's budget.
fn official_context(model_id: &str) -> Option<i64> {
    official_contexts(model_id).first().map(|(tokens, _)| *tokens)
}
fn official_price(model_id: &str) -> Option<(f64, f64)> {
    // 实时目录优先。手写价表和 official_contexts 一个毛病，而且这半边直接是钱：
    // 实测 claude-sonnet-5 表里写 3/15、真实 2/10（多算 50%），而 opus-5、gpt-5.x、
    // qwen、kimi、deepseek、glm 这 8 款表里**根本没有**，一路掉到"连接价"靠人手填。
    //
    // 两项都拿到才用实时值：只有输入价没有输出价的话，混着用会拼出一个两个年代的价格。
    // 纯实时，无硬编码兜底。返回 None 时调用方会掉到"连接兜底价"，再没有就报
    // "该模型没有可用价格，请在连接编辑里填写单模型输入/输出价"——一个可操作的提示，
    // 比拿一张实测 13 款错 6 款的表去自信地算错钱强得多。
    let entry = crate::model_catalog::lookup(model_id)?;
    match (entry.input_price, entry.output_price) {
        (Some(input), Some(output)) => Some((input, output)),
        _ => None,
    }
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
    cache_disabled: bool,
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
    // 价格来自哪一层，决定了缓存价该跟谁：来自模型（每模型覆盖或官方目录）就按模型的输入价
    // 推导；只有当输入价本身就是连接级兜底时，连接级的缓存价才是同一层配置、才该生效。
    let (off_in, off_out, price_is_per_model) = if model_in > 0.0 || model_out > 0.0 {
        (model_in, model_out, true)
    } else if let Some((cat_in, cat_out)) = official_price(model_id) {
        (cat_in, cat_out, true)
    } else {
        (admin_in, admin_out, false)
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
    // 缓存价必须跟着**这个模型**的输入价走，不能用一个连接级常数盖住所有模型。
    //
    // 上游的缓存价本来就是输入价的固定倍数（读 0.1×、写 1.25×，5 分钟 TTL）。连接级那两列
    // 只能填一个数，而一条连接上同时跑 Opus($5)、Sonnet($5)、Fable($10)——线上就填着 3.75，
    // 那是 Sonnet 的写入价（1.25×3），于是 Opus 的缓存写入按 3.75 计，正确值是 6.25；Fable
    // 应当是 12.5。实测 30 天里仅这一项就少收约 $119，而缓存写入恰恰是单价最贵的一类 token。
    //
    // 缓存价三级（2026-08-18 用户要求补上中间那级）：
    //   ① 我手填了 → 用我的。但只在**和输入价同一配置层**时（!price_is_per_model）——
    //      连接级那两列是一条连接上所有模型共用的一个数，给每模型/目录定价的模型用它就是
    //      上面 $119 那个 bug，所以那种情况故意不认它。
    //   ② 我没填 → 用 OpenRouter 对**这个模型**的实时目录价。这是用户点名要的：
    //      「没写就用 openrouter 实时获取的」，比按输入价拍脑袋推算准得多。
    //      目录明确给 0（缓存读免费的模型）也照用，None 才算"目录没有这个数"。
    //   ③ 目录也没有 → 最后才按输入价 × 倍数推算兜底。
    let live_cache = crate::model_catalog::lookup(model_id);
    // 用目录的**真实倍率 × 你实际计费的输入价**，不是照搬目录的绝对缓存价。
    //
    // 关键：off_in 是**你收用户的价**（每模型覆盖 / 连接价），常常在目录成本价上加了价——
    // 线上 claude-opus-5 目录 $5、你收 $15（3×）。缓存价该跟着你的输入价走：照搬目录 $6.25
    // （那是按目录 $5 算的）会把加价模型的缓存按**成本价**收，少收好几倍，而缓存写入恰恰
    // 是单价最贵的一类 token。倍率取自目录（cache/input），比写死的 0.1/1.25 准——实测
    // deepseek 缓存读真实 0.2×、不是默认 0.1×。目录明确给 0（免费缓存）→ 倍率 0 → 收 0。
    let live_in = live_cache.as_ref().and_then(|e| e.input_price).filter(|p| *p > 0.0);
    let cache_ratio = |cache: Option<f64>| match (cache, live_in) {
        (Some(c), Some(ci)) => Some(c / ci),
        _ => None,
    };
    let read_price = if !price_is_per_model && cache_read_price > 0.0 {
        cache_read_price
    } else if let Some(ratio) = cache_ratio(live_cache.as_ref().and_then(|e| e.cache_read_price)) {
        off_in * ratio
    } else {
        off_in * CACHE_READ_FACTOR
    };
    let write_price = if !price_is_per_model && cache_create_price > 0.0 {
        cache_create_price
    } else if let Some(ratio) = cache_ratio(live_cache.as_ref().and_then(|e| e.cache_write_price)) {
        off_in * ratio
    } else {
        off_in * CACHE_WRITE_FACTOR
    };
    // 关闭缓存计费（每线路开关）：缓存读、缓存写都**不收钱**，普通输入照常。
    // 用户："我拉取的模型自带价格和缓存价……新增一个关闭缓存的开关，关闭的话价格一样、
    // 不收缓存钱。" 灰产/便宜渠道用——缓存那点钱干脆不算，输入输出价一分不动。
    let (read_price, write_price) = if cache_disabled { (0.0, 0.0) } else { (read_price, write_price) };
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
    // 走 api_key_store：先查哈希（唯一索引），查不到再查明文并顺手补齐该行。
    // 详见 api_key_store.rs —— 明文列是过渡期产物，清除由单独一次部署完成。
    match crate::api_key_store::lookup_user(&state.db, &token).await {
        Some(u) => Ok(u),
        None => crate::auth::user_from_jwt(&state.db, &state.cfg, &token).await
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
///
/// 从常量改成读设置（`app_settings.free_points_daily`，默认仍是 40）。改动只影响
/// **下一次日切之后**的发放：池子是按 用户 × 自然日 存下来的，SQL 的 CASE 只在
/// `free_points_date` 不是今天时才覆写，所以今天已经领过的用户不受影响。
pub fn free_points_daily() -> i64 {
    crate::settings::free_points_daily()
}

/// The pool is STORED in milli-点 (1 点 = 1000). Whole 点 could not express a small per-call
/// fee: the deduction rounded up, so any non-zero cost cost a full 点 and a 40-点 allowance
/// was always exactly 40 calls regardless of price. Integers throughout — no floats in the
/// money path — just three more decimal places.
pub const MILLI: i64 = 1_000;

pub fn free_milli_points_daily() -> i64 {
    crate::settings::free_milli_points_daily()
}

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
    .bind(free_milli_points_daily())
    .fetch_optional(&state.db)
    .await;
    row.ok().flatten().map(|(n,)| n).unwrap_or(0)
}

/// Spend from the daily pool, in milli-点. `micro_usd` is the call's real provider cost at
/// micro-USD resolution — either the per-model flat fee, or token cost converted up from
/// cents — so per-call and volume billing both land in the same conversion.
///
/// Returns what was **actually** deducted. That used to be a lie: the doc claimed it, but
/// the code returned the full requested `points` even when the pool floored at zero, so a
/// user with 2 点 left who made a 50 点 call had 50 recorded against them in
/// `model_usage.free_points_spent`. Usage history over-reported what people spent, and the
/// daily pool looked exhausted faster than it was.
///
/// Now one statement instead of two. The old version reset the daily grant in
/// `free_points_balance` and then decremented in a second round trip; between them another
/// request could read a balance that no longer existed by the time it spent. Folding the
/// reset into the same statement — behind `FOR UPDATE`, so concurrent spends on one row
/// serialise — makes the read and the write a single atomic step, halves the round trips on
/// a hot path, and lets `LEAST` report the true deduction.
async fn spend_free_points(state: &AppState, uid: uuid::Uuid, micro_usd: i64) -> i64 {
    let points = milli_points_for_micro_usd(micro_usd);
    if points <= 0 {
        return 0;
    }
    let row: Result<Option<(i64,)>, _> = sqlx::query_as(
        "WITH cur AS ( \
             SELECT id, \
                    CASE WHEN free_points_date IS DISTINCT FROM CURRENT_DATE \
                         THEN $3 ELSE free_points END AS avail \
             FROM users WHERE id = $1 FOR UPDATE \
         ) \
         UPDATE users u \
            SET free_points = GREATEST(0, cur.avail - $2), \
                free_points_date = CURRENT_DATE \
           FROM cur \
          WHERE u.id = cur.id \
         RETURNING LEAST(cur.avail, $2)",
    )
    .bind(uid)
    .bind(points)
    .bind(free_milli_points_daily())
    .fetch_optional(&state.db)
    .await;
    match row.ok().flatten() {
        Some((spent,)) => spent.max(0),
        None => 0,
    }
}

/// 免费额度用完之后，是否允许改用付费余额/会员额度继续跑免费模型。
///
/// 默认开。关掉它就回到"免费池空了直接 402"的老行为。用环境变量而不是 app_settings：
/// 这是运营侧的止血开关，不该需要一次迁移才关得掉；网关跑在 systemd/docker 下，
/// 与 MICHAEL_COMPRESSION_ENABLED 是同一套读法。
pub fn free_fallback_to_paid() -> bool {
    std::env::var("MICHAEL_FREE_FALLBACK_PAID").ok().as_deref() != Some("0")
}

/// 全额扣或一点不扣，原子的。返回真正扣掉的毫点（0 = 池子不够，一点没动）。
///
/// 为什么不做部分覆盖：一次调用被劈成"池子出一半、钱包出一半"之后，用量历史里那条
/// 记录就没法诚实地说清是谁付的钱，退款和对账都会跟着含糊。要么池子全出，要么走付费
/// 路径全出。
///
/// `FOR UPDATE` + 同一条语句里顺带补发当日额度：读和写之间不能有第二个请求插进来把
/// 余额吃掉——`spend_free_points` 的注释里记着这个教训。
async fn try_spend_free_points(state: &AppState, uid: uuid::Uuid, points: i64) -> i64 {
    if points <= 0 {
        return 0;
    }
    let row: Result<Option<(i64,)>, _> = sqlx::query_as(
        "WITH cur AS ( \
             SELECT id, \
                    CASE WHEN free_points_date IS DISTINCT FROM CURRENT_DATE \
                         THEN $3 ELSE free_points END AS avail \
             FROM users WHERE id = $1 FOR UPDATE \
         ) \
         UPDATE users u \
            SET free_points = CASE WHEN cur.avail >= $2 THEN cur.avail - $2 ELSE cur.avail END, \
                free_points_date = CURRENT_DATE \
           FROM cur \
          WHERE u.id = cur.id \
         RETURNING (CASE WHEN cur.avail >= $2 THEN $2 ELSE 0 END)::bigint",
    )
    .bind(uid)
    .bind(points)
    .bind(free_milli_points_daily())
    .fetch_optional(&state.db)
    .await;
    match row.ok().flatten() {
        Some((spent,)) => spent.max(0),
        None => 0,
    }
}

/// 一次调用要从免费池扣多少毫点。地板是 1：`free + 不配费用` 若扣 0，这个模型就不是
/// 免费而是**无限**——每日额度永远不动，也就永远没有"用完"这回事。
pub fn free_points_needed(micro_usd: i64) -> i64 {
    milli_points_for_micro_usd(micro_usd).max(1)
}

/// 准入门该问的问题：**这一次调用**免费池付得起吗——不是"池子里还剩不剩一点"。
///
/// 结算是全额扣或一点不扣（见 `bill()` 的 free 分支：`cur.avail >= want` 才减，否则
/// 一分不动）。于是 `balance > 0` 和结算问的不是同一件事：按次计费的免费模型每次 60
/// 毫点，池里剩 40 时结算一点都不扣，余数就永远挂在那儿直到明天日切——而门看到 40 > 0
/// 仍然放行，`admit_billing` 一路 `return Ok(true)`，它后面的"改走会员额度/钱包"和两条
/// 402 整段不可达。后果是双向的：用户要的"免费用完接着扣余额和订阅"到不了；没有余额
/// 的用户也永远收不到 402，欠款无上限地记进钱包。
///
/// 按次费用在准入时就是确定的，直接拿它比。按量计费的免费模型在上游回话之前算不出成本，
/// `free_points_needed(0)` 退到地板 1，等价于旧的 `> 0` —— 那一类不引入任何行为变化。
pub fn free_pool_covers_call(balance: i64, per_call_micro_usd: i64) -> bool {
    balance >= free_points_needed(per_call_micro_usd)
}

/// 亚分零头进位：`(这次要扣的整分, 留到下次的零头)`。
///
/// 钱包和会员额度都是整分，而免费模型常常按次计价到亚分（实测 $0.003/次 = 3000 micro-USD）。
/// 免费池空了之后这类调用落到付费路径，换算成整分是 0 —— 于是**两边都不扣**，模型变成
/// 真正的无限免费。四舍五入到 1 分是 3.3 倍溢价，不收是白送；累计到攒够一分再扣才两头都对。
///
/// 只处理零头：整分部分照旧走 requested_cost，这里不重复收。
pub fn carry_to_cents(carry: i64, add_micro_usd: i64) -> (i64, i64) {
    let total = carry.max(0).saturating_add(add_micro_usd.max(0));
    (total / MICRO_USD_PER_CENT, total % MICRO_USD_PER_CENT)
}

/// 三个准入口（chat / chat_completions / responses）共用的判定。
///
/// 分开写过一次，代价是同一个免费模型从 IDE 能用、从走 /v1/responses 的客户端被判成
/// "请先开通会员或充值额度"——同一份后台配置两个接口两种结果。这次连"免费池空了之后
/// 怎么办"也一起收拢，免得又漂开。
///
/// 返回 Err 就是拒绝；Ok(true) 表示这次由免费池付，Ok(false) 表示走付费路径。
pub fn admit_billing(
    fallback_enabled: bool,
    free_here: bool,
    free_pool_has_room: bool,
    quota_ok: bool,
    credits: i64,
    plan_active: bool,
    q_total: i64,
    q_window: i64,
    q_weekly_cap: i64,
    q_week_used: i64,
) -> Result<bool, AppError> {
    if free_here && free_pool_has_room {
        return Ok(true);
    }
    let paid_ok = quota_ok || credits > 0;
    if paid_ok && (!free_here || fallback_enabled) {
        // 免费池空了就改用会员额度/钱包继续跑。这正是用户要的："免费积分用光了，
        // 也可以消耗付费余额和订阅额度"。
        return Ok(false);
    }
    if free_here && !fallback_enabled {
        return Err(AppError {
            status: StatusCode::PAYMENT_REQUIRED,
            msg: "今日免费额度已用完，明天 0 点重置（或改用付费模型）".into(),
        });
    }
    let tail = if plan_active && q_total <= 0 {
        "总额度已用完"
    } else if plan_active && q_window <= 0 {
        "本时段额度已用完，请等待刷新（每 30 分钟）"
    } else if plan_active && q_weekly_cap > 0 && q_week_used >= q_weekly_cap {
        "本周额度已用完"
    } else {
        "请先开通会员或充值额度"
    };
    let msg = if free_here {
        format!("今日免费额度已用完，付费余额和会员额度也不可用（{tail}）。明天 0 点重置免费额度。")
    } else {
        tail.to_string()
    };
    Err(AppError {
        status: StatusCode::PAYMENT_REQUIRED,
        msg,
    })
}

/// 一笔结算的结局。resettle/恢复 worker 据此决定队列行是了结还是累加 attempts。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BillOutcome {
    /// 扣费成功（含免费池扣点、零费记账、付费提交）。
    Settled,
    /// 认领冲突：这笔已被扣过（模糊提交或并发恢复），跳过——**绝不双扣**。
    AlreadySettled,
    /// 结算失败，已（尝试）入队待恢复。
    Deferred,
}

/// 正常计费入口：**保持原签名不变**（4 个调用点与源断言零改动）。每次生成唯一 settlement_id，
/// 失败则入队后台恢复。返回值在 fire-and-forget 调用点被忽略，但 resettle 会用到。
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
    let settlement_id = uuid::Uuid::new_v4();
    let _ = bill_inner(
        state, uid, conn_id, cost, use_quota, tokens, free_pool, free_micro_usd, settlement_id, false,
    )
    .await;
}

/// 从队列重跑一笔失败结算：**复用**存下的 settlement_id（认领幂等，重跑绝不双扣），
/// 且 `from_recovery=true`——恢复时跳过免费分支、失败不重复入队（worker 记 attempts）。
pub(crate) async fn resettle(state: &AppState, row: &crate::settlement::UnsettledRow) -> BillOutcome {
    let tokens = BillTokens {
        prompt: row.prompt_tokens,
        completion: row.completion_tokens,
        cached: row.cached_tokens,
        cache_creation: row.cache_creation_tokens,
        model_name: row.model_name.clone(),
        estimated: row.estimated,
        request_id: row.request_id.clone(),
        mode: row.ide_mode.clone(),
        tool_turn: row.is_tool_turn,
        emitted_tool: row.emitted_tool.clone(),
    };
    bill_inner(
        state, row.user_id, row.conn_id, row.cost_cents, row.use_quota, &tokens, row.free_pool,
        row.free_micro_usd, row.settlement_id, true,
    )
    .await
}

async fn bill_inner(
    state: &AppState,
    uid: uuid::Uuid,
    conn_id: uuid::Uuid,
    cost: i64,
    use_quota: bool,
    tokens: &BillTokens,
    free_pool: bool,
    free_micro_usd: i64,
    settlement_id: uuid::Uuid,
    // 是否来自后台恢复重跑。它同时决定两件事：恢复时**跳过免费点分支**（免费扣点在
    // settled_requests 账本之外，重跑会双扣——见对抗审查 finding 1/3/5），以及失败时不重复入队。
    from_recovery: bool,
) -> BillOutcome {
    // 入队一笔失败结算的快照：把当前输入原样交给 settlement::queue（无 request_id 的它会自行不入队）。
    let queue_input = |stage: &'static str| crate::settlement::QueueInput {
        settlement_id,
        uid,
        conn_id,
        request_id: tokens.request_id.clone(),
        cost,
        use_quota,
        free_pool,
        free_micro_usd,
        prompt: tokens.prompt,
        completion: tokens.completion,
        cached: tokens.cached,
        cache_creation: tokens.cache_creation,
        model_name: tokens.model_name.clone(),
        estimated: tokens.estimated,
        mode: tokens.mode.clone(),
        tool_turn: tokens.tool_turn,
        emitted_tool: tokens.emitted_tool.clone(),
        stage,
    };

    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(error) => {
            tracing::error!(
                %error, %uid, %conn_id, cost, use_quota, free_pool,
                request_id = tokens.request_id.as_deref().unwrap_or("-"),
                model = %tokens.model_name,
                event = "billing_settlement_failed",
                "failed to begin billing transaction (call served, NOT charged)"
            );
            if !from_recovery {
                crate::settlement::queue(state, queue_input("begin_tx")).await;
            }
            return BillOutcome::Deferred;
        }
    };
    let requested_cost = cost.max(0);
    // Free models bill against the daily points pool, never quota or wallet. Done here rather
    // than at each call site so no biller can forget it: every path that charges a free model
    // lands in this one branch, and the model_usage row below is still written (so usage
    // history and the routing report stay complete — free is a payment source, not a
    // shadow-billing hole).
    // 恢复重跑时**不走免费分支**：队列行必然是付费路径失败（免费成功从不入队），而免费扣点
    // 用的是 &state.db 独立提交、从不写 settled_requests 账本，重跑会在账本之外再扣一次点
    // （跨日切池子回满时尤其明显），甚至升级成「先扣点后扣钱」。恢复一律走下面的付费认领路径。
    if free_pool && !from_recovery {
        // Prefer the model's own micro-USD fee (per-call billing, which may be sub-cent);
        // otherwise convert the token-billed cost up from whole cents. Volume billing and
        // per-call billing therefore both convert to 点 through one path.
        let micro = if free_micro_usd > 0 {
            free_micro_usd
        } else {
            requested_cost.max(0) * MICRO_USD_PER_CENT
        };
        // FLOOR (free_points_needed 里的 .max(1))：a 免费 model must always consume something,
        // even when no fee is configured. Without this, "free + no fee" spent 0 点 — so the
        // model was not merely free, it was UNCAPPED: the daily allowance never moved and
        // there was nothing to run out of, which defeats the entire pool.
        //
        // 全额扣或一点不扣：池子盖得住就由池子付；盖不住就**一点都不扣**，整笔落到下面的
        // 付费路径。此前这里无论如何都要扣（LEAST 到 0 为止）然后直接 return，于是免费额度
        // 见底那一刻起，免费模型既扣不到钱也不再拒绝——用量记着 0 点，钱包和会员额度一分
        // 不动。现在它会真的改用付费余额/会员额度继续，与准入门那条规则对上。
        let want = free_points_needed(micro);
        let spent = try_spend_free_points(state, uid, want).await;
        if spent > 0 {
            // cost_cents stays the REAL provider cost (so operator-side reporting is honest);
            // free_points_spent carries what the user actually paid, in 点.
            record_usage_row(state, uid, conn_id, requested_cost, spent, tokens).await;
            let _ = tx.rollback().await;
            return BillOutcome::Settled;
        }
        if !free_fallback_to_paid() {
            // 开关关掉时保持老行为：池子空了也只走池子，扣不到就记 0。
            record_usage_row(state, uid, conn_id, requested_cost, 0, tokens).await;
            let _ = tx.rollback().await;
            return BillOutcome::Settled;
        }
        // 落下去，按普通付费调用结算（quota → 钱包）。
    }
    // ── 幂等认领（付费路径专用）───────────────────────────────────────────────
    // 到这里说明这笔要走付费结算（不是免费池扣点）。往账本认领这个 settlement_id：
    //   · 正常调用 settlement_id 每次新生成 → 必然插入成功（1 行）→ 继续扣费；
    //   · 恢复重跑、且原始那次的提交其实已落库（「模糊提交」：commit 报错但数据提交了）→
    //     ON CONFLICT 命中（0 行）→ 立刻回滚返回 AlreadySettled，**绝不第二次扣钱**。
    // 认领和下面的扣减/记账在同一个事务里，于是「扣了钱」与「记了账本」共命运：
    // 一起提交或一起回滚，不会出现扣了钱却没账本、或有账本却没扣钱。
    match sqlx::query("INSERT INTO settled_requests (settlement_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(settlement_id)
        .execute(&mut *tx)
        .await
    {
        Ok(claim) if claim.rows_affected() == 0 => {
            tracing::info!(
                %uid, %conn_id, %settlement_id,
                "settlement already claimed (ambiguous-commit or concurrent recovery); skipping to avoid double charge"
            );
            let _ = tx.rollback().await;
            return BillOutcome::AlreadySettled;
        }
        Ok(_) => {}
        Err(error) => {
            tracing::error!(
                %error, %uid, %conn_id, cost, %settlement_id,
                request_id = tokens.request_id.as_deref().unwrap_or("-"),
                model = %tokens.model_name,
                event = "billing_settlement_failed",
                "failed to claim settlement id (call served, NOT charged)"
            );
            if !from_recovery {
                crate::settlement::queue(state, queue_input("claim")).await;
            }
            return BillOutcome::Deferred;
        }
    }
    // 亚分零头：免费模型常按次计价到亚分（$0.003 = 3000 micro-USD），而 requested_cost 是
    // 整分。掉到这里时它换算成整分往往是 0 —— 于是免费池空了之后两边都不扣，模型变成真正的
    // 无限免费。把零头累计起来，攒够一分才真的扣一分（carry_to_cents），余下的留到下一次。
    // 只对**从免费分支掉下来的**调用生效：普通付费模型的价格本来就是整分，不该被改口径。
    let mut carried_cents = 0i64;
    if free_pool && free_micro_usd > 0 && free_fallback_to_paid() {
        let prior: Option<(i64,)> =
            sqlx::query_as("SELECT micro_usd_carry FROM users WHERE id = $1 FOR UPDATE")
                .bind(uid)
                .fetch_optional(&mut *tx)
                .await
                .unwrap_or(None);
        // **只进位零头**，别把整分部分再收一遍。
        //
        // `requested_cost` 已经是这笔调用的整分费用（per_call 模式下 resolve_cost 直接返回
        // per_call_cents），而 `free_micro_usd` 是**同一笔费用**的 micro-USD 写法
        // （per_call_micro_usd，或没配 micro 时的 per_call_cents × 10_000）。整笔丢进
        // carry_to_cents 等于把它换算成分之后再加一次：
        //
        //     $0.05/次  → requested_cost 5¢ + carry 5¢ = 10¢     （2 倍）
        //     $0.003/次 → 每次 1¢（后台把任何非零费用抬到 ≥1 分）+ 每 3.34 次再 1¢ ≈ 4.3 倍
        //
        // 上面那段注释写的就是本意：「requested_cost 是整分……把**零头**累计起来」。
        // 代码没兑现这个不变量。唯一不出错的情形是连接级费用 < $0.005（换算成整分被
        // Math.round 舍成 0）——而那恰好是既有测试假设的场景，所以测试全绿也挡不住。
        //
        // free_fallback_to_paid 默认开，超收会经 split_fused_charge 记成真实负债。
        let carry_input = (free_micro_usd - requested_cost.saturating_mul(MICRO_USD_PER_CENT)).max(0);
        let (cents, rest) = carry_to_cents(prior.map(|(c,)| c).unwrap_or(0), carry_input);
        carried_cents = cents;
        if let Err(error) = sqlx::query("UPDATE users SET micro_usd_carry = $1 WHERE id = $2")
            .bind(rest)
            .bind(uid)
            .execute(&mut *tx)
            .await
        {
            tracing::error!(%error, %uid, carry_cents = rest, "failed to persist sub-cent carry");
        }
    }
    let requested_cost = requested_cost + carried_cents;
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
                tracing::error!(
                    %error, %uid, %conn_id, cost = requested_cost, use_quota,
                    request_id = tokens.request_id.as_deref().unwrap_or("-"),
                    model = %tokens.model_name,
                    event = "billing_settlement_failed",
                    "failed to lock balances for billing (call served, NOT charged)"
                );
                if !from_recovery {
                    crate::settlement::queue(state, queue_input("lock_balances")).await;
                }
                return BillOutcome::Deferred;
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
            tracing::error!(
                %error, %uid, %conn_id,
                quota_cents = charge.quota_cents, wallet_cents = charge.wallet_cents, actual_cost,
                request_id = tokens.request_id.as_deref().unwrap_or("-"),
                model = %tokens.model_name,
                event = "billing_settlement_failed",
                "failed to deduct fused quota and credits (call served, NOT charged; tx rolled back)"
            );
            if !from_recovery {
                crate::settlement::queue(state, queue_input("deduct")).await;
            }
            return BillOutcome::Deferred;
        }
    }
    if let Err(error) = sqlx::query(
        "INSERT INTO model_usage (user_id, model_id, cost_cents, prompt_tokens, completion_tokens, cached_tokens, cache_creation_tokens, model_name, estimated, request_id, ide_mode, is_tool_turn, emitted_tool, settlement_id) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
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
    .bind(settlement_id)
    .execute(&mut *tx)
    .await
    {
        tracing::error!(
            %error, %uid, %conn_id, actual_cost, %settlement_id,
            request_id = tokens.request_id.as_deref().unwrap_or("-"),
            model = %tokens.model_name,
            event = "billing_settlement_failed",
            "failed to insert billing settlement (tx rolled back; call served, NOT charged)"
        );
        if !from_recovery {
            crate::settlement::queue(state, queue_input("insert_usage")).await;
        }
        return BillOutcome::Deferred;
    }
    if let Err(error) = tx.commit().await {
        // 提交失败 ≈ 事务回滚（没扣到钱）——入队补扣。唯一的例外是「模糊提交」：COMMIT 其实
        // 在服务端落了库、只是回执丢了。那种情况账本里已有这条 settlement_id，恢复时会先查到
        // 它并跳过、绝不第二次扣——所以这里放心入队。
        tracing::error!(
            %error, %uid, %conn_id, actual_cost, %settlement_id,
            request_id = tokens.request_id.as_deref().unwrap_or("-"),
            model = %tokens.model_name,
            event = "billing_settlement_failed",
            "failed to commit billing transaction (call served; queued for idempotent recovery)"
        );
        if !from_recovery {
            crate::settlement::queue(state, queue_input("commit")).await;
        }
        return BillOutcome::Deferred;
    }
    BillOutcome::Settled
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
/// The Claude generation in a model id: `claude-opus-4-8` → 4.8, `claude-sonnet-5` → 5.0.
///
/// The thinking switch changed shape at 4.7 — from there on `budget_tokens` is a hard 400, and
/// before it `budget_tokens` is the only switch there is — so the split has to be a comparison,
/// not a list of version strings. 0 means "no recognisable version", which reads as newer than
/// this table and lands on the adaptive shape. Mirrors `_claudeGeneration` in the IDE client.
fn claude_generation(model_lower: &str) -> f64 {
    let bytes = model_lower.as_bytes();
    // Scan left to right, not family by family. Family-first order reads
    // `claude-3-7-sonnet-20250219` as sonnet-20250219 and returns the date as a version;
    // leftmost wins gives 3.7, matching the client's single-regex form. The two-digit caps are
    // the second guard: no release carries a three-digit major or minor, so a date can never be
    // mistaken for one wherever it appears.
    for start in 0..bytes.len() {
        let Some(family) = ["opus", "sonnet", "haiku", "fable", "mythos", "claude"]
            .into_iter()
            .find(|f| model_lower[start..].starts_with(f))
        else {
            continue;
        };
        let mut i = start + family.len();
        if matches!(bytes.get(i), Some(b'-' | b'_' | b'.')) {
            i += 1;
        }
        let major_start = i;
        while matches!(bytes.get(i), Some(c) if c.is_ascii_digit()) {
            i += 1;
        }
        if i == major_start || i - major_start > 2 {
            continue;
        }
        let major: f64 = model_lower[major_start..i].parse().unwrap_or(0.0);
        let mut minor = 0.0;
        if matches!(bytes.get(i), Some(b'-' | b'_' | b'.')) {
            let minor_start = i + 1;
            let mut j = minor_start;
            while matches!(bytes.get(j), Some(c) if c.is_ascii_digit()) {
                j += 1;
            }
            if j > minor_start && j - minor_start <= 2 {
                minor = model_lower[minor_start..j].parse().unwrap_or(0.0);
            }
        }
        return major + minor / 10.0;
    }
    0.0
}

fn anthropic_thinking(model: &str, effort: Option<&str>) -> Option<serde_json::Value> {
    anthropic_thinking_with_display(
        model,
        effort,
        std::env::var("MICHAEL_THINKING_DISPLAY").ok().as_deref(),
    )
}

/// `anthropic_thinking` 的纯函数版：`display` 由调用方给，不读进程环境。
///
/// 分出来是因为**测试改进程环境会串台**。原来那条测试用 set_var/remove_var 验证
/// 反悔开关，注释还写着「no other test reads this variable」——而每一个调用
/// `anthropic_thinking` 的测试都读它。cargo test 默认多线程跑，于是它把
/// MICHAEL_THINKING_DISPLAY=omitted 短暂灌给了并行的别的测试：实测 HEAD 上
/// `cargo test thinking` 连跑 5 次全红，红的还不是它自己。
/// 现在环境只在这一层读一次，判断逻辑本身可以被直接测，谁也不用改全局状态。
fn anthropic_thinking_with_display(
    model: &str,
    effort: Option<&str>,
    display_override: Option<&str>,
) -> Option<serde_json::Value> {
    if std::env::var("MICHAEL_ANTHROPIC_THINKING").ok().as_deref() == Some("0") {
        return None;
    }
    let m = model.to_lowercase();
    let eff = match effort {
        Some(e) if !e.is_empty() && e != "off" => e,
        // Absent is not the same as off: a caller that names no effort is asking for the model's
        // own default, and silently disabling thinking for them would be a different bug in the
        // opposite direction. Only an explicit "off" reaches the arm below.
        None => return None,
        // "off" is not the absence of a thinking key on every model. Opus 5 and Sonnet 5 run
        // ADAPTIVE thinking when `thinking` is omitted, so returning None here meant the cheapest
        // dial setting silently became the deepest one — and because the max_tokens floor below
        // is gated on a thinking key being sent, that turn also kept the bare 8192 default while
        // adaptive thinking ate it, truncating the visible answer. Say disabled out loud there.
        //
        // Only where the default is genuinely off (4.6/4.7/4.8, Sonnet 4.6 and older) is silence
        // the same as off. Fable and Mythos cannot be disabled at all — an explicit disable is a
        // 400 — so they keep returning None and the client hides the button.
        _ => {
            let default_is_on = claude_generation(&m) >= 5.0
                && (m.contains("opus") || m.contains("sonnet"))
                && !m.contains("fable")
                && !m.contains("mythos");
            return default_is_on.then(|| json!({"type":"disabled"}));
        }
    };
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
    // 按代次分流，而不是逐个版本号匹配。原来只点名了 4.6，于是 Sonnet 4.5 / Opus 4.5 /
    // 4.1 / 4.0 全都落到下面的 adaptive 分支——给一族只接受 budget_tokens 的模型发
    // {"type":"adaptive"}。IDE 侧同样按代次分流（_claudeGeneration），两边必须一致，
    // 否则客户端画的档位和网关真正发出去的形状对不上。
    if claude_generation(&m) > 0.0 && claude_generation(&m) <= 4.6 {
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
        // `display` decides whether any thinking TEXT comes back, and the right answer is a
        // property of the route, not of the docs — so it is measured, and it is switchable
        // without a deploy.
        //
        // History: a probe against changhuai.ai found bare adaptive returning 131 characters and
        // `summarized` returning 0, so the field was removed and a comment told the next person
        // not to re-add it without re-running the probe. That probe has now been re-run, against
        // this route (764fe78b) rather than that one, using the gateway's own stream telemetry:
        // EVERY completed Opus 5 stream reports thinking_utf8_chars=0 with bare adaptive. Which
        // is exactly Anthropic's documented default for this family — `display` is "omitted", and
        // omitted streams thinking blocks whose text is an empty string. The old measurement has
        // not been contradicted; it was taken on a different upstream and no longer describes
        // this one.
        //
        // The downside is bounded: the failure this replaces is "no thinking text", and the worst
        // the old measurement predicts is "no thinking text". Set MICHAEL_THINKING_DISPLAY=omitted
        // to go back without shipping a build, and read thinking_utf8_chars to see which won.
        let display = display_override.unwrap_or("summarized");
        if display == "omitted" || display.is_empty() {
            return Some(json!({"type":"adaptive"}));
        }
        return Some(json!({"type":"adaptive","display": display}));
    }
    None
}

/// 客户端拨的档位 → 发给上游的 `output_config.effort`。
///
/// ## 封顶该不该在——2026-08-13 实测过了，结论是「该在，但原来的理由是错的」
///
/// 原注释说「这是转卖渠道不是 Anthropic 直连，它不认识的 effort 词会返回**空 completion**
/// 而不是干净的 400」。这条推断在两个仓库里各写了一份、互相引用，谁也没真打过那一枪。
/// 现在打了：对 zyz 上游的 claude-opus-5 逐个发 `output_config.effort`，同一道题、
/// 非流式、只看返回：
///
/// ```text
///   effort=low     HTTP 200  思考 34 字符      ← 明显更浅
///   effort=medium  HTTP 200  思考 114 字符
///   effort=high    HTTP 200  思考 114 字符
///   effort=xhigh   HTTP 200  思考 141 字符
///   effort=max     HTTP 200  思考 142 字符
///   effort=banana  HTTP 200  思考 114 字符     ← 控制组
///   effort=ULTRA   HTTP 200  思考 161 字符     ← 控制组
///   effort=12345   HTTP 200  思考 365 字符     ← 控制组，比 xhigh 还"深"
/// ```
///
/// 没有一个返回空 completion，也没有一个返回 400 —— 原来的理由是错的。
/// 但结论反过来更硬：**`banana` 和 `high` 一模一样，`12345` 比 `xhigh` 还"深"**。
/// 这条上游对未知的 effort 值是「照收不误、一概不理」，把 low 之外的所有值都落到同一个
/// 默认档上，档位之间那点差异是采样噪声。换一道难题复测同样如此（low 3585 字符，
/// high 8858、xhigh 8303、max 4286——顺序都不单调）。
///
/// 也就是说：**在这条线路上 xhigh 不是一个真档位。** 把它透传过去不会更深，只会在转盘上
/// 多摆一个不起作用的位置——那正是用户抱怨的「思考深度和假的一样」，不是它的解药。
/// 所以封顶保留，而且现在是有实测支撑的保留。
///
/// 那为什么还留着开关：换一条**直连 Anthropic** 的线路（或上游哪天真的支持了），
/// 这段代码就不该拦着。开关默认关，管理员换线路时打开、按上面那套控制组复测一遍
/// （关键是 banana 那一组必须报错或明显不同，否则就是又一个假档位），再决定开不开。
/// 前端的 xhigh 按钮也得等这个开关真的打开、且复测通过之后再加。
fn anthropic_effort_word(requested: &str, passthrough: bool) -> &'static str {
    match (requested, passthrough) {
        ("low", _) => "low",
        ("high", _) => "high",
        // 只有这两个档位受封顶影响——它们是 `high` 之上的两级。
        ("xhigh", true) => "xhigh",
        ("max", true) => "max",
        ("xhigh", false) | ("max", false) => "high",
        _ => "medium",
    }
}


/// OpenAI /chat/completions body → Anthropic /v1/messages body.
#[cfg(test)]

fn oai_to_anthropic(body: &serde_json::Value) -> Result<serde_json::Value, String> {
    oai_to_anthropic_with_cache(body, true, false)
}

fn oai_to_anthropic_with_cache(
    body: &serde_json::Value,
    prompt_cache: bool,
    effort_passthrough: bool,
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
                // An explicit disable is the one thinking shape that means LESS, not more. It
                // used to fall through to the bare-toggle arm and come out as "high".
                if t.get("type").and_then(|v| v.as_str()) == Some("disabled") {
                    return "off";
                }
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
    // An explicit `{"type":"disabled"}` IS a thinking key, and it means the opposite of on. The
    // headroom floor below exists to give adaptive thinking room to stretch; handing it to a turn
    // that will not think just inflates the ceiling.
    let thinking_on = thinking
        .as_ref()
        .is_some_and(|t| t.get("type").and_then(|v| v.as_str()) != Some("disabled"));
    // Anthropic REQUIRES max_tokens. Map from OpenAI, else a generous default.
    let mut max_tokens = body
        .get("max_tokens")
        .and_then(|v| v.as_i64())
        .or_else(|| body.get("max_completion_tokens").and_then(|v| v.as_i64()))
        .filter(|n| *n > 0)
        // 8192 was invented, and it is what a thinking-off turn shipped to a model that can write
        // 128,000 — long answers came back cut in half with no error. Fall back to a real fraction
        // of the model's own ceiling; a client that names max_tokens still wins.
        .unwrap_or_else(|| official_max_output(model_str).map_or(8192, |cap| cap.min(32000)));
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
        // `xhigh` sits between high and max on Anthropic's ladder. 封顶开着的时候它被折成
        // "high"，就得拿 high 的余量（掉到 32000 会让第二深的档比它下面那档还浅——一个深度
        // 控件最不能出的错）；封顶关掉、xhigh 真的发出去了，它就该拿到 high 和 max 之间的
        // 余量，否则"更深的思考"配上"更小的写作空间"，深度会被输出上限反过来卡住。
        let floor = match effort {
            Some("max") => 64000,
            Some("xhigh") if effort_passthrough => 52000,
            Some("high") | Some("xhigh") => 40000,
            _ => 32000,
        };
        let min_mt = (budget + 8000).max(floor);
        if max_tokens < min_mt {
            max_tokens = min_mt;
        }
    }
    // Per model, not a blanket 128000 — Haiku 4.5 caps at 64,000 and would reject the flat value.
    let max_tokens = max_tokens.clamp(1, official_max_output(model_str).unwrap_or(128000));
    out.insert("max_tokens".into(), json!(max_tokens));
    if let Some(t) = &thinking {
        out.insert("thinking".into(), t.clone());
        // 深度旋钮：两个家族用两套，不能混。
        //
        // 旧家族（3.7 / 4.6，thinking.type=enabled）：**不发** output_config.effort。实测聚合
        // 上游（zyz）一旦收到 effort 就把思考流换成一句 "Compatibility reasoning summary."，
        // 完整思考全丢；只发 budget_tokens 时上游按原文回思考流。深度由 budget_tokens 控制。
        //
        // 新家族（4.7/4.8/5/Sonnet 5/Fable/Mythos，thinking.type=adaptive）：**必须发**
        // output_config.effort。这一家族直接拒绝 budget_tokens（400 的原文就是
        // "use thinking.type.adaptive and output_config.effort"），所以上面那条「不发 effort」
        // 一旦套到它身上，深度旋钮就一个都不剩了——adaptive 没有任何深度信号，模型每轮
        // 都只想一点点。用户看到的「思考没有实质内容」就是这么来的：不是没要思考，
        // 是要了思考却没告诉它想多深。
        if t.get("type").and_then(|v| v.as_str()) == Some("adaptive") {
            if let Some(e) = effort.filter(|e| !e.is_empty() && *e != "off") {
                out.insert(
                    "output_config".into(),
                    json!({ "effort": anthropic_effort_word(e, effort_passthrough) }),
                );
            }
        }
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
                // 细粒度工具流式（fine-grained tool streaming）。**不设它，Anthropic 会把工具
                // 入参的 JSON 攒完、校验合法之后才发**——对 write_file 这种把整份文件塞在
                // `content` 里的调用，用户就是盯着一张空的「正在写…」卡片等上几十秒到几分钟，
                // 而客户端那套逐 delta 刷新的实时预览再灵也没东西可显示。这不是我们的 bug，
                // 是 Anthropic 的默认行为，也正是「Claude 写代码要等很久才看得见」的机制成因。
                //
                // 打开之后 input_json_delta 逐段就发，本文件下面的转换会把每段原样转成
                // OpenAI 的 tool_calls[].function.arguments 增量，客户端 _streamWriteContent
                // 就能边收边把正文画进代码卡——和 Anthropic 自家产品里看到的一样。
                //
                // 代价是**中途的 JSON 可能不合法**（这正是缓冲要消除的东西）。客户端本来就按
                // 这个前提写的：增量扫描器容忍半截转义，_settleWritePreview 只在 JSON.parse
                // 成功时才定格，落盘前还有截断判据（finish_reason == "length"）与参数校验。
                // 注意这不是 beta：没有 anthropic-beta 头，就是工具定义上的一个布尔字段。
                a.insert("eager_input_streaming".into(), json!(true));
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
    let mut reasoning = String::new();
    let mut tool_calls: Vec<serde_json::Value> = Vec::new();
    if let Some(content) = av.get("content").and_then(|c| c.as_array()) {
        for b in content {
            match b.get("type").and_then(|t| t.as_str()) {
                Some("thinking") => {
                    if let Some(t) = b.get("thinking").and_then(|v| v.as_str()) {
                        reasoning.push_str(t);
                    }
                }
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
    if !reasoning.is_empty() {
        message.insert("reasoning_content".into(), json!(reasoning));
    }
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

/// Aggregate-only thinking telemetry. The converter never retains thinking text
/// beyond the already-required SSE forwarding path; these counters are solely
/// for diagnosing whether an upstream actually sent visible reasoning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ThinkingStreamTelemetry {
    nonempty_thinking_deltas: u64,
    thinking_utf8_chars: usize,
    /// 可见正文字符数。和 thinking_utf8_chars 一起，才能把「模型没思考」和「思考了但
    /// 文本没回来」分开：前者 output_tokens ≈ 正文量，后者 output_tokens 远大于正文量。
    visible_text_utf8_chars: usize,
    first_native_event_kind: &'static str,
    first_native_event_ms: Option<u64>,
    first_nonempty_thinking_delta_ms: Option<u64>,
    first_nonempty_text_delta_ms: Option<u64>,
    first_tool_use_start_ms: Option<u64>,
    first_nonempty_tool_delta_ms: Option<u64>,
}

impl Default for ThinkingStreamTelemetry {
    fn default() -> Self {
        Self {
            nonempty_thinking_deltas: 0,
            thinking_utf8_chars: 0,
            visible_text_utf8_chars: 0,
            first_native_event_kind: "absent",
            first_native_event_ms: None,
            first_nonempty_thinking_delta_ms: None,
            first_nonempty_text_delta_ms: None,
            first_tool_use_start_ms: None,
            first_nonempty_tool_delta_ms: None,
        }
    }
}

impl ThinkingStreamTelemetry {
    fn first_model_progress_ms(&self) -> Option<u64> {
        [
            self.first_nonempty_thinking_delta_ms,
            self.first_nonempty_text_delta_ms,
            self.first_tool_use_start_ms,
            self.first_nonempty_tool_delta_ms,
        ]
        .into_iter()
        .flatten()
        .min()
    }
}

struct AnthSse {
    buf: Vec<u8>,
    started_at: Instant,
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
    thinking_telemetry: ThinkingStreamTelemetry,
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
        Self::with_tool_argument_rules_started_at(model, tool_argument_rules, Instant::now())
    }

    fn with_tool_argument_rules_started_at(
        model: &str,
        tool_argument_rules: std::collections::HashMap<String, ToolArgumentRules>,
        started_at: Instant,
    ) -> Self {
        AnthSse {
            buf: Vec::new(),
            started_at,
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
            thinking_telemetry: ThinkingStreamTelemetry::default(),
        }
    }

    /// 故障签名：完整收流却只有思考、没有任何 text/tool_use 块，且 stop_reason 是
    /// end_turn（映射后为 "stop"）。官方 API 不会这样收尾——这是中转深思考超限丢块。
    fn thinking_only_end_turn(&self) -> bool {
        self.saw_thinking_block && !self.saw_answer_block && self.stop_reason == "stop"
    }

    /// 反过来的那半边丢块：**要了思考，却一个思考字符都没回，正文倒是好好的**。
    ///
    /// 上游（zyz 聚合）对 Claude 5 一族这件事是不确定的：同一个请求体、同样
    /// `thinking:{type:adaptive,display:summarized}`，几分钟内一次回 2000+ 字思考、
    /// 一次回 0。2026-08-13 的生产遥测里 claude-opus-5 ×2 与 claude-sonnet-5 ×1 都是
    /// thinking_utf8_chars=0，而同一时段 claude-opus-4-6 是 667；手工连打 6 次又全部
    /// 正常（934~2608 字）。所以这不是我们发错了参数，是上游在抽签。
    ///
    /// 单独把它认出来，是因为**这种响应绝不能进缓存**：它一旦被缓存，接下来一小时里
    /// 每一个相同请求都会重放这份没有思考的副本——"有时候不返回思考、然后一直不返回、
    /// 过一阵又好了"里的"一阵"，就是那条 3600 秒的 TTL。
    ///
    /// 注意**不要**顺手给这条线路记思考钳位：钳位是把思考预算调低，对"根本没思考"
    /// 只会更糟。这里只做两件事——不缓存、留一条可统计的日志。
    fn thinking_requested_but_none_returned(&self) -> bool {
        self.saw_answer_block && self.thinking_telemetry.thinking_utf8_chars == 0
    }

    /// 「思考块**开了**，里面却是空的」—— 这才是能归罪于线路的那一种。
    ///
    /// 和上面那条**故意分开**，因为两个用途对判据的要求不同：
    ///
    ///   · 上面那条服务**缓存排除**：任何零思考的响应都不该被缓存一小时反复重放，
    ///     不管零思考的原因是什么。宽一点是对的。
    ///   · 这条服务**线路降权**：只有"上游把思考吞了"才算这条线路的问题。而
    ///     adaptive 这一轮自己决定不想，是 Claude 5 一族的**正常行为**——一个 377 token
    ///     的澄清回复不思考再正常不过。
    ///
    /// 两者刚好被 `saw_thinking_block` 分开：被吞时 thinking 的 content_block 照常开
    /// （见 saw_thinking_block 的两个置位点），只是文本是空串；adaptive 决定不想时
    /// **一个 thinking 块都没有**。
    ///
    /// 不分开的代价是实拍过的：2026-08-19 给静音记号接上选路时用了上面那条宽判据，于是
    /// 每一个正常的不思考轮次都把一条**健康线路**降权 30 分钟，下一轮被迫换线，换到的
    /// 线路若不是原生 Anthropic 协议就补不上 display、思考文本变空串——"偶尔不出思考卡"
    /// 被这个修复本身放大成了"越用越不出"。
    fn thinking_swallowed_by_upstream(&self) -> bool {
        self.saw_thinking_block
            && self.saw_answer_block
            && self.thinking_telemetry.thinking_utf8_chars == 0
    }

    fn thinking_telemetry(&self) -> ThinkingStreamTelemetry {
        self.thinking_telemetry
    }

    /// 诊断用：上游到底**开没开**思考块。
    ///
    /// 这是把「模型这一轮没思考」和「思考块开了但文本是空串（display 的问题）」分开的
    /// 唯一判据 —— 两者的 thinking_utf8_chars 都是 0，日志里长得一模一样。线上 48 小时
    /// 里 ~330 条零思考流一次 `thinking_swallowed_by_upstream` 都没触发，只能推断没开块，
    /// 而推断不该当证据用：直接把它记下来。
    fn saw_thinking_block(&self) -> bool {
        self.saw_thinking_block
    }

    /// 诊断用：上游自报的输出 token 数。Anthropic 把思考算进 output_tokens，所以
    /// 「output_tokens 远大于可见正文字符数」= 确实思考了、只是文本没回来。
    fn output_tokens(&self) -> i64 {
        self.output_tokens
    }

    fn stop_reason_label(&self) -> &str {
        &self.stop_reason
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
            let event_elapsed_ms = self
                .started_at
                .elapsed()
                .as_millis()
                .min(u64::MAX as u128) as u64;
            if self.thinking_telemetry.first_native_event_ms.is_none() {
                self.thinking_telemetry.first_native_event_kind =
                    telemetry_anthropic_event_kind(ev.get("type").and_then(|t| t.as_str()));
                self.thinking_telemetry.first_native_event_ms = Some(event_elapsed_ms);
            }
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
                        self.thinking_telemetry
                            .first_tool_use_start_ms
                            .get_or_insert(event_elapsed_ms);
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
                                if !t.is_empty() {
                                    self.thinking_telemetry
                                        .first_nonempty_text_delta_ms
                                        .get_or_insert(event_elapsed_ms);
                                    self.thinking_telemetry.visible_text_utf8_chars +=
                                        t.chars().count();
                                }
                                self.saw_answer_block = true;
                                self.ensure_role(&mut out);
                                out.extend(self.chunk(json!({"content": t}), None));
                            }
                        }
                        Some("thinking_delta") => {
                            if let Some(t) = ev.pointer("/delta/thinking").and_then(|v| v.as_str())
                            {
                                self.saw_thinking_block = true;
                                if !t.is_empty() {
                                    self.thinking_telemetry.nonempty_thinking_deltas += 1;
                                    self.thinking_telemetry.thinking_utf8_chars += t.chars().count();
                                    self.thinking_telemetry
                                        .first_nonempty_thinking_delta_ms
                                        .get_or_insert(event_elapsed_ms);
                                }
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
                            if !pj.is_empty() {
                                self.thinking_telemetry
                                    .first_nonempty_tool_delta_ms
                                    .get_or_insert(event_elapsed_ms);
                            }
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
            // Anthropic 不单独报思考 token —— 思考算在 output_tokens 里，没有
            // completion_tokens_details 这一层。于是 IDE 那半句「思考 高 · 推理 N」在
            // Claude 线路上永远没数可显示，用户拨了深度看不到任何回执（"和假的一样"）。
            //
            // 这里我们**本来就在逐帧数思考字符**（thinking_utf8_chars，原本只进遥测日志）。
            // 把它一起报上去：字符不是 token，但它是这条线路上唯一真实、可核对的思考量，
            // 比一个永远不出现的数字有用得多。字段名单独取，别冒充 reasoning_tokens。
            "thinking_chars": self.thinking_telemetry.thinking_utf8_chars,
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
    let _uid: uuid::Uuid = match crate::api_key_store::lookup_user(&state.db, &token).await {
        Some(u) => u,
        None => crate::auth::user_from_jwt(&state.db, &state.cfg, &token).await
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
    let gateway_request_started_at = Instant::now();
    let request_id = ide_request_id(&headers)?;
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::unauthorized("缺少 API Key"))?;
    let uid: uuid::Uuid = match crate::api_key_store::lookup_user(&state.db, &token).await {
        Some(u) => {
            crate::api_key_store::touch_last_used(&state.db, &token).await;
            u
        }
        // Also accept the login JWT directly (the IDE authenticates with it).
        None => crate::auth::user_from_jwt(&state.db, &state.cfg, &token).await
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
    // Deliberately metadata-only: this records the requested thinking wire shape
    // without retaining prompts, messages, thinking text, or credentials.
    tracing::info!(
        request_id = request_id.as_deref().unwrap_or(""),
        model = %model_id,
        reasoning_effort = telemetry_reasoning_effort(&body),
        inbound_thinking_type = telemetry_thinking_type(&body),
        "thinking telemetry: inbound chat request"
    );

    let conns = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE active = true ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await?;
    // 「Claude 强力版」：IDE 打开那个开关时带 x-ide-power-route，这一轮只在运维勾了
    // power_route 的线路里挑。
    //
    // 是**筛选**不是排序：用户点了强力版就该走强力线路，退回普通线路等于把他的选择
    // 悄悄改掉——这正是本轮刚从思考档位里拿掉的那种行为。没有可用的强力线路时宁可
    // 明确报错，让人知道后台还没配。
    let want_power = headers
        .get("x-ide-power-route")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    let mut candidates: Vec<Model> = conns
        .into_iter()
        .filter(|m| allowed_ids(m).contains(&model_id))
        .collect();
    if want_power {
        let power: Vec<Model> = candidates.iter().filter(|m| m.power_route).cloned().collect();
        if power.is_empty() {
            return Err(AppError::bad(format!(
                "{model_id} 没有可用的强力版线路——请在后台把某条线路勾上「Claude 强力版」"
            )));
        }
        candidates = power;
    } else {
        // 没点强力版就别把人派到强力线路上。
        //
        // 这条线路从选择器里隐掉之后，它仍然留在普通请求的候选池里 —— 而挑主线路用的是
        // `candidates.first()`，顺序由 `ORDER BY sort, created_at` 决定。也就是说运维哪天
        // 把它的 sort 调前一格，所有普通 Claude 请求就会静默改走强力线路、按它计费，而
        // 界面上没有任何地方看得出来。强力版是**用户点出来的**，不是排序碰出来的。
        //
        // 唯一的例外是这个模型只有强力线路提供 —— 那 candidates 会空，退回去总比让一个
        // 选得到的模型发不出请求强。
        let plain: Vec<Model> = candidates.iter().filter(|m| !m.power_route).cloned().collect();
        if !plain.is_empty() {
            candidates = plain;
        }
    }
    let route_count = candidates.len();
    let primary_conn = candidates
        .first()
        .cloned()
        .ok_or_else(|| AppError::bad(format!("模型 {model_id} 不可用")))?;

    // Refill the 30-minute window + reset the weekly counter when due.
    sqlx::query(
        &format!("UPDATE users SET \
         quota_window_cents = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN LEAST(quota_window_cap_cents, quota_total_cents) ELSE quota_window_cents END, \
         quota_window_reset_at = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN now() + interval '{QUOTA_WINDOW_REFRESH}' ELSE quota_window_reset_at END, \
         quota_week_used_cents = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN 0 ELSE quota_week_used_cents END, \
         quota_week_reset_at = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN now() + interval '7 days' ELSE quota_week_reset_at END \
         WHERE id = $1"),
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
    // 取最便宜的那条免费线路的单次费用——和上面 `any` 同一个口径：只要有一条免费线路
    // 付得起，就还算"免费池能付"。0 = 按量计费，free_pool_covers_call 会退到地板 1。
    let free_call_micro = candidates
        .iter()
        .filter(|c| effective_billing(c, &model_id).2)
        .map(|c| effective_billing_micro(c, &model_id).3)
        .min()
        .unwrap_or(0);
    let free_pool_has_room = free_here
        && free_pool_covers_call(free_points_balance(&state, uid).await, free_call_micro);
    admit_billing(
        free_fallback_to_paid(), free_here, free_pool_has_room, quota_ok, credits,
        plan_active, q_total, q_window, q_weekly_cap, q_week_used,
    )?;

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
    // 总开关 MICHAEL_COMPRESSION_ENABLED，config.rs 里 fail-closed（缺省=关）。
    // 线上 **当前是开的**（容器里 MICHAEL_COMPRESSION_ENABLED=1）。
    //
    // 这里原先写着"发布前审查发现多处会破坏线上请求的缺陷，最严重的是
    // compression_write_back 把每条消息重写成 {role, content}，tool_calls /
    // tool_call_id 全部丢失，agent 模式会被上游直接拒收；开关打开前必须先修完"。
    //
    // 那个缺陷**已经修了**：write_back 现在对钉住段和逐字尾部都是 `.clone()` 原始
    // 消息对象，只有注入的摘要是新造的 system 消息，所以结构字段一个不丢。
    // `write_back_preserves_tool_call_structure` 用带 tool_calls + tool_call_id 的
    // agent 形状把这条不变量钉死了。
    //
    // 注释没跟着改，就成了最坏的一种：它告诉读代码的人"线上这个功能是不安全的、
    // 不该开"，而它其实已经开着并且是好的。谁照着这段注释去把开关关掉，等于无声地
    // 砍掉 1M/2M/5M 三档上下文。所以这里改成陈述现状，而不是留一句过期的警告。
    // 关着时 body 一个字节都不动这一点不变。
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
                        apply_michael_compression(&state, &mut body, &model_id, tier, uid, client_context_window(&headers)).await?;
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
    // A user send maps to exactly one upstream model request. A 502/503/504/429,
    // response-header timeout, or transport error is returned to the IDE immediately;
    // the gateway never replays the same billed prompt on another connection or route.
    // Failed routes still enter the short cooldown so the NEXT user send can prefer a
    // healthier same-model route when the admin has configured one.
    let model_name = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("该模型")
        .to_string();
    // 映射逻辑已提到模块级 `upstream_friendly_message`（测试要能直接调它）。
    let friendly_upstream = upstream_friendly_message;

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
    // Single-shot send. Billing only happens after a successful upstream response.
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
        // Either way the IDE hit its own header timeout before the gateway answered,
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
        // 这一轮到底要不要思考。只有要思考时，「会吞思考的线路」才算缺点——
        // 不要思考的请求走那条线路一点问题都没有，凭空排后面只会白白打乱轮换。
        let wants_thinking = body
            .get("thinking")
            .and_then(|t| t.get("type"))
            .and_then(|t| t.as_str())
            .is_some_and(|t| t != "disabled");
        for candidate in &candidates {
            let cooled = route_cooldown_remaining(candidate.id, now).is_some();
            // 要了思考却一个字都不回的线路：有别的同模型线路可走时排到后面。
            // 和冷却一样只是**重排**，不是排除——到期自动再探，上游恢复了就自己回来。
            let mutes = wants_thinking && route_mutes_thinking(candidate.id, now);
            if route_count > 1 && (cooled || mutes) {
                cooled_candidates.push(candidate);
            } else {
                ordered_candidates.push(candidate);
            }
        }
        ordered_candidates.extend(cooled_candidates);

        // 取到「答复过一次错误」允许的上限；真正能不能走到第二条，由每一轮末尾那句
        // `upstream_answered_with_error` 决定——卡死和发送出错仍然只发一次就收手。
        'routes: for candidate in ordered_candidates
            .into_iter()
            .take(CHAT_UPSTREAM_MAX_ROUTES_WHEN_ANSWERED)
        {
            // 这条线路是不是**完整地回了一个错误响应**。只有它为真时才允许换下一条。
            let mut upstream_answered_with_error = false;
            // protocol="anthropic" → native /v1/messages with translated OpenAI⇄Anthropic body;
            // else OpenAI-compat /chat/completions passthrough. Route ordering still prefers a
            // non-cooled line, but one inbound chat request selects exactly one line and sends once.
            let candidate_anthropic = candidate.protocol == "anthropic";
            let candidate_url = if candidate_anthropic {
                format!("{}/messages", api_base(&candidate.base_url))
            } else {
                format!("{}/chat/completions", api_base(&candidate.base_url))
            };
            let mut candidate_upstream_body = if candidate_anthropic {
                match oai_to_anthropic_with_cache(
                    &body,
                    route_supports_prompt_cache(candidate),
                    // 直通判据 = 线路手工开关 **或** 实时目录说这个模型真支持这一档。
                    //
                    // 默认封顶（xhigh/max → high）当初的理由是"转卖渠道可能不认识这个词、
                    // 会返回空 completion"。那条理由两个仓库的注释互相引用了很久，而
                    // 2026-08-16 直连实测（本网关在用的上游，claude-opus-4-8）：xhigh 和 max
                    // 都 HTTP 200、thinking 块正常返回——推断是错的。用户在界面上拨到"极限"，
                    // 请求里却发 high，那是网关替他改了主意。
                    //
                    // 目录没收录的模型仍然只看手工开关，行为一个字不变。
                    candidate.effort_passthrough
                        || body
                            .get("reasoning_effort")
                            .and_then(|v| v.as_str())
                            .is_some_and(|e| crate::model_catalog::supports_effort(&model_id, e)),
                ) {
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
            let wants_1m =
                candidate_anthropic && wants_1m_context(&model_id, &candidate_upstream_body);
            let candidate_anthropic_beta = candidate_anthropic
                .then(|| anthropic_beta_header(&candidate_upstream_body, wants_1m))
                .flatten();
            if candidate_anthropic {
                // The body has already been normalized to the native Anthropic contract.
                // Keep this to protocol and enum categories only; do not log the body.
                tracing::info!(
                    request_id = request_id.as_deref().unwrap_or(""),
                    model = %model_id,
                    protocol = "anthropic",
                    thinking_type = telemetry_thinking_type(&candidate_upstream_body),
                    output_config_effort = telemetry_output_config_effort(&candidate_upstream_body),
                    beta_context_1m = wants_1m,
                    // 体积判据的输入。留着是为了能在线上直接验证这道门有没有按预期开合，
                    // 而不用从"beta 发没发"反推。
                    body_text_bytes = body_text_bytes(&candidate_upstream_body),
                    beta_interleaved_thinking = candidate_anthropic_beta
                        .as_deref()
                        .is_some_and(|value| value.contains(ANTHROPIC_INTERLEAVED_THINKING_BETA)),
                    beta_effort = candidate_anthropic_beta
                        .as_deref()
                        .is_some_and(|value| value.contains(ANTHROPIC_EFFORT_BETA)),
                    // 请求**形状**（不是内容）。合成请求在这条线路上 89/89 都回了思考，
                    // 而线上同模型同线路只有 ~15% —— 差别只可能在形状里。这几个字段是
                    // 用来把那两群请求区分开的，全部是计数/枚举，不含任何提示词文本。
                    messages_count = candidate_upstream_body
                        .get("messages")
                        .and_then(|m| m.as_array())
                        .map_or(0, |m| m.len()),
                    system_text_bytes = candidate_upstream_body
                        .get("system")
                        .map_or(0, body_text_bytes),
                    tools_count = candidate_upstream_body
                        .get("tools")
                        .and_then(|t| t.as_array())
                        .map_or(0, |t| t.len()),
                    max_tokens = candidate_upstream_body
                        .get("max_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                    step_kind = step_mode(&headers).unwrap_or_else(|| "absent".into()),
                    step_tool_turn = step_is_tool_turn(&candidate_upstream_body)
                        .map_or("unknown", |t| if t { "yes" } else { "no" }),
                    compression_tier = compression_applied
                        .as_ref()
                        .map_or("none", |t| t.as_str()),
                    "thinking telemetry: native Anthropic request"
                );
            }
            let mut route_attempts = 0u32;
            let mut route_failed_transient = false;
            // 持久性鉴权失败（401/403、invalid api key）。这类路由必须**更**该被冷却：
            // key 是坏的，20 秒后它也不会自己好。不冷却的话它一直留在轮换里，下一个
            // 请求可能又挑中它、又 401 —— 用户看到的就是「时好时坏」甚至一直报错。
            let mut route_failed_persistent = false;
            // Never replay a chat prompt inside one user request. Even a transport error can
            // happen after the supplier accepted the body, so a fresh send is not reliably
            // idempotent and may duplicate both model work and billing.
            let candidate_max_attempts = CHAT_UPSTREAM_MAX_ATTEMPTS_PER_ROUTE;
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
                // The first attempt uses the warm HTTP/1.1 pool. A retry after an actual
                // send/status failure owns a client with no idle pool, so it cannot reuse the
                // transport that just failed. Header stalls leave this loop without replaying.
                let chat_client = if attempt == 0 {
                    GW_CHAT_HTTP.clone()
                } else {
                    build_chat_http_client(0)
                };
                let req0 = chat_client.post(&candidate_url);
                // 上游 key 落库是密文（field_crypto，`fc1:...`）。必须先解密再发出去，
                // 否则等于把一段密文当令牌发给上游 → 每条线路一律 401。遗留明文原样透传，
                // 所以对加密/未加密两种行都正确。这一处漏解密，正是「所有模型都用不了」。
                let candidate_key = model_key(&candidate.api_key);
                let mut req = if candidate_anthropic {
                    // 1M context must be explicitly enabled upstream. Anthropic's own API ships
                    // 1M by default on Opus 4.6+/Sonnet 4.6+/Fable, but resellers front it behind
                    // the same beta flag Anthropic uses for Sonnet 4/4.5 — observed verbatim:
                    //   400 {"error":"1m context is fully available; please enable 1m context and retry"}
                    // So: whenever this model's native window is >= 1M, or it has a beta-gated
                    // 1M entry, send the flag. It is a no-op where 1M is already default, and it
                    // is the difference between working and a hard 400 where it is not.
                    let mut r = req0
                        .header("x-api-key", &candidate_key)
                        .header("anthropic-version", "2023-06-01");
                    if let Some(beta) = candidate_anthropic_beta.as_deref() {
                        r = r.header("anthropic-beta", beta);
                    }
                    r.json(&candidate_upstream_body)
                } else {
                    req0.header("Authorization", format!("Bearer {}", candidate_key))
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
                // 最近卡满过的线路只给短探测预算，见 header_wait_for_route。
                let header_wait =
                    remaining.min(header_wait_for_route(max_header_wait, candidate.id, Instant::now()));
                let send_started = Instant::now();
                let sent = match tokio::time::timeout(header_wait, req.send()).await {
                    Ok(result) => {
                        let header_ms = send_started.elapsed().as_millis();
                        match &result {
                            Ok(response) => {
                                tracing::info!(
                                    request_id = request_id.as_deref().unwrap_or(""),
                                    model = %model_id,
                                    route_id = %candidate.id,
                                    attempt = attempt + 1,
                                    fresh_connection = attempt > 0,
                                    upstream_status = response.status().as_u16(),
                                    upstream_header_ms = header_ms,
                                    gateway_request_elapsed_ms = gateway_request_started_at.elapsed().as_millis(),
                                    "upstream response headers received"
                                );
                                // 这条线路又能回话了 —— 撤掉短探测预算，下一次拿回完整耐心。
                                clear_route_stall(candidate.id);
                            }
                            Err(error) => tracing::warn!(
                                request_id = request_id.as_deref().unwrap_or(""),
                                model = %model_id,
                                route_id = %candidate.id,
                                attempt = attempt + 1,
                                fresh_connection = attempt > 0,
                                upstream_header_ms = header_ms,
                                gateway_request_elapsed_ms = gateway_request_started_at.elapsed().as_millis(),
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
                        tracing::warn!(
                            request_id = request_id.as_deref().unwrap_or(""),
                            model = %model_id,
                            url = %candidate_url,
                            attempt = attempt + 1,
                            waited_ms = header_wait.as_millis(),
                            gateway_request_elapsed_ms = gateway_request_started_at.elapsed().as_millis(),
                            "upstream stalled before response headers"
                        );
                        mark_route_stall(candidate.id);
                        // 卡满整段预算才失败，是最该被面板看见的一种坏 —— 这次事故那条线
                        // 44 小时全是这个形状。
                        route_health::spawn_fail(&state, candidate.id, 504);
                        route_failed_transient = true;
                        break;
                    }
                };
                match sent {
                    Ok(r) if r.status().is_success() => {
                        // 真实流量的健康信号。口径是「接得通、认得凭据、开始回话」，
                        // 不是「这一轮流式完整结束」—— 流中途断掉在 agentic IDE 里多半是
                        // 用户按了停止，算成线路故障会把好线路刷红、然后告警被静音。
                        route_health::spawn_ok(&state, candidate.id);
                        success = Some(r);
                        selected_conn = Some(candidate.clone());
                        break 'routes;
                    }
                    Ok(r) => {
                        // 上游把话说完了：它没跑模型，也不会为这一次计费 —— 换线是安全的。
                        upstream_answered_with_error = true;
                        err_status = r.status().as_u16();
                        route_health::spawn_fail(&state, candidate.id, err_status);
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
                            && !upstream_capacity_wording(&err_low)
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
                            // 持久鉴权失败 → 冷却这条线路（见 route_failed_persistent），
                            // 让接下来的请求绕开它、走还能用的同模型线路。
                            if persistent {
                                route_failed_persistent = true;
                            }
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
                        route_health::spawn_fail(&state, candidate.id, 502);
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
            if route_failed_persistent {
                // 坏 key 不会在 20 秒内变好，冷却时间要长得多，避免它反复回到轮换里
                // 又反复 401。到期后会被再探一次；一旦运维在后台把 key 修好，它自然回归。
                mark_route_cooldown_auth(candidate.id);
                tracing::warn!(
                    model = %model_name,
                    provider = %candidate.provider,
                    route_id = %candidate.id,
                    "上游鉴权失败（key 无效/未授权），已冷却这条线路，后续请求改走其它同模型线路"
                );
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
            // 只有「上游把话说完了」才允许再换一条。
            //
            // 卡死（什么都没回来）和发送出错（可能已经发出去了一半）都在这里收手：那两种情况下
            // 上游**可能正在跑这次请求**，再发一次就是重复跑模型、重复计费。这条判据就是
            // 「一次请求只发一次」那条规矩真正想表达的东西——它以前被粗暴地实现成
            // 「一条线路都不许换」，连上游明确说了「我失败了」的情况也一并禁掉。
            if !upstream_answered_with_error {
                break 'routes;
            }
            tracing::info!(
                model = %model_id,
                route_id = %candidate.id,
                status = err_status,
                "upstream answered with an error; trying the next same-model route"
            );
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
                    chat_upstream_attempt_suffix(
                        route_count,
                        attempted_sends,
                        err_status,
                        want_power
                    )
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
        let gateway_request_started_at_task = gateway_request_started_at;
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
                Some(AnthSse::with_tool_argument_rules_started_at(
                    &req_model,
                    tool_argument_rules.clone(),
                    gateway_request_started_at_task,
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
                                first_upstream_chunk_after_headers_ms = response_opened_at.elapsed().as_millis(),
                                first_upstream_chunk_total_ms = gateway_request_started_at_task.elapsed().as_millis(),
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
            // 见 thinking_requested_but_none_returned：要了思考却一个字都没回。
            // 不记钳位（钳位只会更糟），但绝不让它进缓存。
            let thinking_went_missing = complete
                && thinking_clip_probe
                && conv
                    .as_ref()
                    .is_some_and(|c| c.thinking_requested_but_none_returned());
            // 降权只认「块开了却是空的」那一种 —— 见 thinking_swallowed_by_upstream 的注释：
            // adaptive 自己决定不想是正常行为，拿它去降权会把健康线路踢出轮换。
            let thinking_swallowed = complete
                && thinking_clip_probe
                && conv
                    .as_ref()
                    .is_some_and(|c| c.thinking_swallowed_by_upstream());
            if thinking_swallowed {
                // 记下来，让选路绕开它 —— 只打日志的话，下一次请求照样落到同一条线路上。
                mark_thinking_mute(cid);
                tracing::warn!(
                    model = %req_model,
                    route_id = %cid,
                    "upstream returned no thinking despite an explicit thinking request; not caching this response and de-prioritising this route for thinking requests"
                );
            } else if complete && thinking_clip_probe && !thinking_went_missing {
                // 这一轮要了思考、也真的回了 —— 撤掉记号。上游恢复后第一个成功的请求就
                // 让这条线路回到正常轮换，不需要任何人去后台动手。
                clear_thinking_mute(cid);
            }
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
                if thinking_clip_probe && looks_like_relay_truncation(&err) {
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
            if let Some(converter) = conv.as_ref() {
                let thinking = converter.thinking_telemetry();
                // reasoning_content is emitted one-for-one for non-empty
                // thinking_delta payloads, so these forwarded counters are the same
                // aggregate. No reasoning text is retained or logged here.
                tracing::info!(
                    request_id = request_id_task.as_deref().unwrap_or(""),
                    model = %req_model,
                    protocol = "anthropic",
                    stream_result = if complete { "completed" } else { "failed" },
                    nonempty_thinking_delta_chunks = thinking.nonempty_thinking_deltas,
                    thinking_utf8_chars = thinking.thinking_utf8_chars,
                    forwarded_reasoning_content_chunks = thinking.nonempty_thinking_deltas,
                    forwarded_reasoning_content_utf8_chars = thinking.thinking_utf8_chars,
                    // 「零思考」的三种成因在旧日志里完全同形，靠这三个字段分开：
                    //   saw_thinking_block=false            → 模型这一轮压根没思考
                    //   =true 且 chars=0                    → 块开了、文本空（display 侧）
                    //   =false 但 output_tokens >> 正文字符 → 思考了、整块都没回来
                    saw_thinking_block = converter.saw_thinking_block(),
                    visible_text_utf8_chars = thinking.visible_text_utf8_chars,
                    upstream_output_tokens = converter.output_tokens(),
                    stop_reason = converter.stop_reason_label(),
                    first_native_event_kind = thinking.first_native_event_kind,
                    first_native_event_total_ms = thinking.first_native_event_ms,
                    first_model_progress_total_ms = thinking.first_model_progress_ms(),
                    first_nonempty_thinking_delta_total_ms = thinking.first_nonempty_thinking_delta_ms,
                    first_nonempty_text_delta_total_ms = thinking.first_nonempty_text_delta_ms,
                    first_tool_use_start_total_ms = thinking.first_tool_use_start_ms,
                    first_nonempty_tool_delta_total_ms = thinking.first_nonempty_tool_delta_ms,
                    stream_total_ms = gateway_request_started_at_task.elapsed().as_millis(),
                    "thinking telemetry: Anthropic stream outcome"
                );
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
                conn.cache_disabled,
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
            if complete && !relay_dropped_blocks && !thinking_went_missing && !acc.is_empty() && acc.len() < 1_000_000 && response_cache_safe(&acc) {
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
                conn.cache_disabled,
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
    let uid: uuid::Uuid = match crate::api_key_store::lookup_user(&state.db, &token).await {
        Some(u) => {
            crate::api_key_store::touch_last_used(&state.db, &token).await;
            u
        }
        None => crate::auth::user_from_jwt(&state.db, &state.cfg, &token).await
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
        &format!("UPDATE users SET \
         quota_window_cents = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN LEAST(quota_window_cap_cents, quota_total_cents) ELSE quota_window_cents END, \
         quota_window_reset_at = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN now() + interval '{QUOTA_WINDOW_REFRESH}' ELSE quota_window_reset_at END, \
         quota_week_used_cents = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN 0 ELSE quota_week_used_cents END, \
         quota_week_reset_at = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN now() + interval '7 days' ELSE quota_week_reset_at END \
         WHERE id = $1"),
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
    // 免费模型走每日点数池，和会员、钱包并列——这道门必须和 chat_completions 那道一致。
    // 之前只有 chat_completions 做了豁免，于是同一个免费模型：从 IDE（走 /v1/chat/completions）
    // 能用，从任何走 /v1/responses 的客户端就被判成"请先开通会员或充值额度"。同一份后台配置，
    // 两个接口两种结果。
    let free_here = effective_billing(&conn, &model_id).2;
    let free_pool_has_room = free_here
        && free_pool_covers_call(
            free_points_balance(&state, uid).await,
            effective_billing_micro(&conn, &model_id).3,
        );
    admit_billing(
        free_fallback_to_paid(), free_here, free_pool_has_room, quota_ok, credits,
        plan_active, q_total, q_window, q_weekly_cap, q_week_used,
    )?;

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
    // 落库密文 → 解密再发（同 chat 主链路，漏了就是把 `fc1:...` 当令牌发出去）。
    let conn_key = model_key(&conn.api_key);

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

    let resp = match send_once(&url, &conn_key, &body).await {
        Ok(r) => r,
        Err((st, msg)) if is_image_model && msg.to_lowercase().contains("no active plus oauth") => {
            // HD pool empty → fall back to mainline-wrap (model=gpt-5.4) for low-res but functional output.
            tracing::info!(
                "[responses] {model_id} HD pool empty, falling back to gpt-5.4 mainline-wrap"
            );
            if let Some(obj) = body.as_object_mut() {
                obj.insert("model".into(), serde_json::json!("gpt-5.4"));
            }
            match send_once(&url, &conn_key, &body).await {
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
                    // 过一遍脱敏：msg 来自 reqwest 的 Display，末尾会带 ` for url (上游主机…)`，
                    // 而 502 不在 error.rs 的脱敏范围里，等于把上游是谁告诉每个登录用户。
                    "【{model_id}】responses 上游不可用 ({}): {}",
                    st,
                    safe_upstream_error_excerpt(&msg.to_lowercase())
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
                conn.cache_disabled,
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
    let uid: uuid::Uuid = match crate::api_key_store::lookup_user(&state.db, &token).await {
        Some(u) => {
            crate::api_key_store::touch_last_used(&state.db, &token).await;
            u
        }
        None => crate::auth::user_from_jwt(&state.db, &state.cfg, &token).await
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
        &format!("UPDATE users SET \
         quota_window_cents = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN LEAST(quota_window_cap_cents, quota_total_cents) ELSE quota_window_cents END, \
         quota_window_reset_at = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN now() + interval '{QUOTA_WINDOW_REFRESH}' ELSE quota_window_reset_at END, \
         quota_week_used_cents = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN 0 ELSE quota_week_used_cents END, \
         quota_week_reset_at = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN now() + interval '7 days' ELSE quota_week_reset_at END \
         WHERE id = $1"),
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
            "本时段额度已用完，请等待刷新（每 30 分钟）"
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
    // 落库密文 → 解密再发（生成 + 轮询两处都用它）。
    let conn_key = model_key(&conn.api_key);
    let resp = {
        let mut success = None;
        let mut last_err = String::new();
        for attempt in 0u32..3 {
            match GW_HTTP
                .post(&url)
                .header("Authorization", format!("Bearer {}", conn_key))
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
                        // 同上：last_err 是 reqwest 报错原文，直接回给用户会带出上游主机。
                        "【{model_id}】生图上游不可用: {}",
                        safe_upstream_error_excerpt(&last_err.to_lowercase())
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
                    .header("Authorization", format!("Bearer {}", conn_key))
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

    /// SSE 拼文本 + 捞 usage。usage 丢了这条路径就是按 0 结账。
    #[test]
    fn sse_body_yields_text_and_usage() {
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"你好\"}}]}\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"世界\"}}]}\n",
            "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":11,\"completion_tokens\":7}}\n",
            "data: [DONE]\n",
        );
        let (text, usage) = super::text_and_usage_from_body(body);
        assert_eq!(text, "你好世界");
        assert_eq!(usage.unwrap()["prompt_tokens"], 11);
    }

    /// 原生 Anthropic 帧也要认——网关对 Claude 路由用的就是这种形状。
    #[test]
    fn sse_body_reads_native_anthropic_frames() {
        let body = concat!(
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"ab\"}}\n",
            "data: [DONE]\n",
        );
        assert_eq!(super::text_and_usage_from_body(body).0, "ab");
    }

    /// 中转无视 stream:true 直接回 JSON 时按普通补全解析——没有兜底那些线路会整个失效。
    #[test]
    fn plain_json_body_still_parses() {
        let body = r#"{"choices":[{"message":{"content":"ok"}}],"usage":{"prompt_tokens":3}}"#;
        let (text, usage) = super::text_and_usage_from_body(body);
        assert_eq!(text, "ok");
        assert_eq!(usage.unwrap()["prompt_tokens"], 3);
    }
    use super::{
        anthropic_beta_header, anthropic_effort_word, anthropic_thinking,
        anthropic_thinking_with_display, anthropic_to_oai,
        body_text_bytes, upstream_capacity_wording, wants_1m_context, ANTHROPIC_1M_BETA_TEXT_BYTES,
        ANTHROPIC_CONTEXT_WITHOUT_1M_BETA_TOKENS,
        oai_to_anthropic_with_cache, chat_upstream_attempt_suffix,
        chat_upstream_retry_base_delay_ms, claude_generation, clip_thinking_budget, compute_cost,
        is_image_gen_model, official_max_output, official_contexts, model_caps_override,
        mark_thinking_clip, model_price_override, oai_to_anthropic, official_price,
        parse_usage_from_sse, project_quota_package, projected_provider_usd, resolve_cost,
        response_cache_safe, round_multiplier_up, split_fused_charge, thinking_clip_active,
        telemetry_anthropic_event_kind, telemetry_output_config_effort, telemetry_reasoning_effort,
        telemetry_thinking_type,
        tool_argument_rules, upstream_failure_status, validate_openai_sse_eof,
        validate_openai_sse_with_rules, AnthSse, FusedCharge, OpenAiSseValidator,
        CHAT_UPSTREAM_MAX_ATTEMPTS_PER_ROUTE, CHAT_UPSTREAM_MAX_ROUTES_WHEN_ANSWERED,
        THINKING_CLIP_ROUTES, THINKING_CLIP_SAFE_BUDGET,
    };

    /// 计费/预算这些逻辑需要一个**已知的**价格与窗口输入。生产代码里的硬编码能力表
    /// 已经删干净了（实测 13 款错 6 款），所以已知输入由测试自己提供——这才是它该待的地方。
    /// 数值取自 2026-08-16 的真实目录快照。
    fn seed_catalog() {
        use std::sync::Once;
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            use crate::model_catalog::{priced, seed_for_test};
            seed_for_test(&[
                ("claude-opus-4-8", priced(5.0, 25.0, 128_000, vec![1_000_000])),
                ("claude-opus-4-6", priced(5.0, 25.0, 128_000, vec![1_000_000])),
                ("claude-opus-5", priced(5.0, 25.0, 128_000, vec![1_000_000])),
                ("claude-sonnet-5", priced(2.0, 10.0, 128_000, vec![1_000_000])),
                ("claude-fable-5", priced(10.0, 50.0, 128_000, vec![1_000_000])),
                ("claude-haiku-4-5", priced(1.0, 5.0, 64_000, vec![200_000])),
                ("claude-sonnet-4-5", priced(3.0, 15.0, 64_000, vec![200_000])),
                ("claude-opus-4-1", priced(15.0, 75.0, 64_000, vec![200_000])),
                ("gpt-5.5", priced(5.0, 30.0, 128_000, vec![1_050_000])),
                ("gpt-5.4", priced(2.5, 15.0, 128_000, vec![1_050_000])),
                ("gpt-5.4-mini", priced(0.75, 4.5, 128_000, vec![400_000])),
                ("deepseek-v4-flash", priced(0.06146, 0.12292, 32_768, vec![384_000, 1_000_000])),
                ("minimax-m3", priced(0.30, 1.20, 32_000, vec![1_000_000])),
                ("glm-5", priced(0.6, 1.92, 128_000, vec![204_800])),
                ("grok-4.6", priced(2.0, 6.0, 64_000, vec![500_000])),
                ("qwen3.8-max", priced(2.0, 6.0, 131_072, vec![1_000_000])),
                ("kimi-k3", priced(3.0, 15.0, 16_384, vec![974_842])),
            ]);
        });
    }

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

    /// 同一条线路上，一次用户请求只发一次 —— 这条不许松。
    ///
    /// 理由在循环里：传输层失败也可能发生在上游**已经收下 body 之后**，重发会重复跑模型、
    /// 重复计费。所以每条线路只发一次，而且卡死 / 发送出错之后**不换线**。
    #[test]
    fn one_send_per_route_and_no_failover_when_nothing_came_back() {
        assert_eq!(CHAT_UPSTREAM_MAX_ATTEMPTS_PER_ROUTE, 1);

        // 换线由 `upstream_answered_with_error` 一处判定，且只在收到完整错误响应时置位。
        let loop_src = include_str!("models.rs");
        assert!(
            loop_src.contains("if !upstream_answered_with_error {"),
            "换线的闸门不见了：卡死/发送出错必须当场收手，不能换线重发",
        );
        // 用 concat! 拆开写，否则这段断言**自己**也会被 include_str! 数进去（源码里就有这串字面量），
        // 计数永远比真实的多一。
        let set_site = concat!("upstream_answered_with_error", " = true");
        assert_eq!(
            loop_src.matches(set_site).count(),
            1,
            "只允许在「收到完整错误响应」那一支置位；多一处就等于把卡死也放进了换线路径",
        );
    }

    /// 上游**明确回了错误**时要换一条同模型线路。
    ///
    /// 这是原来那条规矩被套错了地方的部分：40 小时里 48 次 GPT 502 全都写着
    /// `route_count=2 attempted_sends=1`，旁边那条线路一次都没试过；那把失效的 key
    /// （`invalid_api_key` → 424）同理，落到它上面的请求直接判死。
    #[test]
    fn an_answered_error_may_fail_over_to_one_more_route() {
        assert_eq!(CHAT_UPSTREAM_MAX_ROUTES_WHEN_ANSWERED, 2);
        assert!(
            CHAT_UPSTREAM_MAX_ROUTES_WHEN_ANSWERED > 1,
            "只取一条候选的话，上面那句换线判定永远走不到",
        );
    }

    #[test]
    fn chat_gateway_error_suffix_reports_single_route_retries() {
        assert_eq!(
            chat_upstream_attempt_suffix(1, 6, 502, false),
            "（已请求 6 次；当前只有 1 条同模型线路；最后状态 502）"
        );
        assert_eq!(
            chat_upstream_attempt_suffix(3, 12, 504, false),
            "（已请求 12 次 / 3 条同模型线路；最后状态 504）"
        );
    }

    /// 面向用户的报错不许指向一个不存在的页面。
    ///
    /// 「模型系统」这个后台页早就没了（控制台左侧现在是「模型线路 → 线路」），而这句话
    /// 会原样发给**每一个**用户——控制台要求 role=admin、nginx 还有一层 auth_request，
    /// 普通用户点进去只会看到 404。一条自信、具体、而且用户照做不了的指引。
    #[test]
    fn the_auth_failure_message_points_somewhere_that_exists() {
        let msg = super::friendly_upstream_for_test(401, "invalid api key");
        assert!(
            !msg.contains("模型系统"),
            "这个后台页面已经不存在了，别再把用户指过去：{msg}"
        );
        assert!(msg.contains("模型线路"), "要指向控制台里真实存在的那一项：{msg}");
        // 普通用户进不了控制台，必须同时给他一条自己能走的路。
        assert!(
            msg.contains("换个模型"),
            "这句话会发给所有用户，不能只写给管理员看：{msg}"
        );
        // 并且要说清重发无用——否则用户会一直重试一条永远不会好的线路。
        assert!(msg.contains("重发"), "要说明重发解决不了配置问题：{msg}");
    }

    /// 「已请求 1 次 / 2 条同模型线路」读起来是"两条都不行"，而实际上另一条一次都没碰过。
    /// 用户据此以为线路全废了，其实重发一次就会自动换线。
    #[test]
    fn chat_gateway_error_suffix_does_not_imply_every_route_was_tried() {
        let msg = chat_upstream_attempt_suffix(2, 1, 401, false);
        assert!(msg.contains("只试了 1 条"), "{msg}");
        assert!(msg.contains("1 条没试过"), "{msg}");
        assert!(msg.contains("重发"), "要把出口说出来：重发一次就会自动换线。{msg}");
        // 真的把所有线路都试过时，不许再说"还有没试过的"
        assert_eq!(
            chat_upstream_attempt_suffix(2, 2, 502, false),
            "（已请求 2 次 / 2 条同模型线路；最后状态 502）"
        );
    }

    /// 判据必须分得清「上游吞了思考」和「adaptive 这轮自己决定不想」。
    ///
    /// 分不清的代价不是少报一条日志，而是**把健康线路降权**：每一个正常的不思考轮次都会
    /// 触发静音记号 → 下一轮被迫换线 → 换到的线路补不上 display → 思考文本变空串。
    /// 于是「偶尔不出思考卡」被自己的修复放大成「越用越不出」。2026-08-19 实际发生过。
    #[test]
    fn adaptive_自己不想_不能被判成上游吞了思考() {
        let mk = |saw_thinking: bool, saw_answer: bool, chars: usize| {
            let mut c = super::AnthSse::new("claude-opus-5");
            c.saw_thinking_block = saw_thinking;
            c.saw_answer_block = saw_answer;
            c.thinking_telemetry.thinking_utf8_chars = chars;
            c
        };
        // 两个判据必须**分别**接到各自的用途上，接反了两边都坏：
        //   降权用宽判据 → 健康线路被踢出轮换；缓存用窄判据 → 零思考响应被缓存一小时重放。
        {
            const SRC: &str = include_str!("models.rs");
            let prod = &SRC[..SRC.find("mod billing_tests").expect("tests module")];
            let mute = format!("{}()", "thinking_swallowed_by_upstream");
            assert!(prod.contains(&mute), "降权判据没接上");
            // 降权那一处读的必须是窄判据。窗口要小：往前取太多会把上面那行
            // `let thinking_swallowed = …` 的**声明**也圈进来，于是不管 if 判的是谁
            // 这条都绿——断言切错范围和断言写错一样坏（本轮已经踩过一次）。
            // 只看紧邻的那个 if。
            let at = prod.find("mark_thinking_mute(cid)").expect("记号点");
            let head = prod[..at].rfind("if ").expect("记号点前面没有 if");
            let cond = &prod[head..at];
            assert!(
                cond.contains("thinking_swallowed"),
                "降权用的还是宽判据 —— adaptive 正常不思考会把健康线路降权：{cond}",
            );
            // 缓存那一处读的必须是宽判据（零思考一律不缓存，不管什么原因）
            assert!(
                prod.contains("&& !thinking_went_missing &&"),
                "缓存判据没接上宽判据",
            );
        }

        // adaptive 决定不想：一个 thinking 块都没有 → **不是**上游的问题，不许记号
        assert!(
            !mk(false, true, 0).thinking_swallowed_by_upstream(),
            "adaptive 正常跳过思考被判成上游吞了 —— 健康线路会被无谓降权 30 分钟"
        );
        // 上游吞了：thinking 块开了、文本是空串 → 这才是要记号的那种
        assert!(
            mk(true, true, 0).thinking_swallowed_by_upstream(),
            "上游真的吞了思考，必须认出来"
        );
        // 正常回了思考 → 不记号
        assert!(!mk(true, true, 1200).thinking_swallowed_by_upstream());
        // 只有思考没有正文 → 那是另一条签名（thinking_only_end_turn），这里不该命中
        assert!(!mk(true, false, 0).thinking_swallowed_by_upstream());
    }

    /// 「问问题他不会去思考」——同模型三条线路里有一条稳定吞掉思考，而用户每次都先撞上它。
    ///
    /// 这件事早就检测出来了（thinking_requested_but_none_returned），但只打日志、不影响选路，
    /// 于是下一次请求照样落到同一条上。这条测的是记号的生命周期：记得下、会过期、能自愈。
    #[test]
    fn 吞掉思考的线路要被记下来_并且能自愈() {
        use std::time::{Duration, Instant};
        let route = uuid::Uuid::new_v4();
        let now = Instant::now();
        // 没记过 → 不影响任何东西
        assert!(!super::route_mutes_thinking(route, now));

        // 要了思考却一个字没回 → 记下
        super::mark_thinking_mute(route);
        assert!(super::route_mutes_thinking(route, Instant::now()));

        // 记号有效期是「这条线路的脾气」那一档，得跨越好几轮请求
        assert!(super::THINKING_MUTE_MEMORY >= Duration::from_secs(10 * 60));

        // 上游恢复、真的回了思考 → 记号立刻撤掉。没有这一条，一条偶尔抽风的线路
        // 会被永久排到后面，而且没有任何人工入口能把它放回来。
        super::clear_thinking_mute(route);
        assert!(!super::route_mutes_thinking(route, Instant::now()));

        // 光有这几个函数不算数——它们得**真的被调用**。这个仓库里"写好了、零调用点、
        // 而且不报错"是反复出现的失败模式，所以这三条钉的是调用点本身。
        // 需要的串一律拼出来找：include_str! 读的是整个文件、包含本测试模块自己。
        {
            const SRC: &str = include_str!("models.rs");
            let mark = format!("{}(cid);", "mark_thinking_mute");
            let clear = format!("{}(cid);", "clear_thinking_mute");
            let read = format!("{}(candidate.id, now)", "route_mutes_thinking");
            assert!(
                SRC.contains(&mark),
                "检测到吞思考却不记号 —— 下一次请求照样落到同一条线路上，等于只打了条日志"
            );
            assert!(
                SRC.contains(&clear),
                "没有撤销记号的调用点 —— 上游恢复了也回不到轮换里，记号会永久生效"
            );
            assert!(
                SRC.contains(&read),
                "选路没有读这个记号 —— 记了也白记，用户照样撞上那条吞思考的线路"
            );
            // 只有要思考的请求才该受影响：不要思考的请求走那条线路毫无问题，
            // 凭空排后面只会白白打乱轮换。
            assert!(
                SRC.contains("wants_thinking && "),
                "记号必须只在这一轮真的要思考时才参与排序"
            );
        }

        // 到期自己失效：即使没人撤，记号也不会永久生效（再探一次是自愈的另一半）
        super::mark_thinking_mute(route);
        assert!(!super::route_mutes_thinking(
            route,
            Instant::now() + super::THINKING_MUTE_MEMORY + Duration::from_secs(1)
        ));
    }

    /// 被强力版开关压成一条线路时，报错要把出口说出来。
    #[test]
    fn chat_gateway_error_suffix_names_the_power_toggle() {
        let msg = chat_upstream_attempt_suffix(1, 1, 504, true);
        assert!(msg.contains("强力版"), "{msg}");
        assert!(msg.contains("关掉它"), "{msg}");

        // 强力版开着但本来就有多条线路时不提它 —— 那时它不是原因。
        assert_eq!(
            chat_upstream_attempt_suffix(3, 3, 502, true),
            "（已请求 3 次 / 3 条同模型线路；最后状态 502）"
        );
    }

    /// 上游把**容量**错误包在 400 + invalid_request_error 里发出来时，不许当成永久失败。
    ///
    /// 实测原文：`请稍后重试，暂无可用渠道，或切换模型`。它自己都在说稍后重试，而旧判据
    /// 只看外面那层 `invalid_request_error`，于是网关不换线路、客户端也不重试——一个几秒后
    /// 就会好的问题变成死路。
    #[test]
    fn a_capacity_error_wearing_a_400_is_still_transient() {
        assert!(upstream_capacity_wording("请稍后重试，暂无可用渠道，或切换模型"));
        assert!(upstream_capacity_wording("no available channel, try again later"));
        assert_eq!(
            upstream_failure_status(400, "请稍后重试，暂无可用渠道，或切换模型"),
            StatusCode::SERVICE_UNAVAILABLE,
            "容量型 400 要以可重试的状态下发，否则等于替上游告诉用户「别再试了」",
        );

        // 真正的请求格式错误不受影响：重发同一份必然同样失败。
        assert_eq!(
            upstream_failure_status(400, "\"thinking.type.enabled\" is not supported for this model."),
            StatusCode::BAD_REQUEST,
        );
        assert!(!upstream_capacity_wording("extra inputs are not permitted"));

        // 和「没有可用**账号**」划清界限：那是配置/账务问题（→424），重试无用。
        assert_eq!(
            upstream_failure_status(500, "no available provider account"),
            StatusCode::FAILED_DEPENDENCY,
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
            resolve_cost("per_call", 3, None, "free-model", 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, false),
            3,
        );
        // free with no fee falls through to token billing, which with zero prices is 0 —
        // legitimately free, and the points pool is simply untouched.
        assert_eq!(
            resolve_cost("rate", 0, None, "free-model", 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, false),
            0,
        );
    }

    /// The operator prices in 点: ¥0.5 = 10 点 → 1 点 = ¥0.05 → the ¥2 daily allowance is
    /// exactly 40 点. Pin the arithmetic so a future edit cannot quietly desync the two.
    #[test]
    fn daily_allowance_is_two_yuan_worth_of_points() {
        assert_eq!(super::free_points_daily(), 40);
        // ¥0.5 buys 10 点, so the daily grant is ¥2.00 exactly.
        let yuan_per_point = 0.5_f64 / 10.0;
        assert!((super::free_points_daily() as f64 * yuan_per_point - 2.0).abs() < 1e-9);
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

    /// 落库字段加密上线后，`models.api_key` 存的是密文（`fc1:...`）。凡是把它当外发凭据
    /// 的地方都必须先 `model_key()` 解密——漏一处，那条链路就把密文当令牌发给上游，
    /// 上游一律 401。这正是「加密上线后所有模型都用不了」的根因：主 chat 链路
    /// （6072/6079）、图像 /responses、images/generations（含轮询）、会话压缩，全都漏了解密，
    /// 而单模型 chat 端点没漏，所以只在网关主路径上暴雷。此测试扫描非测试源码，禁止
    /// 任何把 `.api_key` 字段直接塞进 Authorization/x-api-key 头的写法。
    #[test]
    fn upstream_key_is_always_decrypted_before_send() {
        // 扫【整份文件】，不在 `mod billing_tests` 处截断——本文件把测试模块夹在生产代码
        // 中间，7336 行之后还有真生产代码（compression_summarize 等），截断会把它们漏掉。
        // 为了不误伤本测试自身，所有要搜的 needle 都在运行时用 format! 拼出来，测试源码里
        // 不出现它们的逐字形态，所以整份扫描不会扫到自己。
        let full = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/models.rs"))
            .expect("read models.rs");
        let dot = |holder: &str| format!("{holder}.api_key");

        // —— 反面：任何把【原始】.api_key 字段直接塞进 Authorization/x-api-key 头的写法都禁止。
        // 覆盖 Model 结构在本文件里出现过的全部持有者名字。
        // needle 全部运行时拼装；注释里【绝不写出】拼装后的逐字形态，否则整份扫描会扫到自己。
        for holder in ["conn", "candidate", "model", "vconn", "m"] {
            let bearer_raw = format!("Bearer {{}}\", {}", dot(holder)); // 组装出「Bearer 头直发原始字段」的形态
            assert!(
                !full.contains(&bearer_raw),
                "发现未解密外发：Bearer 直接发了原始 {holder}.api_key（密文），必须先 model_key() 解密",
            );
            let xapi_raw = format!("x-api-key\", &{}", dot(holder)); // 组装出「x-api-key 头直发原始字段」的形态
            assert!(
                !full.contains(&xapi_raw),
                "发现未解密外发：x-api-key 直接发了原始 {holder}.api_key（密文），必须先 model_key() 解密",
            );
        }
        // send_once 以 Bearer 形参外发，其形参只能喂解密后的 conn_key；禁止把原始字段传进去。
        let send_once_raw = format!("send_once(&url, &{}", dot("conn"));
        assert!(
            !full.contains(&send_once_raw),
            "send_once 收到的必须是解密后的 conn_key，不能是原始 conn.api_key",
        );

        // —— 正面：曾漏解密的四条链路，其解密写法必须在位（防止有人整段删掉解密后再裸发，
        // 那样上面的反面检查扫不到）。conn 三处（responses/images/compression）、candidate 一处（主 chat）。
        let decrypt_conn = format!("model_key(&{})", dot("conn"));
        assert!(
            full.matches(&decrypt_conn).count() >= 3,
            "conn.api_key 的解密点少于 3（responses/images/compression 各一），疑似有链路把解密删了",
        );
        let decrypt_candidate = format!("model_key(&{})", dot("candidate"));
        assert!(
            full.contains(&decrypt_candidate),
            "主 chat 链路必须先解密 candidate.api_key",
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
    /// 免费额度用完之后，免费模型改用付费余额/会员额度继续跑。
    ///
    /// 之前是硬 402：免费池见底那一刻，免费模型既扣不到钱也不再让用，而用户的钱包和
    /// 会员额度明明还有。开关关掉时回到老行为。
    #[test]
    #[test]
    /// 准入门问的问题，必须和结算答的问题是同一个。
    ///
    /// 结算全额扣或一点不扣；门却只看 `balance > 0`。于是按次计费的免费模型（60 毫点/次）
    /// 在池里剩 40 时：结算一分不扣，余数挂到明天日切，而门看到 40 > 0 一路放行——
    /// `admit_billing` 直接 `return Ok(true)`，它后面的"改走会员额度/钱包"和两条 402
    /// 整段不可达。用户要的"免费用完接着扣余额和订阅"到不了，没余额的用户也永远收不到
    /// 402，欠款无上限地记进钱包。
    #[test]
    /// 未鉴权的 /api/models 不许下发运营方的加价倍率，上游报错也不许原样透传。
    ///
    /// `rate` 的定义原文就是 "the operator's margin, hidden from users"，而这个接口
    /// 没有任何鉴权（路由上没有 Claims 提取器，nginx 的 location / 也不拦）——
    /// 一条 curl 就能连着加价前的 input_price/output_price 一起取走，两者相除即毛利率。
    ///
    /// 上游报错那条：`data` 是上游完整 JSON，可能含中转商主机名、请求 URL，
    /// 部分中转商还会回显 Authorization。同一份代码别处早就走 safe_upstream_error_excerpt，
    /// 只有这条 502 绕过去，而它对任何登录用户开放。
    fn client_model_list_hides_margin_and_upstream_errors_are_sanitized() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/models.rs"))
            .expect("read models.rs");

        let at = src.find("pub async fn list_for_client").expect("list_for_client 改名了");
        let body: String = src[at..].chars().take(9_000).collect();
        assert!(
            !body.contains("\"rate\": m.rate"),
            "未鉴权接口又开始下发加价倍率——一条 curl 即可还原毛利率",
        );
        // 客户端确实要用 price_source 画定价卡片，别顺手删掉。
        assert!(body.contains("\"price_source\": price_source"), "price_source 被误删，定价卡片会缺信息");

        // 两个坑都在这一条上踩过：
        // ① 不要按字节偏移去切中文源码——`src[chat_at - 600..]` 会落在 UTF-8 字符中间直接 panic。
        // ② 搜索范围必须**排除测试模块自身**：断言里写的那段字面量也在这个文件里，
        //    拿整份 src 去 contains 就是自己喂自己，改坏了实现也照样绿（实测漏掉了一次变异）。
        // 用 rfind 找测试模块，不能用 find：文件里 590 行附近还有一个 #[cfg(test)] 的
        // 辅助函数，按 find 切会把整份生产代码都切没，断言就永远失败。
        let prod_raw = &src[..src.rfind("#[cfg(test)]").unwrap_or(src.len())];
        // **先剥注释再断言**。这一条上一版是被自己的注释喂到的：解释性注释里也写着
        // safe_upstream_error_excerpt，于是把实现改回原样透传，contains 照样命中、测试照样绿
        // （变异测试实测漏掉了一次）。这个坑本轮已经踩过好几次，这里一次性剥干净。
        let prod: String = prod_raw
            .lines()
            .map(|l| match l.find("//") {
                Some(i) => &l[..i],
                None => l,
            })
            .collect::<Vec<_>>()
            .join("\n");
        // 钉的是**这一处**的形状，不是"文件里出现过这个函数名"——同文件另有几处已经正确
        // 走了这个 sanitizer，只 contains 的话它们会替被改坏的那处背书（实测漏掉一次变异）。
        let site = prod
            .find("模型供应商错误 {}: {}")
            .expect("找不到上游错误文案");
        let window: String = prod[site..].chars().take(200).collect();
        assert!(
            window.contains("safe_upstream_error_excerpt"),
            "上游报错又原样透传了：中转商主机名/回显的 Authorization 会直达任何登录用户",
        );
        assert!(
            // 只钉这一处（模型供应商错误）。同文件 1259 / 2023 行另有两处同形拼接，
            // 属于别的处理器，不在这条断言的范围里。
            !prod.contains("\"模型供应商错误 {}: {}\", status.as_u16(), data)"),
            "又把上游完整 JSON 原样拼进错误消息了",
        );
    }

    #[test]
    /// 整分那部分不许被收第二遍。
    ///
    /// requested_cost 已经是这笔调用的整分费用（per_call 模式下 resolve_cost 直接返回
    /// per_call_cents），而 free_micro_usd 是**同一笔费用**的 micro-USD 写法。把整笔丢进
    /// carry_to_cents 等于换算成分之后再加一次：$0.05/次 收 10¢（2 倍），
    /// $0.003/次 因为后台把任何非零费用抬到 ≥1 分，实收约 1.3¢（4.3 倍）。
    ///
    /// 既有的 sub_cent 测试覆盖不到这个：它假设的场景是费用 < $0.005、换算成整分是 0，
    /// 那时 requested_cost 为 0，减不减都一样。所以那条全绿也挡不住这个 bug。
    fn whole_cent_part_is_not_charged_twice() {
        const MICRO: i64 = super::MICRO_USD_PER_CENT;
        // 进位的输入必须是「micro 总额减去已经按整分收掉的部分」。
        let carry_input = |free_micro: i64, requested_cents: i64| -> i64 {
            (free_micro - requested_cents.saturating_mul(MICRO)).max(0)
        };

        // $0.05/次：requested_cost 已收 5¢，micro 也是 50000 → 零头应为 0，不再加收。
        assert_eq!(carry_input(50_000, 5), 0, "整分费用被收了第二遍（2 倍）");
        assert_eq!(super::carry_to_cents(0, carry_input(50_000, 5)), (0, 0));

        // $0.003/次：后台把它抬成 1¢ 收掉，而 micro 只有 3000 → 已经多收了，零头必须为 0，
        // 绝不能再攒着以后又扣一分。
        assert_eq!(carry_input(3_000, 1), 0, "已经超收了还要再攒零头（4.3 倍）");

        // $0.015/次：整分收 1¢，micro 15000 → 只剩 5000 零头该攒着。
        assert_eq!(carry_input(15_000, 1), 5_000, "零头算错了");
        assert_eq!(super::carry_to_cents(6_000, carry_input(15_000, 1)), (1, 1_000));

        // 真正的亚分场景（requested_cost 为 0）行为不变——这是既有测试覆盖的那条路。
        assert_eq!(carry_input(3_000, 0), 3_000);

        // 接线：实现必须真的减掉整分部分，否则上面全是纯函数演算、代码照旧双收。
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/models.rs"))
            .expect("read models.rs");
        let at = src.find("async fn bill(").expect("bill 改名了");
        let body: String = src[at..].chars().take(14_000).collect();
        assert!(
            body.contains("let carry_input = (free_micro_usd - requested_cost.saturating_mul(MICRO_USD_PER_CENT)).max(0);"),
            "进位的输入仍然是整笔 free_micro_usd —— 整分部分会被收第二遍",
        );
        assert!(
            body.contains("carry_to_cents(prior.map(|(c,)| c).unwrap_or(0), carry_input)"),
            "算出来的 carry_input 没有被真的用上",
        );
    }

    #[test]
    /// 亚分零头要累计，不能既不四舍五入也不收。
    ///
    /// 钱包和会员额度都是整分，而免费模型常按次计价到亚分（$0.003 = 3000 micro-USD）。
    /// 免费池空了之后这类调用落到付费路径，换算成整分是 0 —— 两边都不扣，模型变成真正的
    /// 无限免费。进位到 1 分是 3.3 倍溢价，不收是白送；攒够一分再扣才两头都对。
    fn sub_cent_fees_accumulate_instead_of_vanishing() {
        let micro = 3_000; // $0.003/次
        // 前三次都不该扣：3000 / 6000 / 9000 都不到一分。
        let (c1, r1) = super::carry_to_cents(0, micro);
        assert_eq!((c1, r1), (0, 3_000));
        let (c2, r2) = super::carry_to_cents(r1, micro);
        assert_eq!((c2, r2), (0, 6_000));
        let (c3, r3) = super::carry_to_cents(r2, micro);
        assert_eq!((c3, r3), (0, 9_000));
        // 第四次跨过一分：扣 1 分，余 2000 留着。
        let (c4, r4) = super::carry_to_cents(r3, micro);
        assert_eq!((c4, r4), (1, 2_000), "攒够一分就要真的扣一分");

        // 十次总共 30000 micro = 3 分，一分不多一分不少。
        let (mut carry, mut cents) = (0i64, 0i64);
        for _ in 0..10 {
            let (c, rest) = super::carry_to_cents(carry, micro);
            cents += c;
            carry = rest;
        }
        assert_eq!(cents, 3, "十次 $0.003 就是 3 分");
        assert_eq!(carry, 0);

        // 一整分的费用直接扣，不留零头；负数和 0 不产生扣费也不产生负零头。
        assert_eq!(super::carry_to_cents(0, super::MICRO_USD_PER_CENT), (1, 0));
        assert_eq!(super::carry_to_cents(0, 0), (0, 0));
        assert_eq!(super::carry_to_cents(-5, -5), (0, 0), "脏数据不许变成负债");

        // 纯函数对了不等于接进去了 —— 这一步单独钉住，否则把 `+ carried_cents` 删掉
        // 上面每一条都还是绿的，而免费池空了以后依旧一分不扣。
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/models.rs"))
            .expect("read models.rs");
        let at = src.find("async fn bill(").expect("bill 改名了");
        let body: String = src[at..].chars().take(12_000).collect();
        assert!(
            body.contains("let requested_cost = requested_cost + carried_cents;"),
            "零头算出来了却没加进这次的扣费——免费池空了之后仍然一分不扣",
        );
        assert!(
            body.contains("if free_pool && free_micro_usd > 0 && free_fallback_to_paid()"),
            "零头累计的条件变了：它只该对**从免费分支掉下来**的调用生效，普通付费模型的价格本来就是整分",
        );
    }

    #[test]
    /// 结算失败 = 用户被服务了却没扣到钱。日志必须能对账到「谁、哪笔请求、多少钱」，否则一次
    /// DB 抖动就是一笔查无对象的漏收。bill() 是 fire-and-forget，日志是唯一的追账凭证，所以每条
    /// 致命失败分支都要带 uid/conn_id/request_id + 统一事件标记，供告警与重对账脚本 grep。
    fn billing_settlement_failures_are_reconcilable() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/models.rs"))
            .expect("read models.rs");
        let at = src.find("async fn bill(").expect("bill 改名了");
        // 只在 bill() 函数体内找，别扫到别处的 tracing。
        let end = src[at..]
            .find("// ============ Anthropic protocol bridge")
            .map(|e| e + at)
            .expect("bill() 后面的分隔注释不见了");
        let body = &src[at..end];

        // 五个「已服务却没扣费」的致命分支：逐条确认带上全部对账字段。
        for needle in [
            "failed to begin billing transaction",
            "failed to lock balances for billing",
            "failed to deduct fused quota and credits",
            "failed to insert billing settlement",
            "failed to commit billing transaction",
        ] {
            let i = body
                .find(needle)
                .unwrap_or_else(|| panic!("结算失败分支不见了: {needle}"));
            // tracing 宏把字段写在消息**之前**：取这条 error! 调用从宏名到消息之间的片段。
            let call_start = body[..i]
                .rfind("tracing::error!(")
                .expect("失败消息不在 error! 调用里");
            let log = &body[call_start..i];
            assert!(log.contains("%uid"), "{needle}: 日志缺 uid，无法对账到人");
            assert!(log.contains("%conn_id"), "{needle}: 日志缺 conn_id，无法对账到连接");
            assert!(
                log.contains("request_id = tokens.request_id.as_deref()"),
                "{needle}: 日志缺 request_id，无法对账到具体那笔请求",
            );
            assert!(
                log.contains(r#"event = "billing_settlement_failed""#),
                "{needle}: 缺统一事件标记，告警/对账脚本 grep 不到",
            );
        }

        // 亚分零头没落盘是**非致命**（不 return、只丢一点零头），日志更轻——但至少要能对到人。
        let carry_i = body
            .find("failed to persist sub-cent carry")
            .expect("零头分支不见了");
        let carry_log = &body[body[..carry_i].rfind("tracing::error!(").unwrap()..carry_i];
        assert!(carry_log.contains("%uid"), "零头丢失日志也要带 uid");
    }

    #[test]
    /// 幂等结算：付费路径在**扣任何钱之前**先往 settled_requests 认领 settlement_id；认领冲突
    /// （模糊提交或并发恢复）必须回滚返回 AlreadySettled、绝不扣第二次。这是「不重复扣钱」的核心。
    fn paid_settlement_claims_ledger_before_charging_and_bails_on_conflict() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/models.rs"))
            .expect("read models.rs");
        let at = src.find("async fn bill_inner(").expect("bill_inner 改名了");
        let end = src[at..]
            .find("// ============ Anthropic protocol bridge")
            .map(|e| e + at)
            .expect("bill_inner 后的分隔注释不见了");
        let body = &src[at..end];

        // 认领：ON CONFLICT DO NOTHING 往 settled_requests 写。
        let claim = body
            .find("INSERT INTO settled_requests (settlement_id) VALUES ($1) ON CONFLICT DO NOTHING")
            .expect("付费路径必须认领 settlement_id");
        // 扣减 users 余额那条 UPDATE。
        let deduct = body
            .find("UPDATE users SET quota_total_cents")
            .expect("扣减语句不见了");
        assert!(claim < deduct, "必须**先认领、后扣费**——否则模糊提交时恢复会重复扣");

        // 认领冲突（0 行）→ 回滚 + AlreadySettled，且这一段里不能出现扣减。
        let conflict = body
            .find("claim.rows_affected() == 0")
            .expect("必须判认领是否冲突");
        let after = &body[conflict..deduct];
        assert!(
            after.contains("BillOutcome::AlreadySettled") && after.contains("rollback"),
            "认领冲突必须回滚并返回 AlreadySettled，绝不往下扣费",
        );

        // 记账行要带 settlement_id，端到端可追。
        assert!(
            body.contains("emitted_tool, settlement_id) \\\n")
                || body.contains("emitted_tool, settlement_id)"),
            "model_usage 插入必须带 settlement_id 列",
        );
        assert!(
            body.contains(".bind(settlement_id)"),
            "model_usage 插入必须绑定 settlement_id",
        );

        // 提交失败也入队（模糊提交由恢复端先查账本兜住，不会双扣）。
        assert!(
            body.contains("queue_input(\"commit\")"),
            "提交失败必须入队恢复",
        );
        // 五个致命失败分支都要入队。
        for stage in ["begin_tx", "claim", "lock_balances", "deduct", "insert_usage", "commit"] {
            assert!(
                body.contains(&format!("queue_input(\"{stage}\")")),
                "失败分支 {stage} 没有入队恢复",
            );
        }
    }

    #[test]
    /// 恢复重跑**不得走免费点分支**：免费扣点在 settled_requests 账本之外（用 &state.db 独立提交），
    /// 重跑会在账本外再扣一次点、甚至升级成先扣点后扣钱。队列行必然是付费路径失败，恢复一律走付费认领。
    /// （对抗审查 finding 1/3/5）
    fn recovery_never_takes_the_unledgered_free_points_branch() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/models.rs"))
            .expect("read models.rs");
        let at = src.find("async fn bill_inner(").expect("bill_inner 改名了");
        let end = src[at..]
            .find("// ============ Anthropic protocol bridge")
            .map(|e| e + at)
            .expect("分隔注释不见了");
        let body = &src[at..end];
        // 免费分支必须被 from_recovery 守住。
        assert!(
            body.contains("if free_pool && !from_recovery {"),
            "免费点分支必须加 `&& !from_recovery`——否则恢复重跑会在账本之外重复扣免费点（双扣）",
        );
        // resettle 必须以 from_recovery=true 调 bill_inner（走上面那道守卫）。
        let r = src.find("pub(crate) async fn resettle(").expect("resettle 不见了");
        let rbody = &src[r..src[r..].find("\n}\n").map(|e| e + r).unwrap_or(src.len())];
        assert!(rbody.contains("row.settlement_id, true,"), "resettle 必须 from_recovery=true");
    }

    #[test]
    /// bill() 薄壳保持 fire-and-forget 且每次新 settlement_id；resettle 复用存下的 id、不重复入队。
    fn bill_wrapper_and_resettle_wire_settlement_id_correctly() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/models.rs"))
            .expect("read models.rs");
        // 薄壳：新 uuid + from_recovery = false（正常计费，失败要入队）。
        let w = src.find("async fn bill(\n").expect("bill 薄壳不见了");
        let wbody = &src[w..src[w..].find("\n}\n").map(|e| e + w).unwrap_or(src.len())];
        assert!(wbody.contains("let settlement_id = uuid::Uuid::new_v4();"), "每次计费要新生成 settlement_id");
        assert!(wbody.contains("settlement_id, false,"), "正常计费 from_recovery=false（失败要入队）");
        // resettle：复用行里的 settlement_id + from_recovery = true（跳免费分支、不重复入队）。
        let r = src.find("pub(crate) async fn resettle(").expect("resettle 不见了");
        let rbody = &src[r..src[r..].find("\n}\n").map(|e| e + r).unwrap_or(src.len())];
        assert!(rbody.contains("row.settlement_id, true,"), "恢复重跑必须复用 settlement_id 且 from_recovery=true");
        assert!(rbody.contains("request_id: row.request_id.clone()"), "重建 tokens 要带回 request_id");
    }

    #[test]
    /// bill_inner 的 model_usage 插入：列数 == 占位符 == bind 数（运行时 SQL，cargo 查不出对不上）。
    fn model_usage_insert_arity_agrees_in_bill_inner() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/models.rs"))
            .expect("read models.rs");
        let at = src.find("async fn bill_inner(").expect("bill_inner 改名了");
        let stmt_at = src[at..].find("INSERT INTO model_usage").expect("记账插入不见了") + at;
        // 切到 .execute( 为止（全 ASCII，不会切进多字节中文注释里）——列表/占位/bind 都在它之前。
        let stmt_end = src[stmt_at..].find(".execute(").map(|e| stmt_at + e).expect("no execute");
        let stmt = &src[stmt_at..stmt_end];
        let lp = stmt.find('(').unwrap();
        let rp = stmt[lp..].find(')').unwrap() + lp;
        let cols = stmt[lp + 1..rp].matches(',').count() + 1;
        // 最大 $N。
        let mut max_ph = 0usize;
        for tok in stmt.split('$').skip(1) {
            let n: String = tok.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(v) = n.parse::<usize>() {
                max_ph = max_ph.max(v);
            }
        }
        let binds = stmt.matches(".bind(").count();
        assert_eq!(cols, 14, "model_usage 列数变了");
        assert_eq!(max_ph, 14, "占位符和列数对不上");
        assert_eq!(binds, 14, ".bind() 和列数对不上——结算会运行时报错");
    }

    #[test]
    /// 放行靠哪个池子，结算就得扣哪个。
    ///
    /// /api/models/:id/chat 用 quota_ok 放行、却写死 use_quota=false 只扣钱包：只有会员额度、
    /// 钱包是 0 的用户每次调用都在把钱包记成负数，"扣订阅额度"在这条路由上从没发生过。
    fn per_model_chat_route_settles_against_what_admitted_it() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/models.rs"))
            .expect("read models.rs");
        let at = src.find("let (_pre_mode, _pre_percall, pre_free, _pre_micro)").expect("准入块改写了");
        let body: String = src[at..].chars().take(6_000).collect();
        assert!(
            body.contains("use_quota = quota_ok;"),
            "放行用的是会员额度，结算却没带上——钱包会被记成负数",
        );
        let bill_at = body.find("bill(&state, uid, model.id, cost,").expect("结算调用改名了");
        assert!(
            body[bill_at..].starts_with("bill(&state, uid, model.id, cost, use_quota,"),
            "结算又写死成 false 了",
        );
        // 同一个坑的另一半：池子空不空要问"这一次付得起吗"。
        assert!(
            body.contains("!free_pool_covers_call(free_points_balance("),
            "这条路由又退回 `<= 0` 判空了——余数永远清不空，402 和付费判定整段不可达",
        );
    }

    fn admission_asks_the_same_question_settlement_answers() {
        // 每次 60 毫点的免费模型：$0.003 = 3000 micro-USD，按 50 micro-USD/毫点换算正好 60。
        let per_call_micro = 3_000;
        assert_eq!(super::free_points_needed(per_call_micro), 60, "换算口径变了，这条要重算");

        assert!(super::free_pool_covers_call(60, per_call_micro), "刚好够要放行");
        assert!(super::free_pool_covers_call(61, per_call_micro));
        assert!(
            !super::free_pool_covers_call(40, per_call_micro),
            "池里 40 而这次要 60：结算一分不扣，门就不能说「免费池能付」——\
             这正是余数永远清不空、402 整段不可达的那条路"
        );
        assert!(!super::free_pool_covers_call(0, per_call_micro));

        // 按量计费的免费模型在上游回话前算不出成本，退回地板 1 —— 等价于旧的 `> 0`，
        // 那一类的行为一个字节都不变。
        assert!(super::free_pool_covers_call(1, 0));
        assert!(!super::free_pool_covers_call(0, 0));

        // 接上门本身：池子盖不住这一次 → 有会员额度就该改走付费，没有就该 402。
        let admit = |room: bool, quota: bool, credits: i64| {
            super::admit_billing(true, true, room, quota, credits, quota, 100, 100, 0, 0)
        };
        assert_eq!(admit(false, true, 0).ok(), Some(false), "盖不住时要落到付费路径");
        assert!(admit(false, false, 0).is_err(), "既盖不住又没付费资源，必须 402");
        assert_eq!(admit(true, false, 0).ok(), Some(true), "盖得住仍由免费池付");
    }

    fn free_pool_exhaustion_falls_back_to_paid_balances() {
        // 池子还有 → 由池子付
        let ok = |r: Result<bool, super::AppError>| match r {
            Ok(v) => v,
            Err(e) => panic!("不该被拒绝：{}", e.msg),
        };
        let err_msg = |r: Result<bool, super::AppError>| match r {
            Ok(v) => panic!("不该放行（by_pool={v}）"),
            Err(e) => (e.status, e.msg),
        };
        assert!(ok(super::admit_billing(true, true, true, false, 0, true, 100, 100, 0, 0)));
        // 池子空了 + 钱包有余额 → 放行，走付费
        assert!(
            !ok(super::admit_billing(true, true, false, false, 500, false, 0, 0, 0, 0)),
            "免费额度用完后，有余额就该继续能用，且必须走付费路径",
        );
        // 池子空了 + 只有会员额度 → 同样放行
        assert!(
            !ok(super::admit_billing(true, true, false, true, 0, true, 100, 100, 0, 0)),
            "免费额度用完后，有订阅额度就该继续能用",
        );
        // 池子空了 + 两边都没有 → 拒绝，且话要说全（两件事都没了）
        let (status, msg) = err_msg(super::admit_billing(true, true, false, false, 0, false, 0, 0, 0, 0));
        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert!(msg.contains("免费额度已用完"), "实际：{msg}");
        assert!(
            msg.contains("付费余额") && msg.contains("会员额度"),
            "只说免费额度用完，用户会以为充值也没用。实际：{msg}",
        );
        // 开关关掉 → 回到老行为：免费池空了就拒绝，哪怕钱包有钱
        let (_, off) = err_msg(super::admit_billing(false, true, false, false, 500, false, 0, 0, 0, 0));
        assert!(off.contains("明天 0 点重置"), "实际：{off}");
        // 「两边都没了」那句里也有"明天 0 点重置"，光判这四个字会被它喂饱。开关关掉时
        // 用户钱包里明明还有钱，措辞里就不该出现"付费余额也不可用"。
        assert!(!off.contains("付费余额"), "开关关掉时不该走到「两边都没了」那句：{off}");
        // 非免费模型的措辞不受影响
        let (_, paid) = err_msg(super::admit_billing(true, false, false, false, 0, true, 0, 100, 0, 0));
        assert_eq!(paid, "总额度已用完");
        assert!(super::admit_billing(true, false, false, false, 1, false, 0, 0, 0, 0).is_ok());
    }

    /// 免费池按「全额扣或一点不扣」结算：剩 2 点时来一次 50 点的调用，不能把 2 点扣光
    /// 还记 0 —— 那正是"扣不到钱也不拒绝"的旧行为。地板仍然是 1 毫点。
    #[test]
    fn free_points_needed_keeps_the_floor() {
        assert_eq!(super::free_points_needed(0), 1, "免费且不配费用也必须消耗一点，否则就是无限");
        assert_eq!(super::free_points_needed(1), 1);
        assert!(super::free_points_needed(55_000) > 1, "真有费用就按真实金额算");
    }

    #[test]
    fn zero_fee_cannot_silently_mean_unlimited() {
        let full = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/models.rs"))
            .expect("read models.rs");
        let src = &full[..full.find("mod billing_tests").unwrap_or(full.len())];

        // runtime floor on the free path（地板搬进 free_points_needed 了）
        assert!(
            src.contains("milli_points_for_micro_usd(micro_usd).max(1)"),
            "a free-flagged call must always consume at least one milli-点",
        );
        assert!(
            src.contains("let want = free_points_needed(micro);")
                && src.contains("let spent = try_spend_free_points(state, uid, want).await;"),
            "免费池必须走「全额扣或一点不扣」，部分覆盖会让用量记录说不清是谁付的钱",
        );
        // 池子盖不住时必须**落下去**按付费结算，而不是照旧早退。写成 `if true` 之类
        // 无条件早退，免费额度见底那一刻起就既扣不到钱也不再拒绝——钱包和会员额度一分不动。
        assert!(
            src.contains("if spent > 0 {"),
            "免费池的早退必须以「真的扣到了」为条件；无条件早退＝免费额度见底那一刻起，\
             免费模型既扣不到钱也不再拒绝，钱包和会员额度一分不动",
        );
        assert!(
            src.contains("if !free_fallback_to_paid() {"),
            "开关关掉时才保持老行为，默认要落到付费路径",
        );
        assert!(
            src.contains("// 落下去，按普通付费调用结算"),
            "免费分支末尾必须贯穿到下面的付费结算，不能再有第三个 return",
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

    /// 新建连接的 INSERT 必须覆盖 `ModelReq` 能设置的每一个计价字段。
    ///
    /// `per_call_micro_usd` 曾经就漏在这里：列是 20260806 迁移加的，结构体加了字段、
    /// admin_update 也读了，唯独 admin_create 的 INSERT 没写，于是运营新建连接时填的
    /// 每次调用费保存后变成 0，而且没有任何报错 —— 只有 clippy 的
    /// "field is never read" 提过一句。
    ///
    /// 逐字段比对而不是只钉那一个名字：下一次再加计价列时，漏的会是新那个。
    #[test]
    fn admin_create_persists_every_pricing_field() {
        let full = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/models.rs"))
            .expect("read models.rs");
        // ModelReq 的字段表
        let req = full
            .split("pub struct ModelReq {")
            .nth(1)
            .and_then(|s| s.split('}').next())
            .expect("ModelReq struct");
        let fields: Vec<&str> = req
            .lines()
            .filter_map(|l| l.trim().strip_prefix("pub "))
            .filter_map(|l| l.split(':').next())
            .map(|s| s.trim())
            .collect();
        // admin_create 的 INSERT 列表
        let insert = full
            .split("INSERT INTO models (")
            .nth(1)
            .and_then(|s| s.split(')').next())
            .expect("admin_create INSERT");

        // api_key/label/base_url 等一定在；这里关心的是计价与展示字段有没有落库。
        // provider/model_id 用了不同的绑定名，排除掉避免误报。
        let exempt = ["provider", "model_id"];
        for f in fields {
            if exempt.contains(&f) {
                continue;
            }
            assert!(
                insert.contains(f),
                "ModelReq 有字段 `{f}`，但 admin_create 的 INSERT 没有这一列 —— \
                 运营在新建界面填的这个值会被静默丢弃（per_call_micro_usd 就是这么漏的）",
            );
        }
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
        assert_eq!(super::free_milli_points_daily() / mp(three_tenths_of_a_cent), 666);

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
            pts(super::RAW_CENTS_PER_POINT * super::free_points_daily()),
            super::free_points_daily(),
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
                0.0,
                false,
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
                0.0,
                false,
            ),
            35
        );
        // Negative per_call_cents floored to 0.
        assert_eq!(
            resolve_cost("per_call", -5, None, "x", 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, false),
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
                0.0,
                false,
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
                0.0,
                false,
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
                0.0,
                false,
            ),
            16
        );
    }

    // The per-model official catalog returns the published $/1M prices; unknown → None.

    // REAL billing = (in·off_in + out·off_out)/1e6 · 100 · 倍率. Normal agent turn on
    // Claude Opus ($5/$25), 22k in + 2k out:
    //   (22000·5 + 2000·25)/1e6 = $0.16 = 16¢ real cost. × 倍率 3 → 48¢ billed.
    #[test]
    /// 缓存价跟着模型走，不被连接级那一个数盖住。
    ///
    /// 线上 Claude 连接的 cache_create_price 填的是 3.75 —— 那是 Sonnet 的写入价（1.25×$3）。
    /// 同一条连接上还跑着 Opus（$5，应为 6.25）和 Fable（$10，应为 12.5）。缓存写入是单价最贵
    /// 的一类 token，30 天实测仅此一项少收约 $119。连接级两列只在这个模型压根没有输入价时兜底。
    #[test]
    /// 免费模型的豁免必须每个模型调用入口都有，不能只有 chat_completions 有。
    ///
    /// 漏掉的那个接口上，同一份后台配置会给出相反的结果：IDE 走 /v1/chat/completions 能用，
    /// 任何走 /v1/responses 的客户端（Claude Code、Codex 等）被判"请先开通会员或充值额度"。
    /// 这里直接对源码断言，因为这两道门是独立写的、天然会漂。
    #[test]
    fn every_model_entry_point_exempts_free_models_from_the_paid_gate() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/models.rs"))
            .expect("read models.rs");
        for entry in ["pub async fn chat_completions(", "pub async fn responses_proxy("] {
            let start = src.find(entry).unwrap_or_else(|| panic!("{entry} 必须存在"));
            // 按字符边界截断：源码里有中文，直接切字节会落在多字节字符中间而 panic。
            let body: String = src[start..].chars().take(14_000).collect();
            let body = body.as_str();
            // 三个准入口现在共用 admit_billing：分开写过一次，代价是同一个免费模型从
            // IDE 能用、从 /v1/responses 被判成"请先开通会员或充值额度"。
            // 判据是"这个入口在走付费门之前先问过免费池"，不是某一行的具体写法。
            // 原来钉的是 `free_here && free_points_balance(...)` 那串字面量，而那一行必须改：
            // 池子"还剩不剩一点"和结算问的不是同一个问题（见 free_pool_covers_call）。
            let pool = body
                .find("let free_pool_has_room = free_here")
                .unwrap_or_else(|| panic!("{entry} 没有检查每日点数池"));
            assert!(
                body[pool..].starts_with("let free_pool_has_room = free_here\n        && free_pool_covers_call(")
                    || body[pool..pool + 400].contains("free_pool_covers_call("),
                "{entry}：准入门必须问「这一次付得起吗」，不是「还剩不剩一点」",
            );
            let gate = body
                .find("admit_billing(")
                .unwrap_or_else(|| panic!("{entry} 没有走统一的准入判定"));
            assert!(pool < gate, "{entry}：点数池检查必须在付费门之前");
            assert!(
                !body[..gate].contains("今日免费额度已用完"),
                "{entry}：免费池空了不该就地 402，要落到 admit_billing 去看付费余额/会员额度",
            );
        }
    }

    #[test]
    fn cache_prices_follow_the_model_not_the_connection() {
        // Anthropic 形状：出现 cache_read_input_tokens 才走 Anthropic 分支
        // input_tokens 必须非零：compute_cost 在 prompt 和 completion 同时为 0 时提前返回 0，
        // 所以"纯缓存写入"这一形状本身也是不计费的（另一个洞，不在这条测试的范围内）。
        let usage = json!({
            "input_tokens": 1, "output_tokens": 0,
            "cache_read_input_tokens": 0, "cache_creation_input_tokens": 1_000_000,
        });
        let cents = |model: &str, conn_write: f64| {
            compute_cost(Some(&usage), model, 1.0, 0.0, 0.0, 0.0, conn_write, 0.0, 0.0, false)
        };
        // 100 万缓存写入 token，倍率 1：应当 = 1.25 × 该模型输入价，单位美分。
        assert_eq!(cents("claude-opus-4-8", 3.75), 625, "Opus 写入应为 1.25×$5=$6.25");
        assert_eq!(cents("claude-sonnet-5", 3.75), 250, "Sonnet 真实输入价 $2，写入应为 1.25×$2=$2.50");
        assert_eq!(cents("claude-fable-5", 3.75), 1250, "Fable 写入应为 1.25×$10=$12.50");
        // 连接级那个数不得再盖住任何有自己输入价的模型。
        assert_eq!(
            cents("claude-opus-4-8", 3.75),
            cents("claude-opus-4-8", 999.0),
            "连接级缓存价不能影响一个有自己输入价的模型"
        );
        // 但模型没有输入价时，它仍然是兜底（否则就成了白送）。
        assert_eq!(
            compute_cost(Some(&usage), "some-unlisted-model", 1.0, 0.0, 0.0, 0.0, 3.75, 0.0, 0.0, false),
            0,
            "没有输入价也没有连接价 → 0（这是另一个已知洞，此处只固定现状）"
        );
        assert_eq!(
            compute_cost(Some(&usage), "some-unlisted-model", 1.0, 2.0, 8.0, 0.0, 3.75, 0.0, 0.0, false),
            375,
            "只有连接级输入价时，连接级缓存价仍然兜底"
        );
    }

    /// 2026-08-18 用户要求：没手填缓存价时，用 OpenRouter 对**这个模型**的实时目录价，
    /// 而不是按输入价 × 倍数拍脑袋推算。这里给目录种一个明确的实时缓存价，验证它被采用。
    #[test]
    fn unset_cache_price_uses_live_catalog_not_the_estimate() {
        seed_catalog();
        use crate::model_catalog::{seed_for_test, Entry};
        // 一个输入价 $4、但缓存写入实时价 $9 的模型。按旧逻辑（推算）写入 = 1.25×4 = $5；
        // 实时目录说 $9 —— 用户要的是后者。
        seed_for_test(&[(
            "cache-live-model",
            Entry {
                input_price: Some(4.0),
                output_price: Some(16.0),
                cache_read_price: Some(0.5),
                cache_write_price: Some(9.0),
                ..Entry::default()
            },
        )]);
        let write_usage = serde_json::json!({
            // input_tokens 必须 >0，否则 compute_cost 的 prompt<=0&&completion<=0 早返回守卫
            // 会在计价前就返回 0。1 个 token 在 $4/M 下不足 0.001 分，不影响整数分断言。
            "input_tokens": 1, "output_tokens": 0,
            "cache_read_input_tokens": 0, "cache_creation_input_tokens": 1_000_000,
        });
        // 连接级缓存价传 0（没手填）。100 万写入 token、倍率 1 → 应当按实时 $9 = 900 分，
        // 不是推算的 500 分。
        assert_eq!(
            compute_cost(Some(&write_usage), "cache-live-model", 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, false),
            900,
            "没手填缓存价时应当用实时目录 $9，而不是推算的 1.25×$4=$5"
        );

        let read_usage = serde_json::json!({
            "input_tokens": 1, "output_tokens": 0,
            "cache_read_input_tokens": 1_000_000, "cache_creation_input_tokens": 0,
        });
        // 缓存读实时价 $0.5 → 50 分；推算是 0.1×$4 = $0.4 = 40 分。
        assert_eq!(
            compute_cost(Some(&read_usage), "cache-live-model", 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, false),
            50,
            "缓存读也要用实时 $0.5，而不是推算的 $0.4"
        );
    }

    /// 关闭缓存计费（每线路开关）：缓存读、缓存写都**不收钱**，普通输入照常。用户 2026-08-18
    /// 要的："我拉取的模型自带价格和缓存价，新增一个关闭缓存的开关，关了价格一样、不收缓存钱。"
    #[test]
    fn cache_disabled_bills_zero_for_cache_tokens() {
        seed_catalog();
        use crate::model_catalog::{seed_for_test, Entry};
        seed_for_test(&[(
            "cache-off-model",
            Entry {
                input_price: Some(5.0),
                output_price: Some(25.0),
                cache_read_price: Some(0.5),
                cache_write_price: Some(6.25),
                ..Entry::default()
            },
        )]);
        let write_usage = serde_json::json!({
            "input_tokens": 1, "output_tokens": 0,
            "cache_read_input_tokens": 0, "cache_creation_input_tokens": 1_000_000,
        });
        // 开缓存（cache_disabled=false）：100 万缓存写 × $6.25 = 625 分。
        assert_eq!(
            compute_cost(Some(&write_usage), "cache-off-model", 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, false),
            625,
            "开缓存应按真实写入价收"
        );
        // 关缓存（cache_disabled=true）：缓存写不收钱 = 0（只剩那 1 个普通 input token，几乎 0）。
        assert_eq!(
            compute_cost(Some(&write_usage), "cache-off-model", 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, true),
            0,
            "关缓存应当缓存写一分不收"
        );
        // 缓存**读**：关了也不收钱。
        let read_usage = serde_json::json!({
            "input_tokens": 1, "output_tokens": 0,
            "cache_read_input_tokens": 1_000_000, "cache_creation_input_tokens": 0,
        });
        assert_eq!(
            compute_cost(Some(&read_usage), "cache-off-model", 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, true),
            0,
            "关缓存应当缓存读也不收"
        );
        // 普通输入/输出**不受开关影响**：给 100 万普通 input，关缓存照样按 $5 收 = 500 分。
        let plain_usage = serde_json::json!({ "input_tokens": 1_000_000, "output_tokens": 0 });
        assert_eq!(
            compute_cost(Some(&plain_usage), "cache-off-model", 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, true),
            500,
            "关缓存不该动普通输入价"
        );
    }

    /// 加价模型：你把输入价从目录的 $5 覆盖成 $15（3×），缓存价必须跟着放大到 3×，
    /// 不能照搬目录按 $5 算出的绝对值——那会把最贵的缓存写入按成本价收，少收 3 倍。
    /// 这是 2026-08-18 修的核心。
    #[test]
    fn marked_up_input_scales_cache_price_by_the_catalog_ratio() {
        seed_catalog();
        use crate::model_catalog::{seed_for_test, Entry};
        // 目录成本价：输入 $5、缓存写 $6.25（倍率 1.25×）、缓存读 $0.5（0.1×）。
        seed_for_test(&[(
            "markup-model",
            Entry {
                input_price: Some(5.0),
                output_price: Some(25.0),
                cache_read_price: Some(0.5),
                cache_write_price: Some(6.25),
                ..Entry::default()
            },
        )]);
        // 每模型覆盖把输入价加到 $15（compute_cost 的 model_in 参数）。
        let write_usage = serde_json::json!({
            "input_tokens": 1, "output_tokens": 0,
            "cache_read_input_tokens": 0, "cache_creation_input_tokens": 1_000_000,
        });
        // 缓存写：倍率 1.25 × 你的 $15 = $18.75 = 1875 分。照搬目录只有 625 分（少收 3×）。
        assert_eq!(
            compute_cost(Some(&write_usage), "markup-model", 1.0, 0.0, 0.0, 0.0, 0.0, 15.0, 25.0, false),
            1875,
            "加价模型的缓存写没跟着放大——按成本价收了，少收 3 倍"
        );
        // 不加价（off_in 就用目录 $5）时，倍率 × 输入 = 目录绝对值，结果不变（625 分）。
        assert_eq!(
            compute_cost(Some(&write_usage), "markup-model", 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, false),
            625,
            "不加价时应当正好等于目录绝对值"
        );
    }

    /// 目录里真实倍率 ≠ 默认 0.1 的模型（deepseek 缓存读 0.2×），要用真实倍率不是写死的 0.1。
    #[test]
    fn cache_ratio_uses_the_real_catalog_ratio_not_the_hardcoded_factor() {
        seed_catalog();
        use crate::model_catalog::{seed_for_test, Entry};
        // 输入 $1、缓存读 $0.2 → 真实倍率 0.2×（默认写死的是 0.1×）。
        seed_for_test(&[(
            "ratio-model",
            Entry {
                input_price: Some(1.0),
                output_price: Some(2.0),
                cache_read_price: Some(0.2),
                cache_write_price: None,
                ..Entry::default()
            },
        )]);
        let read_usage = serde_json::json!({
            "input_tokens": 1, "output_tokens": 0,
            "cache_read_input_tokens": 1_000_000, "cache_creation_input_tokens": 0,
        });
        // 100 万缓存读 × 真实 $0.2 = 20 分；写死 0.1 只会算 10 分。
        assert_eq!(
            compute_cost(Some(&read_usage), "ratio-model", 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, false),
            20,
            "没用目录的真实倍率 0.2，用了写死的 0.1"
        );
    }

    /// 目录也没有缓存价时（cache_*_price = None），才掉到按输入价推算——最后的兜底不变。
    #[test]
    fn cache_price_falls_back_to_estimate_only_when_catalog_lacks_it() {
        seed_catalog(); // priced(...) 建的条目 cache_*_price 都是 None
        let usage = serde_json::json!({
            "input_tokens": 1, "output_tokens": 0,
            "cache_read_input_tokens": 0, "cache_creation_input_tokens": 1_000_000,
        });
        // claude-fable-5 输入价 $10、目录无缓存价 → 推算写入 1.25×$10 = $12.5 = 1250 分。
        assert_eq!(
            compute_cost(Some(&usage), "claude-fable-5", 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, false),
            1250,
            "目录没有缓存价时，仍按输入价 × 倍数兜底"
        );
    }

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
                0.0,
                false,
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
                0.0,
                false,
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
                0.0,
                false,
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
                false,
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
        seed_catalog();
        let usage = json!({"prompt_tokens": 22000, "completion_tokens": 2000});
        assert_eq!(
            compute_cost(Some(&usage), "gpt-5.5", 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, false),
            17
        );
    }

    // Cheap model on a SMALL call rounds toward 0; the SAME model on a big agentic call
    // bills real money. deepseek-v4-flash ($0.14/$0.28), ×1:
    //   22k+2k  → $0.00364 → 0¢ (sub-cent).   200k+10k → $0.0308 → 3¢.
    #[test]
    fn cheap_model_scales_with_size() {
        seed_catalog();
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
                0.0,
                false,
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
                0.0,
                false,
            ),
            // 1 而不是 3：旧硬编码表把 deepseek-v4-flash 写成 0.14/0.28，真实价是
            // 0.06146/0.12292（便宜一半多）。这条测试原本钉的是那个错价算出来的数。
            // 它真正要守的"成本随规模从 0 涨上来"仍然成立：small=0、big>0。
            1
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
                0.0,
                false,
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
                0.0,
                false,
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
                2.0,
                false,
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
                2.0,
                false,
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
                0.0,
                false,
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
        // 细粒度工具流式必须一直开着：关掉它 Anthropic 就会把整个工具入参攒完才发，
        // write_file 那种把整份文件塞进 content 的调用会让用户对着空卡片干等几十秒到几分钟。
        // 客户端的实时预览是逐 delta 画的，没有增量就没有画面——这条是那套 UI 的前提。
        assert_eq!(
            a["tools"][0]["eager_input_streaming"],
            serde_json::json!(true),
            "工具入参又变回缓冲发送了：Claude 写文件时用户会长时间看不到任何内容"
        );
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

    /// Whether thinking TEXT comes back is a property of the route, not of the docs, so this
    /// is a measurement rather than a rule — and it has been re-measured. An earlier probe
    /// against changhuai.ai had bare adaptive returning 131 characters and `summarized`
    /// returning 0, so `display` was dropped. On the route in service now (764fe78b), the
    /// gateway's own stream telemetry reports thinking_utf8_chars=0 on EVERY completed Opus 5
    /// stream with bare adaptive — which is precisely Anthropic's documented default, where
    /// `display` is "omitted" and omitted streams thinking blocks whose text is empty.
    ///
    /// So the field is sent, and MICHAEL_THINKING_DISPLAY=omitted reverts it without a build.
    /// What must never come back is the situation this test originally guarded: a stack in
    /// which nothing anywhere decides the question.
    #[test]
    fn adaptive_thinking_display_is_measured_not_assumed() {
        for model in ["claude-opus-4-7", "claude-opus-4-8", "claude-opus-5", "claude-sonnet-5", "claude-fable-5"] {
            let t = anthropic_thinking(model, Some("high")).expect("thinking must be requested");
            assert_eq!(t["type"], "adaptive", "{model} must use adaptive");
            assert_eq!(
                t["display"], "summarized",
                "{model}: omitted display streams thinking blocks with EMPTY text, which is the \
                 zero-character reading the live telemetry shows"
            );
        }
        // The escape hatch has to work, or the next person measuring is blocked on a deploy.
        // 走纯函数版：改进程环境会漏给并行跑的其它测试（见 anthropic_thinking_with_display）。
        let reverted =
            anthropic_thinking_with_display("claude-opus-5", Some("high"), Some("omitted")).unwrap();
        assert!(reverted.get("display").is_none(), "the kill switch must actually revert");
        // 而且 env 这一层必须真的接在那个参数上，否则线上那个开关是死的。
        let src = include_str!("models.rs");
        let production = &src[..src.find("mod billing_tests").expect("tests module")];
        assert!(
            production.contains("std::env::var(\"MICHAEL_THINKING_DISPLAY\").ok().as_deref()"),
            "kill switch must still be wired to the environment"
        );
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
        // The adaptive family REJECTS budget_tokens, so effort is its only depth control —
        // omitting it left the model with no depth signal at all, which is exactly what
        // "the thinking has no substance" looked like. The old "never send effort" rule still
        // holds for the enabled/budget family and is asserted separately below.
        assert_eq!(
            a["output_config"], json!({"effort":"high"}),
            "adaptive thinking must carry output_config.effort or it has no depth knob"
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
        // 4.8 is the adaptive family: the client's legacy enabled+budget shape is normalized to
        // adaptive above, so effort must ride along as the depth knob. (The "never send effort"
        // rule applies only to models that genuinely stay on enabled+budget, i.e. 3.7 / 4.6.)
        assert_eq!(a["output_config"], json!({"effort":"high"}), "max maps to high; depth comes from max_tokens headroom");

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
    fn off_means_off_on_the_models_whose_default_is_on() {
        // Opus 5 and Sonnet 5 run ADAPTIVE thinking when the thinking key is absent, so returning
        // None for "off" made the cheapest dial the deepest one. Worse, the max_tokens headroom
        // floor is granted only to turns that announce thinking, so that same turn also ran on the
        // bare default while adaptive thinking consumed it — the answer came back truncated.
        for id in ["claude-opus-5", "claude-sonnet-5"] {
            assert_eq!(
                anthropic_thinking(id, Some("off")),
                Some(json!({"type":"disabled"})),
                "{id} must be told to stop, not merely not told to start"
            );
        }
        // Where the default is genuinely off, silence already says it.
        for id in ["claude-opus-4-8", "claude-opus-4-7", "claude-opus-4-6", "claude-sonnet-4-6"] {
            assert_eq!(anthropic_thinking(id, Some("off")), None, "{id}");
        }
        // Fable and Mythos reject an explicit disable outright; there is no off to offer.
        for id in ["claude-fable-5", "claude-mythos-5"] {
            assert_eq!(anthropic_thinking(id, Some("off")), None, "{id}");
        }
        // Absent is not off. A caller that names no effort wants the model's own default, and
        // disabling thinking for them would be the same bug pointed the other way.
        assert_eq!(anthropic_thinking("claude-opus-5", None), None);

        // A disabled turn must not collect the headroom meant for thinking, and the client's
        // explicit disable must read as "off" rather than as the bare-toggle "high".
        let a = oai_to_anthropic(&json!({
            "model": "claude-opus-5",
            "thinking": {"type": "disabled"},
            "messages": []
        }))
        .unwrap();
        assert_eq!(a["thinking"], json!({"type":"disabled"}));
        assert!(a.get("output_config").is_none(), "a disabled turn has no depth to set");
        // It gets the ordinary per-model default and none of the depth headroom: the floor exists
        // to let adaptive thinking stretch, and handing it to a turn that will not think just
        // inflates the ceiling.
        let deep = oai_to_anthropic(&json!({
            "model": "claude-opus-5", "reasoning_effort": "max", "messages": []
        }))
        .unwrap();
        assert!(
            a["max_tokens"].as_i64().unwrap() < deep["max_tokens"].as_i64().unwrap(),
            "a disabled turn must not collect the headroom meant for thinking"
        );
    }

    #[test]
    fn output_ceilings_are_per_model_instead_of_one_number_for_everything() {
        // The catalogue carried a context window and nothing else, so the pipeline guessed twice:
        // a flat 128000 clamp with no model in scope, and an invented 8192 default.
        assert_eq!(official_max_output("claude-opus-5"), Some(128_000));
        assert_eq!(official_max_output("claude-sonnet-5"), Some(128_000));
        assert_eq!(official_max_output("claude-fable-5"), Some(128_000));
        assert_eq!(official_max_output("claude-opus-4-6"), Some(128_000));
        // Haiku caps at 64,000 and rejects the flat value the clamp used to hand it.
        assert_eq!(official_max_output("claude-haiku-4-5"), Some(64_000));
        assert_eq!(official_max_output("claude-sonnet-4-5"), Some(64_000));
        assert_eq!(official_max_output("claude-opus-4-1"), Some(64_000));
        // Unknown route says nothing rather than having a ceiling invented for it.
        assert_eq!(official_max_output("some-local-llama"), None);

        // The clamp honours it, and a thinking-off turn no longer ships the invented 8192.
        let haiku = oai_to_anthropic(&json!({
            "model": "claude-haiku-4-5", "max_tokens": 128000, "messages": []
        }))
        .unwrap();
        assert_eq!(haiku["max_tokens"], 64_000);
        let bare = oai_to_anthropic(&json!({"model": "claude-opus-5", "messages": []})).unwrap();
        assert!(bare["max_tokens"].as_i64().unwrap() > 8192);
    }

    #[test]
    fn the_thinking_switch_splits_by_generation_not_by_named_version() {
        // Naming versions one at a time is what left Sonnet 4.5, Opus 4.5 and Opus 4.1 on the
        // adaptive shape: only 4.6 was listed, so everything older fell through to the modern
        // branch and was sent `{"type":"adaptive"}` — a mode none of them supports, and on
        // Sonnet 4.5 the effort parameter errors outright.
        for id in [
            "claude-sonnet-4-5",
            "claude-opus-4-5",
            "claude-opus-4-1",
            "claude-opus-4-0",
            "claude-sonnet-4-0",
            "claude-opus-4-6",
            "claude-sonnet-4-6",
        ] {
            let t = anthropic_thinking(id, Some("high")).expect("thinking is configurable here");
            assert_eq!(t["type"], "enabled", "{id} takes an explicit budget");
            assert_eq!(t["budget_tokens"], 24000, "{id}");
        }
        for id in [
            "claude-opus-4-7",
            "claude-opus-4-8",
            "claude-opus-5",
            "claude-sonnet-5",
            "claude-fable-5",
            "claude-mythos-5",
        ] {
            assert_eq!(
                anthropic_thinking(id, Some("high")),
                Some(json!({"type":"adaptive","display":"summarized"})),
                "{id} rejects budget_tokens outright"
            );
        }

        // A dated snapshot suffix is not a minor version, and an unrecognised id reads as newer
        // than this table — the direction the API has moved, and the shape a new model accepts.
        assert_eq!(claude_generation("claude-opus-4-5-20251101"), 4.5);
        assert_eq!(claude_generation("claude-3-7-sonnet-20250219"), 3.7);
        assert_eq!(claude_generation("claude-opus-5"), 5.0);
        assert_eq!(claude_generation("some-unreleased-claude"), 0.0);
        assert_eq!(
            anthropic_thinking("some-unreleased-claude", Some("high")),
            Some(json!({"type":"adaptive","display":"summarized"}))
        );
    }

    /// 同一个请求体，只改 effort_passthrough 这一个开关。
    fn dial(eff: &str, passthrough: bool) -> serde_json::Value {
        oai_to_anthropic_with_cache(
            &json!({"model": "claude-opus-5", "reasoning_effort": eff, "messages": []}),
            true,
            passthrough,
        )
        .unwrap()
    }
    fn headroom(eff: &str, passthrough: bool) -> i64 {
        dial(eff, passthrough)["max_tokens"].as_i64().unwrap()
    }
    fn effort_word(eff: &str, passthrough: bool) -> String {
        dial(eff, passthrough)["output_config"]["effort"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn the_second_deepest_dial_is_not_shallower_than_the_one_below_it() {
        // 封顶开着时 `xhigh` 被折成 effort=high，所以它必须落在 high 的 max_tokens 余量上。
        // 掉回 32000 默认值会让第二深的档比它下面那档还浅。
        assert!(headroom("low", false) <= headroom("high", false));
        assert_eq!(headroom("xhigh", false), headroom("high", false));
        assert!(headroom("max", false) > headroom("high", false));
        // 直通打开之后梯子必须是**严格单调**的：深一档不能比浅一档写得少。
        assert!(headroom("low", true) <= headroom("high", true));
        assert!(headroom("xhigh", true) > headroom("high", true));
        assert!(headroom("max", true) > headroom("xhigh", true));
    }

    /// 封顶默认必须保持旧行为——升级不改变任何现有流量。
    #[test]
    fn the_effort_ceiling_is_on_by_default_and_unchanged() {
        for eff in ["high", "xhigh", "max"] {
            assert_eq!(effort_word(eff, false), "high", "{eff} 在默认配置下应当被折成 high");
        }
        assert_eq!(effort_word("low", false), "low");
        assert_eq!(effort_word("medium", false), "medium");
        // 老的两参数便捷包装（测试用）默认也是封顶的。
        assert_eq!(
            oai_to_anthropic(&json!({"model":"claude-opus-5","reasoning_effort":"max","messages":[]}))
                .unwrap()["output_config"]["effort"],
            json!("high")
        );
    }

    /// 线路上打开直通后，用户拨的档位才真的到达模型。
    ///
    /// 这一条才是「思考深度和假的一样」的正解：`high` 是这一族的 API 默认值，封顶开着的
    /// 时候，IDE 上最深的那一档发出去的东西和什么都不发一模一样。
    #[test]
    fn passthrough_lets_the_deepest_dials_actually_reach_the_model() {
        assert_eq!(effort_word("xhigh", true), "xhigh");
        assert_eq!(effort_word("max", true), "max");
        // 直通只影响 high 之上的两级，浅档一个字都不变。
        for eff in ["low", "medium", "high"] {
            assert_eq!(effort_word(eff, true), effort_word(eff, false), "{eff} 不该受直通影响");
        }
    }

    #[test]
    fn the_effort_word_mapping_is_total_and_never_invents_a_tier() {
        // 认识的档位只有这几个；别的一律落到 medium，不能把未知的词原样发出去。
        for (input, want_off, want_on) in [
            ("low", "low", "low"),
            ("medium", "medium", "medium"),
            ("high", "high", "high"),
            ("xhigh", "high", "xhigh"),
            ("max", "high", "max"),
            ("minimal", "medium", "medium"),
            ("garbage", "medium", "medium"),
            ("", "medium", "medium"),
        ] {
            assert_eq!(anthropic_effort_word(input, false), want_off, "封顶: {input}");
            assert_eq!(anthropic_effort_word(input, true), want_on, "直通: {input}");
        }
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
            "content": [{"type": "thinking", "thinking": "Check the request."}, {"type": "text", "text": "Hello"}, {"type": "tool_use", "id": "t1", "name": "get_time", "input": {"tz": "Asia/Tokyo"}}],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 10, "output_tokens": 5, "cache_read_input_tokens": 3, "cache_creation_input_tokens": 0}
        });
        let o = anthropic_to_oai(&av, "claude-opus-4-8");
        assert_eq!(o["choices"][0]["finish_reason"], "tool_calls");
        assert_eq!(o["choices"][0]["message"]["content"], "Hello");
        assert_eq!(
            o["choices"][0]["message"]["reasoning_content"],
            "Check the request."
        );
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
    fn anthropic_to_oai_ignores_redacted_thinking() {
        let av = json!({
            "content": [
                {"type": "redacted_thinking", "data": "opaque"},
                {"type": "text", "text": "Hello"}
            ]
        });
        let o = anthropic_to_oai(&av, "claude-opus-4-8");
        assert_eq!(o["choices"][0]["message"]["content"], "Hello");
        assert!(o["choices"][0]["message"]
            .get("reasoning_content")
            .is_none());
    }

    /// 反过来那半边：要了思考、正文好好的、思考一个字都没回。
    ///
    /// 这种响应绝不能进缓存——缓存 1 小时意味着接下来一小时每个相同请求都重放这份
    /// 没有思考的副本，用户看到的就是"一直不返回思考，过一阵又好了"。
    #[test]
    fn a_stream_that_returns_no_thinking_at_all_is_recognised_and_kept_out_of_cache() {
        let mut c = AnthSse::new("claude-opus-5");
        // 正文正常，从头到尾没有 thinking 块 —— 实测上游会这样（同一请求体时好时坏）。
        let _ = c.push(b"data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n").unwrap();
        let _ = c.push(b"data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"The bug is reentrancy.\"}}\n\n").unwrap();
        let _ = c.push(b"data: {\"type\":\"content_block_stop\",\"index\":0}\n\n").unwrap();
        let _ = c.push(b"data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":42}}\n\n").unwrap();
        let _ = c.push(b"data: {\"type\":\"message_stop\"}\n\n").unwrap();
        assert!(c.thinking_requested_but_none_returned(), "有正文、零思考 —— 必须认出来");
        assert!(!c.thinking_only_end_turn(), "这不是「只有思考」那一种，别和它混了");

        // 对照：思考正常回来的流不能被误判，否则每一条健康响应都进不了缓存。
        let mut ok = AnthSse::new("claude-opus-5");
        let _ = ok.push(b"data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\"}}\n\n").unwrap();
        let _ = ok.push(b"data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"weighing the lock scope\"}}\n\n").unwrap();
        let _ = ok.push(b"data: {\"type\":\"content_block_stop\",\"index\":0}\n\n").unwrap();
        let _ = ok.push(b"data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n").unwrap();
        let _ = ok.push(b"data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"Release the lock first.\"}}\n\n").unwrap();
        let _ = ok.push(b"data: {\"type\":\"content_block_stop\",\"index\":1}\n\n").unwrap();
        let _ = ok.push(b"data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":42}}\n\n").unwrap();
        let _ = ok.push(b"data: {\"type\":\"message_stop\"}\n\n").unwrap();
        assert!(!ok.thinking_requested_but_none_returned(), "健康的流不能被拦在缓存外");

        // 缓存判据里三个条件必须同时出现，少一个这条修复就是空的。
        let src = include_str!("models.rs");
        let production = &src[..src.find("mod billing_tests").expect("tests module")];
        assert!(production.contains("&& !thinking_went_missing &&"), "缓存判据没接上");
        assert!(production.contains("thinking_requested_but_none_returned()"), "探测没接上");
    }

    /// 「零思考」有三种成因，旧日志里它们**完全同形**（thinking_utf8_chars 都是 0）。
    ///
    /// 线上 48 小时：同一条线路、同一批模型，合成请求 89/89 都回了思考，真实 IDE 流量
    /// 只有 ~15%。要往下查就必须先能分开这三种：
    ///   · 模型这一轮压根没思考           → 没有 thinking 块
    ///   · 块开了但文本是空串（display）  → 有 thinking 块、chars=0
    ///   · 思考了但整块没回来（中转吞掉）→ 没有块，但 output_tokens 远大于可见正文
    /// 前两种靠 saw_thinking_block 分，第三种靠 output_tokens vs 可见正文字符数分。
    #[test]
    fn zero_thinking_streams_are_distinguishable_by_block_and_token_evidence() {
        // 一、没有 thinking 块：模型没思考。output_tokens 和正文量相称。
        let mut none = AnthSse::new("claude-opus-5");
        let _ = none.push(b"data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n").unwrap();
        let _ = none.push("data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Release the lock.\"}}\n\n".as_bytes()).unwrap();
        let _ = none.push(b"data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":6}}\n\n").unwrap();
        assert!(!none.saw_thinking_block(), "没有 thinking 块就该报 false");
        assert_eq!(none.thinking_telemetry().visible_text_utf8_chars, 17);
        assert_eq!(none.output_tokens(), 6, "token 数要如实带出来");

        // 二、块开了、文本是空串：display 侧的问题，不是「没思考」。
        let mut empty = AnthSse::new("claude-opus-5");
        let _ = empty.push(b"data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\"}}\n\n").unwrap();
        let _ = empty.push(b"data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"\"}}\n\n").unwrap();
        assert!(empty.saw_thinking_block(), "块开了就必须是 true —— 这正是两者的分界");
        assert_eq!(empty.thinking_telemetry().thinking_utf8_chars, 0);

        // 可见正文只数**非空** text_delta，且按字符不按字节（中文一个字算一个）。
        let mut cjk = AnthSse::new("claude-opus-5");
        let _ = cjk.push(b"data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"\"}}\n\n").unwrap();
        let _ = cjk.push("data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"a中\"}}\n\n".as_bytes()).unwrap();
        assert_eq!(cjk.thinking_telemetry().visible_text_utf8_chars, 2);

        // 三个字段必须真的进了那条日志，否则线上还是分不开 —— 这条修复就是空的。
        let src = include_str!("models.rs");
        let production = &src[..src.find("mod billing_tests").expect("tests module")];
        for field in [
            "saw_thinking_block = converter.saw_thinking_block()",
            "visible_text_utf8_chars = thinking.visible_text_utf8_chars",
            "upstream_output_tokens = converter.output_tokens()",
        ] {
            assert!(production.contains(field), "遥测没接上：{field}");
        }
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
    fn thinking_telemetry_classifies_only_known_values_and_counts_visible_deltas() {
        let inbound = json!({
            "reasoning_effort": "HIGH",
            "thinking": {"type": "adaptive"}
        });
        assert_eq!(telemetry_reasoning_effort(&inbound), "high");
        assert_eq!(telemetry_thinking_type(&inbound), "adaptive");
        assert_eq!(telemetry_output_config_effort(&json!({"output_config":{"effort":"medium"}})), "medium");

        // Arbitrary caller strings are collapsed to a category rather than retained.
        let untrusted = json!({
            "reasoning_effort": "do not retain this input",
            "thinking": {"type": "unrecognised"},
            "output_config": {"effort": "unrecognised"}
        });
        assert_eq!(telemetry_reasoning_effort(&untrusted), "other");
        assert_eq!(telemetry_thinking_type(&untrusted), "other");
        assert_eq!(telemetry_output_config_effort(&untrusted), "other");

        let mut stream = AnthSse::new("claude-opus-4-8");
        stream
            .push(b"data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"\"}}\n\n")
            .unwrap();
        stream
            .push("data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"a中\"}}\n\n".as_bytes())
            .unwrap();
        let telemetry = stream.thinking_telemetry();
        assert_eq!(telemetry.nonempty_thinking_deltas, 1);
        assert_eq!(telemetry.thinking_utf8_chars, 2);
    }

    #[test]
    fn anthropic_beta_header_keeps_context_and_adaptive_thinking_capabilities() {
        let adaptive = json!({"thinking": {"type": "adaptive"}});
        let beta = anthropic_beta_header(&adaptive, true)
            .expect("adaptive request needs beta capabilities");
        assert_eq!(
            beta,
            "context-1m-2025-08-07,interleaved-thinking-2025-05-14,effort-2025-11-24"
        );
        assert!(!beta.contains("redact-thinking"));

        let no_context = anthropic_beta_header(&adaptive, false)
            .expect("adaptive request needs thinking capabilities");
        assert_eq!(
            no_context,
            "interleaved-thinking-2025-05-14,effort-2025-11-24"
        );
        assert_eq!(anthropic_beta_header(&json!({}), false), None);
        assert_eq!(
            telemetry_anthropic_event_kind(Some("message_start")),
            "message_start"
        );
        assert_eq!(telemetry_anthropic_event_kind(Some("provider_private")), "other");
    }

    /// `context-1m` 按这一次请求的实际体积发，而不是按"这个模型支不支持 1M"发。
    ///
    /// 改之前每一个 Claude 请求都带着它，包括一个 354 token 的请求——近 7 天 11,990 次
    /// Claude 调用里真正超过 20 万 token 的只有 93 次（0.78%）。
    #[test]
    fn the_1m_beta_follows_the_actual_request_size() {
        seed_catalog();

        let small = json!({"messages": [{"role": "user", "content": "写个 hello world"}]});
        assert!(
            !wants_1m_context("claude-fable-5", &small),
            "小请求不该被派到 1M 那条溢价通道上"
        );

        let big = json!({
            "messages": [{
                "role": "user",
                "content": "x".repeat(ANTHROPIC_1M_BETA_TEXT_BYTES),
            }]
        });
        assert!(
            wants_1m_context("claude-fable-5", &big),
            "真的可能超出标准窗口时必须发，否则换来一个硬 400（而且 400 不会 failover）"
        );

        // 模型本身不支持 1M 的，多大都不发。
        let huge = json!({
            "messages": [{"role": "user", "content": "x".repeat(ANTHROPIC_1M_BETA_TEXT_BYTES * 2)}]
        });
        assert!(
            !wants_1m_context("claude-haiku-4-5", &huge),
            "目录说这个模型只有 200k，发 1M flag 没有意义"
        );
    }

    /// 阈值的安全性是可以**证明**的，不是估的：任何 BPE 分词器在 UTF-8 上都满足
    /// token ≤ 字符 ≤ 字节，所以正文字节数低于阈值就意味着 token 数低于阈值。
    /// 阈值本身再低于「不带 beta 时的窗口」，这道门就不可能漏发。
    #[test]
    fn the_size_gate_cannot_underestimate_the_token_count() {
        assert!(
            ANTHROPIC_1M_BETA_TEXT_BYTES < ANTHROPIC_CONTEXT_WITHOUT_1M_BETA_TOKENS,
            "阈值必须低于不带 beta 的窗口，否则存在漏发区间",
        );

        // 只数字符串值：键名和 JSON 结构字符不进模型，不该算进 token 账。
        let body = json!({
            "model": "claude-fable-5",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "abcd"},
                {"role": "assistant", "content": [{"type": "text", "text": "efg"}]}
            ]
        });
        // 值：claude-fable-5(14) + user(4) + abcd(4) + assistant(9) + text(4) + efg(3) = 38。
        // max_tokens 是数字不算；`"text": "efg"` 里的**键**也不算——只有那个 `"type"` 的
        // 值 "text" 进账。
        assert_eq!(body_text_bytes(&body), 38);
        assert_eq!(body_text_bytes(&json!({"n": 1, "b": true, "z": null})), 0);
    }

    #[test]
    fn anthropic_stream_telemetry_separates_control_frame_from_real_progress() {
        let mut stream = AnthSse::new("claude-opus-4-8");
        stream
            .push(b"data: {\"type\":\"message_start\",\"message\":{}}\n\n")
            .unwrap();
        let control = stream.thinking_telemetry();
        assert_eq!(control.first_native_event_kind, "message_start");
        assert!(control.first_native_event_ms.is_some());
        assert!(control.first_model_progress_ms().is_none());

        stream
            .push(b"data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"plan\"}}\n\n")
            .unwrap();
        let progress = stream.thinking_telemetry();
        assert_eq!(progress.first_native_event_kind, "message_start");
        assert!(progress.first_nonempty_thinking_delta_ms.is_some());
        assert_eq!(
            progress.first_model_progress_ms(),
            progress.first_nonempty_thinking_delta_ms
        );
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

    // GLM / Grok 走"透传任意 id"的连接（连接价 0、无按模型覆盖），此前不在目录里 → 一直按
    // 0 计费。默认定价 = 官方牌价进目录（docs.z.ai / x.ai，2026-07），连接倍率照常乘在上面。

    // No usage reported → 0 (never guesses token counts).
    #[test]
    fn no_usage_is_zero() {
        assert_eq!(
            compute_cost(None, "claude-opus-4-8", 3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, false),
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
                0.0,
                false,
            ),
            0
        );
    }

    // Anthropic-style field names (input_tokens/output_tokens) are honored.
    #[test]
    fn 能力数据只来自实时目录_没有硬编码兜底() {
        seed_catalog();
        // 目录里有的，照实取——这些值取自 2026-08-16 的真实目录快照。
        assert_eq!(official_price("claude-opus-4-8"), Some((5.0, 25.0)));
        assert_eq!(official_price("CLAUDE-OPUS-4-6"), Some((5.0, 25.0)), "模型名要大小写不敏感");
        assert_eq!(official_price("gpt-5.5"), Some((5.0, 30.0)));
        // 这两个正是**旧硬编码表写错**的：表里 deepseek 是 0.14/0.28、sonnet-5 是 3/15，
        // 而真实值分别是 0.06146/0.12292 和 2/10。表被删掉的直接原因就是这种错。
        assert_eq!(official_price("deepseek-v4-flash"), Some((0.06146, 0.12292)));
        assert_eq!(official_price("claude-sonnet-5"), Some((2.0, 10.0)));

        // **目录里没有的，必须明确说不知道，不许编。**
        //
        // 以前这里会掉到一张按模型名字符串匹配的硬编码表上——那张表实测在售 13 款里错了
        // 6 款（deepseek-v4-flash 写 128K 而真实 1.05M，少 88%）。它不是安全网：
        // 它会在数据缺席时**自信地给出一个错的数**，而错的数比没有数难发现得多。
        // 返回 None 之后，调用方会掉到连接兜底价，再没有就报"请在连接编辑里填写单模型价"
        // ——一个可操作的提示。
        assert_eq!(official_price("some-unknown-model"), None);
        assert_eq!(official_max_output("some-unknown-model"), None);
        assert!(official_contexts("some-unknown-model").is_empty());
    }

    #[test]
    fn 生产代码里不许再出现按模型名硬编码的能力表() {
        // 这条守的是"别把债又加回来"。判据挑的是那三张表最核心的特征串：它们都是
        // 在 official_* 里按模型名 contains() 分支返回写死的窗口/价格。
        // **只扫测试之前的部分**：整份文件包含这条断言自己写的那几个函数名，
        // 直接 contains 会被自己喂到、永远红。这个仓库里同类断言都是这么切的。
        let full = include_str!("models.rs");
        let src = &full[..full.find("mod billing_tests").unwrap_or(full.len())];
        for banned in [
            "fn official_contexts_static",
            "fn official_price_static",
            "fn official_max_output_static",
        ] {
            assert!(!src.contains(banned),
                "{banned} 又回来了——能力数据只能来自实时目录，硬编码表实测 13 款错 6 款");
        }
        // beta header 不在此列：目录只说窗口存在、不说要带哪个头，那是协议细节。
        // 断言它被**调用**，不是断言它存在：改个名字（context_beta_header_removed）
        // 就能骗过 contains("fn context_beta_header")，变异测试当场证明过。
        assert!(src.contains("context_beta_header(model_id, tokens)"),
            "beta header 映射没有接进 official_contexts —— Sonnet 4 的 1M 会变成静默 413");
    }

    #[test]
    fn 目录漏网的模型走后台手填的兜底_不由代码编() {
        // glm-5.3 在 OpenRouter 目录里确实不存在（只有 5.1/5.2/5-turbo）。硬编码表删掉后
        // 它就没有窗口数据了——兜底本身是需要的，只是**不该由代码编**：那张被删的表实测
        // 在售 13 款里错了 6 款，问题正是"没人知道它错了，它还在自信地用"。
        // 由运维在后台填就没这个毛病：谁填的、对不对，填的人清楚，改了不用发版。
        let caps = serde_json::json!({
            "glm-5.3": { "contexts": [128000, 204800], "max_output": 64000 }
        });
        let (ctxs, out) = model_caps_override(&caps, "glm-5.3");
        assert_eq!(ctxs, vec![128_000, 204_800]);
        assert_eq!(out, Some(64_000));

        // 没填的模型 = 真的不知道，不许变出一个数
        assert_eq!(model_caps_override(&caps, "别的模型"), (Vec::new(), None));
        assert_eq!(model_caps_override(&serde_json::json!({}), "glm-5.3"), (Vec::new(), None));

        // 脏值要挡住：0/负数不是窗口，max_output=0 会让一个 token 都发不出去
        let dirty = serde_json::json!({
            "x": { "contexts": [0, -5, 200000, 200000], "max_output": 0 }
        });
        let (c2, o2) = model_caps_override(&dirty, "x");
        assert_eq!(c2, vec![200_000], "0/负数/重复都要清掉");
        assert_eq!(o2, None, "max_output=0 不能当成真实上限");

        // 和实时侧同一个上限：UI 上不会因为来源不同就突然冒出七八档
        let many = serde_json::json!({ "y": { "contexts": [1,2,3,4,5,6,7,8] } });
        assert_eq!(model_caps_override(&many, "y").0.len(), 5);
    }

    #[test]
    fn anthropic_field_names() {
        seed_catalog();
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
                0.0,
                false,
            ),
            16
        );
    }

    // OpenAI cached-prompt shape: cached input billed at 0.1×. opus, prompt 10000 (8000
    // cached), completion 0, ×1: billable = 2000 + 800 = 2800; 2800·5/1e6 = $0.014 → 1¢.
    #[test]
    fn cached_input_discount() {
        seed_catalog();
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
                0.0,
                false,
            ),
            1
        );
    }

    // A malformed/huge usage can never drain a balance — capped at $50 (5000¢).
    #[test]
    fn ceiling_caps_runaway() {
        seed_catalog();
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
                0.0,
                false,
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
                0.0,
                false,
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
            false,
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
            false,
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
            false,
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

    /// Regression (2026-08-01): the deep-thinking budget must recognise EVERY wire shape
    /// that turns thinking on. It used to key off `budget_tokens > 0` alone, so when the
    /// gateway switched modern Claude to `{"type":"adaptive"}` — which has no budget field —
    /// thinking requests silently fell back to the standard header / 180s idle budget and
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
        // Every enabled OpenAI-shaped dial turns thinking on. This is the only wire
        // signal the adaptive Claude family carries before gateway translation.
        assert!(request_is_deep_thinking(&json!({"reasoning_effort": "low"})));
        assert!(request_is_deep_thinking(&json!({"reasoning_effort": "high"})));
        assert!(request_is_deep_thinking(&json!({"reasoning_effort": "max"})));
        assert!(request_is_deep_thinking(&json!({"reasoning_effort": "xhigh"})));

        // ...and a request with no thinking at all must NOT get the deep budget, or every
        // ordinary chat inherits a 600s idle window and a hung route stops looking hung.
        assert!(!request_is_deep_thinking(&json!({"messages": []})));
        assert!(!request_is_deep_thinking(&json!({"reasoning_effort": "off"})));
        assert!(!request_is_deep_thinking(
            &json!({"thinking": {"type": "disabled"}})
        ));
    }

    #[test]
    fn deep_thinking_uses_the_same_transport_budget() {
        assert_eq!(route_budget_for(true), route_budget_for(false));
    }

    /// Header wait includes provider prefill, so it must cover the measured 5.7-8.5s
    /// first-event latency without consuming the client's full 60s deadline.
    #[test]
    fn per_attempt_header_wait_is_request_aware_and_capped() {
        assert_eq!(
            max_header_wait_for_request(false, false),
            Duration::from_secs(57)
        );
        assert_eq!(
            max_header_wait_for_request(false, true),
            Duration::from_secs(57)
        );
        assert_eq!(
            max_header_wait_for_request(true, true),
            Duration::from_secs(57)
        );
        assert!(DEEP_MAX_HEADER_WAIT < ROUTE_BUDGET);
    }

    fn patience(budget_ms: Option<u64>, deadline_ms: Option<u64>) -> ClientPatience {
        ClientPatience {
            budget_ms,
            deadline_ms,
        }
    }

    #[test]
    fn client_patience_caps_the_gateway_budget_when_the_clocks_agree() {
        let now_ms = 1_000_000;

        // 两个头都在、只差一次上传的往返：绝对时间戳把上传耗时算了进去，所以它更紧，
        // 取它。60s 预算 - 3s 上传 - 750ms 余量 = 56.25s，仍在 ROUTE_BUDGET 之下。
        let (budget, verdict) = route_budget_with_client_patience(
            false,
            patience(Some(60_000), Some(now_ms + 57_000)),
            now_ms,
        );
        assert_eq!(budget, Duration::from_millis(56_250));
        assert_eq!(verdict, ClientPatienceVerdict::ClocksAgree { skew_ms: -3_000 });

        // 客户端说自己只剩 4 秒：这是它自己的定时器，不牵涉任何时钟比对，照办。
        let (budget, verdict) =
            route_budget_with_client_patience(false, patience(Some(4_000), None), now_ms);
        assert_eq!(budget, Duration::from_millis(3_250));
        assert_eq!(verdict, ClientPatienceVerdict::RelativeOnly);

        // 两个头都没有：网关自己的预算。
        let (budget, verdict) = route_budget_with_client_patience(false, patience(None, None), now_ms);
        assert_eq!(budget, ROUTE_BUDGET);
        assert_eq!(verdict, ClientPatienceVerdict::Absent);
    }

    /// 时钟不准的机器**必须**还能用。
    ///
    /// 这是这一组里最重要的一条。绝对截止时间戳是客户端墙上时钟的时间戳，机器慢两分钟
    /// 就恒小于服务端的 now_ms，旧判据算出预算 0 —— 那台机器上每一次请求都在开出上游
    /// 调用之前判死，而且永远如此，服务端日志里还看不出原因。
    #[test]
    fn a_skewed_client_clock_never_zeroes_the_budget() {
        let now_ms = 1_000_000_000;

        // 慢两分钟 + 带相对预算（新客户端）：丢掉绝对时间戳，用相对预算。
        let (budget, verdict) = route_budget_with_client_patience(
            true,
            patience(Some(60_000), Some(now_ms - 120_000)),
            now_ms,
        );
        assert_eq!(budget, ROUTE_BUDGET.min(Duration::from_millis(59_250)));
        assert!(
            matches!(verdict, ClientPatienceVerdict::ClockSkewed { .. }),
            "时钟差两分钟必须被认出来，而不是当成「这个请求已经过期」"
        );

        // 慢两分钟 + 只有绝对时间戳（尚未升级的客户端）：合理性检查兜住，退回网关预算。
        let (budget, verdict) =
            route_budget_with_client_patience(true, patience(None, Some(now_ms - 120_000)), now_ms);
        assert_eq!(budget, ROUTE_BUDGET);
        assert_eq!(
            verdict,
            ClientPatienceVerdict::AbsoluteUntrusted { remaining_ms: 0 }
        );

        // 快两分钟：算出来的剩余量远超客户端自己的耐心，被 ROUTE_BUDGET 封顶即可。
        let (budget, _) =
            route_budget_with_client_patience(false, patience(None, Some(now_ms + 120_000)), now_ms);
        assert_eq!(budget, ROUTE_BUDGET);
    }

    /// 合理性检查的边界：刚好卡在门槛上的绝对时间戳仍然采信，低于门槛才丢弃。
    #[test]
    fn only_an_implausibly_small_absolute_remaining_is_discarded() {
        let now_ms = 1_000_000;
        let threshold_ms = MIN_TRUSTED_ABSOLUTE_REMAINING.as_millis() as u64;

        let (budget, verdict) = route_budget_with_client_patience(
            false,
            patience(None, Some(now_ms + threshold_ms)),
            now_ms,
        );
        assert_eq!(
            budget,
            Duration::from_millis(threshold_ms) - CLIENT_DEADLINE_MARGIN
        );
        assert_eq!(verdict, ClientPatienceVerdict::AbsoluteOnly);

        let (budget, verdict) = route_budget_with_client_patience(
            false,
            patience(None, Some(now_ms + threshold_ms - 1)),
            now_ms,
        );
        assert_eq!(budget, ROUTE_BUDGET);
        assert!(matches!(
            verdict,
            ClientPatienceVerdict::AbsoluteUntrusted { .. }
        ));
    }

    #[test]
    fn client_patience_reads_both_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(RESPONSE_BUDGET_HEADER, "60000".parse().unwrap());
        headers.insert(RESPONSE_DEADLINE_HEADER, "1700000000000".parse().unwrap());
        assert_eq!(
            client_patience_from_headers(&headers),
            ClientPatience {
                budget_ms: Some(60_000),
                deadline_ms: Some(1_700_000_000_000),
            }
        );

        // 垃圾值不得被当成 0（0 会被解读成「一点耐心都没有」）。
        let mut junk = HeaderMap::new();
        junk.insert(RESPONSE_BUDGET_HEADER, "abc".parse().unwrap());
        assert_eq!(client_patience_from_headers(&junk), ClientPatience::default());
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
        // 落库密文 → 解密再发。
        .header("Authorization", format!("Bearer {}", model_key(&conn.api_key)))
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

/// 客户端在模型卡片上选中的上下文窗口（`x-ide-context-window`，单位 token）。
///
/// 为什么要收这个：目录查不到窗口的模型（实测 glm-5.3：OpenRouter 没收录、后台
/// model_caps_override 也没填）在**两边**都退回同一个猜测 —— 客户端按模型名正则给 128k，
/// 这里 `official_context(...).unwrap_or(128_000)` 也给 128k。于是用户在卡片上把窗口拖到
/// 262k，压缩仍然按 128k 切，滑块是个纯装饰。用户的原话是"我想调到用哪个就用哪个"。
///
/// 用户显式点的那一档就是他知道自己这条线路能吃多少，比任何猜测都更接近事实，所以它优先。
/// 只做区间检查，不做"合不合理"的二次判断 —— 那又会变成一个替用户改主意的猜测。
fn client_context_window(headers: &HeaderMap) -> Option<usize> {
    const MIN: usize = 1_000;
    const MAX: usize = 20_000_000;
    headers
        .get("x-ide-context-window")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|n| (MIN..=MAX).contains(n))
}

async fn apply_michael_compression(
    state: &AppState,
    body: &mut serde_json::Value,
    model_id: &str,
    tier: crate::compression::Tier,
    uid: uuid::Uuid,
    client_window: Option<usize>,
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

    // 优先级：用户在卡片上选的 > 目录/后台配置 > 兜底猜测。前两者都是"有人知道这个数"，
    // 最后一个是"没人知道，先给个数别崩"——它不该盖过前面任何一个。
    let native = client_window
        .unwrap_or_else(|| official_context(model_id).unwrap_or(128_000).max(1) as usize)
        .max(1);
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
    let verbatim_tail = mc::verbatim_tail_for_budget(budget);
    let mut plan = mc::plan_to_budget(&msgs, remaining_budget, verbatim_tail, segment_tokens);

    // 提前切出旧段：即使原文暂时还塞得进窗口，也要在请求体逼近 3.5MB 前完成预热并签发
    // 前缀。普通增长型会话因此不会在跨过原生窗口的那一轮突然冷启动。
    // The .min(400_000) clamp is gone. It was written when every Claude model reported a 200K
    // native window, where 2/3 of budget is ~99K and the 400K cap never bound. Native is now 1M
    // on most models (budget ~748K), so the same constant fired at 53% of budget: history that
    // fit verbatim was summarised anyway, and the model received ~44K where 400K was available.
    // A paying subscriber was strictly worse off than a free user. Pre-warm still happens before
    // the window overflows — that is its whole point — just at a share of the real budget rather
    // than at a number that only made sense for a 200K window.
    let prefix_trigger = mc::prefix_trigger_for(budget, verbatim_tail, segment_tokens);
    if carried.summaries.is_empty() && plan.compress.is_empty() && plan.raw_tokens >= prefix_trigger
    {
        plan = mc::plan_for_prefix(&msgs, verbatim_tail, segment_tokens);
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
    // The cap is the tier OR the model's own window, whichever is larger. A tier is a promise of
    // MORE room, never less: on a 1M-native model the M1 cap (1M) equalled native, so a paying
    // subscriber hit a hard 413 at exactly the point a free user was still fine. Paying for
    // context must never buy a smaller ceiling than not paying.
    let effective_cap = tier.capacity_for_native(native);
    if total_raw > effective_cap {
        return Err(AppError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            msg: format!(
                "michael-compression: {} 档最多接受 {} token，当前累计约 {} token",
                tier.as_str(),
                effective_cap,
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
    // Retrieval is a NICE-TO-HAVE and it is measured AFTER JSON escaping inflates it (observed up
    // to ~1.8x the budget it was selected against). If it no longer fits, drop it and send the
    // message. Dropping costs some recalled detail; failing costs the user the message entirely —
    // and because selection is deterministic, the retry fails identically forever, so the request
    // was wedged in "warming" permanently. Not trimmed, because the text is escaped JSON and
    // cutting it mid-string would hand the model malformed context.
    let retrieved = if base_projected.saturating_add(retrieved.tokens) > budget {
        tracing::warn!(
            base_projected, retrieval_tokens = retrieved.tokens, budget,
            "michael-compression: retrieval overshot the budget after escaping; sending without it"
        );
        RetrievedCompressionHistory { text: None, tokens: 0, segment_count: 0, excerpt_count: 0 }
    } else {
        retrieved
    };
    let projected = base_projected.saturating_add(retrieved.tokens);
    if projected > budget {
        // Only the mandatory part alone still overflows — retrieval cannot rescue that, and it
        // is genuinely a warming condition rather than an accounting artifact.
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
/// 这个账号这一小时还能不能再触发一次代看图。
///
/// 按自然小时分桶（键里带小时数），所以不需要滑动窗口也不需要清理：桶自己会过期。
/// Redis 答不上来时**放行** —— 这是一道花钱的闸，不是安全边界，为一次缓存抖动把所有
/// 人的图片识别关掉是更糟的失败方式。真正保证钱不白花的是 bill_vision_call。
async fn vision_budget_ok(state: &AppState, uid: uuid::Uuid) -> bool {
    let hour = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() / 3600)
        .unwrap_or(0);
    let key = format!("vision:{uid}:{hour}");
    let mut redis = state.redis.clone();
    let n: i64 = match redis::cmd("INCR").arg(&key).query_async(&mut redis).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "视觉配额计数失败，放行");
            return true;
        }
    };
    if n == 1 {
        let _: Result<(), redis::RedisError> = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(3600)
            .query_async(&mut redis)
            .await;
    }
    if n > VISION_CALLS_PER_HOUR {
        tracing::info!(%uid, count = n, "视觉识别已超过每小时配额，本次跳过");
        return false;
    }
    true
}

/// 给「替非视觉模型看图」那一次 gpt-5.5 调用记账。
///
/// 和 `bill_compression_call` 是同一个套路，理由也一样：这是服务端**代用户发起**的
/// 上游调用，花的是运营方的 key。不记账的话，它就是一条绕过计费的通道 —— 而且比
/// 压缩那条更划算，因为视觉输入按 $5/M 计价，用户那边只按便宜模型的文本 token 付钱。
///
/// `use_quota=false`：和这条路由上的正餐调用（chat 结尾那次 `bill(..., false, ...)`）
/// 保持一致，走钱包而不是套餐时段额度。看图是用户自己发起的额外动作，不是套餐内含。
async fn bill_vision_call(
    state: &AppState,
    uid: uuid::Uuid,
    vconn: &Model,
    usage: Option<&serde_json::Value>,
) {
    let reported = usage_is_authoritative(usage);
    let (model_in, model_out) = model_price_override(&vconn.model_prices, "gpt-5.5");
    let cost = resolve_cost(
        &vconn.billing_mode,
        vconn.per_call_cents,
        usage.filter(|_| reported),
        "gpt-5.5",
        vconn.rate,
        vconn.input_price,
        vconn.output_price,
        vconn.cache_read_price,
        vconn.cache_create_price,
        model_in,
        model_out,
        vconn.cache_disabled,
    );
    // 单独打标，和聊天、压缩三者在用量表里分得开。
    let mut tokens = extract_bill_tokens(usage.filter(|_| reported), "michael-vision/gpt-5.5", !reported);
    tokens.request_id = None;
    bill(state, uid, vconn.id, cost, false, &tokens, false, 0).await;
}

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
        conn.cache_disabled,
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
mod route_cooldown_tests {
    use super::*;

    /// 鉴权失败（401 坏 key）后，这条线路必须被冷却，好让后续请求绕开它。
    ///
    /// 这是「claude-opus-4-7 时好时坏一直 401」的真因：之前只有**瞬时**故障（502/超时）
    /// 才冷却，401 这类持久鉴权失败**不冷却**，于是坏 key 的线路一直留在轮换里被反复挑中。
    #[test]
    fn auth_failure_cools_the_route_so_next_request_avoids_it() {
        let id = uuid::Uuid::new_v4();
        let now = Instant::now();
        // 冷却前：不在冷却中，会被正常挑选。
        assert!(route_cooldown_remaining(id, now).is_none());
        // 一次鉴权失败后：进入长冷却（远超瞬时的 20 秒）。
        mark_route_cooldown_auth(id);
        let remaining = route_cooldown_remaining(id, Instant::now())
            .expect("鉴权失败后必须处于冷却中");
        assert!(
            remaining > CHAT_UPSTREAM_ROUTE_COOLDOWN,
            "鉴权失败的冷却（{remaining:?}）必须比瞬时冷却（{CHAT_UPSTREAM_ROUTE_COOLDOWN:?}）长——坏 key 不会在 20 秒内变好",
        );
        assert!(remaining <= CHAT_UPSTREAM_AUTH_COOLDOWN);
    }

    /// 冷却只延长、不缩短：已经在更长冷却里的线路，不会被一次新的鉴权失败缩回去。
    #[test]
    fn auth_cooldown_only_extends() {
        let id = uuid::Uuid::new_v4();
        mark_route_cooldown_auth(id);
        let first = route_cooldown_remaining(id, Instant::now()).unwrap();
        // 再来一次（瞬时冷却更短）：不应把剩余时间缩短。
        mark_route_cooldown(id);
        let second = route_cooldown_remaining(id, Instant::now()).unwrap();
        assert!(second + Duration::from_secs(2) >= first, "冷却被缩短了");
    }

    /// 一条挂着不回话的线路，不该让每一个请求都垫满整段表头预算。
    ///
    /// 冷却表管不到这件事：`route_count > 1` 那道判据决定的是"这一轮换条线走"，而这里
    /// 说的正是**换不了**的情形（模型只有一条线，或强力版把候选压成了一条）。
    #[test]
    fn a_stalling_route_gets_a_short_probe_instead_of_the_full_budget() {
        let id = uuid::Uuid::new_v4();
        let base = STANDARD_MAX_HEADER_WAIT;

        // 没有前科：完整预算。
        assert_eq!(header_wait_for_route(base, id, Instant::now()), base);

        // 卡满过一次之后：压到短探测预算——仍然会发，只是失败得起。
        mark_route_stall(id);
        assert_eq!(
            header_wait_for_route(base, id, Instant::now()),
            CHAT_UPSTREAM_STALLED_PROBE_WAIT
        );
        // 上限：要显著更短，否则这条规则什么也没做。取一半以下，保证等待时间真的腰斩。
        assert!(
            CHAT_UPSTREAM_STALLED_PROBE_WAIT * 2 <= STANDARD_MAX_HEADER_WAIT,
            "短探测预算不够短，省不下多少等待时间"
        );
        // 下限：必须高于健康响应的 p90（实测 21.7s），否则一条只是慢的线路会被反复截断，
        // 每次截断又记一次卡死，自己把自己按死在短预算上。
        assert!(
            CHAT_UPSTREAM_STALLED_PROBE_WAIT >= Duration::from_secs(22),
            "短探测预算低于健康响应的 p90，会把「慢」误判成「挂了」"
        );
    }

    /// 自愈：拿到一次表头就撤销短探测，不需要任何人去后台动配置。
    #[test]
    fn one_successful_response_restores_the_full_header_budget() {
        let id = uuid::Uuid::new_v4();
        mark_route_stall(id);
        assert!(route_recently_stalled(id, Instant::now()));

        clear_route_stall(id);
        assert!(!route_recently_stalled(id, Instant::now()));
        assert_eq!(
            header_wait_for_route(STANDARD_MAX_HEADER_WAIT, id, Instant::now()),
            STANDARD_MAX_HEADER_WAIT
        );
    }

    /// 记号会自己过期，不会把一条早就恢复的线路永远按在短探测上。
    #[test]
    fn the_stall_mark_expires_on_its_own() {
        let id = uuid::Uuid::new_v4();
        mark_route_stall(id);
        let after_memory = Instant::now() + CHAT_UPSTREAM_STALL_MEMORY + Duration::from_secs(1);
        assert!(!route_recently_stalled(id, after_memory));
    }
}

#[cfg(test)]
mod vision_billing_tests {
    /// 「替非视觉模型看图」那一次调用必须计费，而且必须在**返回之前**就结掉。
    ///
    /// 这条守的是一个真实存在过、且线上可利用的漏洞：`vision_preprocess` 拿运营方的
    /// key 打一次 gpt-5.5（$5/M 输入）却一分不记。调用方只要挑一个非原生视觉的模型
    /// （生产上 deepseek-v4-flash / glm-5.2 / grok-4.6 / kimi-k3 / qwen3.8-max /
    /// deepseek-v4-pro 都算），随请求塞图片就能触发。
    ///
    /// 而且它跑在下游请求之前，下游一旦非 2xx，外层 handler 直接 return Err —— 那是
    /// 在 bill() 之前。也就是说「故意让下游报错」= 稳定白嫖，这条路由还没有
    /// InFlightGuard，可以无限并发。
    ///
    /// 用源码断言而不是跑一遍：这个函数要数据库和一个真上游才跑得起来，而要守住的
    /// 性质（有没有记账、记在哪一步）在源码层面是确定的。同样的做法见 oauth.rs。
    #[test]
    fn the_vision_helper_call_is_billed_before_it_can_be_skipped() {
        let src = include_str!("models.rs");
        let body = src
            .split("async fn vision_preprocess(")
            .nth(1)
            .expect("找不到 vision_preprocess");
        let body = &body[..body.find("\nasync fn ").unwrap_or(body.len())];

        assert!(
            body.contains("bill_vision_call("),
            "vision_preprocess 必须给它自己发起的上游调用记账，否则这是一条绕过计费的通道",
        );

        // 记账要发生在解析出描述文本**之前**：描述可能为空、可能解析失败，
        // 但钱在收到响应的那一刻就已经花出去了。
        let bill_at = body.find("bill_vision_call(").expect("bill");
        let desc_at = body.find("desc = Some(").expect("desc");
        assert!(
            bill_at < desc_at,
            "记账必须排在取描述之前 —— 上游已经收费了，拿没拿到可用文本不影响这一点",
        );

        // 记账要在函数内部完成，不能指望外层 handler 后面还会走到 bill()。
        // 外层在下游非 2xx 时会直接 return Err，那一条路径根本到不了计费点。
        assert!(
            !body.contains("return;\n    }\n    // billed by caller"),
            "不要把记账推给调用方 —— 下游失败时调用方会提前返回",
        );
    }

    /// models::chat 必须和另外三条上游路由一样持并发闸 —— 它此前是唯一漏的，而且是
    /// 会替用户发起 gpt-5.5 视觉调用的那一条。
    #[test]
    fn chat_route_acquires_the_inflight_guard() {
        let src = include_str!("models.rs");
        let body = src.split("pub async fn chat(").nth(1).expect("chat");
        let body = &body[..body.find("\npub async fn ").unwrap_or(body.len())];
        assert!(
            body.contains("InFlightGuard::acquire(&state, uid).await?"),
            "models::chat 必须取 InFlightGuard，否则一个账号能挂起任意多个上游请求",
        );
        // 必须在发起上游请求之前就拿到 —— 拿晚了等于没拿。chat 里第一次碰上游是
        // vision_preprocess（它就会打 gpt-5.5），闸必须排在它前面。
        let guard_at = body.find("InFlightGuard::acquire").expect("guard");
        let first_upstream = body.find("vision_preprocess(").expect("vision_preprocess call");
        assert!(guard_at < first_upstream, "并发闸必须早于任何上游调用");
    }

    /// 视觉代看图有每小时配额和张数上限，二者都不能被悄悄拿掉。
    #[test]
    fn vision_has_a_budget_and_an_image_cap() {
        let src = include_str!("models.rs");
        assert!(src.contains("const MAX_VISION_IMAGES"), "缺图片张数上限");
        assert!(src.contains("const VISION_CALLS_PER_HOUR"), "缺每小时配额");
        let body = src.split("async fn vision_preprocess(").nth(1).expect("vp");
        let body = &body[..body.find("\nasync fn ").unwrap_or(body.len())];
        assert!(body.contains("images.truncate(MAX_VISION_IMAGES)"), "没有真正截断图片数");
        assert!(body.contains("vision_budget_ok("), "没有查每小时配额");
        // 配额查询必须在选定 vconn（也就是决定要不要真打上游）之前。
        assert!(
            body.find("vision_budget_ok(").unwrap() < body.find("gpt-5.5").unwrap(),
            "配额判定要早于决定发起上游调用",
        );
    }

    /// 触发这条路径的判定不能悄悄变窄：漏掉一个模型 id 就等于给它开一个免费视觉通道。
    #[test]
    fn production_non_vision_models_still_trigger_the_helper() {
        for id in [
            "deepseek-v4-flash",
            "deepseek-v4-pro",
            "glm-5.2",
            "grok-4.6",
            "kimi-k3",
            "qwen3.8-max",
        ] {
            assert!(super::needs_vision_help(id), "{id} 应当走视觉预处理");
        }
        // 原生支持视觉的不该多跑一次。
        for id in ["gpt-5.5", "claude-opus-5", "gemini-3-pro", "qwen2-vl", "o3-mini"] {
            assert!(!super::needs_vision_help(id), "{id} 自己能读图，不该再代看一次");
        }
    }
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
mod route_disable_tests {
    /// A broken per-call route with unpriced models MUST still be disableable. The zero-fee guard
    /// exists to stop unbilled traffic; a route with active=false serves no traffic, so applying
    /// the guard there only traps the operator — whose sole remaining escape was DELETE, which
    /// destroys the api key, enabled-model set, display names and per-model prices.
    #[test]
    fn zero_fee_guard_is_scoped_to_routes_that_still_serve() {
        let src = include_str!("models.rs");
        let i = src
            .find("if active && billing_mode == \"per_call\"")
            .expect("the zero-fee guard must be gated on `active` — disabling must never be blocked");
        // and the gate must sit on the guard itself, not somewhere incidental
        let window = &src[i..i + 200];
        assert!(
            window.contains("per_call_cents == 0") && window.contains("per_call_micro_usd == 0"),
            "the `active &&` gate must be on the zero-fee guard, not on an unrelated condition"
        );
    }
}


#[cfg(test)]
mod upstream_message_tests {
    use super::*;

    /// 上游报错回给用户之前，主机名和凭据都必须消失。
    ///
    /// 每一条都对应一个真实形状：reqwest 的 `Display` 会追加 ` for url (…)`；OpenAI 的
    /// `sk-`、Google 的 `AIza`、以及没有任何前缀的长 token。health.rs 专门为「base_url 不该
    ///出现在登录用户能看到的地方」配了断言，这里守的是同一条线在错误路径上不被绕过。
    #[test]
    fn upstream_errors_never_carry_the_host_or_a_key() {
        let cases = [
            "error sending request for url (https://api.upstream-vendor.com/v1/chat/completions)",
            "connect error http://10.0.0.7:8080/v1/models",
            "invalid api key sk-proj-AbCdEf0123456789AbCdEf0123456789",
            "unauthorized: aizasyd-1234567890abcdefghijklmnop",
            "bad token bearer eyjhbgciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
            "denied 4f2a91c30000111122223333444455556666",
        ];
        for raw in cases {
            let out = safe_upstream_error_excerpt(&raw.to_lowercase());
            assert!(
                !out.contains("http://") && !out.contains("https://"),
                "URL 泄露了: {out}"
            );
            assert!(!out.contains("upstream-vendor"), "上游主机名泄露了: {out}");
            assert!(!out.contains("10.0.0.7"), "上游地址泄露了: {out}");
            assert!(!out.contains("sk-proj-a"), "密钥泄露了: {out}");
            assert!(!out.contains("aizasyd-1"), "密钥泄露了: {out}");
            assert!(
                !out.contains("eyjhbgci") && !out.contains("4f2a91c30000"),
                "长 token 泄露了: {out}"
            );
        }
    }

    /// 一句话里有两个密钥，两个都要处理 —— 原来只替换第一处。
    #[test]
    fn every_occurrence_is_redacted_not_just_the_first() {
        let out = safe_upstream_error_excerpt(
            "sk-aaaa1111bbbb2222cccc failed then sk-dddd3333eeee4444ffff also failed",
        );
        assert!(!out.contains("sk-aaaa1111"), "第一个没脱敏: {out}");
        assert!(!out.contains("sk-dddd3333"), "第二个没脱敏: {out}");
    }

    /// 脱敏不能把话说没了 —— 用户还得知道大概是什么问题。
    #[test]
    fn the_human_readable_part_survives() {
        let out = safe_upstream_error_excerpt(
            "rate limit exceeded for url (https://api.vendor.com/v1/chat)",
        );
        assert!(out.contains("rate limit exceeded"), "有用的部分被删掉了: {out}");
    }


    /// 上游用中文报余额不足时，必须被认出来 —— 而不是压成"上游暂时不可用"。
    ///
    /// 线上真实报文（2026-08-05，claude-sonnet-5 → changhuai.ai）：
    ///   {"error":{"type":"new_api_error","message":
    ///    "预扣费额度失败, 用户剩余额度: ＄0.055828, 需要预扣费额度: ＄0.134302"}}
    /// 旧代码只匹配 insufficient_balance / insufficient account balance 两个英文串，
    /// 于是用户看到的是"线路失败，请换个模型"，真实原因是账户只剩五分钱 —— 排查方向
    /// 被完全带偏。
    #[test]
    fn chinese_balance_errors_are_recognised() {
        let real = r#"{"error":{"type":"new_api_error","message":"预扣费额度失败, 用户剩余额度: ＄0.055828, 需要预扣费额度: ＄0.134302"}}"#;
        let msg = friendly_upstream_for_test(403, real);
        assert!(msg.contains("余额不足"), "必须点名余额不足，实际：{msg}");
        assert!(
            msg.contains("0.055828") && msg.contains("0.134302"),
            "必须把上游说的「还剩多少 / 需要多少」带给用户，实际：{msg}",
        );
        assert!(
            !msg.contains("上游暂时不可用，请换个模型或稍后再试。"),
            "不能再退回那句什么都没说的兜底：{msg}",
        );
    }

    /// 没被任何分支认出来的错误，也必须带上上游原话，而不是一句泛泛的"不可用"。
    #[test]
    fn unmapped_errors_still_carry_the_upstream_text() {
        let msg = friendly_upstream_for_test(418, r#"{"error":{"message":"teapot overheated"}}"#);
        assert!(msg.contains("teapot overheated"), "上游原话必须带出来：{msg}");
        assert!(msg.contains("418"), "状态码要带上，方便对日志：{msg}");
    }

    /// 脱敏不能因为改了兜底而失效。
    #[test]
    fn upstream_text_is_still_key_redacted() {
        let leaked = r#"{"error":{"message":"bad key sk-proj-AAAABBBBCCCCDDDDEEEEFFFFGGGGHHHHIIIIJJJJKKKKLLLL"}}"#;
        let msg = friendly_upstream_for_test(500, leaked);
        assert!(!msg.contains("sk-proj-AAAA"), "密钥不能进用户可见的报错：{msg}");
        assert!(msg.contains("[redacted-key]"), "应留下脱敏标记：{msg}");
    }
}

#[cfg(test)]
mod model_group_tests {
    /// 只有这几个地方读得到 `group_into`。
    ///
    /// 这个功能的全部承诺是「分组只改 IDE 模型选择器上的标题」—— 不改请求走哪条线路、不改
    /// 计费、不改用量归属。只要选线路或算钱的代码读到了这一列，那句话就不再成立。所以这里
    /// 不是检查某一处写法，而是检查它的作用域：出现在别处就是功能变质了。
    const ALLOWED: &[&str] = &[
        "Model",           // 结构体字段本身
        "GroupReq",        // 请求体
        "admin_group",     // 唯一的写入口
        "admin_list",      // 后台列表要显示当前分到哪儿
        "list_for_client", // 唯一的读用途：算 `group` 那个标题
    ];

    /// `pos` 落在哪个顶层条目里 —— 往前找最近一个顶格的 `fn` / `struct` 声明。
    ///
    /// 只认顶格的：函数体里缩进的闭包和内部 fn 不该顶掉外层的名字。长的关键字排在前面，
    /// 好让 `pub async fn foo` 认出 foo 而不是在 `fn ` 上匹配失败。
    fn owner(src: &str, pos: usize) -> &str {
        let mut name = "<file>";
        for line in src[..pos].lines() {
            for kw in ["pub async fn ", "pub fn ", "async fn ", "fn ", "pub struct ", "struct "] {
                if let Some(tail) = line.strip_prefix(kw) {
                    name = tail
                        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                        .next()
                        .unwrap_or("<anon>");
                    break;
                }
            }
        }
        name
    }

    #[test]
    fn grouping_stays_out_of_routing_and_billing() {
        let src = include_str!("models.rs");
        // 测试自己这一段当然满篇都是这个词，从源码里切掉再查。
        let body = &src[..src
            .find("mod model_group_tests")
            .expect("这个测试模块自己得在文件里")];

        let mut seen = 0;
        for (i, _) in body.match_indices("group_into") {
            let who = owner(body, i);
            assert!(
                ALLOWED.contains(&who),
                "`group_into` 出现在 `{who}` 里。分组是纯展示的：一旦选线路或计费的代码读它，\
                 「计费和用量还记在原线路上」就成了假话。要么把它挪回去，要么先想清楚再改这份名单。",
            );
            seen += 1;
        }
        assert!(seen >= 4, "一处都没扫到，说明这个断言其实没在检查什么");
    }

    /// 配错的分组只能让分组不生效，不能让模型从选择器里消失。
    ///
    /// 指向自己、指向已删除的线路、指向一条已停用因而不在这次查询结果里的线路 —— 三种都得
    /// 退回线路自己的名字。少了这一层兜底，一次手滑就能让一整条线路的模型在 IDE 里凭空不见，
    /// 而后台看上去一切正常。
    #[test]
    fn a_broken_grouping_can_never_hide_a_model() {
        let src = include_str!("models.rs");
        let i = src
            .find("let group = m")
            .expect("list_for_client 必须先算出显示标题再往下走");
        // 按字符取窗口，不按字节。这一段前后都是中文注释，字节切片会在某个汉字中间
        // 断开然后 panic —— 报出来的是 "not a char boundary"，和这条测试真正守的东西
        // 毫无关系，会把人引到完全错误的方向去查。
        let window: String = src[i..].chars().take(300).collect();
        let window = window.as_str();
        assert!(
            window.contains(".filter(|target| *target != m.id)"),
            "指向自己要挡掉，否则 label_of 查到的就是它自己，白绕一圈",
        );
        assert!(
            window.contains("unwrap_or(m.label.as_str())"),
            "解析不出目标时必须退回线路自己的名字，不能留空、更不能跳过这个模型",
        );
    }

    /// 写入口只在三种情况下拒收，每一种都得留在原地。
    #[test]
    fn admin_group_refuses_the_three_shapes_that_lie_to_the_operator() {
        let src = include_str!("models.rs");
        let i = src.find("pub async fn admin_group(").expect("写入口得在");
        let body = &src[i..i + 2400];
        assert!(
            body.contains("if target == id"),
            "分到自己名下是个空操作，但界面上看着像生效了",
        );
        assert!(
            body.contains("target_grouped"),
            "目标自己已经分到别处时要拒收：客户端只解析一跳，链式分组的结果和人想的不一样",
        );
        assert!(
            body.contains("has_children"),
            "自己名下还挂着线路时要拒收，否则 A→B、B→A 就成了环",
        );
        assert!(
            body.contains("admin_only(&claims)"),
            "分组改的是所有用户看到的模型列表，必须是管理员",
        );
    }

}

#[cfg(test)]
mod relay_truncation_tests {
    use super::looks_like_relay_truncation;

    /// 检测判据必须认得出协议校验器真实吐出的**每一种**截断错误。
    ///
    /// 这条守的是一次静默失效：原判据里写着 `"ended before protocol completion"`，
    /// 而那句消息早已被改写成 `ended before message_stop` / `ended before tool_use … completed`。
    /// 检测没跟着改，于是"中转丢块自愈"对最高频的几种截断一个都不触发——线路不被钳位，
    /// 客户端把同一个注定失败的请求原样重掷最多 10 次。用户看到的是：内容已经出来一半，
    /// 然后长时间干等，而每一次重试都会再被掐一次。
    ///
    /// 失效方式完全无声：没有报错、没有降级提示，只是自愈不再发生。
    #[test]
    fn 真实截断错误一个都不漏() {
        // 逐条取自协议校验器里 Err(...) 的实际文案
        for err in [
            "anthropic tool call \"write_file\" produced incomplete arguments JSON: EOF",
            "Anthropic stream ended before message_stop",
            "Anthropic stream ended before tool_use \"edit_file\" completed",
            "Anthropic stream ended with an incomplete SSE frame",
            "OpenAI upstream stream ended with an incomplete SSE frame",
            "OpenAI upstream stream ended without terminal data: [DONE]",
            "upstream stream stalled for 180 seconds",
            "OpenAI SSE tool call ended without function.name",
            // 线上最高频的那一种（丢块后 tool_use 的 input 是残的）
            "Anthropic tool call \"edit_file\" is missing required arguments: old_string, new_string",
        ] {
            assert!(
                looks_like_relay_truncation(err),
                "这是中转把后半段掐了，必须触发线路钳位，否则会被原样重掷 10 次：{err}"
            );
        }
    }

    /// 不是截断的失败不能误判——钳位会压低思考预算，对健康线路是纯损失。
    #[test]
    fn 非截断失败不误触发钳位() {
        for err in [
            "upstream sent no response headers within 57s",
            "Anthropic SSE contains invalid UTF-8: bad byte",
            "Anthropic tool_use \"x\" input must be a JSON object",
            "429 Too Many Requests",
            "upstream returned 500",
            // 上游自己报错，不是丢块——钳位帮不上忙
            "Anthropic streaming error: Upstream request failed",
        ] {
            assert!(
                !looks_like_relay_truncation(err),
                "这不是截断，钳位只会白白压低思考预算：{err}"
            );
        }
    }

    /// 判据与校验器的文案必须留在同一份源码里，改一边就该发现另一边。
    ///
    /// 这里直接扫 models.rs：凡是 Err 文案里带 "ended before" / "ended without" /
    /// "incomplete SSE frame" / "incomplete arguments JSON" 的，都必须被判据认出来。
    #[test]
    fn relay_truncation_signatures_stay_in_sync() {
        let src = include_str!("models.rs");
        let mut missed = Vec::new();
        for line in src.lines() {
            let Some(start) = line.find('"') else { continue };
            let rest = &line[start + 1..];
            let Some(end) = rest.find('"') else { continue };
            let text = &rest[..end];
            // 也要扫"参数缺失"这一类：丢块的表现之一是 tool_use 的 input 残缺，
            // 被必填参数校验拦下，而它的文案里一个"截断"字样都没有。上一版漏的就是它。
            let looks_like_truncation_message = text.contains("ended before")
                || text.contains("ended without")
                || text.contains("incomplete SSE frame")
                || text.contains("incomplete arguments JSON")
                || text.contains("is missing required arguments");
            // 只看校验器造出来的错误文案，跳过判据自己那张表和测试用例
            if !looks_like_truncation_message || text.len() < 15 {
                continue;
            }
            if !looks_like_relay_truncation(text) {
                missed.push(text.to_string());
            }
        }
        missed.sort();
        missed.dedup();
        assert!(
            missed.is_empty(),
            "这些截断错误不会触发中转丢块自愈，线路不会被钳位：\n{}",
            missed.join("\n")
        );
    }
}

#[cfg(test)]
mod power_route_tests {
    /// 只看派单函数那一段源码，别让本模块自己的字面量把断言喂饱。
    fn dispatch_src() -> String {
        let s = include_str!("models.rs");
        s[..s.find("mod power_route_tests").unwrap_or(s.len())].to_string()
    }

    #[test]
    fn 强力版是筛选而不是排序() {
        // 用户点了强力版，就该走强力线路。没有可用的就明确报错，不能悄悄退回普通
        // 线路——那等于把他的选择改掉。这一轮刚从思考档位里拿掉过同样的行为
        //（max 被静默降成 high），不能在这儿又长回来。
        let src = dispatch_src();
        let at = src
            .find("let want_power")
            .expect("强力版筛选整段没了，后台那个开关就没人读了");
        let block = &src[at..(at + 900).min(src.len())];
        assert!(
            block.contains("return Err("),
            "没有强力线路时没报错——请求会静默落到普通线路上，用户看不出自己被降级了"
        );
        assert!(
            block.contains("candidates = power"),
            "筛出来的强力线路没被用上，筛选等于白做"
        );
        assert!(
            !block.contains("sort_by") && !block.contains("unwrap_or(candidates)"),
            "强力版被写成了排序/兜底，那是静默降级"
        );
    }

    #[test]
    fn 强力线路不自成一个分组也不吞掉模型() {
        // 用户的原话：「我明明把强力版放入到按钮里面了，为什么还会出现在模型列表里面？」
        // 强力版是悬浮卡片右上角那个开关，不该在选择器里另起一个标题、摆一批和普通分组
        // 重名的模型。
        let src = dispatch_src();
        let at = src
            .find("let power_ids")
            .expect("list_for_client 里算强力 id 并集那段没了，按钮会退回到猜模型名");
        let block = &src[at..(at + 2600).min(src.len())];
        assert!(
            block.contains("if m.power_route && plain_ids.contains(&mid)"),
            "强力线路的条目又开始往列表里推了，那个重复分组会回来"
        );
        assert!(
            block.contains("continue"),
            "只算了并集没有跳过，等于没改"
        );
        // 反过来的那一半：只有强力线路提供的 id 必须照常列出，否则这个模型就再也选不到。
        // 判据是 plain_ids（普通线路的并集），不是"这条线路是不是强力线路"。
        assert!(
            !block.contains("if m.power_route {\n                continue"),
            "按整条线路无条件跳过了 —— 只挂在强力线路上的模型会从选择器里消失"
        );
        // 这一条在 json! 里，离 power_ids 有一大段中文注释的距离，所以扫整段派单源码
        // 而不是窗口。dispatch_src 已经把本测试模块切掉了，不会自己喂饱自己。
        assert!(
            src.contains("\"power_route_available\": power_ids.contains(&mid)"),
            "没下发这个模型有没有强力线路，客户端只能靠猜模型名，按钮会画在没有强力线路的模型上"
        );
        // 开箱默认模型也要下发。不下发的话客户端只能取列表第一个，而那是 enabled_models
        // 的字母序 —— 每个新用户都会落在 claude-fable-5 上（实测硬失败率 18.8%，在售最高）。
        assert!(
            src.contains("\"default\": !default_model_id.is_empty() && mid == default_model_id"),
            "没下发开箱默认模型，新用户仍然由字母序决定用哪个模型"
        );
        // 空设置必须等于"一个都不标"，让客户端沿用旧行为——不能因为没配置就把第一个
        // 模型标成默认，那等于把这个坑原样保留还多一层伪装。
        assert!(
            src.contains("!default_model_id.is_empty()"),
            "没配置时不许标任何模型为默认"
        );
    }

    #[test]
    fn 没点强力版就不该被派到强力线路上() {
        // 强力线路从选择器里隐掉之后，它仍然留在普通请求的候选池里，而主线路取的是
        // candidates.first()，顺序由 ORDER BY sort, created_at 决定 —— 运维调一格 sort，
        // 所有普通请求就静默改走强力线路、按它计费，界面上看不出来。
        let src = dispatch_src();
        let at = src.find("let want_power").expect("派单那段没了");
        let block = &src[at..(at + 2400).min(src.len())];
        assert!(
            block.contains("filter(|m| !m.power_route)"),
            "普通请求没把强力线路排除掉，排序一变就会悄悄接普通流量"
        );
        assert!(
            block.contains("if !plain.is_empty()"),
            "无条件排除了强力线路 —— 只有强力线路提供的模型会变成发不出请求"
        );
    }

    #[test]
    fn 卡片上的缓存价必须和账单读同一条规则() {
        // 缓存价有三级：管理员手填 > 实时目录的真实价 > 按输入价推算。这条规则本来只
        // 长在报价接口里；卡片要显示缓存价，若各写一遍，两处迟早分叉——而分叉的表现是
        // 卡片写一个价、账单按另一个价扣，用户对不上账还查不出原因。
        let src = dispatch_src();
        assert!(
            src.contains("fn cache_prices_for("),
            "缓存价的三级规则没抽成函数，展示和计费又会各写一份"
        );
        // 两边都得**调**它，而不是各自照抄一遍。
        let n = src.matches("cache_prices_for(").count();
        assert!(n >= 3, "cache_prices_for 只出现 {n} 次——有一侧没在调它");
        assert!(
            src.contains("\"cache_read_price\": cache_read"),
            "缓存读价没下发，卡片上那一格永远是空的"
        );
        assert!(
            src.contains("\"cache_write_price\": cache_write"),
            "缓存写价没下发"
        );
        // 三级里的每一级都得在那个函数里，少一级就意味着某种配置下卡片会显示 0。
        let at = src.find("fn cache_prices_for(").unwrap();
        let body = &src[at..(at + 1400).min(src.len())];
        for level in ["model.cache_read_price", "e.cache_read_price", "CACHE_READ_FACTOR"] {
            assert!(body.contains(level), "缓存价少了 {level} 这一级");
        }
    }

    #[test]
    fn 开关必须能从后台存进来也读得出去() {
        // 断的是链路：后台勾选 → 落库 → 派单时读到。任何一环缺失，后台那个复选框
        // 就是个装饰品。
        let src = dispatch_src();
        assert!(src.contains("pub power_route: bool"), "Model 上没有这个字段，派单读不到");
        assert!(
            src.contains("power_route: Option<bool>"),
            "UpdateReq 上没有这个字段，后台勾了也存不进去"
        );
        // 要在**派单那一段**里读到，不是"整个文件里出现过这个名字"——后台的
        // 增删改查里到处都是 power_route，照名字找会被它们喂饱。
        let at = src.find("let want_power").expect("强力版筛选整段没了");
        let block = &src[at..(at + 900).min(src.len())];
        assert!(
            block.contains("m.power_route"),
            "派单时压根没读这个字段，后台勾选不影响任何行为"
        );
        let mig = include_str!("../migrations/20260841_power_route.sql");
        assert!(
            mig.contains("power_route"),
            "迁移没建这一列，线上启动就会因为查不到列而炸"
        );
    }
}
