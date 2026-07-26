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

/// 我们改动系统代理之前的原始状态。
///
/// 改了却不记，就没法还原 —— 而 `Running::drop`（进程被杀 / 应用退出 / 崩溃）此前只杀
/// mitmdump，系统代理仍然指着一个已经死掉的 127.0.0.1:<port>：**整机断网**，用户还得
/// 自己去系统设置里翻出来关掉。
#[cfg(target_os = "macos")]
#[derive(Clone)]
struct SystemProxyBackup {
    service: String,
    http_on: bool,
    http_server: String,
    http_port: String,
    https_on: bool,
    https_server: String,
    https_port: String,
}

#[cfg(target_os = "macos")]
static SYSTEM_PROXY_BACKUP: std::sync::Mutex<Option<SystemProxyBackup>> =
    std::sync::Mutex::new(None);

#[cfg(target_os = "macos")]
fn networksetup(args: &[&str]) -> Result<String, String> {
    let out = crate::process_util::command("networksetup")
        .args(args)
        .output()
        .map_err(|e| format!("networksetup 失败: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

/// 解析 `networksetup -getwebproxy <svc>` 的输出。
#[cfg(target_os = "macos")]
fn parse_proxy_readout(text: &str) -> (bool, String, String) {
    let field = |key: &str| -> String {
        text.lines()
            .find_map(|l| l.strip_prefix(key))
            .map(|v| v.trim_start_matches(':').trim().to_string())
            .unwrap_or_default()
    };
    (
        field("Enabled").eq_ignore_ascii_case("yes"),
        field("Server"),
        field("Port"),
    )
}

/// 把系统代理恢复成我们改动之前的样子。没有备份（不是我们开的）就什么都不做。
#[cfg(target_os = "macos")]
fn restore_system_proxy() {
    let backup = match SYSTEM_PROXY_BACKUP.lock() {
        Ok(mut g) => g.take(),
        Err(_) => None,
    };
    let Some(b) = backup else { return };
    let svc = b.service.as_str();
    if b.http_on && !b.http_server.is_empty() {
        let _ = networksetup(&["-setwebproxy", svc, &b.http_server, &b.http_port]);
        let _ = networksetup(&["-setwebproxystate", svc, "on"]);
    } else {
        let _ = networksetup(&["-setwebproxystate", svc, "off"]);
    }
    if b.https_on && !b.https_server.is_empty() {
        let _ = networksetup(&["-setsecurewebproxy", svc, &b.https_server, &b.https_port]);
        let _ = networksetup(&["-setsecurewebproxystate", svc, "on"]);
    } else {
        let _ = networksetup(&["-setsecurewebproxystate", svc, "off"]);
    }
}

#[cfg(not(target_os = "macos"))]
fn restore_system_proxy() {}

struct Running {
    child: Child,
    port: u16,
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        set_active_port(None); // capture stopped → browser stops routing through it
        // 代理进程没了，系统代理就必须跟着还原 —— 否则整机流量继续指向一个死端口。
        // 这里覆盖所有退出路径：正常停止、应用退出、以及 panic 时的栈展开。
        restore_system_proxy();
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
        if std::path::Path::new(&r).exists() {
            Some(r)
        } else {
            None
        }
    };
    let ca = ca_cert_path().filter(|p| std::path::Path::new(p).exists());
    ProxyStatus {
        running,
        port,
        mitmdump: mitm,
        ca_path: ca,
    }
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
            // 先记下原状态再改。只在第一次开启时记，避免重复开启把我们自己写的值当成
            // "用户原本的设置"存下来。
            if let Ok(mut g) = SYSTEM_PROXY_BACKUP.lock() {
                if g.is_none() {
                    let http = networksetup(&["-getwebproxy", &svc]).unwrap_or_default();
                    let https = networksetup(&["-getsecurewebproxy", &svc]).unwrap_or_default();
                    let (h_on, h_srv, h_port) = parse_proxy_readout(&http);
                    let (s_on, s_srv, s_port) = parse_proxy_readout(&https);
                    *g = Some(SystemProxyBackup {
                        service: svc.clone(),
                        http_on: h_on, http_server: h_srv, http_port: h_port,
                        https_on: s_on, https_server: s_srv, https_port: s_port,
                    });
                }
            }
            run(&["-setwebproxy", &svc, "127.0.0.1", &p])?;
            run(&["-setsecurewebproxy", &svc, "127.0.0.1", &p])?;
            run(&["-setwebproxystate", &svc, "on"])?;
            run(&["-setsecurewebproxystate", &svc, "on"])?;
        } else {
            // 还原用户原本的设置，而不是一律关掉 —— 他可能本来就配着公司代理。
            restore_system_proxy();
            run(&["-setwebproxystate", &svc, "off"]).ok();
            run(&["-setsecurewebproxystate", &svc, "off"]).ok();
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
        s.lines().find_map(|l| {
            l.trim()
                .strip_prefix("interface:")
                .map(|d| d.trim().to_string())
        })
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
        *g = None; // Running::drop 会顺带还原系统代理
    }
    // 即使当时没有在跑的 Running（比如只开了系统代理没起 mitmdump），也要还原。
    restore_system_proxy();
}

#[cfg(all(test, target_os = "macos"))]
mod proxy_backup_tests {
    use super::*;

    /// 解析对了才能还原对。`networksetup -getwebproxy` 的输出是固定的四行键值。
    #[test]
    fn parses_an_enabled_proxy_readout() {
        let out = "Enabled: Yes\nServer: proxy.corp.example\nPort: 8080\nAuthenticated Proxy Enabled: 0\n";
        assert_eq!(
            parse_proxy_readout(out),
            (true, "proxy.corp.example".to_string(), "8080".to_string())
        );
    }

    /// 用户本来没开代理：还原时应该是"关掉"，而不是拿空 server 去 set。
    #[test]
    fn parses_a_disabled_proxy_readout() {
        let out = "Enabled: No\nServer: \nPort: 0\nAuthenticated Proxy Enabled: 0\n";
        let (on, server, _) = parse_proxy_readout(out);
        assert!(!on);
        assert!(server.is_empty());
    }

    #[test]
    fn missing_fields_do_not_panic() {
        assert_eq!(parse_proxy_readout(""), (false, String::new(), String::new()));
        assert_eq!(parse_proxy_readout("garbage"), (false, String::new(), String::new()));
    }
}
