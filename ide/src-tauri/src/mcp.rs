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
    if dead_process || unhealthy {
        active.take();
    }
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
        let mut active = slot.lock().map_err(|_| "MCP session state poisoned")?;
        let dead = match active.as_mut() {
            Some(session) => session
                .child
                .try_wait()
                .map(|status| status.is_some())
                .map_err(|e| format!("无法检查 MCP 服务状态: {e}"))?,
            None => return Ok(false),
        };
        if dead {
            active.take();
            return Ok(false);
        }
        let ping = active
            .as_mut()
            .expect("checked above")
            .request("ping", json!({}), 3);
        if ping.is_err() {
            active.take();
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
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let Some(slot) = find_session_slot(&name)? else {
            return Ok(());
        };
        let session = slot
            .lock()
            .map_err(|_| "MCP session state poisoned")?
            .take();
        drop(session);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
