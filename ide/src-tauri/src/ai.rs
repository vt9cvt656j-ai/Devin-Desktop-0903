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

    // Prompt caching: Anthropic-compatible endpoints reuse content marked with a
    // `cache_control` breakpoint across requests. Tag the large, stable system
    // prompt so it isn't re-billed/re-processed every turn. Gated on Anthropic;
    // other providers (DeepSeek, OpenAI, …) cache the stable prefix automatically,
    // so their payload is left byte-for-byte unchanged.
    if config.base_url.contains("anthropic") {
        if let Some(arr) = payload["messages"].as_array_mut() {
            if let Some(first) = arr
                .iter_mut()
                .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("system"))
            {
                if let Some(text) = first
                    .get("content")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                {
                    first["content"] = serde_json::json!([
                        { "type": "text", "text": text, "cache_control": { "type": "ephemeral" } }
                    ]);
                }
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
    while let Some(chunk) = stream.next().await {
        // A mid-stream read error means the connection dropped partway (common on
        // cross-border / lossy links — "error decoding response body"). Keep what
        // we've already streamed and end the turn gracefully with a friendly note,
        // instead of failing the whole agent run with a raw decode error.
        let chunk = match chunk {
            Ok(c) => c,
            Err(_e) => {
                let _ = on_event.send(AiEvent::Error {
                    message: "连接中断（网络波动），已保留生成的部分，请点重试继续。".to_string(),
                });
                let _ = on_event.send(AiEvent::Done);
                return Ok(());
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
                        }
                    }
                }
            }
        }
    }

    let _ = on_event.send(AiEvent::Done);
    Ok(())
}

/// SSRF guard: only allow public/global addresses, never loopback, private,
/// link-local, CGNAT, multicast, etc.
fn ip_is_public(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            let o = v4.octets();
            !(v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || v4.is_documentation()
                || v4.is_multicast()
                || o[0] == 0
                // CGNAT 100.64.0.0/10
                || (o[0] == 100 && (64..=127).contains(&o[1])))
        }
        std::net::IpAddr::V6(v6) => {
            let s = v6.segments();
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                // unique-local fc00::/7
                || (s[0] & 0xfe00) == 0xfc00
                // link-local fe80::/10
                || (s[0] & 0xffc0) == 0xfe80
                // IPv4-mapped ::ffff:0:0/96 — re-check the embedded v4
                || (s[0] == 0 && s[1] == 0 && s[2] == 0 && s[3] == 0 && s[4] == 0 && s[5] == 0xffff))
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
        if !ip_is_public(a.ip()) {
            return Err("拒绝访问本地/内网地址".into());
        }
    }

    // Redirects disabled so a 3xx can't bounce us to an internal address; a
    // redirect is surfaced to the model so it can choose to follow it (which
    // re-runs this guard on the new URL).
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
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post("https://html.duckduckgo.com/html/")
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36",
        )
        .form(&[("q", q)])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("搜索失败 HTTP {}", resp.status().as_u16()));
    }
    let html = resp.text().await.map_err(|e| e.to_string())?;
    let results = parse_ddg_results(&html);
    if results.is_empty() {
        return Ok(format!("「{q}」没有搜到结果。"));
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
