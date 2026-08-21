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
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
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

// Proxy-setting commands must not overlap, but they must never hold the backup
// mutex while invoking networksetup.
#[cfg(target_os = "macos")]
static SYSTEM_PROXY_OPERATION: once_cell::sync::Lazy<Mutex<()>> =
    once_cell::sync::Lazy::new(|| Mutex::new(()));

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

/// Restore the saved proxy while the caller owns SYSTEM_PROXY_OPERATION.
/// The backup lock is held only long enough to take the saved state.
#[cfg(target_os = "macos")]
fn restore_system_proxy_locked() -> bool {
    let backup = match SYSTEM_PROXY_BACKUP.lock() {
        Ok(mut g) => g.take(),
        Err(_) => None,
    };
    let Some(b) = backup else { return false };
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
    true
}

/// 把系统代理恢复成我们改动之前的样子。没有备份（不是我们开的）就什么都不做。
#[cfg(target_os = "macos")]
fn restore_system_proxy() {
    let Ok(_operation) = SYSTEM_PROXY_OPERATION.lock() else {
        return;
    };
    let _ = restore_system_proxy_locked();
}

#[cfg(not(target_os = "macos"))]
fn restore_system_proxy() {}

struct Running {
    child: Child,
    port: u16,
    generation: u64,
    // A process that lost the start race was never published as the active
    // proxy. Its cleanup must therefore only terminate the child: it must not
    // clear a newer port or restore a proxy owned by another lifecycle.
    owns_lifecycle: bool,
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if self.owns_lifecycle {
            set_active_port(None); // capture stopped → browser stops routing through it
                                   // 代理进程没了，系统代理就必须跟着还原 —— 否则整机流量继续指向一个死端口。
                                   // 这里覆盖所有退出路径：正常停止、应用退出、以及 panic 时的栈展开。
            restore_system_proxy();
        }
    }
}

enum ProxySlot {
    Idle,
    Starting { generation: u64 },
    Stopping { generation: u64 },
    Running(Running),
}

impl ProxySlot {
    fn reserve_start(&mut self, generation: u64) -> bool {
        if matches!(self, Self::Idle) {
            *self = Self::Starting { generation };
            true
        } else {
            false
        }
    }

    fn accepts_start(&self, generation: u64) -> bool {
        matches!(self, Self::Starting { generation: current } if *current == generation)
    }

    fn finish_stop(&mut self, generation: u64) {
        if matches!(self, Self::Stopping { generation: current } if *current == generation) {
            *self = Self::Idle;
        }
    }

    /// Reserve teardown before returning a child. A start that is still doing
    /// blocking setup also becomes Stopping, so a new start cannot overlap it.
    fn stop(&mut self) -> Option<Running> {
        match std::mem::replace(self, Self::Idle) {
            Self::Running(running) => {
                *self = Self::Stopping {
                    generation: running.generation,
                };
                Some(running)
            }
            Self::Starting { generation } => {
                *self = Self::Stopping { generation };
                None
            }
            Self::Stopping { generation } => {
                *self = Self::Stopping { generation };
                None
            }
            Self::Idle => None,
        }
    }
}

pub struct ProxyState {
    inner: Arc<Mutex<ProxySlot>>,
    next_generation: AtomicU64,
}

impl Default for ProxyState {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ProxySlot::Idle)),
            next_generation: AtomicU64::new(0),
        }
    }
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
    // 只读 HOME 的话，Windows 上这里恒为 None——而 mitmproxy 在 Windows 上确实
    // 把证书放在 %USERPROFILE%\.mitmproxy\ 下。拿不到具体路径，抓 HTTPS 时
    // 就没法告诉用户"去装哪个文件"，只能给一句泛泛的"要装证书"。
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    let mut p = std::path::PathBuf::from(home);
    p.push(".mitmproxy");
    p.push("mitmproxy-ca-cert.pem");
    Some(p.to_string_lossy().into_owned())
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
    let (running, port) = match &*state.inner.lock().unwrap() {
        ProxySlot::Running(running) => (true, running.port),
        ProxySlot::Idle | ProxySlot::Starting { .. } | ProxySlot::Stopping { .. } => (false, 0),
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

fn start_proxy_process(
    port: u16,
    generation: u64,
    on_flow: Channel<serde_json::Value>,
) -> Result<Running, String> {
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

    Ok(Running {
        child,
        port,
        generation,
        owns_lifecycle: false,
    })
}

/// Start capturing: spawn mitmdump with the bridge addon and stream each flow to `on_flow`.
#[tauri::command]
pub async fn proxy_start(
    state: tauri::State<'_, ProxyState>,
    port: u16,
    on_flow: Channel<serde_json::Value>,
) -> Result<(), String> {
    let generation = state
        .next_generation
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_add(1);
    {
        let mut slot = state.inner.lock().map_err(|e| e.to_string())?;
        if !slot.reserve_start(generation) {
            return Err("抓包代理已在运行".into());
        }
    }

    let inner = Arc::clone(&state.inner);
    tauri::async_runtime::spawn_blocking(move || {
        let running = match start_proxy_process(port, generation, on_flow) {
            Ok(running) => running,
            Err(error) => {
                if let Ok(mut slot) = inner.lock() {
                    // A cancelled start is represented as Stopping until this
                    // worker has actually finished its blocking setup path.
                    slot.finish_stop(generation);
                    if slot.accepts_start(generation) {
                        *slot = ProxySlot::Idle;
                    }
                }
                return Err(error);
            }
        };

        let stale = {
            let mut slot = inner.lock().map_err(|e| e.to_string())?;
            if slot.accepts_start(generation) {
                let mut running = running;
                running.owns_lifecycle = true;
                *slot = ProxySlot::Running(running);
                set_active_port(Some(port));
                None
            } else {
                Some(running)
            }
        };
        if let Some(running) = stale {
            drop(running);
            if let Ok(mut slot) = inner.lock() {
                slot.finish_stop(generation);
            }
            Err("抓包代理启动已取消".into())
        } else {
            Ok(())
        }
    })
    .await
    .map_err(|e| format!("抓包代理启动任务失败: {e}"))?
}

#[tauri::command]
pub async fn proxy_stop(state: tauri::State<'_, ProxyState>) -> Result<(), String> {
    let running = {
        let mut slot = state.inner.lock().map_err(|e| e.to_string())?;
        slot.stop()
    };
    if let Some(running) = running {
        let generation = running.generation;
        tauri::async_runtime::spawn_blocking(move || drop(running))
            .await
            .map_err(|e| format!("抓包代理停止任务失败: {e}"))?;
        let mut slot = state.inner.lock().map_err(|e| e.to_string())?;
        slot.finish_stop(generation);
    }
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
pub async fn proxy_set_system_proxy(enable: bool, port: u16) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        tauri::async_runtime::spawn_blocking(move || {
            // Serialize complete transactions, while keeping SYSTEM_PROXY_BACKUP
            // free during route/networksetup subprocess calls.
            let _operation = SYSTEM_PROXY_OPERATION.lock().map_err(|e| e.to_string())?;
            if !enable {
                if restore_system_proxy_locked() {
                    return Ok(());
                }
                let svc = macos_primary_service()?;
                let _ = networksetup(&["-setwebproxystate", &svc, "off"]);
                let _ = networksetup(&["-setsecurewebproxystate", &svc, "off"]);
                return Ok(());
            }

            let svc = macos_primary_service()?;
            let backup_needed = SYSTEM_PROXY_BACKUP
                .lock()
                .map_err(|e| e.to_string())?
                .is_none();
            let backup = if backup_needed {
                let http = networksetup(&["-getwebproxy", &svc]).unwrap_or_default();
                let https = networksetup(&["-getsecurewebproxy", &svc]).unwrap_or_default();
                let (http_on, http_server, http_port) = parse_proxy_readout(&http);
                let (https_on, https_server, https_port) = parse_proxy_readout(&https);
                Some(SystemProxyBackup {
                    service: svc.clone(),
                    http_on,
                    http_server,
                    http_port,
                    https_on,
                    https_server,
                    https_port,
                })
            } else {
                None
            };
            if let Some(backup) = backup {
                let mut saved = SYSTEM_PROXY_BACKUP.lock().map_err(|e| e.to_string())?;
                if saved.is_none() {
                    *saved = Some(backup);
                }
            }

            let port = port.to_string();
            networksetup(&["-setwebproxy", &svc, "127.0.0.1", &port])?;
            networksetup(&["-setsecurewebproxy", &svc, "127.0.0.1", &port])?;
            networksetup(&["-setwebproxystate", &svc, "on"])?;
            networksetup(&["-setsecurewebproxystate", &svc, "on"])?;
            Ok(())
        })
        .await
        .map_err(|e| format!("系统代理设置任务失败: {e}"))?
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
    let running = state.inner.lock().ok().and_then(|mut slot| slot.stop());
    // Release lifecycle state before Running::drop kills/waits and restores the
    // system proxy. Shutdown is off the interactive path, so no async handoff.
    drop(running);
    // 即使当时没有在跑的 Running（比如只开了系统代理没起 mitmdump），也要还原。
    restore_system_proxy();
}

#[cfg(test)]
mod proxy_lifecycle_tests {
    use super::*;

    #[test]
    fn cancelled_start_blocks_a_new_generation_until_its_worker_finishes() {
        let mut slot = ProxySlot::Idle;
        assert!(slot.reserve_start(1));
        assert!(slot.accepts_start(1));

        // Stop can arrive while mitmdump resolution/spawn is still blocking.
        // It invalidates that start, but reserves cleanup before another start
        // is admitted, so the stale worker cannot affect a new generation.
        assert!(slot.stop().is_none());
        assert!(matches!(slot, ProxySlot::Stopping { generation: 1 }));
        assert!(!slot.accepts_start(1));
        assert!(!slot.reserve_start(2));

        // This models the old worker finishing after cancellation. Only then
        // can the next generation become the current owner.
        slot.finish_stop(1);
        assert!(matches!(slot, ProxySlot::Idle));
        assert!(slot.reserve_start(2));
        assert!(slot.accepts_start(2));
        assert!(!slot.accepts_start(1));
    }

    #[test]
    fn stale_cleanup_cannot_finish_a_newer_lifecycle() {
        let mut slot = ProxySlot::Stopping { generation: 7 };
        slot.finish_stop(6);
        assert!(matches!(slot, ProxySlot::Stopping { generation: 7 }));

        slot.finish_stop(7);
        assert!(matches!(slot, ProxySlot::Idle));
    }

    #[cfg(not(windows))]
    #[test]
    fn unpublished_old_teardown_cannot_clear_a_new_active_port() {
        // This is the side effect that used to make a newly started proxy
        // disappear from browser routing: an old, cancelled start dropped its
        // child after the new owner had published its port.
        let child = crate::process_util::command("sh")
            .args(["-c", "sleep 30"])
            .spawn()
            .expect("start disposable child");
        set_active_port(Some(18443)); // generation 2 is already active
        drop(Running {
            child,
            port: 18442,
            generation: 1,
            owns_lifecycle: false,
        });
        assert_eq!(active_proxy_port(), Some(18443));
        set_active_port(None);
    }
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
        assert_eq!(
            parse_proxy_readout(""),
            (false, String::new(), String::new())
        );
        assert_eq!(
            parse_proxy_readout("garbage"),
            (false, String::new(), String::new())
        );
    }
}
