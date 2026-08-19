//! Headless-browser screenshot capture, so the AI agent can SEE the web UI it
//! builds (a real visual feedback loop) instead of editing blind. Uses whatever
//! Chromium/Chrome/Edge is installed; degrades with a clear message when none is.

use std::path::Path;
use std::time::{Duration, Instant};

/// One Chromium-family browser the automation can drive.
///
/// 这里是「有哪些浏览器可选」的**唯一**声明处。以前只有一串写死的路径按固定优先级
/// 取第一个装了的，用户没有选择权：装了 Chrome 就只能用 Chrome，哪怕他更想让自动化
/// 去用 Edge、把自己的 Chrome 留给自己——而那恰恰能让「Dock 里两个一模一样的图标」
/// 这件事从根上消失。
pub struct BrowserKind {
    /// 配置里写的名字（`MICHAEL_BROWSER=edge`）。
    pub id: &'static str,
    /// 给人看的名字，出现在提示和报错里。
    pub label: &'static str,
    /// 进程名，用来判断用户此刻是不是正开着它。以前这个判断写死成 "Google Chrome"，
    /// 一旦选了别的浏览器就永远判错——profile 锁的判断也就跟着错。
    pub process_name: &'static str,
    /// macOS 的 bundle id，用来认出系统默认浏览器（LaunchServices 记的就是这个）。
    pub bundle_id: &'static str,
    /// Windows 注册表里 https 的 ProgId 前缀（真实值形如 `ChromeHTML.5G4...`，只比前缀）。
    pub win_progid: &'static str,
    /// Linux `xdg-settings get default-web-browser` 返回的 .desktop 文件名（不含后缀）。
    pub linux_desktop: &'static str,
}

/// 系统默认浏览器，转成本名录里的 id。
///
/// 为什么要有它：`resolve_browser(None)` 以前直接取「装了的里面的第一个」，而那个顺序是
/// BROWSER_KINDS 的书写顺序（chrome 排头）。于是一个日常用 Edge 的用户，自动化照样去开
/// Chrome —— 用户从来没选过 Chrome，是这份名单替他选的。系统默认浏览器是**用户已经表达
/// 过的选择**，没有理由不听。
///
/// 认不出、或者默认的是 Safari / Firefox（都不能用 CDP 驱动）时返回 None，
/// 由调用方回落到原来的顺序 —— 这一层只增加信息，不制造新的失败。
pub fn system_default_browser() -> Option<&'static str> {
    let ident = default_browser_identifier()?.to_ascii_lowercase();
    #[cfg(target_os = "macos")]
    let matches = |k: &BrowserKind| ident == k.bundle_id;
    #[cfg(target_os = "windows")]
    let matches = |k: &BrowserKind| ident.starts_with(&k.win_progid.to_ascii_lowercase());
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let matches = |k: &BrowserKind| {
        ident == k.linux_desktop || ident == format!("{}.desktop", k.linux_desktop)
    };
    BROWSER_KINDS.iter().find(|k| matches(k)).map(|k| k.id)
}

/// 系统层面记的「https 用什么打开」。各平台记法不同，这里只负责把那个原始标识取出来。
fn default_browser_identifier() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        // LaunchServices 的偏好是二进制 plist，交给系统自带的 plutil 转 JSON 再解析，
        // 比自己解二进制格式稳。找 URLScheme == https 的那条。
        let home = std::env::var_os("HOME")?;
        let path = Path::new(&home)
            .join("Library/Preferences/com.apple.LaunchServices/com.apple.launchservices.secure.plist");
        let out = crate::process_util::command("plutil")
            .args(["-convert", "json", "-o", "-", "--"])
            .arg(&path)
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
        let handlers = v.get("LSHandlers")?.as_array()?;
        handlers
            .iter()
            .find(|h| h.get("LSHandlerURLScheme").and_then(|s| s.as_str()) == Some("https"))
            .and_then(|h| h.get("LSHandlerRoleAll").and_then(|s| s.as_str()))
            .map(|s| s.to_string())
    }
    #[cfg(target_os = "windows")]
    {
        let out = crate::process_util::command("reg")
            .args([
                "query",
                r"HKCU\Software\Microsoft\Windows\Shell\Associations\UrlAssociations\https\UserChoice",
                "/v",
                "ProgId",
            ])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        text.lines()
            .find(|l| l.trim_start().starts_with("ProgId"))
            .and_then(|l| l.split_whitespace().last())
            .map(|s| s.to_string())
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let out = crate::process_util::command("xdg-settings")
            .args(["get", "default-web-browser"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (!s.is_empty()).then_some(s)
    }
}

/// 支持的浏览器，同时也是「没指定时」的尝试顺序。
pub const BROWSER_KINDS: &[BrowserKind] = &[
    BrowserKind { id: "chrome", label: "Google Chrome", process_name: "Google Chrome",
        bundle_id: "com.google.chrome", win_progid: "ChromeHTML", linux_desktop: "google-chrome" },
    BrowserKind { id: "edge", label: "Microsoft Edge", process_name: "Microsoft Edge",
        bundle_id: "com.microsoft.edgemac", win_progid: "MSEdgeHTM", linux_desktop: "microsoft-edge" },
    BrowserKind { id: "brave", label: "Brave Browser", process_name: "Brave Browser",
        bundle_id: "com.brave.browser", win_progid: "BraveHTML", linux_desktop: "brave-browser" },
    BrowserKind { id: "chromium", label: "Chromium", process_name: "Chromium",
        bundle_id: "org.chromium.chromium", win_progid: "ChromiumHTM", linux_desktop: "chromium" },
];

/// 某个浏览器在本机可能的可执行文件位置。
fn candidate_paths(id: &str) -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        let app = match id {
            "chrome" => "Google Chrome.app/Contents/MacOS/Google Chrome",
            "edge" => "Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "brave" => "Brave Browser.app/Contents/MacOS/Brave Browser",
            "chromium" => "Chromium.app/Contents/MacOS/Chromium",
            _ => return Vec::new(),
        };
        // 系统级和用户级都找：装到 ~/Applications 的浏览器以前一律找不到。
        let mut out = vec![format!("/Applications/{app}")];
        if let Some(home) = std::env::var_os("HOME") {
            out.push(format!("{}/Applications/{app}", Path::new(&home).display()));
        }
        out
    }
    #[cfg(target_os = "windows")]
    {
        match id {
            "chrome" => vec![
                r"C:\Program Files\Google\Chrome\Application\chrome.exe".into(),
                r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe".into(),
            ],
            "edge" => vec![
                r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe".into(),
                r"C:\Program Files\Microsoft\Edge\Application\msedge.exe".into(),
            ],
            "brave" => vec![
                r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe".into(),
            ],
            "chromium" => Vec::new(),
            _ => Vec::new(),
        }
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        match id {
            "chrome" => vec![
                "google-chrome".into(),
                "google-chrome-stable".into(),
                "chrome".into(),
            ],
            "edge" => vec!["microsoft-edge".into(), "microsoft-edge-stable".into()],
            "brave" => vec!["brave-browser".into(), "brave".into()],
            "chromium" => vec!["chromium".into(), "chromium-browser".into()],
            _ => Vec::new(),
        }
    }
}

/// 把一个候选项解析成真实存在的可执行文件路径。
fn existing_executable(c: &str) -> Option<String> {
    if c.contains('/') || c.contains('\\') {
        return Path::new(c).exists().then(|| c.to_string());
    }
    #[cfg(not(windows))]
    {
        let resolved = crate::process_util::resolve_command(c, None);
        if resolved.contains('/') && Path::new(&resolved).exists() {
            return Some(resolved);
        }
    }
    None
}

/// 本机装了哪些受支持的浏览器。
pub fn installed_browsers() -> Vec<(&'static BrowserKind, String)> {
    BROWSER_KINDS
        .iter()
        .filter_map(|k| {
            candidate_paths(k.id)
                .iter()
                .find_map(|c| existing_executable(c))
                .map(|p| (k, p))
        })
        .collect()
}

/// 解析出这次该用哪个浏览器。
///
/// `pref` 是用户的选择（配置或 `MICHAEL_BROWSER`）。选了但没装时**不静默改用别的**：
/// 返回值第三项带上没找到的那个名字，让调用方如实告诉用户——默默换一个正是这套
/// 代码以前最让人困惑的地方（同一句话，装没装 Chrome 行为完全不同，用户无从预期）。
pub fn resolve_browser(pref: Option<&str>) -> Option<(&'static BrowserKind, String, Option<String>)> {
    pick_browser(&installed_browsers(), pref, system_default_browser())
}

/// 选择本身，抽成纯函数：三个输入全从参数来，一个也不去问环境。
///
/// 不抽的话这条规则测不动——本机的系统默认浏览器恰好就是名单里的第一个，
/// 于是「听系统默认」和「取名单第一个」在这台机器上永远同一个答案，断言怎么写都绿。
/// 守不住东西的测试比没有更糟：它让人以为守住了。
fn pick_browser<'a>(
    installed: &[(&'a BrowserKind, String)],
    pref: Option<&str>,
    system_default: Option<&str>,
) -> Option<(&'a BrowserKind, String, Option<String>)> {
    let want = pref.map(|p| p.trim().to_ascii_lowercase()).filter(|p| !p.is_empty());
    if let Some(ref want) = want {
        if let Some((k, p)) = installed.iter().find(|(k, _)| k.id == want) {
            return Some((k, p.clone(), None));
        }
        // 选了但没装：**不静默改用别的**，把没找到的那个名字带回去让调用方如实说。
        let (k, p) = installed.first()?;
        return Some((k, p.clone(), Some(want.clone())));
    }
    // 没有显式选择时，先问系统默认浏览器。
    //
    // 这一层以前不存在，取的是 BROWSER_KINDS 的**书写顺序**（chrome 排头）。于是一个日常
    // 用 Edge 的人，自动化照样去开 Chrome——他从来没选过 Chrome，是这份名单替他选的。
    // 系统默认是用户已经表达过的选择，优先级理应高于一份写死的顺序、低于他在设置里的
    // 显式指定。认不出、或默认的是 Safari / Firefox（都驱动不了 CDP）时上游给 None，
    // 照旧回落——这一层只增加信息，不制造新的失败路径。
    if let Some(def) = system_default {
        if let Some((k, p)) = installed.iter().find(|(k, _)| k.id == def) {
            return Some((k, p.clone(), None));
        }
    }
    installed.first().map(|(k, p)| (*k, p.clone(), None))
}

/// 运行期设定的浏览器偏好（来自设置界面）。`None` = 没设过，回落到环境变量。
static BROWSER_PREF: std::sync::LazyLock<std::sync::Mutex<Option<String>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

/// 设定/清除自动化要用哪个浏览器。传空字符串表示「自动选」。
pub fn set_browser_pref(id: Option<String>) {
    let cleaned = id
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty() && BROWSER_KINDS.iter().any(|k| k.id == s));
    if let Ok(mut slot) = BROWSER_PREF.lock() {
        *slot = cleaned;
    }
}

/// 当前生效的浏览器偏好：设置界面优先，其次环境变量，都没有就自动选。
pub fn browser_pref() -> Option<String> {
    if let Ok(slot) = BROWSER_PREF.lock() {
        if let Some(ref id) = *slot {
            return Some(id.clone());
        }
    }
    std::env::var("MICHAEL_BROWSER")
        .ok()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
}

/// First installed headless browser, or `None`. 尊重用户选的浏览器。
pub fn find_headless_browser() -> Option<String> {
    resolve_browser(browser_pref().as_deref()).map(|(_, path, _)| path)
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
    let after = url.split_once("://").map(|x| x.1).unwrap_or(url);
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
    run_capture_t(browser, url, out, w, h, 8000)
}

/// Capture with a custom virtual-time budget (ms). A larger budget lets the page
/// run longer before the shot — so a sequence of increasing budgets samples an
/// animation at successive points in time (the basis of the filmstrip capture).
fn run_capture_t(
    browser: &str,
    url: &str,
    out: &str,
    w: u32,
    h: u32,
    budget_ms: u32,
) -> Result<(), String> {
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
        format!("--virtual-time-budget={budget_ms}"),
    ];
    if host_is_local(url) {
        // Intranet / localhost must NOT go through a system/corporate proxy, or
        // Chrome can't reach the user's own dev server.
        args.push("--no-proxy-server".into());
    }
    args.push(url.into());

    let mut child = crate::process_util::command(browser)
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
    // Accept file:// for local html, and map bare local paths → file:// (so "看本地 html" works and
    // never gets mangled to https://file://…). Schemeless hosts default to http.
    if url.len() >= 2 && url.as_bytes()[1] == b':' && url.as_bytes()[0].is_ascii_alphabetic() {
        url = format!("file:///{}", url.replace('\\', "/")); // Windows drive path (D:\… / D:/…)
    } else if url.starts_with('/') {
        url = format!("file://{url}"); // unix absolute path
    } else if !url.contains("://") {
        url = format!("http://{url}");
    }
    if !(url.starts_with("http://") || url.starts_with("https://") || url.starts_with("file://")) {
        return Err("只支持 http/https/file URL".into());
    }
    let browser = find_headless_browser().ok_or_else(|| {
        "未找到无头浏览器。装上 Chrome / Edge / Brave / Chromium 任一款后，智能体就能“看见”页面并据图改进 UI（截图依赖它渲染）。".to_string()
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

/// Capture an ANIMATION as a vertical filmstrip: take `frames` screenshots at
/// successive points in time (increasing virtual-time budgets across `duration_ms`)
/// and stack them top→bottom into ONE labelled image, so the model can SEE motion —
/// a single screenshot can't show an animation. The agent's "eyes" for animation work.
#[tauri::command]
pub async fn capture_url_frames(
    url: String,
    width: Option<u32>,
    height: Option<u32>,
    frames: Option<u32>,
    duration_ms: Option<u32>,
) -> Result<String, String> {
    let mut url = url.trim().to_string();
    // Accept file:// for local html, and map bare local paths → file:// (so "看本地 html" works and
    // never gets mangled to https://file://…). Schemeless hosts default to http.
    if url.len() >= 2 && url.as_bytes()[1] == b':' && url.as_bytes()[0].is_ascii_alphabetic() {
        url = format!("file:///{}", url.replace('\\', "/")); // Windows drive path (D:\… / D:/…)
    } else if url.starts_with('/') {
        url = format!("file://{url}"); // unix absolute path
    } else if !url.contains("://") {
        url = format!("http://{url}");
    }
    if !(url.starts_with("http://") || url.starts_with("https://") || url.starts_with("file://")) {
        return Err("只支持 http/https/file URL".into());
    }
    let browser = find_headless_browser().ok_or_else(|| {
        "未找到无头浏览器。装上 Chrome / Edge / Brave / Chromium 任一款后才能截图。".to_string()
    })?;
    let mut w = width.unwrap_or(1280).clamp(320, 3840);
    let mut h = height.unwrap_or(800).clamp(240, 4000);
    // The filmstrip retains every decoded frame before composing it. Keep each
    // frame to a bounded pixel budget so a max-size five-frame request cannot
    // turn into several hundred megabytes of live image buffers.
    const MAX_FRAME_PIXELS: u64 = 4_000_000;
    let pixels = u64::from(w) * u64::from(h);
    if pixels > MAX_FRAME_PIXELS {
        let ratio = (MAX_FRAME_PIXELS as f64 / pixels as f64).sqrt();
        w = ((w as f64 * ratio).floor() as u32).max(1);
        h = ((h as f64 * ratio).floor() as u32).max(1);
    }
    let n = frames.unwrap_or(4).clamp(2, 5);
    let total = duration_ms.unwrap_or(2400).clamp(400, 8000);

    // Capture each frame at budget = total * (i+1)/n (so frames sample 1/n..1 of the timeline).
    let mut imgs: Vec<image::DynamicImage> = Vec::new();
    for i in 0..n {
        let budget = (total as u64 * (i as u64 + 1) / n as u64).max(200) as u32;
        let out =
            std::env::temp_dir().join(format!("michael_ide_frame_{}.png", uuid::Uuid::new_v4()));
        let out_str = out.to_string_lossy().to_string();
        let (b, u2, o2) = (browser.clone(), url.clone(), out_str.clone());
        tauri::async_runtime::spawn_blocking(move || run_capture_t(&b, &u2, &o2, w, h, budget))
            .await
            .map_err(|e| e.to_string())??;
        if let Ok(bytes) = std::fs::read(&out) {
            let _ = std::fs::remove_file(&out);
            if !bytes.is_empty() {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    imgs.push(img);
                }
            }
        }
    }
    if imgs.is_empty() {
        return Err("逐帧截图为空（确认服务已起、URL 正确）".into());
    }

    // Stack vertically with a 6px dark separator between frames.
    let fw = imgs.iter().map(|i| i.width()).max().unwrap_or(w);
    let sep: u32 = 6;
    let total_h: u32 = imgs.iter().map(|i| i.height()).sum::<u32>() + sep * (imgs.len() as u32 - 1);
    let mut canvas = image::RgbImage::from_pixel(fw, total_h.max(1), image::Rgb([24, 24, 28]));
    let mut y: i64 = 0;
    for img in &imgs {
        let rgb = img.to_rgb8();
        image::imageops::overlay(&mut canvas, &rgb, 0, y);
        y += img.height() as i64 + sep as i64;
    }
    jpeg_data_url(image::DynamicImage::ImageRgb8(canvas), 900, 70)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 浏览器目录里不能有重名或空字段() {
        // 这张表是「有哪些浏览器可选」的唯一声明处，配置项、进程名判断、profile 目录名
        // 全从它派生，重名会让后面每一处都跟着错。
        let mut ids: Vec<&str> = BROWSER_KINDS.iter().map(|k| k.id).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before, "浏览器 id 重名了");
        for k in BROWSER_KINDS {
            assert!(!k.id.is_empty() && !k.label.is_empty() && !k.process_name.is_empty());
            assert_eq!(k.id, k.id.to_ascii_lowercase(), "id 必须小写，配置里是按小写比对的");
            assert!(!candidate_paths(k.id).is_empty(), "{} 一个候选路径都没有", k.id);
        }
        // 不认识的名字不该凭空造出候选路径。
        assert!(candidate_paths("netscape").is_empty());
    }

    #[test]
    fn 选了没装的浏览器要如实说_不能默默换一个() {
        // 默默换一个正是这套代码以前最让人困惑的地方：同一句话，装没装某个浏览器
        // 行为完全不同，用户无从预期，也无从排查。
        let installed = installed_browsers();
        if installed.is_empty() {
            // 一个 Chromium 内核浏览器都没有的机器上，这个函数必须老实返回 None。
            assert!(resolve_browser(None).is_none());
            assert!(resolve_browser(Some("chrome")).is_none());
            return;
        }
        let (first, _) = installed[0];
        // 没指定 → 用装了的第一个，且不报「没找到你要的那个」。
        let (k, _, missing) = resolve_browser(None).unwrap();
        assert_eq!(k.id, first.id);
        assert_eq!(missing, None);
        // 指定了装了的那个 → 就用它。
        let (k, _, missing) = resolve_browser(Some(first.id)).unwrap();
        assert_eq!(k.id, first.id);
        assert_eq!(missing, None);
        // 指定了一个根本不存在的名字 → 仍然给出一个能用的，但**必须**带回没找到的那个。
        let (_, _, missing) = resolve_browser(Some("netscape")).unwrap();
        assert_eq!(missing.as_deref(), Some("netscape"));
        // 大小写和空格不该影响匹配。
        let (k, _, missing) = resolve_browser(Some(&format!("  {}  ", first.id.to_uppercase()))).unwrap();
        assert_eq!(k.id, first.id);
        assert_eq!(missing, None);
    }

    /// 系统默认浏览器必须真的参与「没显式选择时用哪个」这个决定。
    ///
    /// 以前这一层不存在：取的是 BROWSER_KINDS 的书写顺序（chrome 排头），于是日常用 Edge
    /// 的人，自动化照样去开 Chrome。用户的原话是「喜欢一直用自己的调试浏览器，而不是选择
    /// 用户默认的浏览器」。
    #[test]
    fn 没显式选择时要听系统默认浏览器() {
        // 名录里每一项都得能被系统标识认出来，否则这条链在那个浏览器上是断的。
        for k in BROWSER_KINDS {
            assert!(!k.bundle_id.is_empty(), "{} 缺 macOS bundle id", k.id);
            assert!(!k.win_progid.is_empty(), "{} 缺 Windows ProgId", k.id);
            assert!(!k.linux_desktop.is_empty(), "{} 缺 Linux desktop 名", k.id);
            assert_eq!(k.bundle_id, k.bundle_id.to_ascii_lowercase(), "bundle id 按小写比对");
        }
        // 认出来的必须是名录里的 id，不能是随便一个字符串。
        if let Some(id) = system_default_browser() {
            assert!(BROWSER_KINDS.iter().any(|k| k.id == id), "认出了名录外的 id：{id}");
        }
        // 下面用合成输入测决策本身。**不能用本机的真实情况**：这台机器的系统默认浏览器
        // 恰好就是名单里的第一个，「听系统默认」和「取第一个」答案永远相同，断言怎么写都绿。
        let kind = |id: &str| BROWSER_KINDS.iter().find(|k| k.id == id).unwrap();
        // 装了 chrome 和 edge（chrome 在名单里排头），系统默认是 edge。
        let installed = vec![
            (kind("chrome"), "/chrome".to_string()),
            (kind("edge"), "/edge".to_string()),
        ];

        // ① 没显式选择 → 听系统默认，而不是名单顺序。这条是本次修复的全部要点。
        let (k, _, missing) = pick_browser(&installed, None, Some("edge")).unwrap();
        assert_eq!(k.id, "edge", "无偏好时必须听系统默认，不能回到名单顺序里的 chrome");
        assert_eq!(missing, None);

        // ② 显式选择压过系统默认——这一层只填「没选」的空，不许覆盖用户自己的选择。
        let (k, _, _) = pick_browser(&installed, Some("chrome"), Some("edge")).unwrap();
        assert_eq!(k.id, "chrome", "用户明确选了 chrome，系统默认不该把它顶掉");

        // ③ 系统默认是 Safari/Firefox 这类驱动不了 CDP 的（上游给 None）→ 回落名单顺序。
        let (k, _, _) = pick_browser(&installed, None, None).unwrap();
        assert_eq!(k.id, "chrome", "认不出默认浏览器时照旧回落，不许因此失败");

        // ④ 系统默认那个没装 → 同样回落，不能选出一个装都没装的。
        let (k, _, _) = pick_browser(&installed, None, Some("brave")).unwrap();
        assert_eq!(k.id, "chrome");

        // ⑤ 选了没装的 → 如实带回没找到的名字（既有约定，别被这次改动弄丢）。
        let (k, _, missing) = pick_browser(&installed, Some("netscape"), Some("edge")).unwrap();
        assert_eq!(k.id, "chrome", "回落取名单第一个");
        assert_eq!(missing.as_deref(), Some("netscape"), "必须如实说没找到，不能默默换一个");

        // ⑥ 一个都没装 → None，别硬编一个路径出来。
        assert!(pick_browser(&[], None, Some("chrome")).is_none());
    }

    #[test]
    fn 偏好设置只接受名录里的名字() {
        set_browser_pref(Some("edge".into()));
        assert_eq!(browser_pref().as_deref(), Some("edge"));
        // 空字符串 = 自动选，不是一个叫 "" 的浏览器。
        set_browser_pref(Some("   ".into()));
        assert!(browser_pref().is_none() || std::env::var("MICHAEL_BROWSER").is_ok());
        // 名录外的名字直接丢掉，否则会一路带到启动时才炸。
        set_browser_pref(Some("netscape".into()));
        assert!(browser_pref().is_none() || std::env::var("MICHAEL_BROWSER").is_ok());
        set_browser_pref(None);
    }
}
