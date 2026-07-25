//! Generic web tools for the agent — call ANY HTTP API and download files. This
//! is what lets the agent use "online tools": REST APIs (GitHub, weather, public
//! data), webhooks, and crucially the local dev server it just started.
//!
//! Security (grounded in SSRF best practice): reject non-http(s) schemes, disable
//! redirects (a 3xx can't bounce us somewhere internal), and bound time + size.
//! Unlike web_fetch (public-only), http_request INTENTIONALLY allows loopback /
//! LAN — the agent's main use is hitting the server it just built — but it still
//! BLOCKS link-local / cloud-metadata (169.254.0.0/16, fe80::/10), which have no
//! legitimate agent use and are the classic SSRF target.

use std::collections::HashMap;
use std::net::{IpAddr, ToSocketAddrs};
use std::time::Duration;

use futures_util::StreamExt;
use serde::Serialize;

const MAX_BODY: usize = 5 * 1024 * 1024; // cap on the response body we buffer
const MAX_DOWNLOAD: usize = 200 * 1024 * 1024; // cap on a downloaded file

#[derive(Serialize)]
pub struct HttpResponse {
    status: u16,
    ok: bool,
    status_text: String,
    headers: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    redirect_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    redirect_url: Option<String>,
    body: String,
    truncated: bool,
    content_type: String,
    body_encoding: String,
}

/// Block only the SSRF-only targets (link-local / cloud-metadata, multicast,
/// broadcast, unspecified). Loopback + private are allowed on purpose so the
/// agent can hit its own dev server and LAN services.
fn addr_blocked(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_link_local() || v4.is_broadcast() || v4.is_multicast() || v4.is_unspecified()
        }
        IpAddr::V6(v6) => {
            let s = v6.segments();
            v6.is_multicast() || v6.is_unspecified() || (s[0] & 0xffc0) == 0xfe80
        }
    }
}

/// Validate a URL for both tools: http/https only, host resolvable, and no
/// resolved address is a blocked (link-local/metadata) one.
fn validate(url: &str) -> Result<reqwest::Url, String> {
    let parsed = reqwest::Url::parse(url.trim()).map_err(|_| "无效的 URL".to_string())?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return Err("只允许 http/https 链接".into()),
    }
    let host = parsed.host_str().ok_or("URL 缺少主机名")?.to_string();
    let port = parsed.port_or_known_default().unwrap_or(80);
    let addrs: Vec<_> = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|e| format!("DNS 解析失败: {e}"))?
        .collect();
    if addrs.is_empty() {
        return Err("无法解析主机".into());
    }
    for a in &addrs {
        if addr_blocked(a.ip()) {
            return Err("拒绝访问链路本地/云元数据地址(如 169.254.169.254)".into());
        }
    }
    Ok(parsed)
}

fn header_value(
    headers: &reqwest::header::HeaderMap,
    name: reqwest::header::HeaderName,
) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn redirect_target(base: &reqwest::Url, location: Option<&str>) -> Option<String> {
    let location = location?.trim();
    if location.is_empty() {
        return None;
    }
    base.join(location).ok().map(|url| url.to_string())
}

fn charset_from_content_type(content_type: &str) -> Option<String> {
    for part in content_type.split(';').skip(1) {
        let part = part.trim();
        if let Some((key, value)) = part.split_once('=') {
            if key.trim().eq_ignore_ascii_case("charset") {
                let value = value.trim().trim_matches('"').trim_matches('\'').trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

fn charset_from_meta(bytes: &[u8]) -> Option<String> {
    let sample = &bytes[..bytes.len().min(4096)];
    let lower = String::from_utf8_lossy(sample).to_lowercase();
    let idx = lower.find("charset")?;
    let tail = &lower[idx + "charset".len()..];
    let eq = tail.find('=')?;
    let mut value = tail[eq + 1..].trim_start();
    if let Some(stripped) = value.strip_prefix('"').or_else(|| value.strip_prefix('\'')) {
        value = stripped;
    }
    let charset: String = value
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect();
    if charset.is_empty() {
        None
    } else {
        Some(charset)
    }
}

fn is_probably_textual(content_type: &str) -> bool {
    let ct = content_type.to_ascii_lowercase();
    ct.starts_with("text/")
        || ct.contains("json")
        || ct.contains("xml")
        || ct.contains("html")
        || ct.contains("javascript")
        || ct.contains("x-www-form-urlencoded")
}

fn decode_body_text(buf: Vec<u8>, content_type: &str) -> (String, String) {
    let label = charset_from_content_type(content_type).or_else(|| charset_from_meta(&buf));
    if let Some(label) = label {
        if let Some(encoding) = encoding_rs::Encoding::for_label(label.as_bytes()) {
            let (decoded, _, had_errors) = encoding.decode(&buf);
            let name = if had_errors {
                format!("{} (lossy)", encoding.name())
            } else {
                encoding.name().to_string()
            };
            return (decoded.into_owned(), name);
        }
    }
    match String::from_utf8(buf) {
        Ok(text) => (text, "UTF-8".into()),
        Err(error) => {
            let bytes = error.into_bytes();
            if is_probably_textual(content_type) {
                (
                    String::from_utf8_lossy(&bytes).into_owned(),
                    "UTF-8 (lossy)".into(),
                )
            } else {
                (
                    format!("[二进制响应，{} 字节，未作为文本返回]", bytes.len()),
                    "binary".into(),
                )
            }
        }
    }
}

/// Call any HTTP API. `method` = GET/POST/PUT/PATCH/DELETE/HEAD. Optional headers
/// and request body. Returns status + headers + body (text, truncated to 5 MB).
#[tauri::command]
pub async fn http_request(
    method: String,
    url: String,
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
    timeout_secs: Option<u64>,
) -> Result<HttpResponse, String> {
    let parsed = validate(&url)?;
    let m = method.trim().to_uppercase();
    let method = reqwest::Method::from_bytes(m.as_bytes())
        .map_err(|_| format!("不支持的 HTTP 方法: {m}"))?;
    let to = timeout_secs.unwrap_or(30).clamp(1, 120);

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(to))
        .build()
        .map_err(|e| e.to_string())?;

    let request_url = parsed.clone();
    let mut req = client
        .request(method, parsed)
        .header(reqwest::header::USER_AGENT, "Michael-IDE-Agent/1.0");
    if let Some(hs) = headers {
        for (k, v) in hs {
            req = req.header(k, v);
        }
    }
    if let Some(b) = body {
        req = req.body(b);
    }

    let resp = req.send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let redirect_location = header_value(resp.headers(), reqwest::header::LOCATION);
    let redirect_url = redirect_target(&request_url, redirect_location.as_deref());
    let mut hmap = HashMap::new();
    for (k, v) in resp.headers().iter() {
        if let Ok(s) = v.to_str() {
            hmap.insert(k.as_str().to_string(), s.to_string());
        }
    }

    let mut buf: Vec<u8> = Vec::new();
    let mut truncated = false;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        if buf.len() + chunk.len() > MAX_BODY {
            let take = MAX_BODY.saturating_sub(buf.len());
            buf.extend_from_slice(&chunk[..take]);
            truncated = true;
            break;
        }
        buf.extend_from_slice(&chunk);
    }
    let (body_text, body_encoding) = decode_body_text(buf, &content_type);

    Ok(HttpResponse {
        status: status.as_u16(),
        ok: status.is_success(),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        headers: hmap,
        redirect_location,
        redirect_url,
        body: body_text,
        truncated,
        content_type,
        body_encoding,
    })
}

/// Download a file from a URL into the workspace. `dest` is resolved relative to
/// `root` (or absolute) and MUST stay inside `root`. Bounded to 200 MB.
#[tauri::command]
pub async fn download_file(root: String, url: String, dest: String) -> Result<String, String> {
    let parsed = validate(&url)?;

    // Contain the write to the workspace.
    let base = std::path::Path::new(&root);
    let target = {
        let d = std::path::Path::new(&dest);
        if d.is_absolute() {
            d.to_path_buf()
        } else {
            base.join(d)
        }
    };
    if target
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("目标路径不能包含 ..".into());
    }
    if !base.as_os_str().is_empty() && !target.starts_with(base) {
        return Err("只能下载到工作区目录内".into());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(parsed)
        .header(reqwest::header::USER_AGENT, "Michael-IDE-Agent/1.0")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status().as_u16()));
    }
    if let Some(len) = resp.content_length() {
        if len > MAX_DOWNLOAD as u64 {
            return Err("文件过大(>200MB)".into());
        }
    }

    let mut buf: Vec<u8> = Vec::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        if buf.len() + chunk.len() > MAX_DOWNLOAD {
            return Err("文件过大(>200MB)".into());
        }
        buf.extend_from_slice(&chunk);
    }

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("建目录失败: {e}"))?;
    }
    let n = buf.len();
    std::fs::write(&target, &buf).map_err(|e| format!("写入失败: {e}"))?;
    Ok(format!("已下载 {n} 字节到 {}", target.display()))
}

/// True when something is already listening on the local Tor SOCKS5 port.
async fn tor_port_up() -> bool {
    tokio::time::timeout(
        Duration::from_secs(2),
        tokio::net::TcpStream::connect(("127.0.0.1", 9050)),
    )
    .await
    .map(|r| r.is_ok())
    .unwrap_or(false)
}

/// Make sure the local Tor SOCKS5 proxy is up. If it's not, launch the installed `tor`
/// binary in the background and wait for it to bootstrap — so the deep-web tools "just
/// work" and self-heal instead of silently returning nothing whenever Tor is stopped.
/// (This is the fix for "不知道深网工具有没有用" — it can no longer be silently dead.)
pub async fn ensure_tor() -> Result<(), String> {
    if tor_port_up().await {
        return Ok(());
    }
    // Find the tor binary — Finder's minimal PATH omits Homebrew, so probe known locations.
    let bin = [
        "/opt/homebrew/bin/tor",
        "/usr/local/bin/tor",
        "/usr/bin/tor",
    ]
    .iter()
    .find(|p| std::path::Path::new(p).exists())
    .map(|s| s.to_string())
    .or_else(|| {
        crate::process_util::augmented_path(None)
            .split(':')
            .map(|d| format!("{d}/tor"))
            .find(|c| std::path::Path::new(c).exists())
    });
    let bin = match bin {
        Some(b) => b,
        None => return Err("Tor 未安装——深网/.onion 访问需要它。装一下：brew install tor（装完自动就能用，会自愈启动）".into()),
    };
    // Launch detached; tor keeps running in the background after we drop the handle.
    std::process::Command::new(&bin)
        .args(["--SocksPort", "9050", "--quiet"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("启动 tor 失败: {e}"))?;
    // Wait for bootstrap (a cold Tor start builds circuits — up to ~30s).
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        if tor_port_up().await {
            return Ok(());
        }
    }
    Err(
        "tor 已拉起但 30s 内未就绪（网络慢/被墙？）——稍等片刻再试，或手动 brew services start tor"
            .into(),
    )
}

/// Make an HTTP request through the local Tor SOCKS5 proxy (127.0.0.1:9050).
/// Supports .onion URLs and regular URLs (anonymized through Tor).
/// Auto-starts `tor` if it isn't already running (self-heal).
#[tauri::command]
pub async fn tor_request(
    method: String,
    url: String,
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
    timeout_secs: Option<u64>,
) -> Result<HttpResponse, String> {
    let trimmed = url.trim();
    let parsed = reqwest::Url::parse(trimmed).map_err(|_| "无效的 URL".to_string())?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return Err("只允许 http/https 链接".into()),
    }
    let m = method.trim().to_uppercase();
    let method = reqwest::Method::from_bytes(m.as_bytes())
        .map_err(|_| format!("不支持的 HTTP 方法: {m}"))?;
    let to = timeout_secs.unwrap_or(60).clamp(1, 300);

    // Self-heal: make sure Tor is up (auto-launch it if the user's service is stopped).
    ensure_tor().await?;

    let proxy = reqwest::Proxy::all("socks5h://127.0.0.1:9050")
        .map_err(|e| format!("Tor 代理配置失败: {e}"))?;
    let client = reqwest::Client::builder()
        .proxy(proxy)
        .redirect(reqwest::redirect::Policy::limited(5))
        .timeout(Duration::from_secs(to))
        .build()
        .map_err(|e| {
            format!("构建 Tor 客户端失败（tor 是否在运行？brew services start tor）: {e}")
        })?;

    let mut req = client.request(method, parsed).header(
        reqwest::header::USER_AGENT,
        "Mozilla/5.0 (Windows NT 10.0; rv:128.0) Gecko/20100101 Firefox/128.0",
    );
    if let Some(hs) = headers {
        for (k, v) in hs {
            req = req.header(k, v);
        }
    }
    if let Some(b) = body {
        req = req.body(b);
    }

    let resp = req.send().await.map_err(|e| {
        if e.is_connect() {
            format!("Tor 连接失败——确认 tor 在运行：brew services start tor（原始错误: {e}）")
        } else {
            e.to_string()
        }
    })?;
    let status = resp.status();
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let mut hmap = HashMap::new();
    for (k, v) in resp.headers().iter() {
        if let Ok(s) = v.to_str() {
            hmap.insert(k.as_str().to_string(), s.to_string());
        }
    }

    let mut buf: Vec<u8> = Vec::new();
    let mut truncated = false;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        if buf.len() + chunk.len() > MAX_BODY {
            let take = MAX_BODY.saturating_sub(buf.len());
            buf.extend_from_slice(&chunk[..take]);
            truncated = true;
            break;
        }
        buf.extend_from_slice(&chunk);
    }
    let (body_text, body_encoding) = decode_body_text(buf, &content_type);

    Ok(HttpResponse {
        status: status.as_u16(),
        ok: status.is_success(),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        headers: hmap,
        redirect_location: None,
        redirect_url: None,
        body: body_text,
        truncated,
        content_type,
        body_encoding,
    })
}

/// Generate an image through the user's OpenAI-compatible gateway.
/// Dedicated image-generation models (gpt-image-*, dall-e-*) → /images/generations API.
/// Chat-based image models (gpt-4o-image, gemini-flash-image, etc.) → /chat/completions.
/// Saves to the workspace → {path, bytes, data_url}.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn generate_image_chat(
    root: String,
    base_url: String,
    api_key: String,
    model: String,
    prompt: String,
    dest: String,
    width: Option<u32>,
    height: Option<u32>,
    request_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err("缺少图像描述 prompt".into());
    }
    if model.trim().is_empty() {
        return Err("缺少图像模型名".into());
    }
    let b = base_url.trim().trim_end_matches('/');
    if !(b.starts_with("http://") || b.starts_with("https://")) {
        return Err("图像模型 base_url 无效".into());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| e.to_string())?;

    let size_str = image_size_for_model(&model, width, height);

    // Try THREE endpoints in sequence so this works on real OpenAI AND 中转站:
    //   1. /v1/images/generations (standard OpenAI Images API — the image-form request;
    //      relay gateways like Sub2API document exactly this shape: model/prompt/size)
    //   2. /v1/responses with image_generation tool (Codex GPT Image route)
    //   3. /v1/chat/completions (chat-wrapped image gen, last resort)
    // The Images API goes first: text-input routes (responses/chat) on relay gateways
    // often degrade to a text answer instead of an image.
    let (bytes, mime): (Vec<u8>, String) = match try_images_api(
        &client,
        b,
        &api_key,
        &model,
        prompt,
        &size_str,
        request_id.as_deref(),
    )
    .await
    {
        Ok(ok) => ok,
        Err(e0) => {
            match try_responses_api(
                &client,
                b,
                &api_key,
                &model,
                prompt,
                &size_str,
                request_id.as_deref(),
            )
            .await
            {
                Ok(ok) => ok,
                Err(e1) => match try_chat_image_api(
                    &client,
                    b,
                    &api_key,
                    &model,
                    prompt,
                    request_id.as_deref(),
                )
                .await
                {
                    Ok(ok) => ok,
                    Err(e2) => return Err(format!("images: {e0} ｜responses: {e1} ｜chat: {e2}")),
                },
            }
        }
    };

    if bytes.is_empty() {
        return Err("生成结果为空".into());
    }
    if bytes.len() > MAX_DOWNLOAD {
        return Err("生成的图过大".into());
    }
    let basep = std::path::Path::new(&root);
    let target = {
        let d = std::path::Path::new(&dest);
        if d.is_absolute() {
            d.to_path_buf()
        } else {
            basep.join(d)
        }
    };
    if target
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("目标路径不能包含 ..".into());
    }
    if !basep.as_os_str().is_empty() && !target.starts_with(basep) {
        return Err("只能存到工作区目录内".into());
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("建目录失败: {e}"))?;
    }
    std::fs::write(&target, &bytes).map_err(|e| format!("写入失败: {e}"))?;
    let data_url = format!("data:{};base64,{}", mime, crate::capture::b64(&bytes));
    Ok(
        serde_json::json!({ "path": dest, "bytes": bytes.len(), "data_url": data_url, "via": model.trim() }),
    )
}

fn with_ide_request_id(
    request: reqwest::RequestBuilder,
    request_id: Option<&str>,
) -> reqwest::RequestBuilder {
    let Some(value) = request_id.filter(|value| {
        (8..=128).contains(&value.len())
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    }) else {
        return request;
    };
    request.header("x-ide-request-id", value)
}

/// Try /v1/responses with the image_generation built-in tool. This is the modern
/// OpenAI Codex / 中转站 (LaoZhang etc.) route — relay stations wrap gpt-image-2
/// behind ChatGPT Plus accounts via this endpoint. The mainline `model` field is
/// what the relay routes on; the image model itself is fixed by the tool.
async fn try_responses_api(
    client: &reqwest::Client,
    b: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    size: &str,
    request_id: Option<&str>,
) -> Result<(Vec<u8>, String), String> {
    let url = if b.ends_with("/v1") {
        format!("{b}/responses")
    } else {
        format!("{b}/v1/responses")
    };
    let body = serde_json::json!({
        "model": model.trim(),
        "input": prompt,
        "tools": [{
            "type": "image_generation",
            "size": size,
            "quality": "high",
            "output_format": "png",
        }],
    });
    let resp = with_ide_request_id(client.post(&url).bearer_auth(api_key.trim()), request_id)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("responses 请求失败: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "responses HTTP {}: {}",
            status.as_u16(),
            text.chars().take(200).collect::<String>()
        ));
    }
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("responses 解析失败: {e}"))?;
    if let Some(err) = v.get("error") {
        let msg = err["message"]
            .as_str()
            .or(err.as_str())
            .unwrap_or("unknown");
        return Err(format!(
            "responses 上游报错: {}",
            msg.chars().take(200).collect::<String>()
        ));
    }
    // Walk the output array for an image_generation_call item with a base64 result.
    let output = v.get("output").and_then(|o| o.as_array());
    if let Some(arr) = output {
        for item in arr {
            if item["type"] == "image_generation_call" {
                if let Some(b64) = item["result"].as_str().or(item["b64_json"].as_str()) {
                    let raw = if b64.contains(',') {
                        b64.split_once(',').map(|x| x.1).unwrap_or(b64)
                    } else {
                        b64
                    };
                    let bytes = b64_decode(raw).ok_or("responses base64 解码失败")?;
                    return Ok((bytes, "image/png".into()));
                }
                if let Some(raw_url) = item["url"].as_str() {
                    let img_url = if raw_url.starts_with('/') {
                        format!("{}{}", b.trim_end_matches("/v1"), raw_url)
                    } else {
                        raw_url.to_string()
                    };
                    let r = client
                        .get(&img_url)
                        .header(reqwest::header::USER_AGENT, "Michael-IDE-Agent/1.0")
                        .send()
                        .await
                        .map_err(|e| format!("下载生成图失败: {e}"))?;
                    if !r.status().is_success() {
                        return Err(format!("下载生成图 HTTP {}", r.status().as_u16()));
                    }
                    let ct = r
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .and_then(|x| x.to_str().ok())
                        .unwrap_or("image/png")
                        .to_string();
                    let bb = r.bytes().await.map_err(|e| e.to_string())?;
                    return Ok((
                        bb.to_vec(),
                        if ct.starts_with("image/") {
                            ct
                        } else {
                            "image/png".into()
                        },
                    ));
                }
            }
            // Some relays return image as a message-content image_url instead.
            if item["type"] == "message" {
                if let Some(content) = item["content"].as_array() {
                    for part in content {
                        if part["type"] == "output_image" || part["type"] == "image" {
                            if let Some(b64) = part["b64_json"].as_str().or(part["image"].as_str())
                            {
                                let raw = if b64.contains(',') {
                                    b64.split_once(',').map(|x| x.1).unwrap_or(b64)
                                } else {
                                    b64
                                };
                                let bytes = b64_decode(raw).ok_or("responses base64 解码失败")?;
                                return Ok((bytes, "image/png".into()));
                            }
                        }
                    }
                }
            }
        }
    }
    Err(format!(
        "responses 响应里没有 image_generation_call 结果。片段：{}",
        text.chars().take(150).collect::<String>()
    ))
}

/// gpt-image / dall-e only accept a fixed size set — an unsupported size makes the
/// Images API 400 and the whole call degrade to the flaky text routes. Snap to the
/// nearest supported size (landscape/portrait/square) instead of passing raw pixels.
fn image_size_for_model(model: &str, width: Option<u32>, height: Option<u32>) -> String {
    match (width, height) {
        (Some(w), Some(h)) => {
            let m = model.to_lowercase();
            if m.contains("gpt-image") || m.contains("dall-e") || m.contains("dall_e") {
                if w > h {
                    "1536x1024".to_string()
                } else if h > w {
                    "1024x1536".to_string()
                } else {
                    "1024x1024".to_string()
                }
            } else {
                format!("{w}x{h}")
            }
        }
        _ => "auto".to_string(),
    }
}

/// Try /v1/images/generations. Returns (bytes, mime) or an error string.
async fn try_images_api(
    client: &reqwest::Client,
    b: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    size: &str,
    request_id: Option<&str>,
) -> Result<(Vec<u8>, String), String> {
    let url = if b.ends_with("/v1") {
        format!("{b}/images/generations")
    } else {
        format!("{b}/v1/images/generations")
    };
    let m_lower = model.to_lowercase();
    let mut body = serde_json::json!({
        "model": model.trim(),
        "prompt": prompt,
        "n": 1,
        "size": size,
        "quality": "high",
        "response_format": "b64_json",
    });
    if m_lower.contains("gpt-image") {
        // gpt-image-* uses output_format instead of response_format, and returns b64 by default
        body.as_object_mut().unwrap().remove("response_format");
        body.as_object_mut()
            .unwrap()
            .insert("output_format".into(), serde_json::json!("png"));
    }
    let resp = with_ide_request_id(client.post(&url).bearer_auth(api_key.trim()), request_id)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("images-api 请求失败: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "images-api HTTP {}: {}",
            status.as_u16(),
            text.chars().take(200).collect::<String>()
        ));
    }
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("images-api 响应解析失败: {e}"))?;
    if let Some(err) = v.get("error") {
        let msg = err["message"]
            .as_str()
            .or(err.as_str())
            .unwrap_or("unknown");
        return Err(format!(
            "images-api 上游报错: {}",
            msg.chars().take(200).collect::<String>()
        ));
    }
    let item = &v["data"][0];
    if let Some(b64) = item["b64_json"].as_str().or(item["image"].as_str()) {
        let raw = if b64.contains(',') {
            b64.split_once(',').map(|x| x.1).unwrap_or(b64)
        } else {
            b64
        };
        let bytes = b64_decode(raw).ok_or("images-api base64 解码失败")?;
        Ok((bytes, "image/png".into()))
    } else if let Some(raw_url) = item["url"].as_str() {
        let img_url = if raw_url.starts_with('/') {
            format!("{}{}", b.trim_end_matches("/v1"), raw_url)
        } else {
            raw_url.to_string()
        };
        let r = client
            .get(&img_url)
            .header(reqwest::header::USER_AGENT, "Michael-IDE-Agent/1.0")
            .send()
            .await
            .map_err(|e| format!("下载生成图失败: {e}"))?;
        if !r.status().is_success() {
            return Err(format!("下载生成图 HTTP {}", r.status().as_u16()));
        }
        let ct = r
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|x| x.to_str().ok())
            .unwrap_or("image/png")
            .to_string();
        let bb = r.bytes().await.map_err(|e| e.to_string())?;
        Ok((
            bb.to_vec(),
            if ct.starts_with("image/") {
                ct
            } else {
                "image/png".into()
            },
        ))
    } else if let Some(task_id) = v.get("task_id").and_then(|t| t.as_str()) {
        // Async task (custom sizes) — poll until completed.
        let poll_url = if b.ends_with("/v1") {
            format!("{b}/images/generations/{task_id}")
        } else {
            format!("{b}/v1/images/generations/{task_id}")
        };
        for _ in 0..60 {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            let pr = client
                .get(&poll_url)
                .bearer_auth(api_key.trim())
                .send()
                .await
                .map_err(|e| format!("轮询任务失败: {e}"))?;
            if !pr.status().is_success() {
                continue;
            }
            let pt: serde_json::Value = pr.json().await.unwrap_or_default();
            let status = pt["status"].as_str().unwrap_or("");
            if status == "failed" {
                return Err(format!(
                    "生图任务失败: {}",
                    pt["error"].as_str().unwrap_or("unknown")
                ));
            }
            if status != "completed" {
                continue;
            }
            if let Some(arr) = pt.get("data").and_then(|d| d.as_array()) {
                if let Some(first) = arr.first() {
                    if let Some(u) = first["url"].as_str() {
                        let full_url = if u.starts_with('/') {
                            format!("{}{}", b.trim_end_matches("/v1"), u)
                        } else {
                            u.to_string()
                        };
                        let dr = client
                            .get(&full_url)
                            .header(reqwest::header::USER_AGENT, "Michael-IDE-Agent/1.0")
                            .send()
                            .await
                            .map_err(|e| format!("下载生成图失败: {e}"))?;
                        if !dr.status().is_success() {
                            return Err(format!("下载生成图 HTTP {}", dr.status().as_u16()));
                        }
                        let ct = dr
                            .headers()
                            .get(reqwest::header::CONTENT_TYPE)
                            .and_then(|x| x.to_str().ok())
                            .unwrap_or("image/png")
                            .to_string();
                        let bb = dr.bytes().await.map_err(|e| e.to_string())?;
                        return Ok((
                            bb.to_vec(),
                            if ct.starts_with("image/") {
                                ct
                            } else {
                                "image/png".into()
                            },
                        ));
                    }
                }
            }
            return Err("任务完成但无图片数据".into());
        }
        Err("生图任务超时（3分钟）".into())
    } else {
        Err(format!(
            "images-api 返回的 data 里没有 b64_json 或 url。响应片段：{}",
            text.chars().take(150).collect::<String>()
        ))
    }
}

/// Try /v1/chat/completions for chat-wrapped image gen (中转站 routes ChatGPT Plus
/// account image generation through this endpoint).
async fn try_chat_image_api(
    client: &reqwest::Client,
    b: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    request_id: Option<&str>,
) -> Result<(Vec<u8>, String), String> {
    let url = if b.ends_with("/v1") {
        format!("{b}/chat/completions")
    } else {
        format!("{b}/v1/chat/completions")
    };
    let body = serde_json::json!({
        "model": model.trim(),
        "stream": false,
        "messages": [{ "role": "user", "content": prompt }],
        "max_tokens": 1500,
    });
    let resp = with_ide_request_id(client.post(&url).bearer_auth(api_key.trim()), request_id)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("chat 请求失败: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "chat HTTP {}: {}",
            status.as_u16(),
            text.chars().take(200).collect::<String>()
        ));
    }
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("chat 响应解析失败: {e}"))?;
    let img = extract_image_ref(&v).ok_or_else(|| {
        let snippet: String = v["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .chars()
            .take(150)
            .collect();
        format!("chat 回复里没找到图片：{snippet}")
    })?;
    if img.starts_with("data:") {
        let comma = img.find(',').ok_or("data URL 无效")?;
        let meta = &img[5..comma];
        let m = meta.split(';').next().unwrap_or("image/png").to_string();
        let data = &img[comma + 1..];
        Ok((b64_decode(data).ok_or("base64 解码失败")?, m))
    } else if img.starts_with("http://") || img.starts_with("https://") {
        let r = client
            .get(&img)
            .header(reqwest::header::USER_AGENT, "Michael-IDE-Agent/1.0")
            .send()
            .await
            .map_err(|e| format!("下载生成图失败: {e}"))?;
        if !r.status().is_success() {
            return Err(format!("下载生成图 HTTP {}", r.status().as_u16()));
        }
        let ct = r
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|x| x.to_str().ok())
            .unwrap_or("image/png")
            .to_string();
        let bb = r.bytes().await.map_err(|e| e.to_string())?;
        Ok((
            bb.to_vec(),
            if ct.starts_with("image/") {
                ct
            } else {
                "image/png".into()
            },
        ))
    } else {
        Err("无法识别模型返回的图片引用".into())
    }
}

/// Pull an image reference out of a chat-completions response (string content with
/// markdown/URL/data-URL, multimodal image_url parts, or an `images[]` field).
fn extract_image_ref(v: &serde_json::Value) -> Option<String> {
    let msg = &v["choices"][0]["message"];
    if let Some(s) = msg["content"].as_str() {
        if let Some(u) = find_image_in_text(s) {
            return Some(u);
        }
    }
    if let Some(arr) = msg["content"].as_array() {
        for part in arr {
            if part["type"] == "image_url" {
                if let Some(u) = part["image_url"]["url"].as_str() {
                    return Some(u.to_string());
                }
            }
            if let Some(s) = part["text"].as_str() {
                if let Some(u) = find_image_in_text(s) {
                    return Some(u);
                }
            }
        }
    }
    if let Some(arr) = msg["images"].as_array() {
        if let Some(f) = arr.first() {
            if let Some(u) = f.as_str() {
                return Some(u.to_string());
            }
            if let Some(u) = f["url"].as_str() {
                return Some(u.to_string());
            }
            if let Some(u) = f["image_url"]["url"].as_str() {
                return Some(u.to_string());
            }
        }
    }
    None
}

fn find_image_in_text(s: &str) -> Option<String> {
    if let Some(i) = s.find("data:image/") {
        let rest = &s[i..];
        let end = rest
            .find(|c: char| c.is_whitespace() || c == ')' || c == '"' || c == '\'')
            .unwrap_or(rest.len());
        return Some(rest[..end].to_string());
    }
    if let Some(i) = s.find("](") {
        let rest = &s[i + 2..];
        if let Some(end) = rest.find(')') {
            let u = rest[..end].trim();
            if u.starts_with("http") || u.starts_with("data:") {
                return Some(u.to_string());
            }
        }
    }
    if let Some(i) = s.find("http") {
        let rest = &s[i..];
        if rest.starts_with("http://") || rest.starts_with("https://") {
            let end = rest
                .find(|c: char| {
                    c.is_whitespace()
                        || c == ')'
                        || c == '"'
                        || c == '\''
                        || c == '>'
                        || c == ']'
                        || c == '('
                })
                .unwrap_or(rest.len());
            return Some(rest[..end].to_string());
        }
    }
    None
}

/// Minimal standard-base64 decoder (no external crate; capture.rs only has an encoder).
fn b64_decode(s: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> i16 {
        match c {
            b'A'..=b'Z' => (c - b'A') as i16,
            b'a'..=b'z' => (c - b'a' + 26) as i16,
            b'0'..=b'9' => (c - b'0' + 52) as i16,
            b'+' => 62,
            b'/' => 63,
            _ => -1,
        }
    }
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0u32;
    for &c in s.trim().as_bytes() {
        if c == b'=' {
            break;
        }
        let dv = val(c);
        if dv < 0 {
            continue;
        }
        buf = (buf << 6) | dv as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod img_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn image_size_snaps_to_supported_set_for_gpt_image() {
        assert_eq!(
            image_size_for_model("gpt-image-2", Some(2048), Some(2048)),
            "1024x1024"
        );
        assert_eq!(
            image_size_for_model("gpt-image-2", Some(1920), Some(1080)),
            "1536x1024"
        );
        assert_eq!(
            image_size_for_model("dall-e-3", Some(800), Some(1200)),
            "1024x1536"
        );
    }

    #[test]
    fn image_size_passes_through_for_other_models_and_defaults_to_auto() {
        assert_eq!(
            image_size_for_model("flux-pro", Some(1920), Some(1080)),
            "1920x1080"
        );
        assert_eq!(image_size_for_model("gpt-image-2", None, None), "auto");
    }

    #[test]
    fn extract_markdown_url() {
        let v = json!({"choices":[{"message":{"content":"好的，这是图：\n![mockup](https://cdn.example.com/a/b.png)"}}]});
        assert_eq!(
            extract_image_ref(&v).as_deref(),
            Some("https://cdn.example.com/a/b.png")
        );
    }
    #[test]
    fn extract_bare_url() {
        let v =
            json!({"choices":[{"message":{"content":"生成完成 https://img.test/xyz.jpg 请查收"}}]});
        assert_eq!(
            extract_image_ref(&v).as_deref(),
            Some("https://img.test/xyz.jpg")
        );
    }
    #[test]
    fn extract_multimodal_image_url() {
        let v = json!({"choices":[{"message":{"content":[{"type":"text","text":"done"},{"type":"image_url","image_url":{"url":"https://m.test/i.webp"}}]}}]});
        assert_eq!(
            extract_image_ref(&v).as_deref(),
            Some("https://m.test/i.webp")
        );
    }
    #[test]
    fn extract_data_url() {
        let v = json!({"choices":[{"message":{"content":"data:image/png;base64,iVBORw0KG=="}}]});
        assert_eq!(
            extract_image_ref(&v).as_deref(),
            Some("data:image/png;base64,iVBORw0KG==")
        );
    }
    #[test]
    fn extract_images_field() {
        let v = json!({"choices":[{"message":{"content":"","images":[{"url":"https://x.test/p.png"}]}}]});
        assert_eq!(
            extract_image_ref(&v).as_deref(),
            Some("https://x.test/p.png")
        );
    }
    #[test]
    fn no_image_in_plain_text() {
        let v = json!({"choices":[{"message":{"content":"我没法生成图片。"}}]});
        assert_eq!(extract_image_ref(&v), None);
    }
    #[test]
    fn b64_roundtrip() {
        // "Man" → "TWFu"
        assert_eq!(b64_decode("TWFu"), Some(b"Man".to_vec()));
        assert_eq!(b64_decode("aGVsbG8="), Some(b"hello".to_vec()));
    }

    #[test]
    fn redirect_target_resolves_relative_location() {
        let base = reqwest::Url::parse("https://www.4399.com/flash/5.htm").unwrap();
        assert_eq!(
            redirect_target(&base, Some("/flash/5_1.htm")).as_deref(),
            Some("https://www.4399.com/flash/5_1.htm")
        );
        assert_eq!(
            redirect_target(&base, Some("https://www.taptap.cn/app/1")).as_deref(),
            Some("https://www.taptap.cn/app/1")
        );
    }

    #[test]
    fn decode_legacy_chinese_charsets() {
        let (text, enc) =
            decode_body_text(vec![0xD6, 0xD0, 0xCE, 0xC4], "text/html; charset=gb2312");
        assert_eq!(text, "中文");
        assert!(enc.contains("GB"));
    }

    #[test]
    fn decode_charset_from_html_meta() {
        let mut body = b"<html><head><meta charset=\"gbk\"></head><body>".to_vec();
        body.extend_from_slice(&[0xD6, 0xD0, 0xCE, 0xC4]);
        body.extend_from_slice(b"</body></html>");
        let (text, enc) = decode_body_text(body, "text/html");
        assert!(text.contains("中文"));
        assert!(enc.contains("GB"));
    }
}
