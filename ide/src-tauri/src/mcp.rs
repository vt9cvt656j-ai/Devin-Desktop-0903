//! MCP (Model Context Protocol) stdio client for local command-based servers.
//!
//! Transport per spec: the client launches the server as a subprocess and speaks
//! JSON-RPC 2.0 over stdin/stdout, ONE message per line (newline-delimited, no
//! embedded newlines). A bounded stderr tail is retained for actionable failures.
//! Requests have timeouts, so a slow or misbehaving server returns an error.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};

const PROTOCOL_VERSION: &str = "2025-06-18";

struct Session {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<String>,
    stderr_log: Arc<Mutex<VecDeque<String>>>,
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

#[derive(Debug, Serialize)]
pub struct McpTool {
    name: String,
    description: String,
    input_schema: Value,
}

impl Session {
    fn error_with_stderr(&self, message: &str) -> String {
        let tail = self
            .stderr_log
            .lock()
            .ok()
            .map(|lines| lines.iter().cloned().collect::<Vec<_>>().join("\n"))
            .unwrap_or_default();
        if tail.trim().is_empty() {
            message.to_string()
        } else {
            format!("{message}\nMCP stderr:\n{tail}")
        }
    }

    fn respond_to_server_request(&mut self, request: &Value) {
        let Some(id) = request.get("id").cloned() else {
            return;
        };
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        let response = match method {
            "ping" => json!({"jsonrpc":"2.0","id":id,"result":{}}),
            "roots/list" => json!({"jsonrpc":"2.0","id":id,"result":{"roots":[]}}),
            _ => json!({
                "jsonrpc":"2.0",
                "id":id,
                "error":{"code":-32601,"message":format!("Client method not supported: {method}")}
            }),
        };
        if let Ok(line) = serde_json::to_string(&response) {
            let _ = self.stdin.write_all(line.as_bytes());
            let _ = self.stdin.write_all(b"\n");
            let _ = self.stdin.flush();
        }
    }

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
                return Err(self.error_with_stderr(&format!("MCP 请求超时（{method}）")));
            }
            match self.rx.recv_timeout(remaining) {
                Ok(raw) => {
                    let v: Value = match serde_json::from_str(&raw) {
                        Ok(v) => v,
                        Err(_) => continue, // not valid JSON (stray output) — ignore
                    };
                    // Only our response carries our id and no "method".
                    if v.get("method").is_some() {
                        // Notifications need no response. Server→client requests do: ignoring a
                        // ping/roots request leaves standards-compliant servers waiting forever.
                        self.respond_to_server_request(&v);
                        continue;
                    }
                    let matches = v.get("id").and_then(|i| i.as_u64()) == Some(id);
                    if !matches {
                        continue;
                    }
                    if let Some(err) = v.get("error") {
                        let m = err
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("MCP error");
                        return Err(m.to_string());
                    }
                    return Ok(v.get("result").cloned().unwrap_or(Value::Null));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(self.error_with_stderr(&format!("MCP 请求超时（{method}）")))
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(self.error_with_stderr("MCP 服务已退出（stdout 关闭）"))
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
    let ws = if cwd.is_empty() { None } else { Some(cwd) };
    // A GUI-launched app inherits a minimal PATH that misses nvm/volta/homebrew/pipx, so a bare `npx`
    // or `uvx` would fail to spawn ("启动 MCP 服务失败（npx）"). Resolve the launcher against the
    // augmented PATH and hand the subprocess that PATH too, so a resolved `npx` can still find `node`
    // and the server package it launches. Same fix as LSP/debug/tasks/terminal.
    #[cfg(not(windows))]
    let resolved = crate::process_util::resolve_command(command, ws);
    #[cfg(windows)]
    let resolved = command.to_string();
    let mut cmd = crate::process_util::command(&resolved);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(not(windows))]
    cmd.env("PATH", crate::process_util::augmented_path(ws));
    for (k, v) in env {
        cmd.env(k, v); // user-provided env (API keys, or an explicit PATH override) wins
    }
    if !cwd.is_empty() && std::path::Path::new(cwd).is_dir() {
        cmd.current_dir(cwd);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("启动 MCP 服务失败（{command}）: {e}"))?;
    let stdin = child.stdin.take().ok_or("无法获取 MCP stdin")?;
    let stdout = child.stdout.take().ok_or("无法获取 MCP stdout")?;
    let stderr = child.stderr.take().ok_or("无法获取 MCP stderr")?;

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

    let stderr_log = Arc::new(Mutex::new(VecDeque::with_capacity(40)));
    let stderr_sink = Arc::clone(&stderr_log);
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            let line = line.chars().take(500).collect::<String>();
            let Ok(mut log) = stderr_sink.lock() else {
                break;
            };
            if log.len() >= 40 {
                log.pop_front();
            }
            log.push_back(line);
        }
    });

    Ok(Session {
        child,
        stdin,
        rx,
        stderr_log,
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
        let connect_deadline = Instant::now() + Duration::from_secs(60);

        // Handshake.
        session.request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "Michael-IDE", "version": "1.0" }
            }),
            45,
        )?;
        session.notify("notifications/initialized", json!({}));

        // Discover every page. Some MCP servers paginate large tool registries; silently reading
        // only page one makes the missing tools impossible for the model to call.
        let mut tools = Vec::new();
        let mut cursor: Option<String> = None;
        let mut seen_cursors = HashSet::new();
        for _ in 0..100 {
            let remaining = connect_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err("MCP 连接超时（initialize + tools/list 超过 60s）".into());
            }
            let params = cursor
                .as_ref()
                .map(|value| json!({"cursor": value}))
                .unwrap_or_else(|| json!({}));
            let timeout = remaining.as_secs().clamp(1, 15);
            let result = session.request("tools/list", params, timeout)?;
            tools.extend(
                result
                    .get("tools")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
            );
            let next = result
                .get("nextCursor")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            match next {
                Some(next) if seen_cursors.insert(next.clone()) => cursor = Some(next),
                _ => break,
            }
        }
        let mut out = Vec::new();
        for t in tools {
            let tname = t
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
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

fn append_content(text: &mut String, value: &str) {
    if value.is_empty() {
        return;
    }
    if !text.is_empty() {
        text.push('\n');
    }
    text.push_str(value);
}

fn flatten_tool_content(result: &Value) -> String {
    let mut text = String::new();
    if let Some(items) = result.get("content").and_then(Value::as_array) {
        for item in items {
            match item.get("type").and_then(Value::as_str) {
                Some("text") => {
                    if let Some(value) = item.get("text").and_then(Value::as_str) {
                        append_content(&mut text, value);
                    }
                }
                Some("resource") => {
                    let resource = item.get("resource").unwrap_or(item);
                    let uri = resource
                        .get("uri")
                        .and_then(Value::as_str)
                        .unwrap_or("resource");
                    if let Some(value) = resource.get("text").and_then(Value::as_str) {
                        append_content(&mut text, &format!("[resource {uri}]\n{value}"));
                    } else {
                        let mime = resource
                            .get("mimeType")
                            .and_then(Value::as_str)
                            .unwrap_or("application/octet-stream");
                        append_content(&mut text, &format!("[resource {uri}, {mime}, binary]"));
                    }
                }
                Some("resource_link") => {
                    let uri = item.get("uri").and_then(Value::as_str).unwrap_or("");
                    let name = item
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("resource");
                    append_content(&mut text, &format!("[resource link: {name}] {uri}"));
                }
                Some("image") | Some("audio") => {
                    let kind = item.get("type").and_then(Value::as_str).unwrap_or("media");
                    let mime = item
                        .get("mimeType")
                        .and_then(Value::as_str)
                        .unwrap_or("application/octet-stream");
                    let encoded_len = item.get("data").and_then(Value::as_str).map_or(0, str::len);
                    append_content(
                        &mut text,
                        &format!("[{kind} content: {mime}, {encoded_len} base64 characters]"),
                    );
                }
                Some(other) => append_content(&mut text, &format!("[{other} content]")),
                None => {}
            }
        }
    }
    if text.is_empty() {
        serde_json::to_string(result).unwrap_or_default()
    } else {
        text
    }
}

/// Call a tool on a connected MCP server. Returns text plus readable embedded-resource metadata.
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

        let text = flatten_tool_content(&result);
        let is_error = result
            .get("isError")
            .and_then(|e| e.as_bool())
            .unwrap_or(false);
        if is_error {
            return Err(format!("[工具报错] {text}"));
        }
        Ok(text)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Report whether a named session still has a live child process. Dead sessions are evicted so
/// the next connect starts cleanly and the UI never keeps showing a stale green status.
#[tauri::command]
pub async fn mcp_status(name: String) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<bool, String> {
        let mut guard = SESSIONS.lock().map_err(|_| "MCP state poisoned")?;
        let dead = match guard.get_mut(&name) {
            Some(session) => session
                .child
                .try_wait()
                .map(|status| status.is_some())
                .map_err(|e| format!("无法检查 MCP 服务状态: {e}"))?,
            None => return Ok(false),
        };
        if dead {
            guard.remove(&name);
            return Ok(false);
        }
        Ok(true)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stdio_client_handles_server_requests_pagination_calls_and_disconnect() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join("mcp_fixture.mjs");
        let session_name = format!("fixture-{}", std::process::id());
        let tools = mcp_connect(
            session_name.clone(),
            "node".into(),
            Some(vec![fixture.to_string_lossy().into_owned()]),
            None,
            Some(env!("CARGO_MANIFEST_DIR").into()),
        )
        .await
        .expect("fixture MCP server should connect");

        let names = tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["echo", "resource_echo"]);
        assert!(mcp_status(session_name.clone()).await.unwrap());

        let echo = mcp_call(
            session_name.clone(),
            "echo".into(),
            Some(json!({"text":"real MCP call"})),
        )
        .await
        .expect("echo tool should run");
        assert_eq!(echo, "real MCP call");

        let resource = mcp_call(session_name.clone(), "resource_echo".into(), None)
            .await
            .expect("resource tool should run");
        assert!(resource.contains("[resource fixture://proof]"));
        assert!(resource.contains("resource body"));

        mcp_disconnect(session_name.clone()).await.unwrap();
        assert!(!mcp_status(session_name).await.unwrap());
    }

    #[tokio::test]
    async fn startup_failure_includes_server_stderr() {
        let error = mcp_connect(
            format!("missing-fixture-{}", std::process::id()),
            "node".into(),
            Some(vec!["definitely-missing-mcp-server.mjs".into()]),
            None,
            Some(env!("CARGO_MANIFEST_DIR").into()),
        )
        .await
        .expect_err("missing server should fail");
        assert!(error.contains("MCP stderr"), "{error}");
        assert!(error.contains("MODULE_NOT_FOUND") || error.contains("Cannot find module"));
    }

    #[tokio::test]
    #[ignore = "downloads and calls the live official MCP memory server package"]
    async fn live_official_memory_server_connects_and_calls_a_tool() {
        let session_name = format!("official-memory-{}", std::process::id());
        let tools = mcp_connect(
            session_name.clone(),
            "npx".into(),
            Some(vec![
                "-y".into(),
                "@modelcontextprotocol/server-memory".into(),
            ]),
            None,
            Some(env!("CARGO_MANIFEST_DIR").into()),
        )
        .await
        .expect("official memory MCP server should connect");
        assert!(tools.iter().any(|tool| tool.name == "read_graph"));

        let graph = mcp_call(session_name.clone(), "read_graph".into(), None)
            .await
            .expect("read_graph should execute");
        assert!(!graph.trim().is_empty());

        mcp_disconnect(session_name).await.unwrap();
    }
}
