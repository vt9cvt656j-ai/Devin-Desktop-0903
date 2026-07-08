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

/// Check if Google Chrome is currently running (macOS).
#[cfg(target_os = "macos")]
fn is_chrome_running() -> bool {
    std::process::Command::new("pgrep")
        .args(["-x", "Google Chrome"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
fn is_chrome_running() -> bool {
    false
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
    let mut extra: Vec<std::ffi::OsString> = vec![
        "--ignore-certificate-errors".into(),
        "--allow-insecure-localhost".into(),
        "--disable-background-networking".into(),
        "--disable-dev-shm-usage".into(),
        // Kill the nag bubbles that made every automation launch look broken: "个人资料出了点问题",
        // "Chrome 未正确关闭 / 恢复页面", first-run, default-browser, translate/infobars. A controlled
        // browser must start clean.
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
        "--disable-session-crashed-bubble".into(),
        "--hide-crash-restore-bubble".into(),
        "--disable-infobars".into(),
        "--disable-features=Translate,InfobarScreenshot,MediaRouter,OptimizationHints".into(),
        "--disable-backgrounding-occluded-windows".into(),
        proxy_bypass.into(),
    ];
    // If the 抓包 capture proxy is running, route this browser THROUGH it so capture_flows sees the
    // browser's traffic — this is what makes "抓包 + browser 走一遍登录" actually combine.
    // --ignore-certificate-errors (above) makes Chrome accept mitmproxy's MITM cert, so the user
    // doesn't even need to trust the CA for the automation browser.
    if let Some(port) = crate::proxy::active_proxy_port() {
        extra.push(format!("--proxy-server=127.0.0.1:{port}").into());
    }
    let extra_ref: Vec<&std::ffi::OsStr> = extra.iter().map(|s| s.as_os_str()).collect();
    // Use the user's REAL Chrome profile when Chrome is NOT running — this gives the
    // agent their actual cookies, logins, saved passwords, and extensions (the #1 user
    // request: "用我真实浏览器"). When Chrome IS running we can't share the locked profile,
    // so fall back to our own persistent profile at ~/.michael-ide/browser-profile.
    let profile_dir = if !is_chrome_running() {
        #[cfg(target_os = "macos")]
        {
            std::env::var_os("HOME")
                .map(|h| {
                    std::path::PathBuf::from(h)
                        .join("Library/Application Support/Google/Chrome")
                })
                .filter(|p| p.join("Default").exists())
        }
        #[cfg(not(target_os = "macos"))]
        {
            None::<std::path::PathBuf>
        }
    } else {
        None
    }
    .or_else(|| {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(|h| std::path::PathBuf::from(h).join(".michael-ide").join("browser-profile"))
    });
    if let Some(ref p) = profile_dir {
        let _ = std::fs::create_dir_all(p);
        // Remove stale singleton locks left by a previous crash / navigation timeout —
        // otherwise Chrome refuses to reuse the profile and EVERY later browser action
        // fails ("profile in use"). This makes the persistent profile crash-robust.
        // Remove ALL singleton lock/socket files (they're symlinks on macOS). ANY leftover makes
        // Chrome think the profile is "in use" → "个人资料出了点问题". A hard-coded list missed
        // variants; sweep every file whose name starts with "Singleton".
        if let Ok(rd) = std::fs::read_dir(p) {
            for ent in rd.flatten() {
                if ent.file_name().to_string_lossy().starts_with("Singleton") {
                    let _ = std::fs::remove_file(ent.path());
                }
            }
        }
        // Mark the profile as cleanly exited so Chrome doesn't nag "个人资料出了点问题 / 未正确关闭 /
        // 恢复页面" after a previous kill/crash. KEY FIX: if a state file is CORRUPT (unparseable
        // JSON — a real cause of the "profile problem" dialog), the old code silently skipped it and
        // the dialog kept showing. Now we DELETE the corrupt file so Chrome regenerates a pristine one
        // (cookies live in Default/Cookies + Keychain, not these files, so login persistence survives).
        let prefs = p.join("Default").join("Preferences");
        match std::fs::read_to_string(&prefs)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        {
            Some(mut json) => {
                if let Some(prof) = json.get_mut("profile").and_then(|v| v.as_object_mut()) {
                    prof.insert("exit_type".into(), serde_json::Value::String("Normal".into()));
                    prof.insert("exited_cleanly".into(), serde_json::Value::Bool(true));
                }
                let _ = std::fs::write(&prefs, json.to_string());
            }
            None if prefs.exists() => {
                let _ = std::fs::remove_file(&prefs); // corrupt → let Chrome rebuild it clean
            }
            None => {}
        }
        // A corrupt root "Local State" ALSO triggers "个人资料出了点问题"; drop it if unreadable
        // (on macOS it holds no cookie key, so this is safe — Chrome rebuilds it).
        let local_state = p.join("Local State");
        if let Ok(txt) = std::fs::read_to_string(&local_state) {
            if serde_json::from_str::<serde_json::Value>(&txt).is_err() {
                let _ = std::fs::remove_file(&local_state);
            }
        }
    }
    // VISIBLE by default — the user must actually SEE the browser being controlled
    // (headless felt "根本没在控制电脑" because nothing showed on screen). A real Chrome
    // window opens and the agent drives it (fast, node-based). Set MICHAEL_BROWSER_HEADLESS=1
    // to force headless for pure background scraping where no window is wanted.
    let headless = std::env::var("MICHAEL_BROWSER_HEADLESS").ok().as_deref() == Some("1");
    let opts = LaunchOptionsBuilder::default()
        .path(Some(std::path::PathBuf::from(&path)))
        .headless(headless)
        .sandbox(false)
        .ignore_certificate_errors(true)
        .user_data_dir(profile_dir.clone())
        .window_size(Some((1280, 900)))
        .args(extra_ref.clone())
        .build()
        .map_err(|e| e.to_string())?;
    let browser = match Browser::new(opts) {
        Ok(b) => b,
        Err(_) => {
            // The persistent profile is unusable (corrupt / created by a different Chrome version /
            // still locked by a leftover instance). Fall back to a FRESH throwaway profile so the
            // automation browser ALWAYS launches instead of dying on "个人资料出了点问题". We only lose
            // this run's saved cookies, never the agent's ability to drive the browser.
            let fresh = std::env::temp_dir()
                .join(format!("michael-ide-browser-{}", std::process::id()));
            let _ = std::fs::create_dir_all(&fresh);
            let opts2 = LaunchOptionsBuilder::default()
                .path(Some(std::path::PathBuf::from(&path)))
                .headless(headless)
                .sandbox(false)
                .ignore_certificate_errors(true)
                .user_data_dir(Some(fresh))
                .window_size(Some((1280, 900)))
                .args(extra_ref.clone())
                .build()
                .map_err(|e| e.to_string())?;
            Browser::new(opts2).map_err(|e| e.to_string())?
        }
    };
    let tab = browser.new_tab().map_err(|e| e.to_string())?;
    // Slow intranet pages need more headroom than the old 15s.
    tab.set_default_timeout(Duration::from_secs(30));
    Ok(Session {
        _browser: browser,
        tab,
    })
}

/// Try to attach to a user's already-running Chrome that has remote debugging
/// enabled (e.g. launched with `--remote-debugging-port=9222`).  This lets the
/// agent drive the user's REAL browser — with all their cookies, logged-in
/// sessions, and extensions — instead of a separate automation instance.
fn try_connect_existing() -> Option<Session> {
    for port in [9222u16, 9223, 9224, 9225, 9226, 9229] {
        let ws_url = match get_cdp_ws_url(port) {
            Some(url) => url,
            None => continue,
        };
        let browser = match Browser::connect_with_timeout(ws_url, Duration::from_secs(120)) {
            Ok(b) => b,
            Err(_) => continue,
        };
        match browser.new_tab() {
            Ok(tab) => {
                tab.set_default_timeout(Duration::from_secs(30));
                tracing::info!("[browser] attached to existing Chrome on port {port}");
                return Some(Session {
                    _browser: browser,
                    tab,
                });
            }
            Err(_) => continue,
        }
    }
    None
}

/// Query Chrome's `/json/version` endpoint to get the DevTools WebSocket URL.
fn get_cdp_ws_url(port: u16) -> Option<String> {
    use std::io::{Read, Write};
    let addr: std::net::SocketAddr = format!("127.0.0.1:{port}").parse().ok()?;
    let mut stream =
        std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(300)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_millis(1000)))
        .ok()?;
    let req = format!("GET /json/version HTTP/1.0\r\nHost: 127.0.0.1:{port}\r\n\r\n");
    stream.write_all(req.as_bytes()).ok()?;
    let mut buf = Vec::with_capacity(2048);
    let _ = stream.read_to_end(&mut buf);
    let text = std::str::from_utf8(&buf).ok()?;
    let body = text.split("\r\n\r\n").nth(1)?;
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    json.get("webSocketDebuggerUrl")?
        .as_str()
        .map(|s| s.to_string())
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
  try { var ifs=document.querySelectorAll('iframe');
    for(var fi=0;fi<ifs.length&&i<60;fi++){try{
      var idoc=ifs[fi].contentDocument; if(!idoc) continue;
      idoc.querySelectorAll('[data-mref]').forEach(function(e){e.removeAttribute('data-mref')});
      var ifr=ifs[fi].getBoundingClientRect();
      var iels=Array.from(idoc.querySelectorAll(sel));
      for(var ie=0;ie<iels.length&&i<60;ie++){
        var iel=iels[ie]; var ir=iel.getBoundingClientRect();
        if(ir.width<1||ir.height<1) continue;
        var at=ifr.top+ir.top,al=ifr.left+ir.left;
        if(at+ir.height<0||al+ir.width<0||at>innerHeight||al>innerWidth) continue;
        try{var ics=idoc.defaultView.getComputedStyle(iel);
          if(ics.visibility==='hidden'||ics.display==='none'||ics.opacity==='0') continue;
        }catch(_){}
        iel.setAttribute('data-mref',i);
        var itag=iel.tagName.toLowerCase();
        var itype=(iel.getAttribute('type')||'').slice(0,20);
        var itext=(iel.innerText||iel.value||iel.getAttribute('aria-label')||iel.getAttribute('placeholder')||iel.getAttribute('title')||iel.getAttribute('name')||'').trim().replace(/\s+/g,' ').slice(0,60);
        out.push({ref:i,tag:itag,type:itype,text:itext});
        if(DRAW){var ib=document.createElement('div');ib.className='__mcp_som';ib.textContent=String(i);
          ib.style.cssText='position:absolute;z-index:2147483647;background:#d93025;color:#fff;font:bold 11px monospace;padding:0 3px;border-radius:3px;pointer-events:none;line-height:15px;box-shadow:0 0 0 1px #fff;';
          ib.style.left=(al+window.scrollX)+'px';ib.style.top=(at+window.scrollY)+'px';
          document.body.appendChild(ib);}
        i++;
      }
    }catch(e2){}}
  }catch(e3){}
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
                    *guard = Some(match try_connect_existing() {
                        Some(s) => s,
                        None => launch()?,
                    });
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
    // Don't mangle URLs that already carry a scheme — the old code did `https://{url}` for anything
    // not http(s), turning `file:///D:/x.html` into `https://file:///D:/x.html` (看本地 html 就废了).
    let url = if url.starts_with("http://") || url.starts_with("https://")
        || url.starts_with("file://") || url.starts_with("about:") || url.starts_with("data:")
    {
        url
    } else if url.len() >= 2 && url.as_bytes()[1] == b':' && url.as_bytes()[0].is_ascii_alphabetic() {
        // Windows drive path (D:\… or D:/…) → open as a local file
        format!("file:///{}", url.replace('\\', "/"))
    } else if url.starts_with('/') {
        // Unix absolute path (/Users/…/x.html) → local file
        format!("file://{url}")
    } else {
        // bare host / domain → default to https
        format!("https://{url}")
    };
    with_tab(move |tab| {
        tab.navigate_to(&url).map_err(|e| e.to_string())?;
        tab.wait_until_navigated().map_err(|e| e.to_string())?;
        Ok(None)
    })
    .await
}

/// Click an element via JS eval — works inside iframes (same-origin) where
/// CDP's `DOM.querySelector` can't reach. Last-resort fallback.
fn click_via_eval(tab: &Tab, selector: &str) -> Result<(), String> {
    let sel_js = serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".into());
    let js = format!(
        r#"(()=>{{var s={sel_js};
var el=document.querySelector(s);
if(el){{el.scrollIntoView({{block:'center'}});el.click();return 'ok'}}
var fs=document.querySelectorAll('iframe');
for(var k=0;k<fs.length;k++){{try{{
  el=fs[k].contentDocument.querySelector(s);
  if(el){{el.scrollIntoView({{block:'center'}});el.click();return 'ok'}}
}}catch(e){{}}}}
return 'no'}})()"#
    );
    let ro = tab.evaluate(&js, false).map_err(|e| e.to_string())?;
    let v = ro
        .value
        .as_ref()
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if v == "ok" {
        Ok(())
    } else {
        Err(format!("找不到元素: {selector}"))
    }
}

/// Type into an element via JS eval — iframe-aware fallback.
fn type_via_eval(tab: &Tab, selector: &str, text: &str) -> Result<(), String> {
    let sel_js = serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".into());
    let txt_js = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into());
    let js = format!(
        r#"(()=>{{var s={sel_js},t={txt_js};
function fill(el){{el.scrollIntoView({{block:'center'}});el.focus();el.click();
  el.value=t;el.dispatchEvent(new Event('input',{{bubbles:true}}));
  el.dispatchEvent(new Event('change',{{bubbles:true}}));return 'ok'}}
var el=document.querySelector(s);if(el)return fill(el);
var fs=document.querySelectorAll('iframe');
for(var k=0;k<fs.length;k++){{try{{
  el=fs[k].contentDocument.querySelector(s);if(el)return fill(el);
}}catch(e){{}}}}
return 'no'}})()"#
    );
    let ro = tab.evaluate(&js, false).map_err(|e| e.to_string())?;
    let v = ro
        .value
        .as_ref()
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if v == "ok" {
        Ok(())
    } else {
        Err(format!("找不到输入框: {selector}"))
    }
}

/// Click the first element matching a CSS selector.
#[tauri::command]
pub async fn browser_click(selector: String) -> Result<BrowserState, String> {
    with_tab(move |tab| {
        let el = match tab.find_element(&selector) {
            Ok(el) => el,
            Err(_) if selector.starts_with("[data-mref=") || selector.starts_with("[data-mnode=") => {
                enumerate_elements(tab);
                std::thread::sleep(Duration::from_millis(150));
                match tab.find_element(&selector) {
                    Ok(el) => el,
                    Err(_) => {
                        click_via_eval(tab, &selector)?;
                        std::thread::sleep(Duration::from_millis(400));
                        let _ = tab.wait_until_navigated();
                        return Ok(None);
                    }
                }
            }
            Err(_) => {
                click_via_eval(tab, &selector).map_err(|_| format!("找不到元素: {selector}"))?;
                std::thread::sleep(Duration::from_millis(400));
                let _ = tab.wait_until_navigated();
                return Ok(None);
            }
        };
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
        let el = match tab.find_element(&selector) {
            Ok(el) => el,
            Err(_) if selector.starts_with("[data-mref=") || selector.starts_with("[data-mnode=") => {
                enumerate_elements(tab);
                std::thread::sleep(Duration::from_millis(150));
                match tab.find_element(&selector) {
                    Ok(el) => el,
                    Err(_) => {
                        type_via_eval(tab, &selector, &text)?;
                        return Ok(None);
                    }
                }
            }
            Err(_) => {
                type_via_eval(tab, &selector, &text)
                    .map_err(|_| format!("找不到输入框: {selector}"))?;
                return Ok(None);
            }
        };
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

/// Return ALL cookies (including HttpOnly) for the current page via CDP.
#[tauri::command]
pub async fn browser_cookies(domain: Option<String>) -> Result<BrowserState, String> {
    with_tab(move |tab| {
        let all = tab.get_cookies().map_err(|e| format!("获取 cookies 失败: {e}"))?;
        let result = serde_json::to_string_pretty(&all).unwrap_or_else(|_| "[]".into());
        if let Some(ref d) = domain {
            let filtered: Vec<serde_json::Value> = serde_json::from_str::<Vec<serde_json::Value>>(&result)
                .unwrap_or_default()
                .into_iter()
                .filter(|c| c.get("domain").and_then(|v| v.as_str()).unwrap_or("").contains(d.as_str()))
                .collect();
            Ok(Some(serde_json::to_string_pretty(&filtered).unwrap_or_else(|_| "[]".into())))
        } else {
            Ok(Some(result))
        }
    })
    .await
}

/// Read localStorage / sessionStorage from the current page.
#[tauri::command]
pub async fn browser_storage(storage_type: Option<String>) -> Result<BrowserState, String> {
    with_tab(move |tab| {
        let st = storage_type.as_deref().unwrap_or("local");
        let obj = if st == "session" { "sessionStorage" } else { "localStorage" };
        let js = format!(r#"(() => {{
            try {{
                const s = {obj};
                const out = {{}};
                for (let i = 0; i < s.length; i++) {{
                    const k = s.key(i);
                    out[k] = s.getItem(k);
                }}
                return JSON.stringify(out);
            }} catch(e) {{ return JSON.stringify({{ error: e.message }}); }}
        }})()"#);
        let ro = tab.evaluate(&js, false).map_err(|e| e.to_string())?;
        let s = ro.value
            .and_then(|v| v.as_str().map(|x| x.to_string()))
            .unwrap_or_else(|| "{}".into());
        Ok(Some(s))
    })
    .await
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

/// Synchronous close — called from cleanup_stale on webview reload / app exit.
pub fn close_all() {
    if let Ok(mut g) = BROWSER.lock() {
        *g = None;
    }
}

/// Kill orphaned Chrome processes from previous IDE sessions.
/// headless_chrome spawns a process tree (main + GPU + renderer + utility); if the IDE
/// crashes or the webview reloads without dropping the Session, the children become
/// permanent zombies. We sweep for any Chrome whose profile dir matches our temp/persistent
/// patterns and was NOT launched by the current process.
pub fn kill_orphaned_browsers() {
    std::thread::spawn(|| {
        #[cfg(not(windows))]
        {
            let my_pid = std::process::id().to_string();
            let out = match std::process::Command::new("pgrep")
                .args(["-f", "rust-headless-chrome-profile|michael-ide-browser"])
                .output()
            {
                Ok(o) => o,
                Err(_) => return,
            };
            let pids: Vec<&str> = std::str::from_utf8(&out.stdout)
                .unwrap_or("")
                .lines()
                .filter(|p| !p.is_empty() && *p != my_pid)
                .collect();
            for pid in &pids {
                let _ = std::process::Command::new("kill")
                    .args(["-9", pid])
                    .output();
            }
            if !pids.is_empty() {
                eprintln!("[browser] killed {} orphaned Chrome process(es)", pids.len());
            }
        }
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/FI", "IMAGENAME eq chrome.exe", "/FI", "WINDOWTITLE eq rust-headless*"])
                .output();
        }
    });
}
