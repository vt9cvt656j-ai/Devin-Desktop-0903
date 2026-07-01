//! Autonomous browser control for the AI agent: a persistent headless Chrome it
//! can drive end-to-end — navigate, click, type, run JS — and SEE (every action
//! returns a fresh screenshot + the page's visible text). This is what lets the
//! agent test the web apps it builds, fill forms, and browse on its own.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

/// Whether to draw the visible Set-of-Mark number badges. ON for normal agent
/// browsing (visual grounding), but turned OFF while recording a user-facing demo
/// so the captured frames stay clean (refs are still tagged for click-by-index).
static DRAW_MARKS: AtomicBool = AtomicBool::new(true);

use headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption;
use headless_chrome::{Browser, LaunchOptionsBuilder, Tab};
use serde::{Deserialize, Serialize};

struct Session {
    _browser: Browser, // kept alive so the child process & connection survive
    tab: Arc<Tab>,
}

// One shared browser session for the agent. A Mutex is fine — the agent drives it
// one action at a time.
static BROWSER: LazyLock<Mutex<Option<Session>>> = LazyLock::new(|| Mutex::new(None));

/// One interactive element on the page, with a stable `ref` the agent uses to
/// act on it (click/type by number) instead of guessing a CSS selector — the
/// Playwright-MCP / browser-use approach. Matched by `[data-mref="<ref>"]`.
#[derive(Serialize, Deserialize)]
pub struct BrowserElement {
    #[serde(rename = "ref")]
    ref_: u32,
    tag: String,
    #[serde(rename = "type", default)]
    type_: String,
    #[serde(default)]
    text: String,
}

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
    /// Interactive elements (with refs) — the agent clicks/types by ref, and the
    /// screenshot has matching numbered marks (Set-of-Mark) for visual grounding.
    elements: Vec<BrowserElement>,
    /// Optional extra (e.g. a JS eval result).
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<String>,
}

fn launch() -> Result<Session, String> {
    let path = crate::capture::find_headless_browser()
        .ok_or("未找到 Chrome / Chromium / Edge，无法启动浏览器自动化。请先安装其一。")?;
    // Make intranet/localhost dev servers reachable: ignore self-signed certs and
    // bypass any system/corporate proxy for private addresses (so "内网不能访问"
    // doesn't happen), while still letting public sites use the proxy.
    let proxy_bypass = "--proxy-bypass-list=localhost;127.0.0.1;[::1];0.0.0.0;\
        10.*;192.168.*;169.254.*;\
        172.16.*;172.17.*;172.18.*;172.19.*;172.20.*;172.21.*;172.22.*;172.23.*;\
        172.24.*;172.25.*;172.26.*;172.27.*;172.28.*;172.29.*;172.30.*;172.31.*;\
        *.local;<local>";
    let extra: Vec<std::ffi::OsString> = vec![
        "--ignore-certificate-errors".into(),
        "--allow-insecure-localhost".into(),
        "--disable-background-networking".into(),
        "--disable-dev-shm-usage".into(),
        proxy_bypass.into(),
    ];
    let extra_ref: Vec<&std::ffi::OsStr> = extra.iter().map(|s| s.as_os_str()).collect();
    // PERSISTENT profile: keep one stable user-data-dir so cookies / logins survive
    // across sessions. The user logs in ONCE (e.g. scans a login QR) and the agent's
    // browser stays signed in next time — instead of a clean, logged-out browser every
    // run. (This is plain cookie persistence, like any normal browser profile.)
    let profile_dir = std::env::var_os("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".michael-ide").join("browser-profile"));
    if let Some(ref p) = profile_dir {
        let _ = std::fs::create_dir_all(p);
        // Remove stale singleton locks left by a previous crash / navigation timeout —
        // otherwise Chrome refuses to reuse the profile and EVERY later browser action
        // fails ("profile in use"). This makes the persistent profile crash-robust.
        for lock in ["SingletonLock", "SingletonCookie", "SingletonSocket"] {
            let _ = std::fs::remove_file(p.join(lock));
        }
    }
    // VISIBLE by default — the user must actually SEE the browser being controlled
    // (headless felt "根本没在控制电脑" because nothing showed on screen). A real Chrome
    // window opens and the agent drives it (fast, node-based). Set MICHAEL_BROWSER_HEADLESS=1
    // to force headless for pure background scraping where no window is wanted.
    let headless = std::env::var("MICHAEL_BROWSER_HEADLESS").ok().as_deref() == Some("1");
    let opts = LaunchOptionsBuilder::default()
        .path(Some(std::path::PathBuf::from(path)))
        .headless(headless)
        .sandbox(false)
        .ignore_certificate_errors(true)
        .user_data_dir(profile_dir)
        .window_size(Some((1280, 900)))
        .args(extra_ref)
        .build()
        .map_err(|e| e.to_string())?;
    let browser = Browser::new(opts).map_err(|e| e.to_string())?;
    let tab = browser.new_tab().map_err(|e| e.to_string())?;
    // Slow intranet pages need more headroom than the old 15s.
    tab.set_default_timeout(Duration::from_secs(30));
    Ok(Session {
        _browser: browser,
        tab,
    })
}

/// Enumerate the visible, in-viewport interactive elements: tag each with a
/// `data-mref` ref (so click/type can target `[data-mref="N"]`), draw a numbered
/// Set-of-Mark badge on each (so the screenshot shows which is which), and return
/// the compact list. This is what lets the agent act precisely by number instead
/// of guessing selectors — far more reliable and ~20-50x cheaper than pixels.
fn enumerate_elements(tab: &Tab) -> Vec<BrowserElement> {
    // `__DRAW__` is replaced with 1/0 — controls only the VISIBLE badge; refs are
    // always tagged so click-by-index works even with marks off (demo recording).
    let raw = r##"(() => { try {
  document.querySelectorAll('[data-mref]').forEach(e => e.removeAttribute('data-mref'));
  document.querySelectorAll('.__mcp_som').forEach(e => e.remove());
  const DRAW = __DRAW__;
  const sel = 'a[href],button,input:not([type=hidden]),select,textarea,[role=button],[role=link],[role=tab],[role=menuitem],[role=checkbox],[role=switch],[onclick],[contenteditable=""],[contenteditable=true]';
  const els = Array.from(document.querySelectorAll(sel));
  const out = []; let i = 0;
  for (const el of els) {
    if (i >= 60) break;
    const r = el.getBoundingClientRect();
    if (r.width < 1 || r.height < 1) continue;
    if (r.bottom < 0 || r.right < 0 || r.top > innerHeight || r.left > innerWidth) continue;
    const cs = getComputedStyle(el);
    if (cs.visibility === 'hidden' || cs.display === 'none' || cs.opacity === '0') continue;
    el.setAttribute('data-mref', i);
    const tag = el.tagName.toLowerCase();
    const type = (el.getAttribute('type') || '').slice(0, 20);
    let text = (el.innerText || el.value || el.getAttribute('aria-label') || el.getAttribute('placeholder') || el.getAttribute('title') || el.getAttribute('name') || '').trim().replace(/\s+/g, ' ').slice(0, 60);
    out.push({ ref: i, tag: tag, type: type, text: text });
    if (DRAW) {
      const b = document.createElement('div');
      b.className = '__mcp_som';
      b.textContent = String(i);
      b.style.cssText = 'position:absolute;z-index:2147483647;background:#d93025;color:#fff;font:bold 11px monospace;padding:0 3px;border-radius:3px;pointer-events:none;line-height:15px;box-shadow:0 0 0 1px #fff;';
      b.style.left = (r.left + window.scrollX) + 'px';
      b.style.top = (r.top + window.scrollY) + 'px';
      document.body.appendChild(b);
    }
    i++;
  }
  return JSON.stringify(out);
} catch (e) { return '[]'; } })()"##;
    let js = raw.replace("__DRAW__", if DRAW_MARKS.load(Ordering::Relaxed) { "1" } else { "0" });
    match tab.evaluate(&js, false) {
        Ok(ro) => {
            let s = ro
                .value
                .and_then(|v| v.as_str().map(|x| x.to_string()))
                .unwrap_or_else(|| "[]".to_string());
            serde_json::from_str::<Vec<BrowserElement>>(&s).unwrap_or_default()
        }
        Err(_) => Vec::new(),
    }
}

/// Capture the current page state (title / url / visible text / elements / shot).
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
    // Tag + mark interactive elements BEFORE the screenshot so the numbered marks
    // appear in the captured image (Set-of-Mark grounding).
    let elements = enumerate_elements(tab);
    std::thread::sleep(Duration::from_millis(60)); // let the marks paint
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
        elements,
        result,
    })
}

/// Run a closure against the (lazily-launched) shared tab, on a blocking thread.
async fn with_tab<F>(f: F) -> Result<BrowserState, String>
where
    F: Fn(&Tab) -> Result<Option<String>, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || -> Result<BrowserState, String> {
        // Run against the shared tab. If the cached browser's CDP connection is DEAD
        // (it timed out / crashed / was closed — "underlying connection is closed"),
        // toss the stale session, relaunch a fresh browser, and retry ONCE. Without
        // this, a single navigation timeout bricks the browser for the whole session.
        let mut last_err = String::new();
        for attempt in 0..2 {
            let outcome = {
                let mut guard = BROWSER.lock().map_err(|_| "browser state poisoned")?;
                if guard.is_none() {
                    *guard = Some(launch()?);
                }
                let tab = guard.as_ref().unwrap().tab.clone();
                f(&tab).and_then(|result| snapshot(&tab, result))
            };
            match outcome {
                Ok(state) => return Ok(state),
                Err(e) => {
                    last_err = e;
                    if attempt == 0 && is_dead_browser(&last_err) {
                        if let Ok(mut g) = BROWSER.lock() {
                            *g = None; // drop the dead session → next attempt relaunches
                        }
                        continue;
                    }
                    return Err(last_err);
                }
            }
        }
        Err(last_err)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Does this error mean the browser's CDP connection died (so we should relaunch a
/// fresh browser instead of surfacing a dead-connection error to the user)?
fn is_dead_browser(e: &str) -> bool {
    let s = e.to_lowercase();
    s.contains("underlying connection is closed")
        || s.contains("connection is closed")
        || s.contains("connection closed")
        || s.contains("not connected")
        || s.contains("websocket")
        || s.contains("channel")
        || s.contains("no longer")
        || s.contains("session with given id not found")
        || s.contains("target closed")
        || s.contains("browser process")
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

/// Set the file(s) of a <input type=file> matching a CSS selector — so the agent can
/// automate upload forms (which plain typing can't do). `paths` are absolute local paths.
#[tauri::command]
pub async fn browser_upload_file(selector: String, paths: Vec<String>) -> Result<BrowserState, String> {
    with_tab(move |tab| {
        let el = tab
            .find_element(&selector)
            .map_err(|_| format!("找不到文件输入框: {selector}（要选中一个 <input type=file>）"))?;
        let refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
        el.set_input_files(&refs).map_err(|e| format!("设置上传文件失败: {e}"))?;
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
        // 8000 chars (~2.7k tokens worst case) so structured tools — network 抓包,
        // inspect 视觉解析, design — can return their full JSON without truncating
        // mid-string (which would hand the model invalid JSON).
        Ok(Some(val.chars().take(8000).collect()))
    })
    .await
}

/// Re-screenshot the current page (no action) — just look again.
#[tauri::command]
pub async fn browser_screenshot() -> Result<BrowserState, String> {
    with_tab(move |_tab| Ok(None)).await
}

/// Scroll the page vertically (positive = down, negative = up), then re-snapshot —
/// so off-screen interactive elements come into view and get fresh refs/marks.
/// Without this, the agent could only ever act on what was in the first viewport.
#[tauri::command]
pub async fn browser_scroll(amount: i32) -> Result<BrowserState, String> {
    with_tab(move |tab| {
        let js = format!("window.scrollBy(0, {});", amount);
        let _ = tab.evaluate(&js, false);
        std::thread::sleep(Duration::from_millis(350)); // let lazy content / sticky bars settle
        Ok(None)
    })
    .await
}

/// Wait for the page to settle: either until a CSS `selector` appears (polled, up
/// to ~8s) or for a fixed `ms`. Lets the agent act on a LOADED page instead of a
/// transient one (the #1 cause of flaky automation on SPAs / AJAX).
#[tauri::command]
pub async fn browser_wait(selector: Option<String>, ms: Option<u64>) -> Result<BrowserState, String> {
    with_tab(move |tab| {
        // borrow (don't move) selector — the closure is Fn (may run twice on relaunch)
        match selector.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(sel) => {
                let deadline = std::time::Instant::now() + Duration::from_secs(8);
                loop {
                    if tab.find_element(sel).is_ok() {
                        break;
                    }
                    if std::time::Instant::now() > deadline {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(250));
                }
            }
            None => {
                let d = ms.unwrap_or(1500).clamp(100, 15000);
                std::thread::sleep(Duration::from_millis(d));
            }
        }
        Ok(None)
    })
    .await
}

/// Toggle the visible Set-of-Mark number badges. The frontend turns them OFF
/// while recording a user-facing demo (clean frames) and back ON after.
#[tauri::command]
pub fn browser_set_marks(on: bool) {
    DRAW_MARKS.store(on, Ordering::Relaxed);
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
