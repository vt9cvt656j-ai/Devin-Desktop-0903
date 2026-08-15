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

use headless_chrome::protocol::cdp::{
    Emulation::SetDeviceMetricsOverride,
    Page::{AddScriptToEvaluateOnNewDocument, CaptureScreenshotFormatOption},
    Performance::{Enable as EnablePerformanceMetrics, GetMetrics},
};
use headless_chrome::{Browser, LaunchOptionsBuilder, Tab};
use serde::{Deserialize, Serialize};

struct Session {
    _browser: Browser, // kept alive so the child process & connection survive
    tab: Arc<Tab>,
}

/// The state lock protects only which browser session is current. It must never
/// cover CDP calls, screenshots, or process teardown: all of those can block for
/// seconds. `generation` prevents a launch that began before a concurrent close
/// from installing a session after that close has already completed.
#[derive(Default)]
struct BrowserStore {
    session: Option<Session>,
    generation: u64,
}

impl BrowserStore {
    fn invalidate(&mut self) -> Option<Session> {
        self.generation = self.generation.wrapping_add(1);
        self.session.take()
    }

    fn launch_is_current(&self, generation: u64) -> bool {
        self.generation == generation
    }
}

static BROWSER: LazyLock<Mutex<BrowserStore>> =
    LazyLock::new(|| Mutex::new(BrowserStore::default()));

// CDP actions mutate one persistent tab and therefore need serialization, but
// this separate mutex lets close/reload take the browser state immediately.
static BROWSER_OPERATION: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Pending "which browser did you get" note, produced when a session is established
/// and consumed by the first `snapshot` after it. `Option` rather than a plain
/// String so it is delivered exactly once instead of on every action.
static SESSION_NOTE: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

fn set_session_note(note: String) {
    if let Ok(mut slot) = SESSION_NOTE.lock() {
        *slot = Some(note);
    }
}

// Installed before any page script runs. The frontend's browser check reads these
// two arrays, so recording at document creation time preserves boot-time failures
// that would otherwise be gone by the time the agent asks for a health check.
// Keep this deliberately narrow: status/error summaries only, never request or
// response headers/bodies. URLs are stripped of query strings and fragments.
const PAGE_OBSERVER_SCRIPT: &str = r#"(() => {
  try {
    if (window.__MICHAEL_IDE_OBSERVER__) return;
    Object.defineProperty(window, '__MICHAEL_IDE_OBSERVER__', { value: true });
    var MAX_EVENTS = 80;
    var queue = function(name) {
      var items = Array.isArray(window[name]) ? window[name] : [];
      window[name] = items;
      return function(entry) {
        try {
          items.push(entry);
          if (items.length > MAX_EVENTS) items.splice(0, items.length - MAX_EVENTS);
        } catch (_) {}
      };
    };
    var safeText = function(value) {
      try {
        if (value == null) return String(value);
        if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') return String(value).slice(0, 280);
        if (value instanceof Error) return ((value.name || 'Error') + ': ' + (value.message || '')).slice(0, 280);
        return Object.prototype.toString.call(value).slice(0, 80);
      } catch (_) { return '[unavailable]'; }
    };
    var safeArgs = function(args) {
      try { return Array.prototype.map.call(args, safeText).join(' ').slice(0, 280); }
      catch (_) { return '[unavailable]'; }
    };
    var safeUrl = function(value) {
      try {
        var parsed = new URL(String(value || ''), location.href);
        return (parsed.origin + parsed.pathname).slice(0, 240);
      } catch (_) {
        return String(value || '').split('#')[0].split('?')[0].slice(0, 240);
      }
    };
    var clock = function() { try { return performance.now(); } catch (_) { return 0; } };
    var errors = queue('__MERR__');
    var network = queue('__MNET__');

    try {
      var originalError = console.error;
      console.error = function() {
        errors({ level: 'error', msg: safeArgs(arguments) });
        return originalError.apply(console, arguments);
      };
      var originalWarn = console.warn;
      console.warn = function() {
        errors({ level: 'warn', msg: safeArgs(arguments) });
        return originalWarn.apply(console, arguments);
      };
    } catch (_) {}
    try {
      window.addEventListener('error', function(event) {
        errors({
          level: 'error',
          msg: safeText((event && (event.message || (event.error && event.error.message))) || 'script error'),
          src: event && event.filename ? safeUrl(event.filename) + ':' + (event.lineno || '?') : ''
        });
      }, true);
      window.addEventListener('unhandledrejection', function(event) {
        var reason = event && event.reason;
        errors({ level: 'error', msg: ('unhandledrejection: ' + safeText(reason && (reason.message || reason))).slice(0, 280) });
      });
    } catch (_) {}

    try {
      if (window.fetch && !window.fetch.__michaelObserverWrapped) {
        var originalFetch = window.fetch;
        var observedFetch = function(input, init) {
          var started = clock();
          var method = String((init && init.method) || (input && input.method) || 'GET').toUpperCase().slice(0, 12);
          var url = safeUrl(typeof input === 'string' ? input : (input && input.url));
          return originalFetch.apply(this, arguments).then(function(response) {
            if (!response.ok) network({ kind: 'fetch', method: method, url: url, status: response.status, ok: false, ms: Math.round(clock() - started) });
            return response;
          }, function(error) {
            network({ kind: 'fetch', method: method, url: url, status: 0, ok: false, ms: Math.round(clock() - started), error: safeText(error) });
            throw error;
          });
        };
        Object.defineProperty(observedFetch, '__michaelObserverWrapped', { value: true });
        window.fetch = observedFetch;
      }
    } catch (_) {}

    try {
      var Xhr = window.XMLHttpRequest;
      if (Xhr && Xhr.prototype && !Xhr.prototype.__michaelObserverWrapped) {
        var meta = new WeakMap();
        var originalOpen = Xhr.prototype.open;
        var originalSend = Xhr.prototype.send;
        Xhr.prototype.open = function(method, url) {
          meta.set(this, { method: String(method || 'GET').toUpperCase().slice(0, 12), url: safeUrl(url) });
          return originalOpen.apply(this, arguments);
        };
        Xhr.prototype.send = function() {
          var xhr = this;
          var started = clock();
          var details = meta.get(xhr) || { method: 'GET', url: '' };
          var reported = false;
          xhr.addEventListener('loadend', function() {
            if (reported) return;
            reported = true;
            if (xhr.status === 0 || xhr.status >= 400) network({ kind: 'xhr', method: details.method, url: details.url, status: xhr.status || 0, ok: false, ms: Math.round(clock() - started) });
          }, { once: true });
          return originalSend.apply(this, arguments);
        };
        Object.defineProperty(Xhr.prototype, '__michaelObserverWrapped', { value: true });
      }
    } catch (_) {}
  } catch (_) {}
})();"#;

fn configure_new_tab(tab: &Tab) -> Result<(), String> {
    tab.set_default_timeout(Duration::from_secs(30));
    tab.call_method(AddScriptToEvaluateOnNewDocument {
        source: PAGE_OBSERVER_SCRIPT.to_string(),
        world_name: None,
        include_command_line_api: None,
        run_immediately: Some(true),
    })
    .map_err(|e| format!("安装页面错误观测器失败: {e}"))?;
    Ok(())
}

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
    /// Set when the page we landed on is a human-verification / bot wall rather than
    /// the content that was asked for (reCAPTCHA, Cloudflare challenge, "unusual
    /// traffic"). Names which wall it is.
    ///
    /// This exists so the agent STOPS instead of scraping the challenge page and
    /// retrying it forever. The window is visible by design: the human can clear the
    /// check themselves and the run continues. Nothing here tries to defeat, evade or
    /// solve the challenge — announcing the wall is the opposite of hiding from it.
    #[serde(skip_serializing_if = "Option::is_none")]
    blocked: Option<String>,
    /// One-shot note about WHICH browser this session is (attached to an existing one
    /// vs. freshly launched, and with which profile). Delivered on the first action
    /// after the browser comes up, so "why is there a second Chrome icon" is answered
    /// where it happens instead of being a mystery.
    #[serde(skip_serializing_if = "Option::is_none")]
    session_note: Option<String>,
}

/// 用户此刻是不是正开着这个浏览器（macOS）。
///
/// 进程名由 `BrowserKind` 给，不再写死 "Google Chrome"：选了 Edge 却拿 Chrome 的
/// 进程名去问，答案永远是错的，跟着错的是「真实 profile 能不能共享」这个判断。
#[cfg(target_os = "macos")]
fn is_browser_running(process_name: &str) -> bool {
    std::process::Command::new("pgrep")
        .args(["-x", process_name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
fn is_browser_running(_process_name: &str) -> bool {
    false
}

/// 这个浏览器**用户本人**那份配置目录在哪（macOS）。只有显式开了共享才会走到这里。
#[cfg(target_os = "macos")]
fn real_profile_dir(kind_id: &str) -> Option<std::path::PathBuf> {
    let rel = match kind_id {
        "chrome" => "Library/Application Support/Google/Chrome",
        "edge" => "Library/Application Support/Microsoft Edge",
        "brave" => "Library/Application Support/BraveSoftware/Brave-Browser",
        "chromium" => "Library/Application Support/Chromium",
        _ => return None,
    };
    std::env::var_os("HOME")
        .map(|h| std::path::PathBuf::from(h).join(rel))
        .filter(|p| p.join("Default").exists())
}

#[cfg(not(target_os = "macos"))]
fn real_profile_dir(_kind_id: &str) -> Option<std::path::PathBuf> {
    None
}

/// 每个浏览器自己的独立配置目录名（在 `~/.michael-ide/` 下）。
///
/// **不能共用一个目录**：Chrome 和 Edge 的 profile 格式并不互通，同一个目录轮流被两个
/// 浏览器打开会把它写坏，代价是里面攒的全部登录态一起没。
///
/// chrome 保留原来那个不带后缀的名字——已经在自动化窗口里登录过的人不该因为这次改动
/// 平白丢一次登录态。
fn profile_dir_name(kind_id: &str) -> String {
    if kind_id == "chrome" {
        "browser-profile".to_string()
    } else {
        format!("browser-profile-{kind_id}")
    }
}

/// 用户配置要加载的浏览器扩展目录（未打包的目录，不是 .crx）。
static EXTENSION_DIRS: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// 设定要加载的扩展目录列表。只保留真实存在的目录——传进来一个不存在的路径，
/// 浏览器要么整个启动失败、要么默默忽略，两种都比在这里先筛掉更难查。
pub fn set_extension_dirs(dirs: Vec<String>) {
    let kept: Vec<String> = dirs
        .into_iter()
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty() && std::path::Path::new(d).is_dir())
        .collect();
    if let Ok(mut slot) = EXTENSION_DIRS.lock() {
        *slot = kept;
    }
}

fn extension_dirs() -> Vec<String> {
    if let Ok(slot) = EXTENSION_DIRS.lock() {
        if !slot.is_empty() {
            return slot.clone();
        }
    }
    std::env::var("MICHAEL_BROWSER_EXTENSIONS")
        .ok()
        .map(|v| {
            v.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && std::path::Path::new(s).is_dir())
                .collect()
        })
        .unwrap_or_default()
}

fn launch() -> Result<Session, String> {
    let (kind, path, missing_pref) = crate::capture::resolve_browser(
        crate::capture::browser_pref().as_deref(),
    )
    .ok_or("未找到 Chrome / Edge / Brave / Chromium，无法启动浏览器自动化。请先安装其一。")?;
    let exts = extension_dirs();
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
        // 这一条要**同时**带上 headless_chrome 默认参数里那份的值（TranslateUI /
        // BlinkGenPropertyTrees），并在下面把默认那条摘掉。
        //
        // 原来是两条 --disable-features 一起传，靠「Chromium 对重复开关只认最后一次」
        // 让我们这条覆盖掉默认那条——也就是默认那份的值被静默丢掉了。之前没出问题只是
        // 因为那两个值本来就不重要；但现在开始用 ignore_default_args 管理默认参数，
        // 再留着这种靠拼接顺序生效的隐式行为，出问题时根本没法推理。
        "--disable-features=Translate,TranslateUI,BlinkGenPropertyTrees,InfobarScreenshot,MediaRouter,OptimizationHints".into(),
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
    // 真实 Chrome profile = 用户的全部 cookie、登录态、保存的密码和扩展。要不要把它交给
    // 自动化，必须是一个**显式选择**。
    //
    // 之前的判据是「Chrome 此刻没在跑」—— 一个偶发状态。同一句话让 AI 去访问某个网站，
    // 你的浏览器开着就是隔离 profile，关着就是把全部登录态交出去，用户完全无从预期；
    // 而恶意网页只要能让自动化访问它，就能顺着这份登录态操作你已登录的任何站点。
    //
    // 现在改成显式开关 `MICHAEL_BROWSER_USE_REAL_PROFILE=1`。能力完整保留，只是要你自己
    // 点头；不设时一律用 ~/.michael-ide/browser-profile 这份独立 profile。
    let use_real_profile = std::env::var("MICHAEL_BROWSER_USE_REAL_PROFILE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    // 浏览器在跑时它的 profile 是锁着的，共享不了——即使显式开了也只能退回独立 profile。
    let browser_running = is_browser_running(kind.process_name);
    let real_profile = if use_real_profile && !browser_running {
        real_profile_dir(kind.id)
    } else {
        None
    };
    let used_real_profile = real_profile.is_some();
    let profile_name = profile_dir_name(kind.id);
    let profile_dir = real_profile.or_else(|| {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(|h| {
                std::path::PathBuf::from(h)
                    .join(".michael-ide")
                    .join(&profile_name)
            })
    });
    // Say which browser this is and why, once, on the first action after it opens.
    // The separate Dock icon is not a bug to be hidden — it is a real second Chrome,
    // and the only honest thing to do is name the reason it had to be started.
    let mut note = if used_real_profile {
        format!(
            "已用你本人的 {} 配置启动自动化浏览器（登录态/cookie 都在），但它是一个独立进程，所以 Dock 里会多一个图标。",
            kind.label
        )
    } else {
        // 这里只说**已经证实**的原因，不推荐没验证过的开关。
        // 真实 profile 那条路（MICHAEL_BROWSER_USE_REAL_PROFILE）在浏览器运行时必定
        // 失败（配置目录被 SingletonLock 锁着），所以它不是一条能推荐的路，别写成建议。
        let why = if use_real_profile && browser_running {
            format!("你已开着 {}，它把配置目录锁住了，共享不了", kind.label)
        } else {
            "接管一个已经开着的浏览器，需要那个实例在启动时就开了调试端口，而端口是启动期参数，跑起来之后加不上".to_string()
        };
        format!(
            "自动化用的是 **{}**，是新起的一个实例（Dock 里多出来的那个图标就是它）。\
原因：先扫了 9222-9229 调试端口没找到可接管的实例——{why}。\
它用的是独立配置 ~/.michael-ide/{profile_name}：全新、没登录过，\
所以像 Google 这类站点更容易弹人机验证。\
**在这个窗口里登录一次会被记住**，下次直接就是登录态——这是让它少弹验证的正路。",
            kind.label
        )
    };
    // 选了一个没装的浏览器，不能默默换一个了事——那是这套代码以前最让人困惑的地方。
    if let Some(ref want) = missing_pref {
        let have = crate::capture::installed_browsers()
            .iter()
            .map(|(k, _)| k.id)
            .collect::<Vec<_>>()
            .join(" / ");
        note = format!(
            "你指定的浏览器「{want}」没装，这次改用了 {}。本机装了：{}。\n{note}",
            kind.label,
            if have.is_empty() { "无".into() } else { have }
        );
    }
    // 扩展：Chrome 137 起彻底不理 --load-extension（实测 151 连坏 manifest 都不报错，
    // 加 --disable-features=DisableLoadExtensionCommandLineSwitch 也救不回来）。
    // 配了却不生效必须当场说清楚，不然用户会以为是自己路径写错了。
    if !exts.is_empty() && kind.id == "chrome" {
        note = format!(
            "{note}\n注意：配了 {} 个浏览器扩展，但 Chrome 从 137 起已经不再接受命令行加载扩展，这些扩展**不会生效**。要用扩展得把自动化浏览器换成 Edge / Brave / Chromium（本机没装的话要先装）。",
            exts.len()
        );
    }
    set_session_note(note);
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
                    prof.insert(
                        "exit_type".into(),
                        serde_json::Value::String("Normal".into()),
                    );
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
    // 扩展要生效，两件事缺一不可：把扩展目录交给浏览器，**并且**把这个库默认加的
    // `--disable-extensions` 摘掉——只做前一半的话浏览器照样一个扩展都不加载。
    //
    // 只摘 `--disable-extensions` 这一个。`--enable-automation` 同在那份默认参数里，
    // 摘掉它就是**隐藏自动化身份**，也就是反检测——这个产品不做，由测试钉住。
    let ext_os: Vec<std::ffi::OsString> = exts.iter().map(std::ffi::OsString::from).collect();
    let ext_ref: Vec<&std::ffi::OsStr> = ext_os.iter().map(|s| s.as_os_str()).collect();
    let mut drop_defaults: Vec<&std::ffi::OsStr> = vec![
        // 我们自己那条 --disable-features 已经把默认那条的值并进去了，摘掉默认的，
        // 让命令行上只剩一条——不再依赖「重复开关取最后一次」这种隐式覆盖。
        std::ffi::OsStr::new("--disable-features=TranslateUI,BlinkGenPropertyTrees"),
    ];
    if !ext_ref.is_empty() {
        drop_defaults.push(std::ffi::OsStr::new("--disable-extensions"));
    }
    let opts = LaunchOptionsBuilder::default()
        .path(Some(std::path::PathBuf::from(&path)))
        .headless(headless)
        .sandbox(false)
        .ignore_certificate_errors(true)
        .user_data_dir(profile_dir.clone())
        .window_size(Some((1280, 900)))
        .args(extra_ref.clone())
        .extensions(ext_ref.clone())
        .ignore_default_args(drop_defaults.clone())
        .build()
        .map_err(|e| e.to_string())?;
    let browser = match Browser::new(opts) {
        Ok(b) => b,
        Err(_) => {
            // The persistent profile is unusable (corrupt / created by a different Chrome version /
            // still locked by a leftover instance). Fall back to a FRESH throwaway profile so the
            // automation browser ALWAYS launches instead of dying on "个人资料出了点问题". We only lose
            // this run's saved cookies, never the agent's ability to drive the browser.
            let fresh =
                std::env::temp_dir().join(format!("michael-ide-browser-{}", std::process::id()));
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
            set_session_note(
                "上一份浏览器配置用不了（损坏或被残留进程锁住），已改用一次性临时配置启动。\
本次的登录态不会保留，但自动化不会因此中断。"
                    .into(),
            );
            Browser::new(opts2).map_err(|e| e.to_string())?
        }
    };
    let tab = browser.new_tab().map_err(|e| e.to_string())?;
    // Slow intranet pages need more headroom than the old 15s. Install the
    // document observer before this new tab navigates anywhere.
    configure_new_tab(&tab)?;
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
        let (ws_url, brand) = match get_cdp_ws_url(port) {
            Some(pair) => pair,
            None => continue,
        };
        let browser = match Browser::connect_with_timeout(ws_url, Duration::from_secs(120)) {
            Ok(b) => b,
            Err(_) => continue,
        };
        match browser.new_tab() {
            Ok(tab) => {
                if configure_new_tab(&tab).is_err() {
                    continue;
                }
                tracing::info!("[browser] attached to existing {brand} on port {port}");
                set_session_note(format!(
                    "已接管你已经开着的 {brand}（调试端口 {port}），没有另起浏览器——用的是你本人的登录态、cookie 和扩展。"
                ));
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
fn get_cdp_ws_url(port: u16) -> Option<(String, String)> {
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
    // **重建**地址，而不是照用返回值里的那个。
    //
    // 我们只是"探到 127.0.0.1:<port> 上有个 /json/version 能回 JSON"就连过去，而回什么
    // 完全由那个进程说了算 —— 本机任意程序（含被装上的恶意 npm 包）抢先占住 9222-9229
    // 里的一个端口，就能把 webSocketDebuggerUrl 指向别处，从而接管全部浏览器自动化：
    // 看到你访问的每个页面、每次输入。
    //
    // 真实 Chrome/Edge 返回的就是 ws://127.0.0.1:<同一端口>/devtools/browser/<uuid>，
    // 所以只取 path 重新拼是无损的。
    let raw = json.get("webSocketDebuggerUrl")?.as_str()?;
    let after_scheme = raw
        .strip_prefix("ws://")
        .or_else(|| raw.strip_prefix("wss://"))?;
    let path = after_scheme.find('/').map(|i| &after_scheme[i..])?;
    if !path.starts_with("/devtools/") {
        return None;
    }
    // 这个端口后面到底是谁。以前这个字段被丢掉了，于是接管了 Edge 也照样告诉用户
    // 「已接管你的 Chrome」——而那正是用户唯一能拿来核对「接管的是不是我那个窗口」
    //   的信息。认不出来就说「浏览器」，别默认叫 Chrome。
    let brand = brand_from_version(json.get("Browser").and_then(|v| v.as_str()));
    Some((format!("ws://127.0.0.1:{port}{path}"), brand))
}

/// `/json/version` 的 `Browser` 字段（形如 `Chrome/151.0` 或 `Edg/126.0`）→ 人话牌子名。
///
/// 认不出来就说「浏览器」，**不默认叫 Chrome**：这句话是用户唯一能拿来核对
/// 「接管的到底是不是我那个窗口」的信息，说错了比不说更糟。
fn brand_from_version(raw: Option<&str>) -> String {
    let Some(raw) = raw else {
        return "浏览器".to_string();
    };
    match raw.split('/').next().unwrap_or(raw).trim() {
        "Edg" | "Edge" => "Microsoft Edge".to_string(),
        "Chrome" | "HeadlessChrome" => "Google Chrome".to_string(),
        "Brave" => "Brave".to_string(),
        "Chromium" => "Chromium".to_string(),
        other if !other.is_empty() => other.to_string(),
        _ => "浏览器".to_string(),
    }
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
    let js = raw.replace(
        "__DRAW__",
        if DRAW_MARKS.load(Ordering::Relaxed) {
            "1"
        } else {
            "0"
        },
    );
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

/// Name the human-verification wall this page is, if it is one.
///
/// Detection only. There is deliberately no counterpart that solves, bypasses or
/// hides from these checks — the agent's correct move on hitting one is to stop and
/// hand the visible window back to the person driving it, which it cannot do while
/// it thinks a challenge page is the article it asked for.
///
/// Both halves matter: the DOM probe catches the widget even when the challenge is
/// embedded in an otherwise normal-looking page, and the URL/title check catches the
/// walls that ship no widget at all (a bare 403, Google's `/sorry/` redirect).
fn detect_wall(tab: &Tab, title: &str, url: &str, text: &str) -> Option<String> {
    let probe = r#"(function(){try{
  var hit=[];
  var q=function(s){try{return !!document.querySelector(s)}catch(e){return false}};
  if(q('iframe[src*="recaptcha/api2"],iframe[src*="recaptcha/enterprise"],.g-recaptcha,#recaptcha,#captcha-form')) hit.push('reCAPTCHA');
  if(q('iframe[src*="hcaptcha.com"],.h-captcha')) hit.push('hCaptcha');
  if(q('iframe[src*="challenges.cloudflare.com"],.cf-turnstile,#cf-challenge-running,#challenge-form,#challenge-stage')) hit.push('Cloudflare 验证');
  if(q('#px-captcha,#px-captcha-wrapper')) hit.push('PerimeterX');
  if(q('iframe[src*="arkoselabs"],iframe[src*="funcaptcha"]')) hit.push('Arkose/FunCaptcha');
  return hit.join('、');
}catch(e){return ''}})()"#;
    let mut found = tab
        .evaluate(probe, false)
        .ok()
        .and_then(|ro| ro.value)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();
    if found.is_empty() {
        found = detect_wall_static(title, url, text).unwrap_or_default();
    }
    if found.is_empty() {
        None
    } else {
        Some(found)
    }
}

/// The half of wall detection that needs no page access: walls that ship no widget
/// at all (a bare 403, Google's `/sorry/` redirect, Cloudflare's interstitial).
///
/// Split out from `detect_wall` because it is pure and therefore testable, and
/// because a false positive here is expensive in a specific way: it would tell the
/// user to go clear a verification on a page that has none. So the signals are
/// graded. Only endpoints that exist *solely* to serve a challenge count on their
/// own; every keyword that could plausibly appear in ordinary content ("captcha",
/// "access denied", "unusual traffic") additionally requires a nearly empty page —
/// an interstitial has almost no text, an article about CAPTCHAs has plenty.
fn detect_wall_static(title: &str, url: &str, text: &str) -> Option<String> {
    let lower_url = url.to_ascii_lowercase();
    let lower_title = title.to_ascii_lowercase();
    let lower_text = text.to_ascii_lowercase();
    // Paths that are challenge endpoints and nothing else.
    let strong_url = ["/sorry/index", "/cdn-cgi/challenge-platform/", "/recaptcha/api2/"]
        .iter()
        .any(|needle| lower_url.contains(needle));
    // Titles a real page does not carry.
    let strong_title = ["just a moment", "attention required", "人机身份验证"]
        .iter()
        .any(|needle| lower_title.contains(needle));
    if strong_url || strong_title {
        return Some("人机验证 / 反爬拦截".to_string());
    }
    // An interstitial is a nearly empty page. Below this threshold the weaker
    // keywords stop being ambiguous; above it, the page has real content and the
    // same words are just words.
    let thin = text.chars().count() < 400;
    if !thin {
        return None;
    }
    let weak = ["captcha", "验证码"]
        .iter()
        .any(|needle| lower_url.contains(needle) || lower_title.contains(needle))
        || [
            "unusual traffic",
            "verify you are human",
            "are you a robot",
            "access denied",
            "异常流量",
            "检测到异常",
        ]
        .iter()
        .any(|needle| lower_title.contains(needle) || lower_text.contains(needle));
    if weak {
        Some("人机验证 / 反爬拦截".to_string())
    } else {
        None
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
    let blocked = detect_wall(tab, &title, &url, &text);
    let session_note = SESSION_NOTE.lock().ok().and_then(|mut slot| slot.take());
    Ok(BrowserState {
        title,
        url,
        text,
        screenshot,
        elements,
        result,
        blocked,
        session_note,
    })
}

fn current_or_launch_tab() -> Result<Arc<Tab>, String> {
    let generation = {
        let state = BROWSER.lock().map_err(|_| "browser state poisoned")?;
        if let Some(session) = state.session.as_ref() {
            return Ok(session.tab.clone());
        }
        state.generation
    };

    // Launching Chrome can be slow. Keep the state lock free so close/reload can
    // invalidate this attempt while the browser process is coming up.
    let launched = match try_connect_existing() {
        Some(session) => session,
        None => launch()?,
    };
    let mut launched = Some(launched);
    let tab = {
        let mut state = BROWSER.lock().map_err(|_| "browser state poisoned")?;
        if !state.launch_is_current(generation) {
            None
        } else if let Some(session) = state.session.as_ref() {
            Some(session.tab.clone())
        } else {
            state.session = launched.take();
            state.session.as_ref().map(|session| session.tab.clone())
        }
    };

    // A close won the race or another session was installed. The unused browser
    // is intentionally dropped after releasing the state lock.
    drop(launched);
    tab.ok_or_else(|| "browser was closed while starting".to_string())
}

/// Remove a dead session only when it still owns the tab that failed. A newer
/// launch or a concurrent close must not be torn down by an older request.
fn take_current_session_for_tab(tab: &Arc<Tab>) -> Result<Option<Session>, String> {
    let mut state = BROWSER.lock().map_err(|_| "browser state poisoned")?;
    if state
        .session
        .as_ref()
        .is_some_and(|session| Arc::ptr_eq(&session.tab, tab))
    {
        Ok(state.session.take())
    } else {
        Ok(None)
    }
}

/// Remove the visible session and invalidate in-flight launches. The caller must
/// drop the returned session outside `BROWSER` and preferably under the operation
/// lock so an active CDP request is allowed to finish first.
fn take_browser_session() -> Option<Session> {
    let mut state = BROWSER.lock().ok()?;
    state.invalidate()
}

/// Run a closure against the (lazily-launched) shared tab, on a blocking thread.
async fn with_tab<F>(f: F) -> Result<BrowserState, String>
where
    F: Fn(&Tab) -> Result<Option<String>, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || -> Result<BrowserState, String> {
        // The operation lock preserves the single-tab action order without making
        // close/reload wait on the browser-state mutex.
        let _operation = BROWSER_OPERATION
            .lock()
            .map_err(|_| "browser operation state poisoned")?;
        // Run against the shared tab. If the cached browser's CDP connection is DEAD
        // (it timed out / crashed / was closed — "underlying connection is closed"),
        // toss the stale session, relaunch a fresh browser, and retry ONCE. Without
        // this, a single navigation timeout bricks the browser for the whole session.
        let mut last_err = String::new();
        for attempt in 0..2 {
            let tab = current_or_launch_tab()?;
            // CDP calls, the performance-sampling sleep, and screenshots run
            // without BROWSER locked. The operation lock above still serializes
            // mutations of the persistent tab.
            let outcome = f(&tab).and_then(|result| snapshot(&tab, result));
            match outcome {
                Ok(state) => return Ok(state),
                Err(e) => {
                    last_err = e;
                    if attempt == 0 && is_dead_browser(&last_err) {
                        // Do not tear down a replacement session that was installed
                        // after this tab failed. Drop the stale process outside the
                        // state lock while the operation remains serialized.
                        drop(take_current_session_for_tab(&tab)?);
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

fn is_navigation_wait_timeout(e: &str) -> bool {
    let s = e.to_lowercase();
    s.contains("event waited for never came")
        || s.contains("waited for never came")
        || s.contains("timed out waiting")
        || s.contains("timeout while waiting")
        || s.contains("navigation timeout")
}

fn normalize_navigation_url(url: &str) -> String {
    let url = url.trim();
    // Preserve URLs that already carry a scheme. Local HTML and data documents
    // are first-class preview targets for the IDE, so they must reach Chrome
    // unchanged instead of being rewritten as https://file:///... .
    if url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("file://")
        || url.starts_with("about:")
        || url.starts_with("data:")
    {
        url.to_string()
    } else if url.len() >= 2 && url.as_bytes()[1] == b':' && url.as_bytes()[0].is_ascii_alphabetic()
    {
        // Windows drive path (D:\... or D:/...) -> local file URL.
        format!("file:///{}", url.replace('\\', "/"))
    } else if url.starts_with('/') {
        // Unix absolute path (/Users/.../index.html) -> local file URL.
        format!("file://{url}")
    } else {
        // Bare host / domain defaults to HTTPS.
        format!("https://{url}")
    }
}

/// Open a URL (launches the browser on first use).
#[tauri::command]
pub async fn browser_navigate(url: String) -> Result<BrowserState, String> {
    let url = normalize_navigation_url(&url);
    with_tab(move |tab| {
        tab.navigate_to(&url).map_err(|e| e.to_string())?;
        match tab.wait_until_navigated() {
            Ok(_) => Ok(None),
            Err(e) => {
                let msg = e.to_string();
                if is_navigation_wait_timeout(&msg) {
                    // Old / redirected / non-UTF8 pages sometimes never emit the lifecycle event
                    // headless_chrome waits for, even though Chrome has a usable DOM. Returning a
                    // snapshot is far better evidence than failing the whole card and making the
                    // model retry random URLs.
                    std::thread::sleep(Duration::from_millis(900));
                    Ok(Some(format!(
                        "[NAVIGATION_WAIT_TIMEOUT] 页面没有发出完整导航完成事件，已改为读取当前 DOM/截图继续验证；不要把这当成 URL 错误，先看当前 url/title/text/nodes。原始错误: {}",
                        msg.chars().take(180).collect::<String>()
                    )))
                } else {
                    Err(msg)
                }
            }
        }
    })
    .await
}

/// Click an element via JS eval — works inside iframes (same-origin) where
/// CDP's `DOM.querySelector` can't reach. Last-resort fallback.
fn click_via_eval(tab: &Tab, selector: &str) -> Result<(), String> {
    let sel_js = serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".into());
    let js = format!(
        r#"(()=>{{var s={sel_js};
function docs(){{var out=[document],fs=document.querySelectorAll('iframe');for(var k=0;k<fs.length;k++){{try{{if(fs[k].contentDocument)out.push(fs[k].contentDocument)}}catch(e){{}}}}return out}}
function find(){{var ds=docs();for(var d=0;d<ds.length;d++){{try{{var el=ds[d].querySelector(s);if(el)return el}}catch(e){{}}}}return null}}
function visible(el){{try{{var r=el.getBoundingClientRect(),cs=(el.ownerDocument.defaultView||window).getComputedStyle(el);return r.width>1&&r.height>1&&cs.visibility!=='hidden'&&cs.display!=='none'&&Number(cs.opacity||1)>0.01}}catch(e){{return false}}}}
function disabled(el){{try{{return !!(el.disabled||el.getAttribute('aria-disabled')==='true'||el.closest('[disabled],[aria-disabled="true"]'))}}catch(e){{return false}}}}
function brief(el){{try{{if(!el)return'';var t=String(el.innerText||el.textContent||el.getAttribute('aria-label')||'').replace(/\s+/g,' ').trim().slice(0,60);return el.tagName.toLowerCase()+(el.id?'#'+el.id:'')+(t?' "'+t+'"':'')}}catch(e){{return'element'}}}}
function clickable(el){{try{{var q='a[href],button,input:not([type=hidden]),select,textarea,[role=button],[role=link],[role=tab],[role=menuitem],[role=option],[onclick],label,summary';return el.matches(q)?el:(el.closest(q)||el)}}catch(e){{return el}}}}
function point(el){{var doc=el.ownerDocument||document,win=doc.defaultView||window,r=el.getBoundingClientRect(),pts=[[.5,.5],[.25,.5],[.75,.5],[.5,.25],[.5,.75]];for(var i=0;i<pts.length;i++){{var x=Math.max(1,Math.min((win.innerWidth||1)-2,r.left+r.width*pts[i][0])),y=Math.max(1,Math.min((win.innerHeight||1)-2,r.top+r.height*pts[i][1])),top=null;try{{top=doc.elementFromPoint(x,y)}}catch(e){{}}if(top&&(top===el||el.contains(top)))return{{ok:true,x:x,y:y,top:top}};}}return{{ok:false,top:top}}}}
function ptr(el,name,p,down){{var w=el.ownerDocument.defaultView||window,c={{bubbles:true,cancelable:true,composed:true,view:w,clientX:p.x,clientY:p.y,screenX:p.x,screenY:p.y,button:0,buttons:down?1:0}};try{{if(/^pointer/.test(name)&&typeof PointerEvent!=='undefined'){{var pi=Object.assign({{}},c,{{pointerId:1,pointerType:'mouse',isPrimary:true,pressure:down?0.5:0}});el.dispatchEvent(new PointerEvent(name,pi));return}}}}catch(e){{}}try{{el.dispatchEvent(new MouseEvent(name.replace(/^pointer/,'mouse'),c))}}catch(e){{}}}}
var raw=find();if(!raw)return'no';
var el=clickable(raw);if(disabled(el))return'disabled '+brief(el);
try{{el.scrollIntoView({{block:'center',inline:'center',behavior:'instant'}})}}catch(e){{try{{el.scrollIntoView({{block:'center',inline:'center'}})}}catch(e2){{}}}}
if(!visible(el))return'not_visible '+brief(el);
var p=point(el);if(!p.ok)return'covered by '+brief(p.top);
ptr(el,'pointerover',p,false);ptr(el,'pointermove',p,false);ptr(el,'mouseover',p,false);ptr(el,'mousemove',p,false);ptr(el,'pointerdown',p,true);ptr(el,'mousedown',p,true);
try{{el.focus({{preventScroll:true}})}}catch(e){{try{{el.focus()}}catch(e2){{}}}}
ptr(el,'pointerup',p,false);ptr(el,'mouseup',p,false);
try{{el.click()}}catch(e){{try{{el.dispatchEvent(new MouseEvent('click',{{bubbles:true,cancelable:true,composed:true,view:el.ownerDocument.defaultView||window,clientX:p.x,clientY:p.y,button:0}}))}}catch(e2){{return String(e2&&e2.message||e2)}}}}
return'ok'}})()"#
    );
    let ro = tab.evaluate(&js, false).map_err(|e| e.to_string())?;
    let v = ro.value.as_ref().and_then(|v| v.as_str()).unwrap_or("");
    if v == "ok" {
        Ok(())
    } else {
        Err(format!("元素不可点击或未找到: {selector} ({v})"))
    }
}

/// Type into an element via JS eval — iframe-aware fallback.
fn type_via_eval(tab: &Tab, selector: &str, text: &str) -> Result<(), String> {
    let sel_js = serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".into());
    let txt_js = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into());
    let js = format!(
        r#"(()=>{{var s={sel_js},t={txt_js};
function docs(){{var out=[document],fs=document.querySelectorAll('iframe');for(var k=0;k<fs.length;k++){{try{{if(fs[k].contentDocument)out.push(fs[k].contentDocument)}}catch(e){{}}}}return out}}
function find(){{var ds=docs();for(var d=0;d<ds.length;d++){{try{{var el=ds[d].querySelector(s);if(el)return el}}catch(e){{}}}}return null}}
function target(el){{try{{if(el.tagName==='LABEL'&&el.control)return el.control;if(el.matches('input:not([type=hidden]),textarea,select,[contenteditable=""],[contenteditable=true]'))return el;return el.querySelector('input:not([type=hidden]),textarea,select,[contenteditable=""],[contenteditable=true]')||el}}catch(e){{return el}}}}
function fire(el,name,data){{try{{if(name==='input'&&typeof InputEvent!=='undefined')el.dispatchEvent(new InputEvent('input',{{bubbles:true,cancelable:true,inputType:'insertText',data:data||t}}));else el.dispatchEvent(new Event(name,{{bubbles:true,cancelable:true}}))}}catch(e){{try{{el.dispatchEvent(new Event(name,{{bubbles:true}}))}}catch(e2){{}}}}}}
function nativeSet(el,val){{if(el.isContentEditable){{try{{el.focus()}}catch(e){{}}el.textContent=val;fire(el,'input',val);fire(el,'change',val);return true}}
 if(el.tagName==='SELECT'){{var opts=Array.prototype.slice.call(el.options||[]),hit=opts.find(function(o){{return String(o.value)===String(val)}})||opts.find(function(o){{return String(o.textContent||'').trim()===String(val).trim()}});if(hit)val=hit.value;try{{var sd=Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype,'value');if(sd&&sd.set)sd.set.call(el,val);else el.value=val}}catch(e){{el.value=val}}fire(el,'input',val);fire(el,'change',val);return true}}
 if('value'in el){{try{{var proto=Object.getPrototypeOf(el),desc=proto&&Object.getOwnPropertyDescriptor(proto,'value'),base=el.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype,desc2=Object.getOwnPropertyDescriptor(base,'value');(desc&&desc.set?desc:desc2).set.call(el,val)}}catch(e){{el.value=val}}fire(el,'beforeinput',val);fire(el,'input',val);fire(el,'change',val);return true}}return false}}
var raw=find();if(!raw)return'no';var el=target(raw);
try{{el.scrollIntoView({{block:'center',inline:'center',behavior:'instant'}})}}catch(e){{try{{el.scrollIntoView({{block:'center',inline:'center'}})}}catch(e2){{}}}}
try{{el.focus()}}catch(e){{}}try{{el.click()}}catch(e){{}}
if(!nativeSet(el,t))return'not_editable';
var actual=el.isContentEditable?String(el.textContent||''):String(el.value||'');
return actual===String(t)||actual.indexOf(String(t))>=0?'ok':'value_not_applied '+actual.slice(0,80)}})()"#
    );
    let ro = tab.evaluate(&js, false).map_err(|e| e.to_string())?;
    let v = ro.value.as_ref().and_then(|v| v.as_str()).unwrap_or("");
    if v == "ok" {
        Ok(())
    } else {
        Err(format!("输入未生效或未找到: {selector} ({v})"))
    }
}

/// Click the first element matching a CSS selector.
#[tauri::command]
pub async fn browser_click(selector: String) -> Result<BrowserState, String> {
    with_tab(move |tab| {
        match click_via_eval(tab, &selector) {
            Ok(_) => {
                std::thread::sleep(Duration::from_millis(260));
                let _ = tab.wait_until_navigated();
                return Ok(None);
            }
            Err(first_err)
                if selector.starts_with("[data-mref=") || selector.starts_with("[data-mnode=") =>
            {
                enumerate_elements(tab);
                std::thread::sleep(Duration::from_millis(80));
                if click_via_eval(tab, &selector).is_ok() {
                    std::thread::sleep(Duration::from_millis(260));
                    let _ = tab.wait_until_navigated();
                    return Ok(None);
                }
                let _ = first_err;
            }
            Err(_) => {}
        }
        let el = match tab.find_element(&selector) {
            Ok(el) => el,
            Err(_)
                if selector.starts_with("[data-mref=") || selector.starts_with("[data-mnode=") =>
            {
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
        match type_via_eval(tab, &selector, &text) {
            Ok(_) => return Ok(None),
            Err(first_err)
                if selector.starts_with("[data-mref=") || selector.starts_with("[data-mnode=") =>
            {
                enumerate_elements(tab);
                std::thread::sleep(Duration::from_millis(80));
                if type_via_eval(tab, &selector, &text).is_ok() {
                    return Ok(None);
                }
                let _ = first_err;
            }
            Err(_) => {}
        }
        let el = match tab.find_element(&selector) {
            Ok(el) => el,
            Err(_)
                if selector.starts_with("[data-mref=") || selector.starts_with("[data-mnode=") =>
            {
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
pub async fn browser_upload_file(
    selector: String,
    paths: Vec<String>,
) -> Result<BrowserState, String> {
    with_tab(move |tab| {
        let el = tab
            .find_element(&selector)
            .map_err(|_| format!("找不到文件输入框: {selector}（要选中一个 <input type=file>）"))?;
        let refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
        el.set_input_files(&refs)
            .map_err(|e| format!("设置上传文件失败: {e}"))?;
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

fn stringify_eval_result(value: Option<serde_json::Value>, description: Option<String>) -> String {
    value
        .map(|value| match value {
            serde_json::Value::String(text) => text,
            other => other.to_string(),
        })
        .or(description)
        .unwrap_or_default()
}

/// 页内 UI 规格提取器。
///
/// 不是"把 DOM 全吐出来"——那既超上限又没用。1:1 还原真正需要的是**这个页面实际用了
/// 哪些设计决定**：真在用的配色（按覆盖面积排序，不是声明里出现过就算）、字体组合、
/// 间距刻度、圆角、阴影，加上可见元素的盒模型骨架。
///
/// 这份东西是**读出来的事实**，不是看着截图猜的——所以"从网站还原"能做到接近 1:1，
/// 而"只给一张图"不能。两者的差别就在这里。
const UI_EXTRACT_JS: &str = r####"
(() => {
  const MAX_NODES = __MAX_NODES__;
  const px = (v) => Math.round(parseFloat(v) || 0);
  const seenBg = new Map();
  const seenText = new Map();
  const seenType = new Map();
  const spacing = new Map();
  const radii = new Map();
  const shadows = new Map();
  const nodes = [];
  const assets = [];

  const norm = (c) => {
    if (!c) return "";
    const m = String(c).match(/rgba?\(([^)]+)\)/);
    if (!m) return String(c);
    const p = m[1].split(",").map((x) => parseFloat(x));
    if (p.length >= 4 && p[3] === 0) return "";
    const h = (n) => Math.max(0, Math.min(255, Math.round(n))).toString(16).padStart(2, "0");
    return "#" + h(p[0]) + h(p[1]) + h(p[2]);
  };
  const bump = (map, key, weight) => { if (key) map.set(key, (map.get(key) || 0) + weight); };

  const vw = window.innerWidth, vh = window.innerHeight;
  // 必须把 html 和 body 自己算进去。
  //
  // 原来只走 body.querySelectorAll("*")——那**不含 body 和 html 自身**，于是页面真正的
  // 底色一次都没被采到。在 example.com 上实测：背景是白的，提取出来的主色却是文字的
  // #000000。照这份规格还原会做出一个黑底页面，而且模型没有任何依据察觉不对。
  const roots = [document.documentElement, document.body].filter(Boolean);
  const all = document.body ? [...roots, ...document.body.querySelectorAll("*")] : roots;
  for (const el of all) {
    let r;
    try { r = el.getBoundingClientRect(); } catch (e) { continue; }
    if (!r || r.width < 1 || r.height < 1) continue;
    if (r.bottom < -50 || r.top > vh + 400) continue;
    let cs;
    try { cs = getComputedStyle(el); } catch (e) { continue; }
    if (cs.display === "none" || cs.visibility === "hidden" || parseFloat(cs.opacity) === 0) continue;
    const area = r.width * r.height;

    // 背景色和文字色分开统计。混在一张表里模型分不清"这是底色"还是"这是字色"——
    // 而这两者在还原时是完全不同的用途，搞反了整页观感就反了。
    bump(seenBg, norm(cs.backgroundColor), area);
    const text = (el.childElementCount === 0 ? (el.textContent || "") : "").trim();
    if (text) {
      bump(seenText, norm(cs.color), Math.max(area, 400));
      const key = [cs.fontFamily, cs.fontSize, cs.fontWeight, cs.lineHeight, cs.letterSpacing, norm(cs.color)].join("|");
      const prev = seenType.get(key);
      if (prev) { prev.count += 1; }
      else {
        seenType.set(key, {
          family: cs.fontFamily, size: cs.fontSize, weight: cs.fontWeight,
          lineHeight: cs.lineHeight, letterSpacing: cs.letterSpacing,
          color: norm(cs.color), count: 1, sample: text.slice(0, 40)
        });
      }
    }
    const spaceKeys = ["paddingTop", "paddingBottom", "paddingLeft", "paddingRight", "gap", "rowGap", "columnGap"];
    for (const k of spaceKeys) {
      const v = px(cs[k]);
      if (v > 0 && v <= 200) bump(spacing, v, 1);
    }
    if (cs.borderRadius && cs.borderRadius !== "0px") bump(radii, cs.borderRadius, 1);
    if (cs.boxShadow && cs.boxShadow !== "none") bump(shadows, cs.boxShadow, 1);

    if (el.tagName === "IMG" && el.currentSrc) {
      assets.push({ type: "img", src: String(el.currentSrc).slice(0, 300), alt: (el.alt || "").slice(0, 60),
        x: px(r.left), y: px(r.top), w: px(r.width), h: px(r.height) });
    }
    const bg = cs.backgroundImage;
    if (bg && bg !== "none" && bg.indexOf("url(") === 0) {
      assets.push({ type: "bg", src: bg.slice(4, 200), x: px(r.left), y: px(r.top), w: px(r.width), h: px(r.height) });
    } else if (bg && bg.indexOf("gradient") >= 0) {
      assets.push({ type: "gradient", src: bg.slice(0, 200), x: px(r.left), y: px(r.top), w: px(r.width), h: px(r.height) });
    }

    nodes.push({
      _area: area,
      tag: el.tagName.toLowerCase(),
      cls: (typeof el.className === "string" ? el.className : "").split(/\s+/).filter(Boolean).slice(0, 3).join(" "),
      text: text.slice(0, 60),
      x: px(r.left), y: px(r.top), w: px(r.width), h: px(r.height),
      bg: norm(cs.backgroundColor),
      color: text ? norm(cs.color) : "",
      font: text ? (cs.fontSize + "/" + cs.fontWeight) : "",
      display: cs.display,
      dir: cs.display.indexOf("flex") >= 0 ? cs.flexDirection : (cs.display.indexOf("grid") >= 0 ? String(cs.gridTemplateColumns || "").slice(0, 60) : ""),
      pad: [px(cs.paddingTop), px(cs.paddingRight), px(cs.paddingBottom), px(cs.paddingLeft)].join(" "),
      radius: cs.borderRadius === "0px" ? "" : cs.borderRadius,
      border: cs.borderTopWidth === "0px" ? "" : (cs.borderTopWidth + " " + cs.borderTopStyle + " " + norm(cs.borderTopColor)),
      shadow: cs.boxShadow === "none" ? "" : String(cs.boxShadow).slice(0, 80)
    });
  }

  nodes.sort((a, b) => b._area - a._area);
  const top = nodes.slice(0, MAX_NODES).map((n) => { delete n._area; return n; });
  const rank = (m, n) => Array.from(m.entries()).sort((a, b) => b[1] - a[1]).slice(0, n);

  const fonts = [];
  try { document.fonts.forEach((f) => { if (fonts.indexOf(f.family) < 0) fonts.push(f.family); }); } catch (e) {}

  return JSON.stringify({
    url: location.href,
    title: document.title,
    viewport: { w: vw, h: vh, dpr: window.devicePixelRatio || 1 },
    pageHeight: Math.round(document.documentElement.scrollHeight),
    pageBackground: (() => {
      // 页面底色：body → html 往上找第一个不透明的；都透明就是浏览器默认白。
      for (const el of roots) {
        try { const c = norm(getComputedStyle(el).backgroundColor); if (c) return c; } catch (e) {}
      }
      return "#ffffff";
    })(),
    palette: rank(seenBg, 10).map((e) => ({ hex: e[0], role: "background", coverage: Math.round(e[1]) }))
      .concat(rank(seenText, 6).map((e) => ({ hex: e[0], role: "text", coverage: Math.round(e[1]) }))),
    typography: Array.from(seenType.values()).sort((a, b) => b.count - a.count).slice(0, 12),
    spacingScale: rank(spacing, 10).map((e) => e[0]).sort((a, b) => a - b),
    radii: rank(radii, 6).map((e) => e[0]),
    shadows: rank(shadows, 5).map((e) => e[0]),
    fonts: fonts.slice(0, 8),
    assets: assets.slice(0, 30),
    nodes: top,
    nodeTotal: nodes.length
  });
})()
"####;

/// 提取当前页面的 UI 规格（配色 / 字体 / 间距 / 圆角 / 阴影 / 可见元素盒模型 / 资源）。
///
/// 和 `browser_eval` 的区别不只是脚本：那个函数把结果砍在 8000 字符，对整页规格远远不够，
/// 而且砍完恰好是半截 JSON。这里给 60000，并且**超了如实说**——一份被截断的规格如果冒充
/// 完整，模型会照着它还原，然后对着缺失的部分反复困惑。
#[tauri::command]
pub async fn browser_extract_ui(max_nodes: Option<u32>) -> Result<BrowserState, String> {
    let n = max_nodes.unwrap_or(120).clamp(10, 400);
    let script = UI_EXTRACT_JS.replace("__MAX_NODES__", &n.to_string());
    with_tab(move |tab| {
        let ro = tab.evaluate(&script, true).map_err(|e| e.to_string())?;
        let val = stringify_eval_result(ro.value, ro.description);
        let total = val.chars().count();
        if total > 60000 {
            let head: String = val.chars().take(60000).collect();
            return Ok(Some(format!(
                "{head}\n\n[已截断] UI 规格共 {total} 字符，只返回了前 60000——**上面的 JSON 是半截的**。把 max_nodes 调小（如 60）重取，或者先只还原首屏。"
            )));
        }
        Ok(Some(val))
    })
    .await
}

/// Run JavaScript in the page and return its (stringified) result.
#[tauri::command]
pub async fn browser_eval(script: String) -> Result<BrowserState, String> {
    with_tab(move |tab| {
        let ro = tab.evaluate(&script, true).map_err(|e| e.to_string())?;
        let val = stringify_eval_result(ro.value, ro.description);
        // 8000 chars (~2.7k tokens worst case) so structured tools — network 抓包,
        // inspect 视觉解析, design — can return their full JSON without truncating
        // mid-string (which would hand the model invalid JSON).
        //
        // 注释的意图是「8000 够用所以不会截断」，但代码里没有任何判断——超了就直接砍，
        // 而且恰恰砍出注释自己担心的那种半截 JSON。调用方看到「整页节点清单」，相信
        // nodes[] 是全集；中等复杂度的页面上 JSON 早就超过 8000，后半截连同结构一起没了，
        // 而它无从察觉。截断本身可以接受，**不说**不行。
        let total = val.chars().count();
        if total > 8000 {
            let head: String = val.chars().take(8000).collect();
            return Ok(Some(format!(
                "{head}\n\n[已截断] 本次页面求值结果共 {total} 字符，只返回了前 8000——**上面的 JSON 很可能是半截的，不要当成完整结构**。缩小选择器范围或分批取。"
            )));
        }
        Ok(Some(val))
    })
    .await
}

fn metric_values(
    metrics: &[headless_chrome::protocol::cdp::Performance::Metric],
) -> std::collections::HashMap<&str, f64> {
    metrics
        .iter()
        .filter(|metric| metric.value.is_finite())
        .map(|metric| (metric.name.as_str(), metric.value))
        .collect()
}

fn non_negative_delta(
    before: &std::collections::HashMap<&str, f64>,
    after: &std::collections::HashMap<&str, f64>,
    name: &str,
) -> Option<f64> {
    let delta = after.get(name)? - before.get(name)?;
    (delta >= 0.0 && delta.is_finite()).then_some(delta)
}

/// Sample Chrome's CDP Performance domain over a bounded interval. `TaskDuration`
/// and `Timestamp` are cumulative seconds, so their delta gives real main-thread
/// busy time rather than an event-loop-delay approximation.
#[tauri::command]
pub async fn browser_performance_sample(sample_ms: Option<u64>) -> Result<BrowserState, String> {
    let sample_ms = sample_ms.unwrap_or(750).clamp(250, 5_000);
    with_tab(move |tab| {
        tab.call_method(EnablePerformanceMetrics { time_domain: None })
            .map_err(|e| format!("启用 Chrome 性能指标失败: {e}"))?;
        let first = tab
            .call_method(GetMetrics(None))
            .map_err(|e| format!("读取第一次 Chrome 性能指标失败: {e}"))?;
        std::thread::sleep(Duration::from_millis(sample_ms));
        let second = tab
            .call_method(GetMetrics(None))
            .map_err(|e| format!("读取第二次 Chrome 性能指标失败: {e}"))?;

        let before = metric_values(&first.metrics);
        let after = metric_values(&second.metrics);
        let timestamp_seconds = non_negative_delta(&before, &after, "Timestamp");
        let task_seconds = non_negative_delta(&before, &after, "TaskDuration");
        let busy_percent = match (task_seconds, timestamp_seconds) {
            (Some(task), Some(elapsed)) if elapsed > 0.0 => {
                Some((task / elapsed * 100.0).min(100.0))
            }
            _ => None,
        };
        let milliseconds =
            |name: &str| non_negative_delta(&before, &after, name).map(|seconds| seconds * 1_000.0);
        let count = |name: &str| non_negative_delta(&before, &after, name);
        let value = serde_json::json!({
            "sampleWindowMs": sample_ms,
            "cpu": {
                "source": "Chrome CDP Performance.getMetrics",
                "metric": "TaskDuration / Timestamp",
                "busyPercent": busy_percent,
                "elapsedMs": timestamp_seconds.map(|seconds| seconds * 1_000.0),
                "taskDurationMs": task_seconds.map(|seconds| seconds * 1_000.0),
                "scriptDurationMs": milliseconds("ScriptDuration"),
                "layoutDurationMs": milliseconds("LayoutDuration"),
                "recalcStyleDurationMs": milliseconds("RecalcStyleDuration"),
                "layoutCount": count("LayoutCount"),
                "recalcStyleCount": count("RecalcStyleCount"),
            }
        });
        Ok(Some(value.to_string()))
    })
    .await
}

/// Re-screenshot the current page (no action) — just look again.
#[tauri::command]
pub async fn browser_screenshot() -> Result<BrowserState, String> {
    with_tab(move |_tab| Ok(None)).await
}

fn validate_viewport(
    width: u32,
    height: u32,
    device_scale_factor: Option<f64>,
    mobile: Option<bool>,
) -> Result<(u32, u32, f64, bool), String> {
    if !(240..=7680).contains(&width) || !(240..=7680).contains(&height) {
        return Err("viewport 宽高必须在 240..=7680 像素之间".into());
    }
    let scale = device_scale_factor.unwrap_or(1.0);
    if !scale.is_finite() || !(0.5..=4.0).contains(&scale) {
        return Err("device_scale_factor 必须是 0.5..=4.0 的有限数值".into());
    }
    // CDP screenshots are rendered at device pixels. A legal CSS viewport such
    // as 7680x7680 at scale 4 needs nearly 1 GiB for one RGBA frame and can
    // freeze the entire desktop process while it is encoded and sent over IPC.
    const MAX_VIEWPORT_DEVICE_PIXELS: f64 = 24_000_000.0;
    let device_pixels = width as f64 * height as f64 * scale * scale;
    if device_pixels > MAX_VIEWPORT_DEVICE_PIXELS {
        return Err("viewport 与 device_scale_factor 的组合过大（最多 2400 万设备像素）".into());
    }
    Ok((width, height, scale, mobile.unwrap_or(false)))
}

/// Override the persistent browser tab's viewport through CDP Emulation. This
/// changes CSS media-query/layout metrics as well as the following screenshot,
/// enabling deterministic desktop/mobile checks without launching another browser.
#[tauri::command]
pub async fn browser_set_viewport(
    width: u32,
    height: u32,
    device_scale_factor: Option<f64>,
    mobile: Option<bool>,
) -> Result<BrowserState, String> {
    let (width, height, device_scale_factor, mobile) =
        validate_viewport(width, height, device_scale_factor, mobile)?;
    with_tab(move |tab| {
        tab.call_method(SetDeviceMetricsOverride {
            width,
            height,
            device_scale_factor,
            mobile,
            scale: None,
            screen_width: Some(width),
            screen_height: Some(height),
            position_x: Some(0),
            position_y: Some(0),
            dont_set_visible_size: None,
            screen_orientation: None,
            viewport: None,
            display_feature: None,
            device_posture: None,
        })
        .map_err(|e| format!("设置浏览器 viewport 失败: {e}"))?;
        std::thread::sleep(Duration::from_millis(150));
        Ok(None)
    })
    .await
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
pub async fn browser_wait(
    selector: Option<String>,
    ms: Option<u64>,
) -> Result<BrowserState, String> {
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

/// 自动化用哪个浏览器 + 要加载哪些扩展。
///
/// 做成命令而不是只认环境变量：面向用户的能力要能在界面里选，改一个开关不该等于
/// 改代码重新发版。`browser` 传 "" 表示自动选。返回本机实际装了哪些，界面据此列选项
/// ——不然用户只能对着一个下拉框猜哪个真的能用。
#[tauri::command]
pub fn browser_set_preference(
    browser: Option<String>,
    extensions: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    crate::capture::set_browser_pref(browser);
    if let Some(dirs) = extensions {
        set_extension_dirs(dirs);
    }
    Ok(browser_state_json())
}

/// 只读当前状态。和 setter 分开，因为 setter 传空就是「自动选」——拿它来读会把用户
/// 刚选的浏览器清掉，而这种 bug 只在「打开面板看一眼再关掉」时才出现，最难查。
#[tauri::command]
pub fn browser_get_preference() -> Result<serde_json::Value, String> {
    Ok(browser_state_json())
}

fn browser_state_json() -> serde_json::Value {
    let installed: Vec<serde_json::Value> = crate::capture::installed_browsers()
        .iter()
        .map(|(k, path)| serde_json::json!({ "id": k.id, "label": k.label, "path": path }))
        .collect();
    serde_json::json!({
        "installed": installed,
        "active": crate::capture::browser_pref(),
        "extensions": extension_dirs(),
        // Chrome 从 137 起彻底不理 --load-extension（实测 151 连坏 manifest 都不报错）。
        // 界面据此告诉用户「选了 Chrome 的话扩展这栏是白填的」，而不是让他自己撞。
        "extensionsWorkOn": ["edge", "brave", "chromium"],
    })
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
        let all = tab
            .get_cookies()
            .map_err(|e| format!("获取 cookies 失败: {e}"))?;
        let result = serde_json::to_string_pretty(&all).unwrap_or_else(|_| "[]".into());
        if let Some(ref d) = domain {
            let filtered: Vec<serde_json::Value> =
                serde_json::from_str::<Vec<serde_json::Value>>(&result)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|c| {
                        c.get("domain")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .contains(d.as_str())
                    })
                    .collect();
            Ok(Some(
                serde_json::to_string_pretty(&filtered).unwrap_or_else(|_| "[]".into()),
            ))
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
        let obj = if st == "session" {
            "sessionStorage"
        } else {
            "localStorage"
        };
        let js = format!(
            r#"(() => {{
            try {{
                const s = {obj};
                const out = {{}};
                for (let i = 0; i < s.length; i++) {{
                    const k = s.key(i);
                    out[k] = s.getItem(k);
                }}
                return JSON.stringify(out);
            }} catch(e) {{ return JSON.stringify({{ error: e.message }}); }}
        }})()"#
        );
        let ro = tab.evaluate(&js, false).map_err(|e| e.to_string())?;
        let s = ro
            .value
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
        let session = take_browser_session();
        if let Some(session) = session {
            // Let an in-flight CDP call finish before tearing down its Browser.
            // The state has already been cleared, so new close/reload calls do
            // not wait for Chrome's process tree to exit.
            let _operation = BROWSER_OPERATION
                .lock()
                .map_err(|_| "browser operation state poisoned")?;
            drop(session);
        }
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(())
}

/// Synchronous close — called from cleanup_stale on webview reload / app exit.
/// 退出 App 时**同步**收掉浏览器。
///
/// `close_all` 把 drop 丢进后台线程，为的是不让 Chrome 的进程树拖住 Tauri 的事件线程——
/// 那对 cleanup_stale（重载）是对的。退出时不行：进程马上就没了，那个后台线程根本来不及跑，
/// Chrome 于是成了孤儿留在机器上。终端、LSP、调试适配器、代理、MCP、自动化服务在退出那条
/// 分支里全都收了，唯独浏览器没有——而它是这堆里最重的一个。
///
/// 所以在当前线程上 drop，并给操作锁一个上限：一次卡住的 CDP 调用不该把「退出」变成
/// 退不出去。等不到就照样 drop——留一个跑着的 Chrome，比让用户按了退出没反应要好。
pub fn close_all_blocking(wait: std::time::Duration) {
    let Some(session) = take_browser_session() else {
        return;
    };
    let deadline = std::time::Instant::now() + wait;
    loop {
        match BROWSER_OPERATION.try_lock() {
            // 拿到锁：在锁的保护下 drop，让在途的 CDP 调用先收尾。
            Ok(_operation) => {
                drop(session);
                return;
            }
            // 锁中毒说明持有者 panic 过，等下去没有意义。
            Err(std::sync::TryLockError::Poisoned(_)) => {
                drop(session);
                return;
            }
            Err(std::sync::TryLockError::WouldBlock) => {
                if std::time::Instant::now() >= deadline {
                    drop(session);
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
    }
}

pub fn close_all() {
    let session = take_browser_session();
    if let Some(session) = session {
        // Dropping a headless Chrome session can wait for its process tree.
        // cleanup_stale calls this from the Tauri event thread, so take state
        // synchronously then wait for any current CDP operation in the background.
        std::thread::spawn(move || {
            let _operation = BROWSER_OPERATION.lock();
            drop(session);
        });
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
                crate::elog!(
                    "[browser] killed {} orphaned Chrome process(es)",
                    pids.len()
                );
            }
        }
        #[cfg(windows)]
        {
            // 按镜像名清理，所以每个可能被选中的浏览器都要单独来一遍——以前只写了
            // chrome.exe，于是自动化跑 Edge 或 Brave 时残留进程一个都清不掉，它们
            // 继续占着 profile 目录，下一次启动就全线失败。
            for image in ["chrome.exe", "msedge.exe", "brave.exe", "chromium.exe"] {
                let _ = std::process::Command::new("taskkill")
                    .args([
                        "/F",
                        "/FI",
                        &format!("IMAGENAME eq {image}"),
                        "/FI",
                        "WINDOWTITLE eq rust-headless*",
                    ])
                    .output();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // 一篇长文，用来喂「正文足够长 → 弱信号不算数」那条规则。
    const ARTICLE: &str = "x的正文很长，足够长到不像一张过渡页。这一段会被重复很多遍以越过阈值。";

    fn long_body() -> String {
        ARTICLE.repeat(40)
    }

    #[test]
    fn 挡在验证墙上要认出来_不管它带不带控件() {
        // 只用来提供挑战的端点：单凭 URL 就该判定，正文再长也一样。
        assert!(detect_wall_static("Google", "https://www.google.com/sorry/index?continue=x", "").is_some());
        assert!(detect_wall_static("", "https://例子.com/cdn-cgi/challenge-platform/h/b/x", "").is_some());
        // 正常页面不会顶着这些标题。
        assert!(detect_wall_static("Just a moment...", "https://例子.com/a", "").is_some());
        assert!(detect_wall_static("Attention Required! | Cloudflare", "https://例子.com/a", "").is_some());
    }

    #[test]
    fn 一篇讲验证码的文章不能被当成验证墙() {
        // 这是这个函数最容易犯、代价也最高的错：误判会让用户跑去一个根本没有验证的页面
        // 上「点一下」。所以凡是正常内容里也会出现的词，都必须再满足「页面几乎没正文」。
        let body = long_body();
        assert_eq!(detect_wall_static("reCAPTCHA 是怎么工作的", "https://blog.example.com/how-captcha-works", &body), None);
        assert_eq!(detect_wall_static("如何应对 access denied 报错", "https://example.com/faq", &body), None);
        assert_eq!(detect_wall_static("聊聊 unusual traffic 告警", "https://example.com/ops", &body), None);
        // 普通页面完全不该触发。
        assert_eq!(detect_wall_static("首页", "https://example.com/", &body), None);
    }

    #[test]
    fn 弱信号加上几乎空白的正文_才算撞墙() {
        // 过渡页的共同点是「没有内容」——弱信号只在这种页面上才不再有歧义。
        assert!(detect_wall_static("", "https://example.com/captcha", "请完成验证").is_some());
        assert!(detect_wall_static(
            "",
            "https://example.com/x",
            "Our systems have detected unusual traffic from your computer network."
        )
        .is_some());
        // 同样的词，配上真实正文，就只是词而已。
        assert_eq!(
            detect_wall_static("", "https://example.com/x", &format!("unusual traffic {}", long_body())),
            None
        );
    }

    #[test]
    fn 每个浏览器一份独立配置_chrome_保留原路径() {
        // 共用一个目录会把 profile 写坏（Chrome 和 Edge 的格式不互通），
        // 而给 chrome 也加后缀则会让已经登录过的人平白丢一次登录态。
        assert_eq!(profile_dir_name("chrome"), "browser-profile");
        assert_eq!(profile_dir_name("edge"), "browser-profile-edge");
        assert_eq!(profile_dir_name("brave"), "browser-profile-brave");
        let all: Vec<String> = ["chrome", "edge", "brave", "chromium"]
            .iter()
            .map(|k| profile_dir_name(k))
            .collect();
        let uniq: std::collections::HashSet<_> = all.iter().collect();
        assert_eq!(uniq.len(), all.len(), "两个浏览器指向了同一个配置目录");
    }

    #[test]
    fn 接管到谁就说谁_认不出来也不许默认说成_chrome() {
        assert_eq!(brand_from_version(Some("Edg/126.0.2592.87")), "Microsoft Edge");
        assert_eq!(brand_from_version(Some("Chrome/151.0.7922.138")), "Google Chrome");
        assert_eq!(brand_from_version(Some("HeadlessChrome/151.0")), "Google Chrome");
        assert_eq!(brand_from_version(Some("Brave/1.68")), "Brave");
        // 这条是重点：说错牌子比不说更糟——它是用户唯一能核对「接管的是不是我那个
        // 窗口」的信息。
        assert_eq!(brand_from_version(None), "浏览器");
        assert_eq!(brand_from_version(Some("")), "浏览器");
    }

    #[test]
    fn 摘默认参数只摘该摘的两个_绝不摘自动化标志() {
        // `--enable-automation` 和 `--disable-extensions` 同在这个库的默认参数里。
        // 摘掉前者就是隐藏自动化身份，也就是反检测——这个产品不做，这条钉死它。
        let src = include_str!("browser.rs");
        assert!(
            src.contains("OsStr::new(\"--disable-extensions\")"),
            "扩展要生效必须摘掉 --disable-extensions"
        );
        assert!(
            !src.contains("OsStr::new(\"--enable-automation\")"),
            "不许把 --enable-automation 摘掉：那是反检测"
        );
    }

    #[test]
    fn 命令行上只能有一条_disable_features() {
        // 以前两条一起传，靠「Chromium 对重复开关只认最后一次」让我们这条覆盖默认那条，
        // 于是默认那份的值被静默丢掉。现在默认那条被显式摘掉，我们这条必须把它的值
        // 并进来——否则这个"合并"是假的，只是把丢弃换了个写法。
        let src = include_str!("browser.rs");
        let ours = src
            .lines()
            .find(|l| l.contains("\"--disable-features=Translate,"))
            .expect("找不到我们那条 --disable-features");
        for inherited in ["TranslateUI", "BlinkGenPropertyTrees"] {
            assert!(ours.contains(inherited), "默认参数里的 {inherited} 没并进来");
        }
        assert!(
            src.contains("OsStr::new(\"--disable-features=TranslateUI,BlinkGenPropertyTrees\")"),
            "默认那条 --disable-features 必须被显式摘掉，不能靠拼接顺序覆盖"
        );
    }

    #[test]
    fn 会话说明只发一次_不是每个动作都重复() {
        // 「为什么 Dock 里多了一个 Chrome」只需要在浏览器起来的那一次说清楚。
        // 每个动作都带一遍的话，模型上下文会被同一段话反复占满。
        *SESSION_NOTE.lock().unwrap() = None;
        set_session_note("另起了一个自动化浏览器".into());
        let first = SESSION_NOTE.lock().unwrap().take();
        assert_eq!(first.as_deref(), Some("另起了一个自动化浏览器"));
        let second = SESSION_NOTE.lock().unwrap().take();
        assert_eq!(second, None, "同一条说明不该被重复投递");
    }

    #[test]
    fn viewport_validation_applies_defaults_and_preserves_mobile() {
        assert_eq!(
            validate_viewport(390, 844, None, Some(true)).unwrap(),
            (390, 844, 1.0, true)
        );
        assert_eq!(
            validate_viewport(1440, 900, Some(2.0), None).unwrap(),
            (1440, 900, 2.0, false)
        );
    }

    #[test]
    fn viewport_validation_rejects_unsafe_metrics() {
        assert!(validate_viewport(0, 900, None, None).is_err());
        assert!(validate_viewport(390, 9000, None, None).is_err());
        assert!(validate_viewport(390, 844, Some(f64::NAN), None).is_err());
        assert!(validate_viewport(390, 844, Some(4.1), None).is_err());
        assert!(validate_viewport(7680, 7680, Some(4.0), None).is_err());
    }

    #[test]
    fn page_observer_is_bounded_and_does_not_capture_payloads() {
        assert!(PAGE_OBSERVER_SCRIPT.contains("MAX_EVENTS = 80"));
        assert!(PAGE_OBSERVER_SCRIPT.contains("__MERR__"));
        assert!(PAGE_OBSERVER_SCRIPT.contains("__MNET__"));
        assert!(!PAGE_OBSERVER_SCRIPT.contains("reqHeaders"));
        assert!(!PAGE_OBSERVER_SCRIPT.contains("reqBody"));
        assert!(!PAGE_OBSERVER_SCRIPT.contains("responseText"));
        assert!(!PAGE_OBSERVER_SCRIPT.contains("setRequestHeader"));
    }

    #[test]
    fn eval_result_does_not_double_encode_json_strings() {
        let json = r#"{"healthy":true,"errors":[]}"#;
        assert_eq!(
            stringify_eval_result(Some(serde_json::Value::String(json.into())), None),
            json
        );
        assert_eq!(
            stringify_eval_result(Some(serde_json::json!({"healthy": true})), None),
            r#"{"healthy":true}"#
        );
    }

    #[test]
    fn navigation_wait_timeout_is_recoverable() {
        assert!(is_navigation_wait_timeout(
            "The event waited for never came"
        ));
        assert!(is_navigation_wait_timeout(
            "navigation timeout after 30000 ms"
        ));
        assert!(!is_navigation_wait_timeout("DNS resolution failed"));
    }

    #[test]
    fn navigation_url_preserves_preview_targets_and_normalizes_paths() {
        assert_eq!(
            normalize_navigation_url(" file:///tmp/site/index.html "),
            "file:///tmp/site/index.html"
        );
        assert_eq!(
            normalize_navigation_url("/tmp/site/index.html"),
            "file:///tmp/site/index.html"
        );
        assert_eq!(
            normalize_navigation_url(r"C:\site\index.html"),
            "file:///C:/site/index.html"
        );
        assert_eq!(
            normalize_navigation_url("data:text/html,<h1>Preview</h1>"),
            "data:text/html,<h1>Preview</h1>"
        );
        assert_eq!(
            normalize_navigation_url("localhost:5174"),
            "https://localhost:5174"
        );
    }

    #[test]
    fn cdp_metric_delta_rejects_missing_and_regressing_counters() {
        let before =
            std::collections::HashMap::from([("Timestamp", 100.0), ("TaskDuration", 42.0)]);
        let after =
            std::collections::HashMap::from([("Timestamp", 100.75), ("TaskDuration", 42.15)]);
        assert!(
            (non_negative_delta(&before, &after, "Timestamp").unwrap() - 0.75).abs() < f64::EPSILON
        );
        assert!((non_negative_delta(&before, &after, "TaskDuration").unwrap() - 0.15).abs() < 1e-9);
        assert_eq!(non_negative_delta(&before, &after, "LayoutDuration"), None);

        let regressed = std::collections::HashMap::from([("TaskDuration", 41.9)]);
        assert_eq!(
            non_negative_delta(&before, &regressed, "TaskDuration"),
            None
        );
    }

    #[test]
    fn close_invalidates_a_concurrent_browser_launch_generation() {
        let store = Arc::new(Mutex::new(BrowserStore::default()));
        let launch_generation = store.lock().unwrap().generation;
        let closer = Arc::clone(&store);

        std::thread::spawn(move || {
            closer.lock().unwrap().invalidate();
        })
        .join()
        .unwrap();

        assert!(!store.lock().unwrap().launch_is_current(launch_generation));
    }
}
