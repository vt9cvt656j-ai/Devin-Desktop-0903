//! Headless-browser screenshot capture, so the AI agent can SEE the web UI it
//! builds (a real visual feedback loop) instead of editing blind. Uses whatever
//! Chromium/Chrome/Edge is installed; degrades with a clear message when none is.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// Candidate headless-browser binaries / app paths, in preference order.
fn candidate_browsers() -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        vec![
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".into(),
            "/Applications/Chromium.app/Contents/MacOS/Chromium".into(),
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge".into(),
            "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser".into(),
            "google-chrome".into(),
            "chromium".into(),
        ]
    }
    #[cfg(target_os = "windows")]
    {
        vec![
            r"C:\Program Files\Google\Chrome\Application\chrome.exe".into(),
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe".into(),
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe".into(),
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe".into(),
        ]
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        vec![
            "google-chrome".into(),
            "google-chrome-stable".into(),
            "chromium".into(),
            "chromium-browser".into(),
            "chrome".into(),
            "microsoft-edge".into(),
            "brave-browser".into(),
        ]
    }
}

/// First installed headless browser, or `None`.
pub fn find_headless_browser() -> Option<String> {
    for c in candidate_browsers() {
        if c.contains('/') || c.contains('\\') {
            if Path::new(&c).exists() {
                return Some(c);
            }
        } else {
            #[cfg(not(windows))]
            {
                let resolved = crate::process_util::resolve_command(&c, None);
                if resolved.contains('/') && Path::new(&resolved).exists() {
                    return Some(resolved);
                }
            }
        }
    }
    None
}

/// Minimal standard base64 (no external crate, no line wrapping) for the data URL.
pub fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(T[(b0 >> 2) as usize] as char);
        out.push(T[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(T[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(T[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Downscale (to <= `max_w` wide) and JPEG-encode an image into a compact
/// `data:image/jpeg;base64,...`. Screenshots (esp. full-screen PNGs) are huge —
/// this keeps them ~100-200 KB so they don't blow the AI request body limit
/// (the "413 Payload Too Large" the gateway returns).
pub fn jpeg_data_url(img: image::DynamicImage, max_w: u32, quality: u8) -> Result<String, String> {
    let img = if img.width() > max_w && max_w > 0 {
        let h = ((img.height() as u64 * max_w as u64) / img.width().max(1) as u64).max(1) as u32;
        img.resize(max_w, h, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let rgb = image::DynamicImage::ImageRgb8(img.to_rgb8()); // JPEG has no alpha
    let mut buf: Vec<u8> = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality)
        .encode_image(&rgb)
        .map_err(|e| e.to_string())?;
    Ok(format!("data:image/jpeg;base64,{}", b64(&buf)))
}

/// Decode raw image bytes (e.g. a PNG screenshot) and re-encode as a compact JPEG data URL.
pub fn bytes_to_jpeg_data_url(bytes: &[u8], max_w: u32, quality: u8) -> Result<String, String> {
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    jpeg_data_url(img, max_w, quality)
}

/// True if the URL's host is loopback / private-LAN / `.local` / a bare hostname
/// — i.e. an intranet address that must bypass any system proxy. Otherwise Chrome
/// routes localhost/LAN through a corporate proxy and "can't reach" the dev server
/// (the user's "内网不能访问").
fn host_is_local(url: &str) -> bool {
    let after = url.splitn(2, "://").nth(1).unwrap_or(url);
    let authority = after.split(['/', '?', '#']).next().unwrap_or("");
    let hostport = authority.rsplit('@').next().unwrap_or(authority); // drop userinfo
    // host without port — handle bracketed IPv6 like [::1]:3000
    let host = if let Some(rest) = hostport.strip_prefix('[') {
        rest.split(']').next().unwrap_or(rest)
    } else {
        hostport.split(':').next().unwrap_or(hostport)
    };
    let h = host.trim().to_ascii_lowercase();
    if h.is_empty() {
        return false;
    }
    if h == "localhost"
        || h.ends_with(".localhost")
        || h.ends_with(".local")
        || h == "::1"
        || h == "0.0.0.0"
    {
        return true;
    }
    if h.starts_with("127.")
        || h.starts_with("10.")
        || h.starts_with("192.168.")
        || h.starts_with("169.254.")
    {
        return true;
    }
    // 172.16.0.0 – 172.31.255.255
    if let Some(rest) = h.strip_prefix("172.") {
        if let Ok(n) = rest.split('.').next().unwrap_or("").parse::<u8>() {
            if (16..=31).contains(&n) {
                return true;
            }
        }
    }
    // A bare hostname with no dot (e.g. "myserver", "dev") is almost always intranet.
    !h.contains('.')
}

fn run_capture(browser: &str, url: &str, out: &str, w: u32, h: u32) -> Result<(), String> {
    let mut args: Vec<String> = vec![
        "--headless=new".into(),
        "--disable-gpu".into(),
        "--no-sandbox".into(),
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
        "--disable-extensions".into(),
        "--disable-background-networking".into(),
        "--disable-sync".into(),
        "--disable-dev-shm-usage".into(),
        "--hide-scrollbars".into(),
        // Local dev servers love self-signed HTTPS — don't let a cert error wedge
        // the capture into a timeout ("内网不能访问").
        "--ignore-certificate-errors".into(),
        "--allow-insecure-localhost".into(),
        format!("--window-size={w},{h}"),
        format!("--screenshot={out}"),
        // Let dynamic pages render before the shot. The budget fires even on pages
        // with idle-but-open sockets (HMR / websockets), so it won't hang there.
        "--virtual-time-budget=8000".into(),
    ];
    if host_is_local(url) {
        // Intranet / localhost must NOT go through a system/corporate proxy, or
        // Chrome can't reach the user's own dev server.
        args.push("--no-proxy-server".into());
    }
    args.push(url.into());

    let mut child = Command::new(browser)
        .args(&args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("启动浏览器失败: {e}"))?;

    // Headless --screenshot exits on its own; bound it so a wedged page can't hang.
    // Slow intranet pages / heavy SPAs need more headroom than the old 20s.
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    // If the page rendered enough to leave a non-empty file, use it
                    // rather than failing outright.
                    if std::fs::metadata(out).map(|m| m.len() > 0).unwrap_or(false) {
                        return Ok(());
                    }
                    return Err("截图超时（页面加载太久）。本地/内网地址已自动绕过代理并忽略自签证书；确认服务已启动且 URL 正确，或稍后重试。".into());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}

/// Screenshot `url` with a headless browser; returns a `data:image/png;base64,...`.
#[tauri::command]
pub async fn capture_url(
    url: String,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<String, String> {
    let mut url = url.trim().to_string();
    // Be forgiving: a bare host like "192.168.1.5:3000" or "localhost:5173" is a
    // valid intranet target — default it to http:// instead of rejecting it.
    if !url.contains("://") {
        url = format!("http://{url}");
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("只支持 http/https URL".into());
    }
    let browser = find_headless_browser().ok_or_else(|| {
        "未找到无头浏览器。装上 Google Chrome / Chromium / Edge 后，智能体就能“看见”页面并据图改进 UI（截图依赖它渲染）。".to_string()
    })?;
    let w = width.unwrap_or(1280).clamp(320, 3840);
    let h = height.unwrap_or(800).clamp(240, 4000);

    let out = std::env::temp_dir().join(format!("michael_ide_shot_{}.png", uuid::Uuid::new_v4()));
    let out_str = out.to_string_lossy().to_string();

    let url2 = url.clone();
    let out2 = out_str.clone();
    tauri::async_runtime::spawn_blocking(move || run_capture(&browser, &url2, &out2, w, h))
        .await
        .map_err(|e| e.to_string())??;

    let bytes = std::fs::read(&out).map_err(|e| format!("读取截图失败: {e}"))?;
    let _ = std::fs::remove_file(&out);
    if bytes.is_empty() {
        return Err("截图为空（页面可能没加载出来，确认服务已起、URL 正确）".into());
    }
    // Re-encode as a downscaled JPEG so the data URL stays small (the raw PNG can
    // be multi-MB and blow the AI request body limit → 413).
    bytes_to_jpeg_data_url(&bytes, w.min(1280), 68)
}
