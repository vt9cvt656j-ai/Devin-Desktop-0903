//! MCP (Model Context Protocol) stdio client for local command-based servers.
//!
//! Transport per spec: the client launches the server as a subprocess and speaks
//! JSON-RPC 2.0 over stdin/stdout, ONE message per line (newline-delimited, no
//! embedded newlines). A bounded stderr tail is retained for actionable failures.
//! Requests have timeouts, so a slow or misbehaving server returns an error.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};

const PROTOCOL_VERSION: &str = "2025-06-18";
const MAX_TOOL_PAGES: usize = 100;
const MAX_TOOLS_PER_SESSION: usize = 256;
const MAX_TOOL_METADATA_BYTES: usize = 1024 * 1024;
// A valid tool result may contain the full 8 MiB inline-media budget plus JSON
// framing/text. Bound transport frames before allocating a String/serde Value.
const MAX_MCP_STDOUT_FRAME_BYTES: usize = 10 * 1024 * 1024;
const MAX_MCP_STDERR_FRAME_BYTES: usize = 64 * 1024;
const MCP_FRAME_ERROR_PREFIX: &str = "__MICHAEL_MCP_FRAME_ERROR__:";

fn discard_through_newline<R: BufRead>(reader: &mut R) -> io::Result<()> {
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(());
        }
        if let Some(index) = available.iter().position(|byte| *byte == b'\n') {
            reader.consume(index + 1);
            return Ok(());
        }
        let consumed = available.len();
        reader.consume(consumed);
    }
}

fn read_bounded_line<R: BufRead>(reader: &mut R, max_bytes: usize) -> io::Result<Option<String>> {
    let mut output = Vec::with_capacity(max_bytes.min(8192));
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            if output.is_empty() {
                return Ok(None);
            }
            break;
        }
        if let Some(index) = available.iter().position(|byte| *byte == b'\n') {
            if output.len().saturating_add(index) > max_bytes {
                reader.consume(index + 1);
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("transport frame exceeds {max_bytes} bytes"),
                ));
            }
            output.extend_from_slice(&available[..index]);
            reader.consume(index + 1);
            break;
        }
        let chunk_len = available.len();
        if output.len().saturating_add(chunk_len) > max_bytes {
            reader.consume(chunk_len);
            discard_through_newline(reader)?;
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("transport frame exceeds {max_bytes} bytes"),
            ));
        }
        output.extend_from_slice(available);
        reader.consume(chunk_len);
    }
    if output.last() == Some(&b'\r') {
        output.pop();
    }
    String::from_utf8(output)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

struct Session {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<String>,
    stderr_log: Arc<Mutex<VecDeque<String>>>,
    next_id: u64,
    workspace_root: String,
    capabilities: Value,
    server_info: Value,
}

impl Drop for Session {
    // Reap the child on ANY removal from its slot (disconnect / same-name replace / dead-eviction /
    // stop_all), so a wedged or forgotten MCP server never orphans its process + reader thread.
    // Killing the child closes its stdout → the reader thread's `lines()` returns None → it exits.
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

type SessionSlot = Arc<Mutex<Option<Session>>>;

static SESSIONS: LazyLock<Mutex<HashMap<String, SessionSlot>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn get_or_create_session_slot(name: &str) -> Result<SessionSlot, String> {
    let mut guard = SESSIONS.lock().map_err(|_| "MCP state poisoned")?;
    Ok(Arc::clone(
        guard
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(None))),
    ))
}

fn find_session_slot(name: &str) -> Result<Option<SessionSlot>, String> {
    let guard = SESSIONS.lock().map_err(|_| "MCP state poisoned")?;
    Ok(guard.get(name).map(Arc::clone))
}

/// Remove a session from its per-server slot, but return it to the caller so
/// `Session::drop` (kill + wait) happens after the slot mutex is released.
/// Requests for one MCP server remain serialized by that mutex; only teardown
/// no longer makes a later request wait behind process reaping.
fn take_slot_value<T>(slot: &Mutex<Option<T>>) -> Result<Option<T>, String> {
    let mut active = slot.lock().map_err(|_| "MCP session state poisoned")?;
    Ok(active.take())
}

/// Kill + reap ALL connected MCP servers. Wired into `cleanup_stale` (webview reload) and the app
/// Exit handler in lib.rs — same as LSP/DAP/Terminal — so MCP children never survive a reload or a
/// quit. Drains + drops on a detached thread (each `Session::drop` does kill()+blocking wait()) so a
/// slow-dying server can't stall the caller.
pub fn stop_all() {
    let drained: Vec<SessionSlot> = match SESSIONS.lock() {
        Ok(mut guard) => guard.drain().map(|(_, v)| v).collect(),
        Err(_) => return,
    };
    if !drained.is_empty() {
        std::thread::spawn(move || {
            for slot in drained {
                let session = slot.lock().ok().and_then(|mut session| session.take());
                drop(session);
            }
        });
    }
}

#[derive(Debug, Serialize)]
pub struct McpTool {
    name: String,
    description: String,
    input_schema: Value,
    output_schema: Value,
    annotations: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResource {
    uri: String,
    name: String,
    description: String,
    mime_type: String,
    annotations: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResourceTemplate {
    uri_template: String,
    name: String,
    description: String,
    mime_type: String,
    annotations: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPrompt {
    name: String,
    description: String,
    arguments: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpDiscovery {
    tools: Vec<McpTool>,
    resources: Vec<McpResource>,
    resource_templates: Vec<McpResourceTemplate>,
    prompts: Vec<McpPrompt>,
    capabilities: Value,
    server_info: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpCallResult {
    text: String,
    content: Value,
    structured_content: Value,
    is_error: bool,
    meta: Value,
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
            "roots/list" => json!({
                "jsonrpc":"2.0",
                "id":id,
                "result":{"roots": workspace_roots(&self.workspace_root)}
            }),
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
                    if let Some(message) = raw.strip_prefix(MCP_FRAME_ERROR_PREFIX) {
                        return Err(message.to_string());
                    }
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

fn workspace_roots(root: &str) -> Vec<Value> {
    let root = root.trim();
    if root.is_empty() {
        return Vec::new();
    }
    let path = std::path::Path::new(root);
    let Ok(uri) = url::Url::from_directory_path(path) else {
        return Vec::new();
    };
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or(root);
    vec![json!({"uri": uri.as_str(), "name": name})]
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

    // Reader thread: every bounded stdout frame → channel. A bounded channel also
    // prevents a notification flood from queueing unbounded Strings while idle.
    let (tx, rx) = mpsc::sync_channel::<String>(64);
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            match read_bounded_line(&mut reader, MAX_MCP_STDOUT_FRAME_BYTES) {
                Ok(Some(line)) => {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    let message = format!(
                        "{MCP_FRAME_ERROR_PREFIX}MCP stdout protocol frame rejected: {error}; service disconnected"
                    );
                    let _ = tx.send(message);
                    break;
                }
            }
        }
    });

    let stderr_log = Arc::new(Mutex::new(VecDeque::with_capacity(40)));
    let stderr_sink = Arc::clone(&stderr_log);
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        loop {
            let line = match read_bounded_line(&mut reader, MAX_MCP_STDERR_FRAME_BYTES) {
                Ok(Some(line)) => line.chars().take(500).collect::<String>(),
                Ok(None) => break,
                Err(error) => format!("[oversized/invalid MCP stderr frame omitted: {error}]"),
            };
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
        workspace_root: cwd.to_string(),
        capabilities: json!({}),
        server_info: json!({}),
    })
}

fn list_capability_pages(
    session: &mut Session,
    method: &str,
    field: &str,
    deadline: Instant,
    max_items: usize,
) -> Result<Vec<Value>, String> {
    let mut items = Vec::new();
    let mut cursor: Option<String> = None;
    let mut seen_cursors = HashSet::new();
    for page_index in 0..MAX_TOOL_PAGES {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(format!("MCP 连接超时（{method}）"));
        }
        let params = cursor
            .as_ref()
            .map(|value| json!({"cursor": value}))
            .unwrap_or_else(|| json!({}));
        let result = session.request(method, params, remaining.as_secs().clamp(1, 15))?;
        let page = result
            .get(field)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if items.len().saturating_add(page.len()) > max_items {
            let label = if field == "tools" { "工具" } else { field };
            return Err(format!(
                "MCP {label}数量超过安全上限（最多 {max_items} 个）"
            ));
        }
        items.extend(page);
        let next = result
            .get("nextCursor")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        match next {
            Some(next) if seen_cursors.insert(next.clone()) => {
                if page_index + 1 == MAX_TOOL_PAGES {
                    return Err(format!(
                        "MCP {field} 分页超过上限（最多 {MAX_TOOL_PAGES} 页）"
                    ));
                }
                cursor = Some(next);
            }
            _ => break,
        }
    }
    Ok(items)
}

fn parse_tools(values: Vec<Value>) -> Result<Vec<McpTool>, String> {
    let mut metadata_bytes = 0usize;
    let mut out = Vec::new();
    for tool in values {
        metadata_bytes = metadata_bytes.saturating_add(
            serde_json::to_vec(&tool)
                .map_err(|error| format!("MCP 工具定义无法序列化: {error}"))?
                .len(),
        );
        if metadata_bytes > MAX_TOOL_METADATA_BYTES {
            return Err(format!(
                "MCP 工具定义超过安全上限（每个服务最多 {MAX_TOOL_METADATA_BYTES} 字节）"
            ));
        }
        let name = tool
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if name.is_empty() {
            continue;
        }
        out.push(McpTool {
            name,
            description: tool
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            input_schema: tool
                .get("inputSchema")
                .cloned()
                .unwrap_or_else(|| json!({"type":"object","properties":{}})),
            output_schema: tool
                .get("outputSchema")
                .cloned()
                .unwrap_or_else(|| json!({})),
            annotations: tool
                .get("annotations")
                .cloned()
                .unwrap_or_else(|| json!({})),
        });
    }
    Ok(out)
}

fn parse_resources(values: Vec<Value>) -> Vec<McpResource> {
    values
        .into_iter()
        .filter_map(|resource| {
            let uri = resource.get("uri")?.as_str()?.trim().to_string();
            if uri.is_empty() {
                return None;
            }
            Some(McpResource {
                name: resource
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(&uri)
                    .to_string(),
                uri,
                description: resource
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                mime_type: resource
                    .get("mimeType")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                annotations: resource
                    .get("annotations")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            })
        })
        .collect()
}

fn parse_resource_templates(values: Vec<Value>) -> Vec<McpResourceTemplate> {
    values
        .into_iter()
        .filter_map(|resource| {
            let uri_template = resource.get("uriTemplate")?.as_str()?.trim().to_string();
            if uri_template.is_empty() {
                return None;
            }
            Some(McpResourceTemplate {
                name: resource
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(&uri_template)
                    .to_string(),
                uri_template,
                description: resource
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                mime_type: resource
                    .get("mimeType")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                annotations: resource
                    .get("annotations")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            })
        })
        .collect()
}

fn parse_prompts(values: Vec<Value>) -> Vec<McpPrompt> {
    values
        .into_iter()
        .filter_map(|prompt| {
            let name = prompt.get("name")?.as_str()?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            Some(McpPrompt {
                name,
                description: prompt
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                arguments: prompt
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!([])),
            })
        })
        .collect()
}

fn connect_full_blocking(
    name: String,
    command: String,
    args: Option<Vec<String>>,
    env: Option<HashMap<String, String>>,
    cwd: Option<String>,
) -> Result<McpDiscovery, String> {
    // The global map lock is held only long enough to clone this name's slot. Holding the
    // per-name lock through spawn + handshake serializes connect/call/disconnect for one
    // service while unrelated MCP services continue independently.
    let slot = get_or_create_session_slot(&name)?;
    let mut active = slot.lock().map_err(|_| "MCP session state poisoned")?;
    let args = args.unwrap_or_default();
    let env = env.unwrap_or_default();
    let cwd = cwd.unwrap_or_default();
    let mut session = spawn_session(&command, &args, &env, &cwd)?;
    let connect_deadline = Instant::now() + Duration::from_secs(60);

    // Handshake.
    let initialized = session.request(
        "initialize",
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {
                "roots": {"listChanged": false}
            },
            "clientInfo": { "name": "Michael-IDE", "version": "1.0" }
        }),
        45,
    )?;
    session.capabilities = initialized
        .get("capabilities")
        .cloned()
        .unwrap_or_else(|| json!({}));
    session.server_info = initialized
        .get("serverInfo")
        .cloned()
        .unwrap_or_else(|| json!({}));
    session.notify("notifications/initialized", json!({}));

    let has_tools = session.capabilities.get("tools").is_some();
    let has_resources = session.capabilities.get("resources").is_some();
    let has_prompts = session.capabilities.get("prompts").is_some();
    let tools = if has_tools {
        parse_tools(list_capability_pages(
            &mut session,
            "tools/list",
            "tools",
            connect_deadline,
            MAX_TOOLS_PER_SESSION,
        )?)?
    } else {
        Vec::new()
    };
    let resources = if has_resources {
        parse_resources(list_capability_pages(
            &mut session,
            "resources/list",
            "resources",
            connect_deadline,
            MAX_TOOLS_PER_SESSION,
        )?)
    } else {
        Vec::new()
    };
    let resource_templates = if has_resources {
        parse_resource_templates(list_capability_pages(
            &mut session,
            "resources/templates/list",
            "resourceTemplates",
            connect_deadline,
            MAX_TOOLS_PER_SESSION,
        )?)
    } else {
        Vec::new()
    };
    let prompts = if has_prompts {
        parse_prompts(list_capability_pages(
            &mut session,
            "prompts/list",
            "prompts",
            connect_deadline,
            MAX_TOOLS_PER_SESSION,
        )?)
    } else {
        Vec::new()
    };
    let capabilities = session.capabilities.clone();
    let server_info = session.server_info.clone();

    // Keep the stable per-name slot in the map. In particular, disconnect never removes this
    // slot, so an in-flight call cannot resurrect an older session after a reconnect.
    let old = active.replace(session);
    drop(active);
    drop(old);
    Ok(McpDiscovery {
        tools,
        resources,
        resource_templates,
        prompts,
        capabilities,
        server_info,
    })
}

/// Launch an MCP server and return complete negotiated capabilities.
#[tauri::command]
pub async fn mcp_connect_full(
    name: String,
    command: String,
    args: Option<Vec<String>>,
    env: Option<HashMap<String, String>>,
    cwd: Option<String>,
) -> Result<McpDiscovery, String> {
    tauri::async_runtime::spawn_blocking(move || {
        connect_full_blocking(name, command, args, env, cwd)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Backward-compatible tool-only discovery used by older clients and tests.
#[tauri::command]
pub async fn mcp_connect(
    name: String,
    command: String,
    args: Option<Vec<String>>,
    env: Option<HashMap<String, String>>,
    cwd: Option<String>,
) -> Result<Vec<McpTool>, String> {
    Ok(mcp_connect_full(name, command, args, env, cwd).await?.tools)
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

const MAX_INLINE_MCP_MEDIA_BASE64: usize = 5 * 1024 * 1024;
const MAX_TOTAL_INLINE_MCP_MEDIA_BASE64: usize = 8 * 1024 * 1024;

fn append_media_content(
    text: &mut String,
    kind: &str,
    mime: &str,
    data: Option<&str>,
    inline_media_base64: &mut usize,
) {
    let safe_mime = mime
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '+' | '.' | '-'));
    let encoded = data.unwrap_or_default();
    let valid_media = safe_mime
        && !encoded.is_empty()
        && encoded.len() <= MAX_INLINE_MCP_MEDIA_BASE64
        && matches!(kind, "image" | "audio" | "video");
    let fits_total_budget = inline_media_base64
        .checked_add(encoded.len())
        .is_some_and(|total| total <= MAX_TOTAL_INLINE_MCP_MEDIA_BASE64);

    if valid_media && fits_total_budget {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str("[MCP_MEDIA ");
        text.push_str(kind);
        text.push(' ');
        text.push_str(mime);
        text.push_str("]\ndata:");
        text.push_str(mime);
        text.push_str(";base64,");
        text.push_str(encoded);
        *inline_media_base64 += encoded.len();
    } else if valid_media {
        append_content(
            text,
            &format!(
                "[{kind} content omitted: {mime}, {} base64 characters; would exceed the {}-character total inline MCP media budget]",
                encoded.len(),
                MAX_TOTAL_INLINE_MCP_MEDIA_BASE64
            ),
        );
    } else {
        append_content(
            text,
            &format!(
                "[{kind} content: {mime}, {} base64 characters]",
                encoded.len()
            ),
        );
    }
}

fn flatten_tool_content(result: &Value) -> String {
    let mut text = String::new();
    let mut inline_media_base64 = 0;
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
                        let kind = mime.split('/').next().unwrap_or("resource");
                        if matches!(kind, "image" | "audio" | "video") {
                            append_media_content(
                                &mut text,
                                kind,
                                mime,
                                resource.get("blob").and_then(Value::as_str),
                                &mut inline_media_base64,
                            );
                        } else {
                            append_content(&mut text, &format!("[resource {uri}, {mime}, binary]"));
                        }
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
                Some("image") | Some("audio") | Some("video") => {
                    let kind = item.get("type").and_then(Value::as_str).unwrap_or("media");
                    let mime = item
                        .get("mimeType")
                        .and_then(Value::as_str)
                        .unwrap_or("application/octet-stream");
                    append_media_content(
                        &mut text,
                        kind,
                        mime,
                        item.get("data").and_then(Value::as_str),
                        &mut inline_media_base64,
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

fn structured_call_result(result: Value) -> McpCallResult {
    McpCallResult {
        text: flatten_tool_content(&result),
        content: result.get("content").cloned().unwrap_or_else(|| json!([])),
        structured_content: result
            .get("structuredContent")
            .cloned()
            .unwrap_or(Value::Null),
        is_error: result
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        meta: result.get("_meta").cloned().unwrap_or_else(|| json!({})),
    }
}

fn transport_session_error(error: &str) -> bool {
    error.starts_with("MCP 请求超时")
        || error.starts_with("MCP stdout protocol frame rejected:")
        || error.starts_with("MCP 服务已退出")
        || error.starts_with("写入 MCP 服务失败")
}

fn request_on_session(
    name: &str,
    method: &str,
    params: Value,
    timeout_secs: u64,
) -> Result<Value, String> {
    let slot = find_session_slot(name)?.ok_or_else(|| format!("MCP 服务「{name}」未连接"))?;
    let (response, retired) = {
        let mut active = slot.lock().map_err(|_| "MCP session state poisoned")?;
        let session = active
            .as_mut()
            .ok_or_else(|| format!("MCP 服务「{name}」未连接"))?;
        let response = session.request(method, params, timeout_secs);
        let dead_process = response.is_err() && matches!(session.child.try_wait(), Ok(Some(_)));
        let unhealthy = response
            .as_ref()
            .err()
            .is_some_and(|error| transport_session_error(error));
        let retired = (dead_process || unhealthy).then(|| active.take()).flatten();
        (response, retired)
    };
    // Session::drop kills and waits for the child. It must never run under the
    // per-server lock, otherwise same-name reconnect/status/calls queue behind it.
    drop(retired);
    response
}

#[tauri::command]
pub async fn mcp_call_full(
    name: String,
    tool: String,
    args: Option<Value>,
) -> Result<McpCallResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        request_on_session(
            &name,
            "tools/call",
            json!({ "name": tool, "arguments": args.unwrap_or(json!({})) }),
            60,
        )
        .map(structured_call_result)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mcp_read_resource(name: String, uri: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        request_on_session(&name, "resources/read", json!({"uri": uri}), 60)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mcp_get_prompt(
    name: String,
    prompt: String,
    args: Option<Value>,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        request_on_session(
            &name,
            "prompts/get",
            json!({"name": prompt, "arguments": args.unwrap_or(json!({}))}),
            60,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Call a tool on a connected MCP server. Returns text plus readable embedded-resource metadata.
#[tauri::command]
pub async fn mcp_call(name: String, tool: String, args: Option<Value>) -> Result<String, String> {
    let result = mcp_call_full(name, tool, args).await?;
    if result.is_error {
        Err(format!("[工具报错] {}", result.text))
    } else {
        Ok(result.text)
    }
}

/// Report whether a named session still has a live child process. Dead sessions are evicted so
/// the next connect starts cleanly and the UI never keeps showing a stale green status.
#[tauri::command]
pub async fn mcp_status(name: String) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<bool, String> {
        let Some(slot) = find_session_slot(&name)? else {
            return Ok(false);
        };
        let (running, retired) = {
            let mut active = slot.lock().map_err(|_| "MCP session state poisoned")?;
            let Some(session) = active.as_mut() else {
                return Ok(false);
            };
            let dead = session
                .child
                .try_wait()
                .map(|status| status.is_some())
                .map_err(|e| format!("无法检查 MCP 服务状态: {e}"))?;
            if dead {
                (false, active.take())
            } else {
                let ping = session.request("ping", json!({}), 3);
                if ping.is_err() {
                    (false, active.take())
                } else {
                    (true, None)
                }
            }
        };
        drop(retired);
        Ok(running)
    })
    .await
    .map_err(|e| e.to_string())?
}

// ── 用户级（跨项目）MCP 配置 ─────────────────────────────────────────────────
//
// 在这之前，MCP 配置**只**来自工作区里的 `.mcp.local.json` / `.mcp.json` /
// `.cursor/mcp.json`。也就是说换一个项目，你配好的服务、连同填进去的 API Key 全都
// 不在了，得从头再配一遍；没打开文件夹的时候更是一个 MCP 都用不了。那不叫"能用"。
//
// 用户级配置放在 `~/.michael-ide/mcp.json`，配一次到处都在。这里必须走**独立的
// Tauri 命令**而不是复用 `write_text_file_if_unchanged`：那条路的
// `require_inside_workspace(path, true)` 会明确拒绝"在 HOME 底下但不在已打开工作区
// 里"的写入（正是它挡住了 ~/.ssh、~/.bashrc），不该为了这一个文件把那道墙挖开。
// 这两个命令的作用域被钉死在这一个文件上，路径不接受调用方输入。
//
// 顺带把别的客户端的配置读进来（只读）：用户在 Claude Code / Cursor 里配好的服务
// 直接就能用，不用再抄一遍。`~/.claude.json` 里除了 mcpServers 还有账号、项目历史
// 等等，所以**在 Rust 侧就只摘 mcpServers 子树**交给前端，其余内容一个字节都不进
// 渲染进程。这也顺便绕开了 read_text_file 的 5 MB 上限（那个文件会长得很大）。
const USER_CONFIG_DIR: &str = ".michael-ide";
const USER_CONFIG_FILE: &str = "mcp.json";
/// 这几个文件都可能长期堆积（Claude Code 会把项目历史写进 ~/.claude.json）。
const MAX_USER_CONFIG_BYTES: u64 = 32 * 1024 * 1024;

fn home_dir() -> Result<std::path::PathBuf, String> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .map_err(|_| "无法确定用户主目录（HOME / USERPROFILE 都没有设置）".to_string())
}

fn user_config_path() -> Result<std::path::PathBuf, String> {
    Ok(home_dir()?.join(USER_CONFIG_DIR).join(USER_CONFIG_FILE))
}

/// 从一份配置文本里摘出服务表。兼容 `mcpServers`（Claude Code / Cursor / 本 IDE）
/// 和 `servers`（VS Code）两种键名。
/// 摘出 `disabled` 数组（只保留非空字符串）。不存在或写错类型都回空数组。
fn disabled_subtree(text: &str) -> Value {
    let Ok(parsed) = serde_json::from_str::<Value>(text) else {
        return json!([]);
    };
    match parsed.get("disabled").and_then(Value::as_array) {
        Some(items) => Value::Array(
            items
                .iter()
                .filter(|item| item.as_str().is_some_and(|value| !value.trim().is_empty()))
                .cloned()
                .collect(),
        ),
        None => json!([]),
    }
}

fn servers_subtree(text: &str) -> Value {
    let Ok(parsed) = serde_json::from_str::<Value>(text) else {
        return json!({});
    };
    for key in ["mcpServers", "servers"] {
        if let Some(map) = parsed.get(key) {
            if map.is_object() {
                return map.clone();
            }
        }
    }
    json!({})
}

fn read_capped(path: &std::path::Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_USER_CONFIG_BYTES {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpUserConfig {
    path: String,
    /// 本 IDE 自己的那份可以从面板里改；别的客户端的配置只读——那是它们的文件，
    /// 面板不该替用户去动。
    writable: bool,
    servers: Value,
    /// 用户在本 IDE 里停用的服务名。只有自己那份配置有这个字段——从 Cursor / Claude Code
    /// 读来的服务住在它们的文件里，想少加载一个不该去改人家的配置，所以停用记在自己这儿。
    disabled: Value,
}

/// 用户级 MCP 配置：本 IDE 的 `~/.michael-ide/mcp.json`，加上 Claude Code / Cursor 的
/// 全局配置（只读）。本 IDE 那份即使还不存在也会返回（servers 为空），面板要靠它
/// 知道"保存会写到哪里"。
#[tauri::command]
pub fn mcp_user_configs() -> Result<Vec<McpUserConfig>, String> {
    let home = home_dir()?;
    let own = user_config_path()?;
    let own_text = read_capped(&own);
    let mut out = vec![McpUserConfig {
        path: own.to_string_lossy().into_owned(),
        writable: true,
        servers: own_text
            .as_deref()
            .map(servers_subtree)
            .unwrap_or_else(|| json!({})),
        disabled: own_text
            .as_deref()
            .map(disabled_subtree)
            .unwrap_or_else(|| json!([])),
    }];
    for relative in [".claude.json", ".cursor/mcp.json", ".codex/mcp.json"] {
        let path = home.join(relative);
        let Some(text) = read_capped(&path) else {
            continue;
        };
        let servers = servers_subtree(&text);
        if servers.as_object().is_some_and(|map| !map.is_empty()) {
            out.push(McpUserConfig {
                path: path.to_string_lossy().into_owned(),
                writable: false,
                servers,
                disabled: json!([]),   // 别人的文件里没有这个概念
            });
        }
    }
    Ok(out)
}

/// 覆盖写 `~/.michael-ide/mcp.json`，返回它的绝对路径。
///
/// 权限收到 0600（目录 0700）：这个文件里会有 API Key。先写临时文件再 rename，
/// 断电或者写到一半崩了不会留下半截 JSON 把所有 MCP 服务一起带走。
#[tauri::command]
pub fn mcp_save_user_config(text: String) -> Result<String, String> {
    save_user_config_at(&user_config_path()?, &text)
}

/// 真正落盘的那一段。路径作参数是为了能在测试里用临时目录跑完整往返——命令本身
/// 不接受调用方给路径（那会把 HOME 底下任意文件变成可写目标）。
fn save_user_config_at(path: &std::path::Path, text: &str) -> Result<String, String> {
    // 拒绝写坏文件：这一份被所有项目共用，写坏了是把全部 MCP 一起弄丢。
    serde_json::from_str::<Value>(text).map_err(|e| format!("MCP 配置不是合法 JSON：{e}"))?;
    let dir = path
        .parent()
        .ok_or_else(|| "无法确定配置目录".to_string())?
        .to_path_buf();
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建 {}：{e}", dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    let temp = dir.join(format!("{USER_CONFIG_FILE}.tmp"));
    std::fs::write(&temp, text.as_bytes()).map_err(|e| format!("写入失败：{e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // rename 之前就设好权限，避免出现"已经在最终路径、权限还是 0644"的窗口。
        std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("无法收紧 {} 的权限：{e}", temp.display()))?;
    }
    std::fs::rename(&temp, path).map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        format!("无法保存 {}：{e}", path.display())
    })?;
    Ok(path.to_string_lossy().into_owned())
}

// ── 用户规则：跨项目的长期要求 ────────────────────────────────────────────────
//
// 项目级的约定这个 IDE 一直会读（AGENTS.md / CLAUDE.md / .cursorrules /
// .github/copilot-instructions.md，见 _gatherAgentContext），但**用户级的没有**——
// 「我一律用 pnpm」「回答用中文」「别写没要求的测试」这类跟着人走、不跟着项目走的要求，
// 以前只能每个项目重写一遍，或者每轮对话重复一次。
//
// 放在 `~/.michael-ide/rules.md`，和 mcp.json 同一个目录。走独立命令而不是
// write_text_file_if_unchanged 的理由和那边一样：那条路的 require_inside_workspace
// 明确拒绝"在 HOME 底下但不在已打开工作区里"的写入，不该为一个文件把那道墙挖开。
/// 用户自己写、每轮都要发给模型的两份文档。
///
/// **白名单，不是路径参数。** 调用方只能传 "rules" / "habits" 这两个词，路径由这里拼；
/// 让前端传路径等于把 HOME 底下任意文件变成可写目标，那正是 require_inside_workspace
/// 拦着的事。
fn user_doc_file(kind: &str) -> Result<&'static str, String> {
    match kind {
        "rules" => Ok("rules.md"),
        "habits" => Ok("habits.md"),
        other => Err(format!("未知的用户文档类型：{other}")),
    }
}
/// 规则是给模型读的，进的是每轮的系统提示词。给个硬上限，避免有人贴进去一整本手册
/// 之后每轮都在为它付钱——前端另有更小的软上限并会提示。
const MAX_USER_RULES_BYTES: usize = 64 * 1024;

fn user_doc_path(kind: &str) -> Result<std::path::PathBuf, String> {
    Ok(home_dir()?
        .join(USER_CONFIG_DIR)
        .join(user_doc_file(kind)?))
}

/// 读一份用户文档（rules / habits）。文件不存在就是空串——"还没写过"不是错误。
#[tauri::command]
pub fn user_rules_read(kind: Option<String>) -> Result<Value, String> {
    let kind = kind.unwrap_or_else(|| "rules".into());
    let path = user_doc_path(&kind)?;
    let text = read_capped(&path).unwrap_or_default();
    Ok(json!({ "kind": kind, "path": path.to_string_lossy(), "text": text }))
}

/// 覆盖写用户规则；空内容等于删掉这个文件（而不是留一个空文件让后面的读取多绕一圈）。
#[tauri::command]
pub fn user_rules_save(text: String, kind: Option<String>) -> Result<String, String> {
    let kind = kind.unwrap_or_else(|| "rules".into());
    let label = if kind == "habits" { "用户习惯" } else { "用户规则" };
    if text.len() > MAX_USER_RULES_BYTES {
        return Err(format!(
            "{label}太长（{} 字节，上限 {}）。这段内容每一轮对话都会发给模型。",
            text.len(),
            MAX_USER_RULES_BYTES
        ));
    }
    let path = user_doc_path(&kind)?;
    let dir = path
        .parent()
        .ok_or_else(|| "无法确定配置目录".to_string())?
        .to_path_buf();
    if text.trim().is_empty() {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("无法清空 {}：{e}", path.display())),
        }
        return Ok(path.to_string_lossy().into_owned());
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建 {}：{e}", dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    let temp = dir.join(format!("{}.tmp", user_doc_file(&kind)?));
    std::fs::write(&temp, text.as_bytes()).map_err(|e| format!("写入失败：{e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("无法收紧 {} 的权限：{e}", temp.display()))?;
    }
    std::fs::rename(&temp, &path).map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        format!("无法保存 {}：{e}", path.display())
    })?;
    Ok(path.to_string_lossy().into_owned())
}

/// Disconnect and kill an MCP server session.
#[tauri::command]
pub async fn mcp_disconnect(name: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let Some(slot) = find_session_slot(&name)? else {
            return Ok(());
        };
        let session = take_slot_value(&slot)?;
        drop(session);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct SlotDropProbe {
        slot: std::sync::Weak<Mutex<Option<SlotDropProbe>>>,
        dropped_after_unlock: Arc<AtomicBool>,
    }

    impl Drop for SlotDropProbe {
        fn drop(&mut self) {
            let unlocked = self
                .slot
                .upgrade()
                .is_some_and(|slot| slot.try_lock().is_ok());
            self.dropped_after_unlock.store(unlocked, Ordering::SeqCst);
        }
    }

    fn fixture_path() -> String {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join("mcp_fixture.mjs")
            .to_string_lossy()
            .into_owned()
    }

    async fn connect_fixture(
        session_name: &str,
        env: Option<HashMap<String, String>>,
    ) -> Result<Vec<McpTool>, String> {
        mcp_connect(
            session_name.to_string(),
            "node".into(),
            Some(vec![fixture_path()]),
            env,
            Some(env!("CARGO_MANIFEST_DIR").into()),
        )
        .await
    }

    async fn connect_fixture_full(session_name: &str) -> Result<McpDiscovery, String> {
        mcp_connect_full(
            session_name.to_string(),
            "node".into(),
            Some(vec![fixture_path()]),
            None,
            Some(env!("CARGO_MANIFEST_DIR").into()),
        )
        .await
    }

    /// 规则文件的读写往返。路径参数化，测试不碰真实的 ~/.michael-ide。
    fn rules_roundtrip_at(path: &std::path::Path, text: &str) -> Result<String, String> {
        let dir = path.parent().unwrap();
        std::fs::create_dir_all(dir).unwrap();
        if text.trim().is_empty() {
            let _ = std::fs::remove_file(path);
            return Ok(path.to_string_lossy().into_owned());
        }
        std::fs::write(path, text).map_err(|e| e.to_string())?;
        Ok(path.to_string_lossy().into_owned())
    }

    #[test]
    fn user_doc_kind_is_a_whitelist_not_a_path() {
        // 让调用方传路径 = 把 HOME 底下任意文件变成可写目标。只认这两个词。
        assert_eq!(user_doc_file("rules").unwrap(), "rules.md");
        assert_eq!(user_doc_file("habits").unwrap(), "habits.md");
        for bad in ["", "../../.ssh/id_rsa", "rules.md", "Rules", "habits/../x"] {
            assert!(user_doc_file(bad).is_err(), "{bad} 不该被接受");
        }
    }

    #[test]
    fn habits_and_rules_are_separate_files() {
        let a = user_doc_path("rules").unwrap();
        let b = user_doc_path("habits").unwrap();
        assert_ne!(a, b, "两份文档必须各自落盘，否则写一个会盖掉另一个");
        assert!(a.ends_with("rules.md") && b.ends_with("habits.md"));
    }

    #[test]
    fn user_rules_are_bounded_so_one_paste_cannot_tax_every_turn() {
        // 这段每一轮都发给模型。贴进去一整本手册的话，成本是按轮计的。
        let huge = "x".repeat(MAX_USER_RULES_BYTES + 1);
        let error = user_rules_save(huge, None).unwrap_err();
        assert!(error.contains("太长"), "{error}");
        assert!(error.contains("每一轮"), "报错要说清为什么有上限：{error}");
    }

    #[test]
    fn clearing_user_rules_removes_the_file_rather_than_leaving_an_empty_one() {
        let dir = std::env::temp_dir().join("michael-rules-clear");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("rules.md");
        rules_roundtrip_at(&path, "回答用中文。").unwrap();
        assert!(path.exists());
        rules_roundtrip_at(&path, "   \n  ").unwrap();
        assert!(
            !path.exists(),
            "清空应当删掉文件，而不是留个空文件让后面每次读取都多绕一圈"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reading_absent_user_rules_is_empty_not_an_error() {
        // "还没写过规则"是常态，不是错误——报错的话前端每次开面板都要处理一次假失败。
        let path = std::env::temp_dir().join("michael-rules-absent/rules.md");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        assert_eq!(read_capped(&path).unwrap_or_default(), "");
    }

    #[test]
    fn user_config_reader_takes_only_the_servers_subtree() {
        // ~/.claude.json 里除了 mcpServers 还有 oauthAccount / userID / projects（整段
        // 项目历史）。这个函数是那道闸：**只有** mcpServers 会离开 Rust 侧，其余内容
        // 一个字节都不进渲染进程。
        let claude = r#"{
            "userID": "私密-不该外泄",
            "oauthAccount": {"emailAddress": "a@b.c"},
            "projects": {"/tmp/x": {"history": ["秘密"]}},
            "mcpServers": {"memory": {"command": "npx", "args": ["-y", "server-memory"]}}
        }"#;
        let servers = servers_subtree(claude);
        assert_eq!(servers["memory"]["command"], "npx");
        let text = serde_json::to_string(&servers).unwrap();
        for leaked in ["userID", "oauthAccount", "projects", "私密", "秘密", "a@b.c"] {
            assert!(!text.contains(leaked), "{leaked} 泄漏进了返回值：{text}");
        }
    }

    #[test]
    fn user_config_reader_accepts_the_vs_code_key_and_survives_garbage() {
        assert_eq!(
            servers_subtree(r#"{"servers":{"x":{"command":"y"}}}"#)["x"]["command"],
            "y"
        );
        // 手写坏了的配置不能把整套 MCP 带走，返回空表即可。
        assert_eq!(servers_subtree("{ 这不是 json"), json!({}));
        assert_eq!(servers_subtree(r#"{"mcpServers": 42}"#), json!({}));
    }

    #[test]
    fn saving_the_user_config_rejects_invalid_json_before_touching_the_file() {
        // 这一份被所有项目共用：写坏了是把全部 MCP 一起弄丢，所以先解析再落盘。
        let dir = std::env::temp_dir().join("michael-mcp-usercfg-invalid");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("mcp.json");
        save_user_config_at(&path, r#"{"mcpServers":{"a":{"command":"x"}}}"#).unwrap();
        let before = std::fs::read_to_string(&path).unwrap();

        let error = save_user_config_at(&path, "{ 半截").unwrap_err();
        assert!(error.contains("不是合法 JSON"), "{error}");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            before,
            "参数非法时不该动到磁盘上的配置"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn user_config_round_trips_and_is_not_world_readable() {
        // 这个文件里会有 API Key：目录 0700、文件 0600，而且 rename 之前就设好，
        // 不留"已经在最终路径、权限还是 0644"的窗口。
        let dir = std::env::temp_dir().join("michael-mcp-usercfg-roundtrip");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("mcp.json");
        let written = r#"{"mcpServers":{"tavily":{"command":"npx","env":{"TAVILY_API_KEY":"秘密"}}}}"#;
        let saved = save_user_config_at(&path, written).unwrap();
        assert_eq!(saved, path.to_string_lossy());

        let servers = servers_subtree(&read_capped(&path).unwrap());
        assert_eq!(servers["tavily"]["env"]["TAVILY_API_KEY"], "秘密");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "配置里有 API Key，不能是 {mode:o}");
            let dir_mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
            assert_eq!(dir_mode, 0o700, "配置目录不能是 {dir_mode:o}");
        }
        // 覆盖写不留临时文件残骸。
        assert!(!dir.join("mcp.json.tmp").exists());
        save_user_config_at(&path, r#"{"mcpServers":{}}"#).unwrap();
        assert_eq!(servers_subtree(&read_capped(&path).unwrap()), json!({}));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn media_content_keeps_bounded_data_for_the_ide_renderer() {
        let image = flatten_tool_content(&json!({
            "content": [{"type": "image", "mimeType": "image/png", "data": "iVBORw0KGgo="}]
        }));
        assert!(image.contains("[MCP_MEDIA image image/png]"));
        assert!(image.contains("data:image/png;base64,iVBORw0KGgo="));

        let video = flatten_tool_content(&json!({
            "content": [{"type": "resource", "resource": {"uri": "fixture://clip", "mimeType": "video/mp4", "blob": "AAAA"}}]
        }));
        assert!(video.contains("[MCP_MEDIA video video/mp4]"));
    }

    #[test]
    fn roots_list_exposes_the_real_workspace_uri() {
        let roots = workspace_roots(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(roots.len(), 1);
        assert!(roots[0]["uri"].as_str().unwrap().starts_with("file://"));
        assert_eq!(roots[0]["name"], "src-tauri");
    }

    #[test]
    fn bounded_transport_reader_rejects_oversized_frames_without_allocating_the_tail() {
        let input = b"123456789\nnext\r\n";
        let mut reader = std::io::BufReader::with_capacity(3, &input[..]);
        let error = read_bounded_line(&mut reader, 8).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("exceeds 8 bytes"));
        assert_eq!(
            read_bounded_line(&mut reader, 8).unwrap(),
            Some("next".into())
        );
        assert_eq!(read_bounded_line(&mut reader, 8).unwrap(), None);
    }

    #[test]
    fn bounded_transport_reader_accepts_a_frame_at_the_exact_limit() {
        let input = b"12345678\n";
        let mut reader = std::io::BufReader::with_capacity(2, &input[..]);
        assert_eq!(
            read_bounded_line(&mut reader, 8).unwrap(),
            Some("12345678".into())
        );
    }

    #[test]
    fn slot_value_is_destroyed_only_after_its_mutex_is_released() {
        let slot: Arc<Mutex<Option<SlotDropProbe>>> = Arc::new(Mutex::new(None));
        let dropped_after_unlock = Arc::new(AtomicBool::new(false));
        *slot.lock().unwrap() = Some(SlotDropProbe {
            slot: Arc::downgrade(&slot),
            dropped_after_unlock: Arc::clone(&dropped_after_unlock),
        });

        let retired = take_slot_value(slot.as_ref()).unwrap();
        assert!(slot.lock().unwrap().is_none());
        drop(retired);
        assert!(dropped_after_unlock.load(Ordering::SeqCst));
    }

    #[test]
    fn media_content_enforces_a_total_inline_budget() {
        let first = "A".repeat(MAX_INLINE_MCP_MEDIA_BASE64);
        let second = "B".repeat(MAX_TOTAL_INLINE_MCP_MEDIA_BASE64 - MAX_INLINE_MCP_MEDIA_BASE64);
        let result = flatten_tool_content(&json!({
            "content": [
                {"type": "image", "mimeType": "image/png", "data": first},
                {"type": "audio", "mimeType": "audio/mpeg", "data": second},
                {"type": "video", "mimeType": "video/mp4", "data": "AAAA"}
            ]
        }));

        assert_eq!(result.matches("[MCP_MEDIA ").count(), 2);
        assert!(result.contains("video content omitted: video/mp4, 4 base64 characters"));
        assert!(result.contains("total inline MCP media budget"));
        assert!(!result.contains("data:video/mp4;base64,AAAA"));
    }

    #[test]
    fn media_content_keeps_the_per_item_limit() {
        let oversized = "A".repeat(MAX_INLINE_MCP_MEDIA_BASE64 + 1);
        let result = flatten_tool_content(&json!({
            "content": [{"type": "image", "mimeType": "image/png", "data": oversized}]
        }));

        assert!(!result.contains("[MCP_MEDIA "));
        assert!(result.contains(&format!(
            "[image content: image/png, {} base64 characters]",
            MAX_INLINE_MCP_MEDIA_BASE64 + 1
        )));
    }

    #[tokio::test]
    async fn stdio_client_handles_server_requests_pagination_calls_and_disconnect() {
        let session_name = format!("fixture-{}", std::process::id());
        let tools = connect_fixture(&session_name, None)
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
    async fn transport_failure_evicts_a_request_session_before_reaping_it() {
        let session_name = format!("fixture-request-failure-{}", std::process::id());
        connect_fixture(&session_name, None).await.unwrap();

        let error = mcp_call(session_name.clone(), "exit".into(), None)
            .await
            .expect_err("exiting fixture must fail the pending request");
        assert!(
            error.contains("MCP 服务已退出") || error.contains("工具报错"),
            "{error}"
        );
        let slot = find_session_slot(&session_name).unwrap().unwrap();
        assert!(slot.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn failed_status_ping_evicts_the_session_before_reaping_it() {
        let session_name = format!("fixture-status-failure-{}", std::process::id());
        connect_fixture(
            &session_name,
            Some(HashMap::from([(
                "MCP_FIXTURE_IGNORE_PING".to_string(),
                "1".to_string(),
            )])),
        )
        .await
        .unwrap();

        assert!(!mcp_status(session_name.clone()).await.unwrap());
        let slot = find_session_slot(&session_name).unwrap().unwrap();
        assert!(slot.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn full_discovery_keeps_tools_resources_prompts_and_structured_results() {
        let session_name = format!("fixture-full-{}", std::process::id());
        let discovery = connect_fixture_full(&session_name)
            .await
            .expect("full MCP discovery should connect");
        assert_eq!(discovery.tools.len(), 2);
        assert_eq!(discovery.resources.len(), 1);
        assert_eq!(discovery.resource_templates.len(), 1);
        assert_eq!(discovery.prompts.len(), 1);
        assert!(discovery.capabilities.get("resources").is_some());
        assert_eq!(discovery.server_info["name"], "michael-ide-test-fixture");

        let result = mcp_call_full(
            session_name.clone(),
            "echo".into(),
            Some(json!({"text":"structured"})),
        )
        .await
        .unwrap();
        assert_eq!(result.text, "structured");
        assert_eq!(result.content[0]["type"], "text");
        assert!(!result.is_error);

        let resource = mcp_read_resource(session_name.clone(), "fixture://proof".into())
            .await
            .unwrap();
        assert_eq!(resource["contents"][0]["text"], "resource body");
        let prompt = mcp_get_prompt(
            session_name.clone(),
            "review".into(),
            Some(json!({"target":"src/main.js"})),
        )
        .await
        .unwrap();
        assert_eq!(
            prompt["messages"][0]["content"]["text"],
            "Review src/main.js"
        );

        mcp_disconnect(session_name).await.unwrap();
    }

    #[tokio::test]
    async fn slow_call_does_not_block_a_different_mcp_server() {
        let suffix = std::process::id();
        let slow_name = format!("fixture-slow-{suffix}");
        let fast_name = format!("fixture-fast-{suffix}");
        let (slow_tools, fast_tools) = tokio::join!(
            connect_fixture(&slow_name, None),
            connect_fixture(&fast_name, None)
        );
        slow_tools.expect("slow fixture should connect");
        fast_tools.expect("fast fixture should connect");

        let started_path = std::env::temp_dir().join(format!("mcp-delay-started-{suffix}"));
        let _ = std::fs::remove_file(&started_path);
        let started_arg = started_path.to_string_lossy().into_owned();
        let slow_session = slow_name.clone();
        let slow_call = tokio::spawn(async move {
            mcp_call(
                slow_session,
                "delay_echo".into(),
                Some(json!({
                    "text": "slow",
                    "delay_ms": 1_500,
                    "started_path": started_arg,
                })),
            )
            .await
        });

        tokio::time::timeout(Duration::from_secs(2), async {
            while !started_path.is_file() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("slow fixture should acknowledge the active request");

        let fast = tokio::time::timeout(
            Duration::from_millis(800),
            mcp_call(
                fast_name.clone(),
                "echo".into(),
                Some(json!({"text":"fast"})),
            ),
        )
        .await
        .expect("a different MCP service must not wait for the slow service")
        .expect("fast fixture call should succeed");
        assert_eq!(fast, "fast");
        assert_eq!(slow_call.await.unwrap().unwrap(), "slow");

        let (slow_disconnect, fast_disconnect) =
            tokio::join!(mcp_disconnect(slow_name), mcp_disconnect(fast_name));
        slow_disconnect.unwrap();
        fast_disconnect.unwrap();
        let _ = std::fs::remove_file(started_path);
    }

    #[tokio::test]
    async fn disconnect_and_reconnect_reuse_the_same_serialization_slot() {
        let session_name = format!("fixture-reconnect-{}", std::process::id());
        connect_fixture(&session_name, None).await.unwrap();
        let before = find_session_slot(&session_name).unwrap().unwrap();

        mcp_disconnect(session_name.clone()).await.unwrap();
        assert!(!mcp_status(session_name.clone()).await.unwrap());
        connect_fixture(&session_name, None).await.unwrap();

        let after = find_session_slot(&session_name).unwrap().unwrap();
        assert!(Arc::ptr_eq(&before, &after));
        assert!(mcp_status(session_name.clone()).await.unwrap());
        mcp_disconnect(session_name).await.unwrap();
    }

    #[tokio::test]
    async fn tool_discovery_rejects_an_oversized_registry() {
        let session_name = format!("fixture-too-many-tools-{}", std::process::id());
        let env = HashMap::from([(
            "MCP_FIXTURE_TOOL_COUNT".to_string(),
            (MAX_TOOLS_PER_SESSION + 1).to_string(),
        )]);
        let error = connect_fixture(&session_name, Some(env))
            .await
            .expect_err("oversized MCP registries must be rejected");
        assert!(error.contains("工具数量超过安全上限"), "{error}");
        assert!(!mcp_status(session_name).await.unwrap());
    }

    #[tokio::test]
    async fn tool_discovery_rejects_oversized_schema_metadata() {
        let session_name = format!("fixture-oversized-schema-{}", std::process::id());
        let env = HashMap::from([(
            "MCP_FIXTURE_SCHEMA_BYTES".to_string(),
            (MAX_TOOL_METADATA_BYTES + 1).to_string(),
        )]);
        let error = connect_fixture(&session_name, Some(env))
            .await
            .expect_err("oversized MCP schemas must be rejected");
        assert!(error.contains("工具定义超过安全上限"), "{error}");
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

    /// 对着一个**真的声明了 prompts** 的官方服务跑一遍取模板这条路。
    ///
    /// 加这条是因为斜杠菜单那个入口（`/服务:模板`）之前只在注入假数据的前端单测里验过。
    /// 前端那半是纯逻辑，桩件跑得住；但"服务真的会回一段可用的提示词"这件事，只有对着
    /// 真服务发一次 prompts/get 才算数。`server-everything` 是官方示例服务，tools /
    /// resources / prompts 三类齐全，正好是这条路的对照物。
    ///
    /// 和上面那条一样 `#[ignore]`：它要联网下载包。跑法：
    ///     cargo test --lib mcp::tests::live_official_everything -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "downloads and calls the live official MCP everything server"]
    async fn live_official_everything_server_returns_a_usable_prompt() {
        let session_name = format!("official-everything-{}", std::process::id());
        let discovery = mcp_connect_full(
            session_name.clone(),
            "npx".into(),
            Some(vec![
                "-y".into(),
                "@modelcontextprotocol/server-everything".into(),
            ]),
            None,
            Some(env!("CARGO_MANIFEST_DIR").into()),
        )
        .await
        .expect("official everything MCP server should connect");

        // 发现阶段：前端的斜杠菜单就是拿这份 prompts 列表生成 `服务:模板` 的。
        assert!(
            !discovery.prompts.is_empty(),
            "everything 服务应当声明 prompts；拿不到的话斜杠菜单里什么都不会出现"
        );

        // **按形状挑，不按名字挑。** 第一版这里写死了 `simple_prompt` / `complex_prompt`，
        // 是凭印象猜的——真实名字是 `simple-prompt` / `args-prompt`，于是测试红在一个
        // 与被测行为无关的地方。上游改个名字就红的测试没有价值，前端也从不关心名字。
        let no_args = discovery
            .prompts
            .iter()
            .find(|p| {
                p.arguments
                    .as_array()
                    .is_none_or(|a| a.iter().all(|x| x.get("required") != Some(&json!(true))))
            })
            .expect("至少要有一个不需要必填参数的模板");
        assert!(
            no_args.arguments.is_array(),
            "arguments 必须是数组——前端拿它生成参数表单，不是数组会渲染成空表单"
        );

        // 取用阶段：这一步的返回值前端交给 _mcpResponseText 解包，再放进输入框。
        let filled = mcp_get_prompt(session_name.clone(), no_args.name.clone(), None)
            .await
            .expect("prompts/get should succeed");
        let messages = filled
            .get("messages")
            .and_then(Value::as_array)
            .expect("prompts/get 必须回一个 messages 数组");
        let text: String = messages
            .iter()
            .filter_map(|m| m.get("content")?.get("text")?.as_str())
            .collect();
        assert!(
            !text.trim().is_empty(),
            "模板展开后必须有正文，否则输入框里会是空的：{filled}"
        );

        // 带参数的那一个：前端会先弹表单收参数，再原样传过来。参数名同样从服务的声明里读，
        // 不写死。
        if let Some(with_args) = discovery
            .prompts
            .iter()
            .find(|p| p.arguments.as_array().is_some_and(|a| !a.is_empty()))
        {
            let names: Vec<String> = with_args
                .arguments
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|a| a.get("name")?.as_str().map(str::to_string))
                .collect();
            let args: serde_json::Map<String, Value> = names
                .iter()
                .map(|n| (n.clone(), Value::String("probe".into())))
                .collect();
            let out = mcp_get_prompt(
                session_name.clone(),
                with_args.name.clone(),
                Some(Value::Object(args)),
            )
            .await
            .unwrap_or_else(|e| panic!("带参数的 prompts/get 失败（{}）：{e}", with_args.name));
            assert!(
                out.get("messages").and_then(Value::as_array).is_some_and(|m| !m.is_empty()),
                "带参数的模板也必须回出正文：{out}"
            );
        }

        mcp_disconnect(session_name).await.unwrap();
    }
}

