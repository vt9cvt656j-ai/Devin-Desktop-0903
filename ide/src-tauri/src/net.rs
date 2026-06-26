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
    body: String,
    truncated: bool,
    content_type: String,
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
    let method =
        reqwest::Method::from_bytes(m.as_bytes()).map_err(|_| format!("不支持的 HTTP 方法: {m}"))?;
    let to = timeout_secs.unwrap_or(30).clamp(1, 120);

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(to))
        .build()
        .map_err(|e| e.to_string())?;

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
    let body_text = match String::from_utf8(buf) {
        Ok(s) => s,
        Err(e) => format!("[二进制响应，{} 字节，未作为文本返回]", e.into_bytes().len()),
    };

    Ok(HttpResponse {
        status: status.as_u16(),
        ok: status.is_success(),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        headers: hmap,
        body: body_text,
        truncated,
        content_type,
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
