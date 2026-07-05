//! System-wide MITM HTTP/HTTPS capture (真·小黄鸟 / HttpCanary-style), built on mitmproxy.
//!
//! We do NOT hand-roll TLS interception — that is exactly the part that must be bulletproof, so we
//! drive `mitmdump` (the mitmproxy CLI) as a subprocess with a tiny bridge addon that streams every
//! request+response (headers + bodies) as one JSON line per flow, prefixed with a marker. The Rust
//! side spawns it, filters the marker lines out of mitmdump's own logging, and forwards each flow to
//! the frontend over a Tauri Channel. The frontend renders the inspector + replay UI.

use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::process::{Child, Stdio};
use std::sync::Mutex;
use tauri::ipc::Channel;

const FLOW_MARKER: &str = "__MFLOW__";

/// Port of the currently-running capture proxy (if any). Read by browser.rs so the automation
/// browser routes THROUGH the capture proxy — that's what makes "抓包 + browser 走一遍" actually
/// work (otherwise the browser's traffic never reaches mitmproxy and capture_flows stays empty).
static CAPTURE_PROXY_PORT: once_cell::sync::Lazy<Mutex<Option<u16>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(None));
pub fn active_proxy_port() -> Option<u16> {
    CAPTURE_PROXY_PORT.lock().ok().and_then(|g| *g)
}
fn set_active_port(p: Option<u16>) {
    if let Ok(mut g) = CAPTURE_PROXY_PORT.lock() {
        *g = p;
    }
}

// mitmproxy addon: dump each completed flow (or errored flow) as one marker-prefixed JSON line.
const BRIDGE_ADDON: &str = r#"
import json, sys

def _dump(flow):
    try:
        req = flow.request
        resp = getattr(flow, "response", None)
        def hdrs(h):
            try:
                return {k: v for k, v in h.items()}
            except Exception:
                return {}
        def text(m, cap):
            try:
                t = m.get_text(strict=False)
                return (t or "")[:cap]
            except Exception:
                return ""
        rec = {
            "id": flow.id,
            "method": req.method,
            "scheme": req.scheme,
            "host": req.pretty_host,
            "path": req.path,
            "url": req.pretty_url,
            "reqHeaders": hdrs(req.headers),
            "reqBody": text(req, 20000),
            "status": (resp.status_code if resp else 0),
            "respHeaders": (hdrs(resp.headers) if resp else {}),
            "ctype": (resp.headers.get("content-type", "") if resp else ""),
            "respBody": (text(resp, 200000) if resp else ""),
            "size": (len(resp.raw_content) if (resp and resp.raw_content) else 0),
            "ms": (int(((resp.timestamp_end or 0) - (req.timestamp_start or 0)) * 1000)
                   if (resp and resp.timestamp_end and req.timestamp_start) else 0),
            "error": (str(flow.error) if getattr(flow, "error", None) else ""),
        }
        sys.stdout.write("__MFLOW__" + json.dumps(rec, ensure_ascii=False) + "\n")
        sys.stdout.flush()
    except Exception as e:
        sys.stderr.write("mflow-bridge-err: %r\n" % (e,))

def response(flow):
    _dump(flow)

def error(flow):
    _dump(flow)
"#;

struct Running {
    child: Child,
    port: u16,
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        set_active_port(None); // capture stopped → browser stops routing through it
    }
}

#[derive(Default)]
pub struct ProxyState {
    inner: Mutex<Option<Running>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStatus {
    pub running: bool,
    pub port: u16,
    pub mitmdump: Option<String>,
    pub ca_path: Option<String>,
}

fn ca_cert_path() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let p = format!("{home}/.mitmproxy/mitmproxy-ca-cert.pem");
    Some(p)
}

/// Is mitmdump installed / resolvable?
#[tauri::command]
pub fn proxy_available() -> bool {
    #[cfg(not(windows))]
    {
        let resolved = crate::process_util::resolve_command("mitmdump", None);
        std::path::Path::new(&resolved).exists() || resolved.contains('/')
    }
    #[cfg(windows)]
    {
        true
    }
}

#[tauri::command]
pub fn proxy_status(state: tauri::State<ProxyState>) -> ProxyStatus {
    let guard = state.inner.lock().unwrap();
    let (running, port) = match guard.as_ref() {
        Some(r) => (true, r.port),
        None => (false, 0),
    };
    let mitm = {
        let r = crate::process_util::resolve_command("mitmdump", None);
        if std::path::Path::new(&r).exists() { Some(r) } else { None }
    };
    let ca = ca_cert_path().filter(|p| std::path::Path::new(p).exists());
    ProxyStatus { running, port, mitmdump: mitm, ca_path: ca }
}

/// Start capturing: spawn mitmdump with the bridge addon and stream each flow to `on_flow`.
#[tauri::command]
pub fn proxy_start(
    state: tauri::State<ProxyState>,
    port: u16,
    on_flow: Channel<serde_json::Value>,
) -> Result<(), String> {
    let mut guard = state.inner.lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        return Err("抓包代理已在运行".into());
    }

    // Write the bridge addon to a stable temp path.
    let script = std::env::temp_dir().join("michael_ide_mitm_bridge.py");
    std::fs::write(&script, BRIDGE_ADDON).map_err(|e| format!("写入桥接脚本失败: {e}"))?;

    #[cfg(not(windows))]
    let resolved = crate::process_util::resolve_command("mitmdump", None);
    #[cfg(windows)]
    let resolved = "mitmdump".to_string();
    if !std::path::Path::new(&resolved).exists() && !resolved.contains('/') {
        return Err("未找到 mitmdump（请先安装 mitmproxy）".into());
    }

    let mut cmd = crate::process_util::command(&resolved);
    cmd.args([
        "--listen-host",
        "127.0.0.1",
        "--listen-port",
        &port.to_string(),
        "-q", // quiet mitmproxy's own logging; only our __MFLOW__ lines matter
        "-s",
        &script.to_string_lossy(),
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null());
    #[cfg(not(windows))]
    cmd.env("PATH", crate::process_util::augmented_path(None));

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("启动 mitmdump 失败: {e}"))?;

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let sink = on_flow.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if let Some(rest) = line.strip_prefix(FLOW_MARKER) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(rest) {
                    let _ = sink.send(v);
                }
            }
        }
    });

    *guard = Some(Running { child, port });
    set_active_port(Some(port)); // browser.rs routes the automation browser through this port
    Ok(())
}

#[tauri::command]
pub fn proxy_stop(state: tauri::State<ProxyState>) -> Result<(), String> {
    let mut guard = state.inner.lock().map_err(|e| e.to_string())?;
    *guard = None; // Drop kills + reaps mitmdump
    Ok(())
}

/// Return the mitmproxy CA cert PEM path (created by mitmproxy on its first run) so the UI can help
/// the user trust it.
#[tauri::command]
pub fn proxy_ca_path() -> Option<String> {
    ca_cert_path().filter(|p| std::path::Path::new(p).exists())
}

/// macOS: point the active network service's HTTP+HTTPS proxy at 127.0.0.1:<port> (or turn it off),
/// so ALL apps route through the capture proxy — the "真·小黄鸟" whole-system capture.
#[tauri::command]
pub fn proxy_set_system_proxy(enable: bool, port: u16) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // Find the primary active network service (the one with a default route).
        let svc = macos_primary_service()?;
        let run = |args: &[&str]| -> Result<(), String> {
            let out = crate::process_util::command("networksetup")
                .args(args)
                .output()
                .map_err(|e| format!("networksetup 失败: {e}"))?;
            if out.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).to_string())
            }
        };
        let p = port.to_string();
        if enable {
            run(&["-setwebproxy", &svc, "127.0.0.1", &p])?;
            run(&["-setsecurewebproxy", &svc, "127.0.0.1", &p])?;
            run(&["-setwebproxystate", &svc, "on"])?;
            run(&["-setsecurewebproxystate", &svc, "on"])?;
        } else {
            run(&["-setwebproxystate", &svc, "off"])?;
            run(&["-setsecurewebproxystate", &svc, "off"])?;
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (enable, port);
        Err("系统代理自动配置目前仅支持 macOS；请手动把系统/浏览器代理设为 127.0.0.1:<port>".into())
    }
}

#[cfg(target_os = "macos")]
fn macos_primary_service() -> Result<String, String> {
    // `route get default` → interface (e.g. en0); map it to a networksetup service name.
    let dev = {
        let out = crate::process_util::command("route")
            .args(["get", "default"])
            .output()
            .map_err(|e| format!("route 失败: {e}"))?;
        let s = String::from_utf8_lossy(&out.stdout).to_string();
        s.lines()
            .find_map(|l| l.trim().strip_prefix("interface:").map(|d| d.trim().to_string()))
    };
    let list = crate::process_util::command("networksetup")
        .args(["-listnetworkserviceorder"])
        .output()
        .map_err(|e| format!("networksetup 失败: {e}"))?;
    let text = String::from_utf8_lossy(&list.stdout).to_string();
    // Blocks look like: "(1) Wi-Fi\n(Hardware Port: Wi-Fi, Device: en0)"
    if let Some(dev) = dev {
        let mut last_name: Option<String> = None;
        for line in text.lines() {
            let t = line.trim();
            if let Some(idx) = t.find(") ") {
                if t.starts_with('(') {
                    last_name = Some(t[idx + 2..].trim().to_string());
                }
            }
            if t.contains(&format!("Device: {dev}")) {
                if let Some(n) = last_name.take() {
                    return Ok(n);
                }
            }
        }
    }
    // Fallback: Wi-Fi is the overwhelmingly common case.
    Ok("Wi-Fi".to_string())
}

pub fn stop_all(state: &ProxyState) {
    if let Ok(mut g) = state.inner.lock() {
        *g = None;
    }
}
