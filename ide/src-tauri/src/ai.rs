use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use std::time::Duration;
use tauri::ipc::Channel;

/// Shared HTTP client. The agentic loop fires many sequential requests; a single
/// pooled client reuses TCP+TLS connections (keep-alive) instead of doing a fresh
/// handshake on every turn — the main source of "backend feels laggy" between
/// turns. No total `.timeout()` is set because chat responses stream open-ended;
/// only the connect phase is bounded.
static HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(8)
        .tcp_keepalive(Duration::from_secs(60))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});

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
    let base = config.base_url.trim_end_matches('/');
    if !(base.starts_with("http://") || base.starts_with("https://")) {
        return Err("AI base URL must start with http:// or https://".into());
    }
    let url = format!("{base}/chat/completions");
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
        return Err(format!("AI request failed ({status}): {text}"));
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

async fn ai_chat_inner(
    config: AiConfig,
    messages: Vec<serde_json::Value>,
    tools: Option<Vec<serde_json::Value>>,
    on_event: Channel<AiEvent>,
) -> Result<(), String> {
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
    let mut payload = serde_json::json!({
        "model": config.model,
        "stream": true,
        "messages": messages,
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

    // ── Prompt caching for Anthropic / Claude upstreams ─────────────────────
    // Anthropic reuses content marked with a `cache_control` breakpoint and serves
    // the LONGEST matching cached prefix (up to 4 breakpoints). We mark (1) the
    // system prompt — kept byte-stable in main.js so it caches across every turn —
    // and (2) a ROLLING breakpoint on the most recent user message, so each new
    // turn finds the prior conversation already cached instead of re-billing it.
    //
    // Gated on the MODEL name (the gateway routes by model — base_url is always the
    // gateway, so the old base_url check never fired). Only PLAIN-STRING content is
    // reshaped, so tool/multimodal messages — and any non-Anthropic upstream — are
    // left byte-for-byte untouched and can't break. DeepSeek / OpenAI / Kimi cache
    // the stable prefix automatically, so they need no markup at all.
    let model_lc = config.model.to_ascii_lowercase();
    if config.base_url.contains("anthropic")
        || model_lc.contains("claude")
        || model_lc.contains("anthropic")
    {
        if let Some(arr) = payload["messages"].as_array_mut() {
            if let Some(first) = arr
                .iter_mut()
                .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("system"))
            {
                mark_cache_breakpoint(first);
            }
            if let Some(last_user) = arr
                .iter_mut()
                .rev()
                .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
            {
                mark_cache_breakpoint(last_user);
            }
        }
    }

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
        let message = format!("AI request failed ({status}): {text}");
        let _ = on_event.send(AiEvent::Error {
            message: message.clone(),
        });
        return Err(message);
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
    // Stall guard for the hung turn ("半天不走内容"): the upstream sends the response
    // headers, then stops producing — either fully silent (no bytes) OR dribbling
    // keepalive/ping bytes with no actual content (which also defeats the gateway's
    // own idle timeout, since from its side bytes are still flowing). Poll the read
    // on a short interval and end the turn only when NO real progress
    // (token/reasoning/tool-call) has happened for STALL_LIMIT — so a slow-but-alive
    // stream is never killed, but a genuinely stuck one stops instead of hanging.
    const READ_POLL: std::time::Duration = std::time::Duration::from_secs(10);
    const STALL_LIMIT: std::time::Duration = std::time::Duration::from_secs(75);
    let mut last_progress = std::time::Instant::now();
    loop {
        let chunk = match tokio::time::timeout(READ_POLL, stream.next()).await {
            Ok(Some(Ok(c))) => c,
            // A mid-stream read error means the connection dropped partway (common
            // on cross-border / lossy links — "error decoding response body"). Keep
            // what we've already streamed and end the turn gracefully.
            Ok(Some(Err(_e))) => {
                let _ = on_event.send(AiEvent::Error {
                    message: "连接中断（网络波动），已保留生成的部分，请点重试继续。".to_string(),
                });
                let _ = on_event.send(AiEvent::Done);
                return Ok(());
            }
            Ok(None) => break, // stream ended normally
            Err(_elapsed) => {
                // No bytes this interval — only bail if nothing has progressed at all.
                if last_progress.elapsed() >= STALL_LIMIT {
                    let _ = on_event.send(AiEvent::Error {
                        message: "模型长时间无响应（连接卡住），已停止本轮，请点重试。".to_string(),
                    });
                    let _ = on_event.send(AiEvent::Done);
                    return Ok(());
                }
                continue;
            }
        };
        buf.extend_from_slice(&chunk);

        // Server-sent events are newline-delimited `data: {...}` lines.
        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            let line_bytes: Vec<u8> = buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line_bytes);
            let line = line.trim();
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data == "[DONE]" {
                let _ = on_event.send(AiEvent::Done);
                return Ok(());
            }
            if data.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                // Usage normally rides the FINAL chunk (choices may be empty there).
                // Cached-prompt tokens are reported differently per provider — take
                // whichever field is present: OpenAI/DeepSeek `prompt_tokens_details
                // .cached_tokens`, DeepSeek `prompt_cache_hit_tokens`, or Anthropic-
                // style `cache_read_input_tokens`.
                if let Some(usage) = v.get("usage").filter(|u| u.is_object()) {
                    let prompt = usage["prompt_tokens"].as_u64().unwrap_or(0);
                    let completion = usage["completion_tokens"].as_u64().unwrap_or(0);
                    let cached = usage["prompt_tokens_details"]["cached_tokens"]
                        .as_u64()
                        .or_else(|| usage["prompt_cache_hit_tokens"].as_u64())
                        .or_else(|| usage["cache_read_input_tokens"].as_u64())
                        .unwrap_or(0);
                    if prompt > 0 || completion > 0 {
                        let _ = on_event.send(AiEvent::Usage {
                            prompt_tokens: prompt as u32,
                            completion_tokens: completion as u32,
                            cached_tokens: cached as u32,
                        });
                    }
                }
                let delta = &v["choices"][0]["delta"];
                // Thinking / reasoning stream (DeepSeek/MiniMax: reasoning_content; some: reasoning).
                if let Some(rt) = delta["reasoning_content"]
                    .as_str()
                    .or_else(|| delta["reasoning"].as_str())
                {
                    if !rt.is_empty() {
                        let _ = on_event.send(AiEvent::Reasoning {
                            delta: rt.to_string(),
                        });
                        last_progress = std::time::Instant::now();
                    }
                }
                if let Some(text) = delta["content"].as_str() {
                    if !text.is_empty() {
                        let _ = on_event.send(AiEvent::Token {
                            delta: text.to_string(),
                        });
                        last_progress = std::time::Instant::now();
                    }
                }
                if let Some(tcs) = delta["tool_calls"].as_array() {
                    for tc in tcs {
                        let index = tc["index"].as_u64().unwrap_or(0) as u32;
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
                            last_progress = std::time::Instant::now();
                        }
                    }
                }
            }
        }

        // Bytes may keep flowing (keepalive / ping comments) with no content — if
        // there has been no real progress for STALL_LIMIT, treat it as a stall too.
        if last_progress.elapsed() >= STALL_LIMIT {
            let _ = on_event.send(AiEvent::Error {
                message: "模型长时间无响应（连接卡住），已停止本轮，请点重试。".to_string(),
            });
            let _ = on_event.send(AiEvent::Done);
            return Ok(());
        }
    }

    let _ = on_event.send(AiEvent::Done);
    Ok(())
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

    // Resolve and require every resolved address to be public (SSRF guard).
    let addrs: Vec<_> = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|e| format!("DNS 解析失败: {e}"))?
        .collect();
    if addrs.is_empty() {
        return Err("无法解析主机".into());
    }
    for a in &addrs {
        if !ip_fetch_allowed(a.ip()) {
            return Err("拒绝访问该地址（link-local / 元数据 / 多播等）".into());
        }
    }

    // Redirects disabled so a 3xx can't silently bounce us to a link-local /
    // metadata address; the redirect is surfaced to the model so it can choose to
    // follow it (which re-runs ip_fetch_allowed on the new URL).
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(parsed)
        .header(reqwest::header::USER_AGENT, "Michael-IDE-Agent/1.0")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    if status.is_redirection() {
        let loc = resp
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("(unknown)");
        return Ok(format!(
            "[{} 重定向到: {loc}\n如需跟进，请用该完整 URL 再次调用 web_fetch。]",
            status.as_u16()
        ));
    }
    if !status.is_success() {
        return Err(format!("HTTP {}", status.as_u16()));
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

/// Web search (DuckDuckGo HTML, no API key) so the agent can FIND docs/articles,
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
    let results = ddg_search_multi(q).await;
    if results.is_empty() {
        return Ok(format!(
            "「{q}」这次没搜到结果（搜索引擎可能临时限流，或关键词太宽泛）。可以：① 换更具体、用英文的关键词重试；② 直接用 web_fetch 打开你已知的相关网址 / 官方文档读全文。"
        ));
    }
    let mut out = format!("搜索「{q}」的结果：\n");
    for (i, (title, url, snippet)) in results.iter().take(8).enumerate() {
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

/// Try several DuckDuckGo endpoints / UAs, return the first non-empty result set.
async fn ddg_search_multi(q: &str) -> Vec<(String, String, String)> {
    const UAS: [&str; 2] = [
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36",
    ];
    // 1) html.duckduckgo.com — richest snippets.
    for ua in UAS {
        if let Ok(client) = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
        {
            if let Ok(resp) = client
                .post("https://html.duckduckgo.com/html/")
                .header(reqwest::header::USER_AGENT, ua)
                .form(&[("q", q), ("kl", "wt-wt")])
                .send()
                .await
            {
                if let Ok(html) = resp.text().await {
                    let r = parse_ddg_results(&html);
                    if !r.is_empty() {
                        return r;
                    }
                }
            }
        }
    }
    // 2) Bing — a DIFFERENT engine, so a DDG block falls through to it.
    let bing_url = format!(
        "https://www.bing.com/search?q={}&setlang=en",
        urlencoding(q)
    );
    for ua in UAS {
        if let Ok(client) = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
        {
            if let Ok(resp) = client
                .get(&bing_url)
                .header(reqwest::header::USER_AGENT, ua)
                .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
                .send()
                .await
            {
                if let Ok(html) = resp.text().await {
                    let r = parse_bing(&html);
                    if !r.is_empty() {
                        return r;
                    }
                }
            }
        }
    }
    // 3) lite.duckduckgo.com — much lighter HTML, rarely blocked.
    for ua in UAS {
        if let Ok(client) = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
        {
            if let Ok(resp) = client
                .post("https://lite.duckduckgo.com/lite/")
                .header(reqwest::header::USER_AGENT, ua)
                .form(&[("q", q)])
                .send()
                .await
            {
                if let Ok(html) = resp.text().await {
                    let r = parse_ddg_lite(&html);
                    if !r.is_empty() {
                        return r;
                    }
                }
            }
        }
    }
    Vec::new()
}

/// Minimal percent-encoding for a query string (no extra crate).
fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
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
        if href.starts_with("http") && !title.is_empty() && !out.iter().any(|(_, u, _)| u == &href) {
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
