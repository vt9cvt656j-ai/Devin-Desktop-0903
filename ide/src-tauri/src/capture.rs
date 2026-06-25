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
fn find_headless_browser() -> Option<String> {
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
fn b64(data: &[u8]) -> String {
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

fn run_capture(browser: &str, url: &str, out: &str, w: u32, h: u32) -> Result<(), String> {
    let mut child = Command::new(browser)
        .args([
            "--headless=new",
            "--disable-gpu",
            "--no-sandbox",
            "--no-first-run",
            "--disable-extensions",
            "--hide-scrollbars",
            &format!("--window-size={w},{h}"),
            &format!("--screenshot={out}"),
            // Let dynamic pages render a little before the shot is taken.
            "--virtual-time-budget=5000",
            url,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("启动浏览器失败: {e}"))?;

    // Headless --screenshot exits on its own; bound it so a wedged page can't hang.
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("截图超时（页面加载太久）".into());
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
    let url = url.trim().to_string();
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
    const MAX: usize = 4 * 1024 * 1024;
    if bytes.len() > MAX {
        return Err(format!(
            "截图过大（{} KB），请用更小的窗口尺寸重试",
            bytes.len() / 1024
        ));
    }
    Ok(format!("data:image/png;base64,{}", b64(&bytes)))
}
