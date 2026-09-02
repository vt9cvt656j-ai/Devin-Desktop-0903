use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Stdio};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::State;

use crate::process_util;

#[derive(Serialize, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[allow(dead_code)]
pub enum DapEvent {
    Message { data: String },
    Started { adapter: String },
    Error { message: String },
    Stopped { adapter: String },
}

struct DapProcess {
    child: Child,
    stdin_tx: std::sync::mpsc::Sender<String>,
}

// Kill the debug adapter on drop so clearing the map (reload / exit) reaps it and
// its reader threads instead of leaving it resident.
impl Drop for DapProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Default)]
pub struct DebugManager {
    inner: Mutex<HashMap<String, DapProcess>>,
}

impl DebugManager {
    /// Kill every debug adapter and clear the map — reaps a previous page session
    /// on webview reload and on app exit.
    pub fn stop_all(&self) {
        // Drain + reap on a DETACHED thread — DapProcess::drop does kill()+blocking wait();
        // off-thread keeps boot (cleanup_stale) and app exit from stalling on it.
        let drained: Vec<DapProcess> = match self.inner.lock() {
            Ok(mut inner) => inner.drain().map(|(_, v)| v).collect(),
            Err(_) => return,
        };
        if !drained.is_empty() {
            std::thread::spawn(move || drop(drained));
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct DapConfig {
    pub adapter_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
}

// Default *DAP* adapter commands (must speak the Debug Adapter Protocol over
// stdio). python/go/lldb have clean stdio adapters; node realistically needs
// vscode-js-debug, so its default is a placeholder users override in the
// advanced launcher.
/// Windows 上 `python3` 通常不存在：python.org 的包只产出 `python.exe`，而 PATH 上
/// 那个叫 `python3.exe` 的多半是微软商店的「应用执行别名」—— 跑它会弹商店页面，
/// 调试器表现成「点了没反应」。`lsp.rs` 的 DEFAULT_PYTHON_NAMES 早就按平台分了名字，
/// 这里是漏下的第二处（调试面板下拉框和 .vscode/launch.json 两条路都落在这儿）。
#[cfg(windows)]
const DEFAULT_PYTHON_CMD: &str = "python";
#[cfg(not(windows))]
const DEFAULT_PYTHON_CMD: &str = "python3";

const KNOWN_ADAPTERS: &[(&str, &str, &[&str])] = &[
    ("python", DEFAULT_PYTHON_CMD, &["-m", "debugpy.adapter"]),
    ("go", "dlv", &["dap"]),
    ("lldb", "lldb-dap", &[]),
    ("node", "js-debug-adapter-stdio", &[]),
];

fn find_adapter(id: &str) -> Option<(&'static str, &'static [&'static str])> {
    KNOWN_ADAPTERS
        .iter()
        .find(|(a, _, _)| *a == id)
        .map(|(_, cmd, args)| (*cmd, *args))
}

fn prune_stopped(inner: &mut HashMap<String, DapProcess>) {
    inner.retain(|_, proc| match proc.child.try_wait() {
        Ok(Some(_)) => false,
        Ok(None) => true,
        Err(_) => false,
    });
}

fn encode_dap_message(content: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{}", content.len(), content)
}

#[tauri::command(async)]
pub fn dap_start(
    state: State<DebugManager>,
    config: DapConfig,
    on_event: Channel<DapEvent>,
) -> Result<(), String> {
    let mut inner = state.inner.lock().map_err(|e| e.to_string())?;
    prune_stopped(&mut inner);

    if inner.contains_key(&config.adapter_id) {
        return Err(format!(
            "Debug adapter '{}' is already running",
            config.adapter_id
        ));
    }
    if inner.len() >= process_util::MAX_CHILD_PROCESSES {
        return Err("too many debug adapters running; stop one first".into());
    }

    let command = if config.command.is_empty() {
        let (cmd, _) = find_adapter(&config.adapter_id).ok_or_else(|| {
            format!(
                "no known debug adapter for '{}'; provide a custom command",
                config.adapter_id
            )
        })?;
        cmd.to_string()
    } else {
        config.command.clone()
    };
    let args: Vec<String> = if config.command.is_empty() {
        let (_, default_args) = find_adapter(&config.adapter_id).ok_or_else(|| {
            format!(
                "no known debug adapter for '{}'; provide a custom command",
                config.adapter_id
            )
        })?;
        default_args.iter().map(|arg| (*arg).to_string()).collect()
    } else {
        config.args.clone()
    };

    // 和 lsp.rs 同一处坑：Windows 分支原来是裸名字直接 spawn，而 Rust 的 Command 在
    // Windows 上只补 .exe、不查 PATHEXT。调试适配器同样多是 npm 装的 *.cmd
    // （js-debug-adapter-stdio），于是「装了却起不来，也没人说为什么」。
    let resolved = process_util::resolve_command(&command, config.cwd.as_deref());

    let mut builder = crate::process_util::command(&resolved);
    builder
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(ref cwd) = config.cwd {
        builder.current_dir(cwd);
    }
    // PATH 两个平台都要设。这里以前关在 cfg(not(windows)) 里，而子进程继承的是**这个
    // 进程**的 PATH —— GUI 启动的应用拿到的那份很窄：nvm / volta / 用户级 npm 前缀全不在
    // 里面，于是适配器就算被拉起来，它自己再去找 node/python 也找不到。
    // lsp.rs:334 和 mcp.rs:1180 都已经把这个 cfg 删掉并写了同样的理由，这里是漏下的第三处。
    builder.env("PATH", process_util::augmented_path(config.cwd.as_deref()));

    let mut child = builder
        .spawn()
        .map_err(|e| format!("failed to start '{}': {}", command, e))?;

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stderr = child.stderr.take().ok_or("no stderr")?;

    let (stdin_tx, stdin_rx) = std::sync::mpsc::channel::<String>();

    let mut stdin_handle = child.stdin.take().ok_or("no stdin")?;
    std::thread::spawn(move || {
        while let Ok(msg) = stdin_rx.recv() {
            let encoded = encode_dap_message(&msg);
            if stdin_handle.write_all(encoded.as_bytes()).is_err() {
                break;
            }
            if stdin_handle.flush().is_err() {
                break;
            }
        }
    });

    let adapter = config.adapter_id.clone();
    let evt = on_event.clone();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let body = match crate::content_length_frame::read_frame(&mut reader) {
                Ok(Some(body)) => body,
                Ok(None) => break,
                Err(error) => {
                    tracing::warn!("[dap-{adapter}] invalid protocol frame: {error}");
                    break;
                }
            };
            let data = String::from_utf8_lossy(&body).to_string();
            if evt.send(DapEvent::Message { data }).is_err() {
                break;
            }
        }
        let _ = evt.send(DapEvent::Stopped { adapter });
    });

    let adapter2 = config.adapter_id.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(l) => tracing::debug!("[dap-{}] {}", adapter2, l),
                Err(_) => break,
            }
        }
    });

    let _ = on_event.send(DapEvent::Started {
        adapter: config.adapter_id.clone(),
    });

    inner.insert(config.adapter_id, DapProcess { child, stdin_tx });
    Ok(())
}

#[tauri::command(async)]
pub fn dap_send(
    state: State<DebugManager>,
    adapter_id: String,
    message: String,
) -> Result<(), String> {
    let inner = state.inner.lock().map_err(|e| e.to_string())?;
    let proc = inner
        .get(&adapter_id)
        .ok_or_else(|| format!("no debug adapter '{adapter_id}'"))?;
    proc.stdin_tx
        .send(message)
        .map_err(|e| format!("failed to send to DAP: {e}"))
}

#[tauri::command(async)]
pub fn dap_stop(state: State<DebugManager>, adapter_id: String) -> Result<(), String> {
    let removed = {
        let mut inner = state.inner.lock().map_err(|e| e.to_string())?;
        inner.remove(&adapter_id)
    };
    if let Some(mut proc) = removed {
        let _ = proc.child.kill();
        // DapProcess::drop waits for the adapter. Do that after releasing the
        // manager lock so dap_send/list never queue behind process shutdown.
        drop(proc);
    }
    Ok(())
}

#[derive(Serialize)]
pub struct DapInfo {
    adapter: String,
    running: bool,
}

#[tauri::command(async)]
pub fn dap_list(state: State<DebugManager>) -> Result<Vec<DapInfo>, String> {
    let mut inner = state.inner.lock().map_err(|e| e.to_string())?;
    prune_stopped(&mut inner);
    let known: Vec<&str> = KNOWN_ADAPTERS.iter().map(|(a, _, _)| *a).collect();
    let mut out: Vec<DapInfo> = known
        .into_iter()
        .map(|a| DapInfo {
            adapter: a.to_string(),
            running: inner.contains_key(a),
        })
        .collect();
    for key in inner.keys() {
        if !out.iter().any(|i| i.adapter == *key) {
            out.push(DapInfo {
                adapter: key.clone(),
                running: true,
            });
        }
    }
    Ok(out)
}
