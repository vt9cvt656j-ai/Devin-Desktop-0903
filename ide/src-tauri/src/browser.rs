//! Autonomous browser control for the AI agent: a persistent headless Chrome it
//! can drive end-to-end — navigate, click, type, run JS — and SEE (every action
//! returns a fresh screenshot + the page's visible text). This is what lets the
//! agent test the web apps it builds, fill forms, and browse on its own.

use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption;
use headless_chrome::{Browser, LaunchOptionsBuilder, Tab};
use serde::Serialize;

struct Session {
    _browser: Browser, // kept alive so the child process & connection survive
    tab: Arc<Tab>,
}

// One shared browser session for the agent. A Mutex is fine — the agent drives it
// one action at a time.
static BROWSER: LazyLock<Mutex<Option<Session>>> = LazyLock::new(|| Mutex::new(None));

/// What every browser action returns: the page state AFTER the action, so the
/// agent always sees the result of what it just did.
#[derive(Serialize)]
pub struct BrowserState {
    title: String,
    url: String,
    /// Visible text of the page (truncated) — cheap, searchable context.
    text: String,
    /// `data:image/png;base64,...` — fed back to the model as an image.
    screenshot: String,
    /// Optional extra (e.g. a JS eval result).
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<String>,
}

fn launch() -> Result<Session, String> {
    let path = crate::capture::find_headless_browser()
        .ok_or("未找到 Chrome / Chromium / Edge，无法启动浏览器自动化。请先安装其一。")?;
    let opts = LaunchOptionsBuilder::default()
        .path(Some(std::path::PathBuf::from(path)))
        .headless(true)
        .sandbox(false)
        .window_size(Some((1280, 900)))
        .build()
        .map_err(|e| e.to_string())?;
    let browser = Browser::new(opts).map_err(|e| e.to_string())?;
    let tab = browser.new_tab().map_err(|e| e.to_string())?;
    tab.set_default_timeout(Duration::from_secs(15));
    Ok(Session {
        _browser: browser,
        tab,
    })
}

/// Capture the current page state (title / url / visible text / screenshot).
fn snapshot(tab: &Tab, result: Option<String>) -> Result<BrowserState, String> {
    let title = tab.get_title().unwrap_or_default();
    let url = tab.get_url();
    let text = tab
        .find_element("body")
        .ok()
        .and_then(|e| e.get_inner_text().ok())
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(3000)
        .collect::<String>();
    let png = tab
        .capture_screenshot(CaptureScreenshotFormatOption::Png, None, None, true)
        .map_err(|e| e.to_string())?;
    // Compact JPEG so the screenshot data URL doesn't blow the AI body limit (413).
    let screenshot = crate::capture::bytes_to_jpeg_data_url(&png, 1280, 68)?;
    Ok(BrowserState {
        title,
        url,
        text,
        screenshot,
        result,
    })
}

/// Run a closure against the (lazily-launched) shared tab, on a blocking thread.
async fn with_tab<F>(f: F) -> Result<BrowserState, String>
where
    F: FnOnce(&Tab) -> Result<Option<String>, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || -> Result<BrowserState, String> {
        let mut guard = BROWSER.lock().map_err(|_| "browser state poisoned")?;
        if guard.is_none() {
            *guard = Some(launch()?);
        }
        let tab = guard.as_ref().unwrap().tab.clone();
        let result = f(&tab)?;
        snapshot(&tab, result)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Open a URL (launches the browser on first use).
#[tauri::command]
pub async fn browser_navigate(url: String) -> Result<BrowserState, String> {
    let url = url.trim().to_string();
    let url = if url.starts_with("http://") || url.starts_with("https://") {
        url
    } else {
        format!("https://{url}")
    };
    with_tab(move |tab| {
        tab.navigate_to(&url).map_err(|e| e.to_string())?;
        tab.wait_until_navigated().map_err(|e| e.to_string())?;
        Ok(None)
    })
    .await
}

/// Click the first element matching a CSS selector.
#[tauri::command]
pub async fn browser_click(selector: String) -> Result<BrowserState, String> {
    with_tab(move |tab| {
        let el = tab
            .find_element(&selector)
            .map_err(|_| format!("找不到元素: {selector}"))?;
        el.click().map_err(|e| e.to_string())?;
        std::thread::sleep(Duration::from_millis(400));
        let _ = tab.wait_until_navigated();
        Ok(None)
    })
    .await
}

/// Type text into the element matching a CSS selector (clicks it first).
#[tauri::command]
pub async fn browser_type(selector: String, text: String) -> Result<BrowserState, String> {
    with_tab(move |tab| {
        let el = tab
            .find_element(&selector)
            .map_err(|_| format!("找不到输入框: {selector}"))?;
        el.click().map_err(|e| e.to_string())?;
        el.type_into(&text).map_err(|e| e.to_string())?;
        Ok(None)
    })
    .await
}

/// Press a key on the page (e.g. "Enter" to submit a form, "Tab", "Escape").
#[tauri::command]
pub async fn browser_press(key: String) -> Result<BrowserState, String> {
    with_tab(move |tab| {
        tab.press_key(&key).map_err(|e| e.to_string())?;
        std::thread::sleep(Duration::from_millis(400));
        let _ = tab.wait_until_navigated();
        Ok(None)
    })
    .await
}

/// Run JavaScript in the page and return its (stringified) result.
#[tauri::command]
pub async fn browser_eval(script: String) -> Result<BrowserState, String> {
    with_tab(move |tab| {
        let ro = tab
            .evaluate(&script, true)
            .map_err(|e| e.to_string())?;
        let val = ro
            .value
            .map(|v| v.to_string())
            .or(ro.description)
            .unwrap_or_default();
        Ok(Some(val.chars().take(4000).collect()))
    })
    .await
}

/// Re-screenshot the current page (no action) — just look again.
#[tauri::command]
pub async fn browser_screenshot() -> Result<BrowserState, String> {
    with_tab(move |_tab| Ok(None)).await
}

/// Close the browser and free the session.
#[tauri::command]
pub async fn browser_close() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        if let Ok(mut g) = BROWSER.lock() {
            *g = None; // dropping Session kills the Chrome process
        }
    })
    .await
    .map_err(|e| e.to_string())
}
