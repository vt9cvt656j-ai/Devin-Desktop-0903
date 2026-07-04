//! Minimal MCP (Model Context Protocol) stdio client — lets the agent plug into
//! ANY external MCP server (filesystem, GitHub, Slack, databases, hundreds of
//! community tools) the same way Claude Code / Codex / Cursor do.
//!
//! Transport per spec: the client launches the server as a subprocess and speaks
//! JSON-RPC 2.0 over stdin/stdout, ONE message per line (newline-delimited, no
//! embedded newlines). stderr is logging — we drop it. Every request has a hard
//! timeout, so a slow / wedged / misbehaving server can NEVER hang the agent: it
//! just fails honestly and the tool reports it.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};

const PROTOCOL_VERSION: &str = "2025-06-18";

struct Session {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<String>,
    next_id: u64,
}

impl Drop for Session {
    // Reap the child on ANY removal from the map (disconnect / same-name replace / dead-eviction /
    // stop_all), so a wedged or forgotten MCP server never orphans its process + reader thread.
    // Killing the child closes its stdout → the reader thread's `lines()` returns None → it exits.
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

static SESSIONS: LazyLock<Mutex<HashMap<String, Session>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Kill + reap ALL connected MCP servers. Wired into `cleanup_stale` (webview reload) and the app
/// Exit handler in lib.rs — same as LSP/DAP/Terminal — so MCP children never survive a reload or a
/// quit. Drains + drops on a detached thread (each `Session::drop` does kill()+blocking wait()) so a
/// slow-dying server can't stall the caller.
pub fn stop_all() {
    let drained: Vec<Session> = match SESSIONS.lock() {
        Ok(mut guard) => guard.drain().map(|(_, v)| v).collect(),
        Err(_) => return,
    };
    if !drained.is_empty() {
        std::thread::spawn(move || drop(drained));
    }
}

#[derive(Serialize)]
pub struct McpTool {
    name: String,
    description: String,
    input_schema: Value,
}

impl Session {
    /// Send a JSON-RPC request and read lines until the matching response arrives
    /// or the deadline passes. Notifications and server→client requests are skipped.
    fn request(&mut self, method: &str, params: Value, timeout_secs: u64) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let msg = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        let line = serde_json::to_string(&msg).map_err(|e| e.to_string())?;
        self.stdin
            .write_all(line.as_bytes())
            .and_then(|_| self.stdin.write_all(b"\n"))
            .and_then(|_| self.stdin.flush())
            .map_err(|e| format!("写入 MCP 服务失败: {e}"))?;

        let deadline = Instant::now() + Duration::from_secs(timeout_secs);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(format!("MCP 请求超时（{method}）"));
            }
            match self.rx.recv_timeout(remaining) {
                Ok(raw) => {
                    let v: Value = match serde_json::from_str(&raw) {
                        Ok(v) => v,
                        Err(_) => continue, // not valid JSON (stray output) — ignore
                    };
                    // Only our response carries our id and no "method".
                    if v.get("method").is_some() {
                        continue; // server→client request or notification — skip
                    }
                    let matches = v.get("id").and_then(|i| i.as_u64()) == Some(id);
                    if !matches {
                        continue;
                    }
                    if let Some(err) = v.get("error") {
                        let m = err.get("message").and_then(|m| m.as_str()).unwrap_or("MCP error");
                        return Err(m.to_string());
                    }
                    return Ok(v.get("result").cloned().unwrap_or(Value::Null));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(format!("MCP 请求超时（{method}）"))
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err("MCP 服务已退出（stdout 关闭）".into())
                }
            }
        }
    }

    fn notify(&mut self, method: &str, params: Value) {
        let msg = json!({"jsonrpc":"2.0","method":method,"params":params});
        if let Ok(line) = serde_json::to_string(&msg) {
            let _ = self.stdin.write_all(line.as_bytes());
            let _ = self.stdin.write_all(b"\n");
            let _ = self.stdin.flush();
        }
    }
}

fn spawn_session(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
    cwd: &str,
) -> Result<Session, String> {
    let mut cmd = crate::process_util::command(command);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for (k, v) in env {
        cmd.env(k, v);
    }
    if !cwd.is_empty() && std::path::Path::new(cwd).is_dir() {
        cmd.current_dir(cwd);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("启动 MCP 服务失败（{command}）: {e}"))?;
    let stdin = child.stdin.take().ok_or("无法获取 MCP stdin")?;
    let stdout = child.stdout.take().ok_or("无法获取 MCP stdout")?;

    // Reader thread: every stdout line → channel. Ends when stdout closes.
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    if tx.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    Ok(Session {
        child,
        stdin,
        rx,
        next_id: 1,
    })
}

/// Launch an MCP server, run the initialize handshake, and return its tool list.
/// Replaces any existing session of the same name.
#[tauri::command]
pub async fn mcp_connect(
    name: String,
    command: String,
    args: Option<Vec<String>>,
    env: Option<HashMap<String, String>>,
    cwd: Option<String>,
) -> Result<Vec<McpTool>, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<McpTool>, String> {
        let args = args.unwrap_or_default();
        let env = env.unwrap_or_default();
        let cwd = cwd.unwrap_or_default();
        let mut session = spawn_session(&command, &args, &env, &cwd)?;

        // Handshake.
        session.request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "Michael-IDE", "version": "1.0" }
            }),
            20,
        )?;
        session.notify("notifications/initialized", json!({}));

        // Discover tools.
        let result = session.request("tools/list", json!({}), 15)?;
        let tools = result
            .get("tools")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for t in tools {
            let tname = t.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
            if tname.is_empty() {
                continue;
            }
            out.push(McpTool {
                name: tname,
                description: t
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string(),
                input_schema: t
                    .get("inputSchema")
                    .cloned()
                    .unwrap_or_else(|| json!({"type":"object","properties":{}})),
            });
        }

        // Replace any prior session of this name (kill the old child).
        let mut guard = SESSIONS.lock().map_err(|_| "MCP state poisoned")?;
        if let Some(mut old) = guard.remove(&name) {
            let _ = old.child.kill();
        }
        guard.insert(name, session);
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Call a tool on a connected MCP server. Returns the tool's text content.
#[tauri::command]
pub async fn mcp_call(name: String, tool: String, args: Option<Value>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let mut guard = SESSIONS.lock().map_err(|_| "MCP state poisoned")?;
        let session = guard
            .get_mut(&name)
            .ok_or_else(|| format!("MCP 服务「{name}」未连接"))?;
        let req = session.request(
            "tools/call",
            json!({ "name": tool, "arguments": args.unwrap_or(json!({})) }),
            60,
        );
        // If the call failed AND the server process has actually exited, evict the session so its
        // child + reader thread are reaped now (Session::drop) instead of lingering forever; a later
        // reconnect respawns cleanly. A mere slow-tool timeout on a still-alive server is kept.
        let dead = req.is_err() && matches!(session.child.try_wait(), Ok(Some(_)));
        let result = match req {
            Ok(r) => r,
            Err(e) => {
                if dead {
                    guard.remove(&name);
                }
                return Err(e);
            }
        };

        // Flatten the content blocks into text.
        let mut text = String::new();
        if let Some(items) = result.get("content").and_then(|c| c.as_array()) {
            for it in items {
                match it.get("type").and_then(|t| t.as_str()) {
                    Some("text") => {
                        if let Some(s) = it.get("text").and_then(|s| s.as_str()) {
                            if !text.is_empty() {
                                text.push('\n');
                            }
                            text.push_str(s);
                        }
                    }
                    Some(other) => {
                        text.push_str(&format!("[{other} 内容，已省略]"));
                    }
                    None => {}
                }
            }
        }
        if text.is_empty() {
            text = serde_json::to_string(&result).unwrap_or_default();
        }
        let is_error = result.get("isError").and_then(|e| e.as_bool()).unwrap_or(false);
        if is_error {
            return Err(format!("[工具报错] {text}"));
        }
        Ok(text)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Disconnect and kill an MCP server session.
#[tauri::command]
pub async fn mcp_disconnect(name: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        if let Ok(mut guard) = SESSIONS.lock() {
            if let Some(mut s) = guard.remove(&name) {
                let _ = s.child.kill();
            }
        }
    })
    .await
    .map_err(|e| e.to_string())
}
