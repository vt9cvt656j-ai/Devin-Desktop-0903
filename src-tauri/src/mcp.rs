//! MCP (Model Context Protocol) stdio client for local command-based servers.
//!
//! Transport per spec: the client launches the server as a subprocess and speaks
//! JSON-RPC 2.0 over stdin/stdout, ONE message per line (newline-delimited, no
//! embedded newlines). A bounded stderr tail is retained for actionable failures.
//! Requests have timeouts, so a slow or misbehaving server returns an error.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};

/// 改环境变量的用例和读它的用例必须排队：`std::env` 是**进程级**的，而 Rust 默认多线程
/// 跑测试——一个用例把 MCP_TIMEOUT 设上，另一个正好在断言默认预算，就是一次偶发红，
/// 而且每次红的不是同一条，最难查的那一类。
#[cfg(test)]
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 离开作用域就把这两个变量清掉——**包括断言失败 panic 出去的那条路**。少了它，
/// 一条失败的用例会把脏环境留给后面所有用例，红的看起来像是别人。
#[cfg(test)]
struct ClearTimeoutEnv;
#[cfg(test)]
impl Drop for ClearTimeoutEnv {
    fn drop(&mut self) {
        unsafe {
            std::env::remove_var("MCP_TIMEOUT");
            std::env::remove_var("MCP_TOOL_TIMEOUT");
        }
    }
}

const PROTOCOL_VERSION: &str = "2025-06-18";
/// 这个客户端认得的协议版本，新到旧。
///
/// `initialize` 是一次**协商**：客户端报自己想要的版本，服务回它实际会用的那个，两边不
/// 一定相同。以前这个回值一个字都没看过——服务回什么都当没事发生，然后带着一套它根本
/// 不认的请求形状往下走。故障因此全部推迟到后面某次 `tools/call`，报出来的是
/// "Invalid params" 之类看不出所以然的话，而真正的原因（版本对不上）在握手那一刻就已经
/// 明明白白写在回包里了。
const SUPPORTED_PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

/// 服务在 `initialize` 里回的版本能不能用。
///
/// 不声明版本的照旧放行：规范要求回，但确实有服务不回，而这一条既没证据说它不兼容、
/// 也不值得为此拒绝一次本来能用的连接。真正要拦的是**明确回了一个我们不认的版本**——
/// 那是唯一一种"继续往下走一定会以别的方式失败"的情况，早报一步，报的还是真原因。
///
/// 这里不把版本存下来：目前没有任何一处按版本分叉行为，存了就是死字段。等到真有分叉
/// 的那天再连着它的消费者一起加。
fn check_protocol_version(initialized: &Value) -> Result<(), String> {
    let Some(version) = initialized
        .get("protocolVersion")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|version| !version.is_empty())
    else {
        return Ok(());
    };
    if SUPPORTED_PROTOCOL_VERSIONS.contains(&version) {
        return Ok(());
    }
    Err(format!(
        "MCP 协议版本对不上：服务要用 {version}，这个客户端支持 {}。请把该服务升级或降级到其中一个版本。",
        SUPPORTED_PROTOCOL_VERSIONS.join(" / ")
    ))
}
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

/// 连续多少次「发出去了、一个字都没回来」才认定这个服务是真卡死了，可以杀。
///
/// 单次超时**不是**死亡证明：它只说明子进程此刻还没答——可能正在跑一个慢工具、正在装依赖、
/// 正在加载一个大索引。真死了有更硬的证据（`child.try_wait()` 能拿到退出码），所以这个计数器
/// 只负责兜住剩下那种「活着但永远不回话」的情况。取 3 而不是 1，是因为 1 就等于旧行为——
/// 一次慢调用顺手把服务进程杀掉，用户看到的是「点了一下，服务自己没了」。
const MAX_CONSECUTIVE_TIMEOUTS: u32 = 3;

/// 这条请求超时后能不能发 `notifications/cancelled`。
///
/// 抽成独立函数是为了能测：唯一的例外（`initialize`）要靠一次 45 秒的握手超时才能在集成
/// 测试里走到，那种测试没人会跑，等于这条规范约束没有守卫。
fn cancellable_request(method: &str) -> bool {
    method != "initialize"
}

/// 等回应的时候多久睁一次眼。
///
/// `recv_timeout(remaining)` 一睡就是一整段预算，中间谁把取消标志立起来都看不见——用户点了
/// 「停」，还得等那 60 秒走完才停得下来，那不叫取消。切成小片的代价是一秒醒五次（一次原子读），
/// 换来的是点下去就停。
const CANCEL_POLL: Duration = Duration::from_millis(200);

/// 被用户取消时返回的错误串。
///
/// **开头绝不能撞上 `transport_session_error` 认的那几个前缀。** 撞上了就是「取消一次 = 服务
/// 被杀一次」，而取消的全部意义正是把服务留着继续用。
const CANCELLED_ERROR: &str = "MCP 请求已取消";

/// 服务端诊断和 stderr 共用的那条尾巴的上限。给人看的，不是日志文件。
const MAX_SERVER_LOG_LINES: usize = 40;
const MAX_SERVER_LOG_LINE_CHARS: usize = 500;

/// 往那条尾巴里塞一行；返回 false 表示这条尾巴已经废了（锁中毒），调用方该收手。
///
/// 服务自己的诊断（`notifications/message`）和它的 stderr 走**同一条**尾巴：这个模块没有
/// `AppHandle`/`Emitter`（它不是 Tauri State，也不知道自己属于哪个窗口），为它单开一条事件
/// 通道等于凭空多出一份生命周期要维护；而用户要的只是「出事时能看到服务说了什么」，两种来源
/// 按时间混在一起反而更像一份日志。
fn push_server_log(sink: &Mutex<VecDeque<String>>, line: String) -> bool {
    let Ok(mut log) = sink.lock() else {
        return false;
    };
    if log.len() >= MAX_SERVER_LOG_LINES {
        log.pop_front();
    }
    log.push_back(line.chars().take(MAX_SERVER_LOG_LINE_CHARS).collect());
    true
}

/// 一条服务端发来的帧被怎么消化的。
enum Absorbed {
    /// **我们这条**请求的进度：静默预算重新计时。
    OurProgress,
    /// 记下了（清单变更 / 服务日志 / 不是我们这条的进度），不动预算。
    Noted,
    /// 不是我们认识的通知——交回给 `respond_to_server_request`，它可能是服务反过来问我们的请求。
    Passthrough,
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
    /// 连着几次请求超时没等到回应。**任何**一次回应都清零，包括服务回的 JSON-RPC 错误——
    /// 那也是「它还在听、还在答」的证据，只是答得不合心意。
    consecutive_timeouts: u32,
    /// 「把手上这条丢掉」的开关。
    ///
    /// 它**必须**放在会话这把 Mutex 之外（这里存的只是一个共享句柄，真正的值在
    /// `SIDE_CHANNELS` 里）：要取消的那条请求此刻正握着这把锁，`mcp_cancel` 再去抢锁，
    /// 只会一直等到它自己结束——等于什么都没取消。
    cancel: Arc<AtomicBool>,
    /// 服务自己宣告「我的清单变了」。没有独立的读线程能收这些通知（单个 `Receiver` 归
    /// `Session` 独占，后台再起一个读线程会把在飞的响应偷走），所以通知只能靠某条请求
    /// 把它从管道里读进来——每轮的 `mcp_status` ping 就是那一下。
    ///
    /// **但 ping 只负责读进来，取走标志的是 `mcp_take_changes` 自己**：`status_at` 从头到尾
    /// 不碰这三个字段。这句话以前写成「由 ping 排空」，是假的——而正因为没人排空，
    /// 一个宣告过清单变化的服务（比如登录后才长出工具的远程服务）在整根重连之前，
    /// 新工具永远不会出现。
    tools_changed: bool,
    resources_changed: bool,
    prompts_changed: bool,
    /// 远端服务正在等用户在浏览器里完成授权：initialize 超时了，但子进程还活着。
    /// 这种会话不能杀（杀了 mcp-remote 的回调服务器就没了，令牌永远落不到 ~/.mcp-auth），
    /// 也不能当正常会话去 ping（握手还没完成，发别的请求是不合规的）。
    awaiting_auth: bool,
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

/// 会话表的键：**窗口 + 根目录 + 服务名**，三段缺一不可。
///
/// 以前只拿服务名当键。可 `filesystem` / `github` / `memory` 这些名字，两个项目十有八九取得
/// 一模一样——于是打开第二个项目时 `connect_full_blocking` 会直接替换掉同名槽位：第一个项目的
/// 服务进程被 `Session::drop` 杀掉，而它那边还在飞的调用会落到第二个项目的进程上，用的是别人的
/// cwd、别人的 env、别人的数据。前端只能靠「根目录一变就整片 [BLOCKED]」硬挡，代价是开了第二个
/// 项目标签页，第一个就废了。
///
/// 窗口标签由 Tauri 在命令里注入，前端一个字都不用管；根目录由前端给（已归一化；**空串是合法
/// 值**，意思是「没开文件夹」——那时用户级的服务照样得能跑，不是「参数没传」）。名字放最后，是
/// 为了保住「同键即替换」这条 `Session::drop` 赖以成立的语义：**不要**把启动参数指纹塞进键里，
/// 那会让每次改配置都留下一个再也没人回收的旧槽位和一个活着的孤儿进程。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SessionKey {
    window: String,
    root: String,
    name: String,
}

impl SessionKey {
    fn new(window: &str, root: &str, name: &str) -> Self {
        SessionKey {
            window: window.to_string(),
            root: root.to_string(),
            name: name.to_string(),
        }
    }

    /// 「没连上」这句话对用户只有服务名有意义——窗口标签和根目录是内部分区，说出来只会让人
    /// 以为是自己配错了服务名。
    fn missing(&self) -> String {
        format!("MCP 服务「{}」未连接", self.name)
    }
}

static SESSIONS: LazyLock<Mutex<HashMap<SessionKey, SessionSlot>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 一个服务身上**不经过会话锁**就能碰到的两样东西。
///
/// 故意和 `SESSIONS` 分开，因为这两件事都必须在「一条请求正握着会话锁」的时候还能做到：
/// - 取消开关：要取消的正是那条请求，去抢锁只会等到它自己结束，等于没取消；
/// - 服务日志：服务卡住的时候才最需要看它说了什么，而那时候锁正被卡住的那条请求占着。
///   顺带还有一个好处——连接失败时会话根本没建起来，但这条尾巴还在，面板照样看得到
///   服务启动时喊了什么。
#[derive(Clone)]
struct SideChannel {
    cancel: Arc<AtomicBool>,
    log: Arc<Mutex<VecDeque<String>>>,
}

static SIDE_CHANNELS: LazyLock<Mutex<HashMap<SessionKey, SideChannel>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn get_or_create_session_slot(key: &SessionKey) -> Result<SessionSlot, String> {
    let mut guard = SESSIONS.lock().map_err(|_| "MCP state poisoned")?;
    Ok(Arc::clone(
        guard
            .entry(key.clone())
            .or_insert_with(|| Arc::new(Mutex::new(None))),
    ))
}

fn find_session_slot(key: &SessionKey) -> Result<Option<SessionSlot>, String> {
    let guard = SESSIONS.lock().map_err(|_| "MCP state poisoned")?;
    Ok(guard.get(key).map(Arc::clone))
}

fn side_channel(key: &SessionKey) -> Result<SideChannel, String> {
    let mut guard = SIDE_CHANNELS.lock().map_err(|_| "MCP state poisoned")?;
    Ok(guard
        .entry(key.clone())
        .or_insert_with(|| SideChannel {
            cancel: Arc::new(AtomicBool::new(false)),
            log: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_SERVER_LOG_LINES))),
        })
        .clone())
}

/// 已经登记过的那份；没登记过就是没连过这个服务。
fn find_side_channel(key: &SessionKey) -> Option<SideChannel> {
    SIDE_CHANNELS.lock().ok()?.get(key).cloned()
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
    if let Ok(mut side) = SIDE_CHANNELS.lock() {
        side.clear();
    }
    reap_detached(drained);
}

/// 关掉一个窗口时，收掉**只属于这个窗口**的 MCP 会话。
///
/// 以前 lib.rs 一个 `on_window_event` 都没注册：关掉一个副窗口，它拉起来的 MCP 子进程会一直
/// 活到整个 App 退出——用户开开关关几次项目窗口，机器上就堆着一串没人用的 node/uvx 进程。
/// 按窗口分区之后这个洞更大了：每个窗口都有自己的一整套会话，泄漏是成倍的。
///
/// `cleanup_stale` 补不了这个：它按**进程**收尸，多窗口时会连别的窗口正在用的一起杀掉，所以
/// 那边多窗口直接跳过（见 lib.rs 里的说明）。只有键里带着窗口标签，才能只收这一个窗口的。
pub fn stop_window(label: &str) {
    let drained: Vec<SessionSlot> = match SESSIONS.lock() {
        Ok(mut guard) => {
            let doomed: Vec<SessionKey> = guard
                .keys()
                .filter(|key| key.window == label)
                .cloned()
                .collect();
            doomed.iter().filter_map(|key| guard.remove(key)).collect()
        }
        Err(_) => return,
    };
    if let Ok(mut side) = SIDE_CHANNELS.lock() {
        side.retain(|key, _| key.window != label);
    }
    reap_detached(drained);
}

/// 把会话从槽位里取出来、在**另一条线程**上扔掉。
///
/// 每一次 `Session::drop` 都是 kill + 阻塞 wait，串起来足够让窗口关闭的回调卡住肉眼可见的一段
/// 时间；而这个回调跑在事件循环上，卡住的是整个 App。取值和丢弃分开也保住了「不在锁里析构」
/// 这条纪律（见 `take_slot_value`）。
fn reap_detached(drained: Vec<SessionSlot>) {
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

    /// 超时之后，告诉服务「这条别做了」。
    ///
    /// 没有这一条的话，客户端这边已经放弃、服务那边还在跑：一次误点的 30 秒查询会继续占着
    /// 它的连接和配额，而结果回来时我们只会当成陌生 id 丢掉。规范里 `notifications/cancelled`
    /// 就是干这个的。
    ///
    /// **`initialize` 例外，规范明令禁止取消它。** 而且握手这条路的超时报错和别处长得一样
    /// （`MCP 请求超时（initialize）`，不是 `MCP 连接超时`——后者只在 `list_capability_pages`
    /// 整体截止时出现），所以靠错误串区分是不成立的，只能在这里按方法名挡住。
    fn cancel_in_flight(&mut self, method: &str, id: u64, reason: &str) {
        if !cancellable_request(method) {
            return;
        }
        self.notify(
            "notifications/cancelled",
            json!({"requestId": id, "reason": reason}),
        );
    }

    /// 把一条服务端来的帧消化掉：进度、清单变更、服务自己的日志。
    ///
    /// 这些以前**全被丢掉**（`request` 只认两种帧：我们的回应，和服务反过来问我们的请求），
    /// 于是没有任何东西能延长一次长任务的等待、能发现工具列表变了、能让用户看到服务自己的
    /// 诊断——服务说什么都等于没说。
    fn absorb(&mut self, frame: &Value, id: u64) -> Absorbed {
        match frame.get("method").and_then(Value::as_str).unwrap_or("") {
            "notifications/progress" => {
                // **令牌必须对上。** 一条已经被放弃的旧请求可能还在报进度，让它一直把当前
                // 这条的静默预算续下去，等于这条永远不会超时。没带令牌的进度也不算数——
                // 归不到具体哪条请求上（本仓库的 fixture 就一直在发这种，
                // `a_call_that_outruns_its_budget_leaves_the_same_child_usable` 靠它还得
                // 能正常超时）。
                let token = frame.get("params").and_then(|params| params.get("progressToken"));
                let ours = token.and_then(Value::as_u64) == Some(id)
                    // 有些服务把令牌回成字符串（JS 那边 Number → String 很容易发生）。
                    || token
                        .and_then(Value::as_str)
                        .and_then(|text| text.parse::<u64>().ok())
                        == Some(id);
                if ours {
                    Absorbed::OurProgress
                } else {
                    Absorbed::Noted
                }
            }
            "notifications/tools/list_changed" => {
                self.tools_changed = true;
                Absorbed::Noted
            }
            "notifications/resources/list_changed" => {
                self.resources_changed = true;
                Absorbed::Noted
            }
            "notifications/prompts/list_changed" => {
                self.prompts_changed = true;
                Absorbed::Noted
            }
            "notifications/message" => {
                self.record_server_message(frame);
                Absorbed::Noted
            }
            _ => Absorbed::Passthrough,
        }
    }

    /// 服务自己报的一行诊断。`data` 按规范可以是任意 JSON，不只是字符串。
    fn record_server_message(&self, frame: &Value) {
        let params = frame.get("params");
        let level = params
            .and_then(|params| params.get("level"))
            .and_then(Value::as_str)
            .unwrap_or("info");
        let logger = params
            .and_then(|params| params.get("logger"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let body = match params.and_then(|params| params.get("data")) {
            Some(Value::String(text)) => text.clone(),
            Some(other) => other.to_string(),
            None => String::new(),
        };
        let line = if logger.is_empty() {
            format!("[MCP {level}] {body}")
        } else {
            format!("[MCP {level}] {logger}: {body}")
        };
        push_server_log(&self.stderr_log, line);
    }

    /// Send a JSON-RPC request and read lines until the matching response arrives
    /// or the deadline passes. Notifications and server→client requests are skipped.
    fn request(&mut self, method: &str, params: Value, timeout_secs: u64) -> Result<Value, String> {
        self.request_inner(method, params, timeout_secs, timeout_secs, false)
    }

    /// 会报进度的请求：`idle_secs` 是「多久没动静就放弃」，每收到一条本请求的进度就重新计时；
    /// `max_secs` 是无论如何都不再等下去的总长。
    ///
    /// **只有 `tools/call` 走这条。** `initialize` / `ping` / `tools/list` 那套分页的预算是固定
    /// 的：它们要么必须很快（握手、心跳），要么由 `list_capability_pages` 的总截止时间管着，
    /// 给它们开一个能被服务自己推后的预算，等于把连接卡死的可能性拱手让出去。
    fn request_with_progress(
        &mut self,
        method: &str,
        params: Value,
        idle_secs: u64,
        max_secs: u64,
    ) -> Result<Value, String> {
        self.request_inner(method, params, idle_secs, max_secs, true)
    }

    fn request_inner(
        &mut self,
        method: &str,
        mut params: Value,
        idle_secs: u64,
        max_secs: u64,
        wants_progress: bool,
    ) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        // 令牌只能在这儿铸：id 是这一层才有的东西。用 id 本身当令牌，服务把它原样回在
        // `notifications/progress` 里，我们据此认出「这条进度是我这条请求的」。
        if wants_progress {
            if let Value::Object(map) = &mut params {
                if let Value::Object(meta) = map.entry("_meta").or_insert_with(|| json!({})) {
                    meta.insert("progressToken".into(), json!(id));
                }
            }
        }
        // 上一次点的「停」不能顺延到这一条：请求可能刚好在点下去之前就返回了，标志留着的话
        // 下一条请求会当场被取消，而用户什么都没做。清在**发出去之前**——清在之后的话，正好
        // 落在这中间的那一次取消会被自己抹掉，用户点了没反应。
        self.cancel.store(false, Ordering::SeqCst);

        let msg = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        let line = serde_json::to_string(&msg).map_err(|e| e.to_string())?;
        self.stdin
            .write_all(line.as_bytes())
            .and_then(|_| self.stdin.write_all(b"\n"))
            .and_then(|_| self.stdin.flush())
            .map_err(|e| format!("写入 MCP 服务失败: {e}"))?;

        let mut deadline = Instant::now() + Duration::from_secs(idle_secs);
        // 静默预算可以被进度一次次推后，总时长不行：一个每秒报一次进度、永远不给结果的服务
        // 会把这一轮永远吊在这里。
        let hard_deadline = Instant::now() + Duration::from_secs(max_secs.max(idle_secs));
        loop {
            // 每一圈都看一眼取消标志——服务话多的时候可能一直有帧进来，永远走不到下面那个
            // 超时分支。
            if self.cancel.swap(false, Ordering::SeqCst) {
                // 规范禁止取消 initialize，但「不再等」是我们自己的事，只是不发通知。
                self.cancel_in_flight(method, id, "user cancelled");
                return Err(format!("{CANCELLED_ERROR}（{method}）"));
            }
            let remaining = deadline
                .min(hard_deadline)
                .saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                self.consecutive_timeouts += 1;
                self.cancel_in_flight(method, id, "client timeout");
                return Err(self.error_with_stderr(&format!("MCP 请求超时（{method}）")));
            }
            match self.rx.recv_timeout(remaining.min(CANCEL_POLL)) {
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
                        match self.absorb(&v, id) {
                            // 服务还在干活，只是还没干完——静默预算从现在重新算。
                            Absorbed::OurProgress => {
                                deadline = Instant::now() + Duration::from_secs(idle_secs);
                            }
                            Absorbed::Noted => {}
                            Absorbed::Passthrough => self.respond_to_server_request(&v),
                        }
                        continue;
                    }
                    let matches = v.get("id").and_then(|i| i.as_u64()) == Some(id);
                    if !matches {
                        continue;
                    }
                    // 它答了——不管答的是结果还是错误，「通道是通的」这件事已经成立，
                    // 之前攒下的超时计数就此清零（见 MAX_CONSECUTIVE_TIMEOUTS）。
                    self.consecutive_timeouts = 0;
                    if let Some(err) = v.get("error") {
                        return Err(jsonrpc_error_text(err));
                    }
                    return Ok(v.get("result").cloned().unwrap_or(Value::Null));
                }
                // 切片到期 ≠ 预算到期。回到圈首去看取消标志和真正的截止时间。
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
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

/// 服务回的 JSON-RPC 错误，摊平成一行给用户看的话。
///
/// 以前只取 `message`，于是 `-32602` 那一类的具体原因（哪个参数不对、schema 差在哪）全在
/// `data` 里被扔掉了，用户和模型看到的只有一句「Invalid arguments」，没法据此改。
///
/// **不往这里贴 stderr 尾巴。** 服务答了话就说明它是活的，而那条尾巴里多半是启动时的噪音；
/// 把它钉在一个 `-32602` 后面只会让真正的原因更难找（`error_with_stderr` 是给「一个字都没
/// 回来」的那几种失败用的）。开头保持服务原文，不加中文前缀——`transport_session_error`
/// 靠前缀分类，这里绝不能碰上。
fn jsonrpc_error_text(error: &Value) -> String {
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("MCP error");
    let mut text = message.to_string();
    if let Some(code) = error.get("code").and_then(Value::as_i64) {
        text.push_str(&format!("（错误码 {code}）"));
    }
    match error.get("data") {
        None | Some(Value::Null) => {}
        Some(Value::String(data)) => text.push_str(&format!("\n详情：{data}")),
        Some(data) => text.push_str(&format!("\n详情：{data}")),
    }
    text
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
    // 那条日志尾巴由调用方（`SIDE_CHANNELS`）持有：连接失败时这里根本不会有 Session 交出去，
    // 而恰恰是那时候用户最需要看服务在启动时喊了什么。
    stderr_log: Arc<Mutex<VecDeque<String>>>,
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

    let stderr_sink = Arc::clone(&stderr_log);
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        loop {
            let line = match read_bounded_line(&mut reader, MAX_MCP_STDERR_FRAME_BYTES) {
                Ok(Some(line)) => line,
                Ok(None) => break,
                Err(error) => format!("[oversized/invalid MCP stderr frame omitted: {error}]"),
            };
            // 服务通过 `notifications/message` 报的诊断进的是同一条尾巴，所以截断和上限
            // 也归 push_server_log 一处管（见那里）。
            if !push_server_log(&stderr_sink, line) {
                break;
            }
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
        consecutive_timeouts: 0,
        // 真正的开关由 connect 按键装进来；spawn 这一层还不知道自己会被挂到哪个键上。
        cancel: Arc::new(AtomicBool::new(false)),
        tools_changed: false,
        resources_changed: false,
        prompts_changed: false,
        awaiting_auth: false,
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

/// 本地服务的握手预算。装依赖、加载索引都在这段里，45 秒是既有的经验值。
const LOCAL_HANDSHAKE_SECS: u64 = 45;
/// 远端服务的握手预算。
///
/// 远端 MCP 是靠 `npx mcp-remote` 桥出去的，而它**第一次**连接会打开浏览器让用户完成 OAuth：
/// 在用户点完之前，`initialize` 根本不会有回音。45 秒里没人读得完一页授权说明，于是必然超时；
/// 更糟的是超时之后会话被丢弃、`Session::drop` 顺手杀掉 mcp-remote，它正开着的 OAuth 回调
/// 服务器也跟着没了——用户在浏览器里点了「同意」，回调打到一个已经关掉的端口，令牌永远写不进
/// `~/.mcp-auth`，下次连还是从头再来。所以远端要给足人操作的时间，超时了也不能杀。
const REMOTE_HANDSHAKE_SECS: u64 = 180;

/// 环境变量给的超时，换算成秒。
///
/// 单位跟着 Claude Code 走：`MCP_TIMEOUT` / `MCP_TOOL_TIMEOUT` 都是**毫秒**。同一个变量名
/// 在两个工具里差 1000 倍的话，用户照着文档填的 30000 会变成八小时。
///
/// 空的、非数字、0 都当没设：0 的字面意思是"立刻超时"，没人是这个意思，而认下来的话
/// 每一次请求都会当场失败，比不认更难查。
///
/// # 生效范围：从终端启动才读得到
///
/// 读的是**本进程**的环境。macOS 上从 Dock / Finder 启动的 GUI 进程不继承登录 shell 的
/// 环境，所以写在 `.zshrc` 里的这两个变量在打包版里是看不见的——`open -a` 或双击启动时
/// 这个开关等于没有。从终端跑（`cargo tauri dev`、或者直接执行 .app 里的可执行文件）时
/// 正常生效；想让 GUI 启动也认，得 `launchctl setenv MCP_TIMEOUT 300000` 再重登录。
///
/// 没走 `process_util::login_shell_env()` 是有意的：那个函数每次调用都要真起一个登录
/// shell（预算 4 秒），而且**故意不缓存**——它返回的表里装着用户的真密钥，多留一份就多
/// 一份泄漏面。为了两个整数让每个 GUI 用户在第一次连 MCP 时多等最多 4 秒，绝大多数人还
/// 根本没设过这两个变量，这笔账不划算。
///
/// 也没有落到 `~/.michael-ide/mcp.json` 里：面板保存走的是"从投影重建整份文档"那条路
/// （见 `save_user_config_at`），投影里没有的键会被静默删掉——用户在面板里改一次服务，
/// 手写的超时就没了。真要做成配置项，得连读写投影一起改，不是加一个键的事。
fn env_timeout_secs(name: &str) -> Option<u64> {
    let millis: u64 = std::env::var(name).ok()?.trim().parse().ok()?;
    (millis > 0).then(|| millis.div_ceil(1000).max(1))
}

fn handshake_budget(remote: bool) -> u64 {
    env_timeout_secs("MCP_TIMEOUT").unwrap_or(if remote {
        REMOTE_HANDSHAKE_SECS
    } else {
        LOCAL_HANDSHAKE_SECS
    })
}

fn connect_full_blocking(
    key: SessionKey,
    command: String,
    args: Option<Vec<String>>,
    env: Option<HashMap<String, String>>,
    cwd: Option<String>,
    remote: bool,
) -> Result<McpDiscovery, String> {
    connect_full_within(key, command, args, env, cwd, remote, handshake_budget(remote))
}

/// 握手预算作参数的理由和 `save_user_config_at` 的路径一样：只为了能验。「远端超时但进程还
/// 活着 ⇒ 把会话留在槽位里等授权」这条如果只能靠真的等满 180 秒才走得到，那它等于没有守卫。
/// 命令那一层不接受调用方给预算——预算由 `handshake_budget` 按 remote 决定。
fn connect_full_within(
    key: SessionKey,
    command: String,
    args: Option<Vec<String>>,
    env: Option<HashMap<String, String>>,
    cwd: Option<String>,
    remote: bool,
    handshake_secs: u64,
) -> Result<McpDiscovery, String> {
    // The global map lock is held only long enough to clone this key's slot. Holding the
    // per-key lock through spawn + handshake serializes connect/call/disconnect for one
    // service while unrelated MCP services continue independently.
    let slot = get_or_create_session_slot(&key)?;
    let mut active = slot.lock().map_err(|_| "MCP session state poisoned")?;
    let args = args.unwrap_or_default();
    let env = env.unwrap_or_default();
    let cwd = cwd.unwrap_or_default();
    let side = side_channel(&key)?;
    // 上一轮留下的取消标志必须清掉：否则「上次点了取消、请求已经先一步结束」会让新会话的
    // 第一条请求当场被取消掉，而用户什么都没点。上一个子进程留下的日志同理——新起了一个
    // 进程还混着上一个的输出，只会让人对着一份不属于它的报错找原因。
    side.cancel.store(false, Ordering::SeqCst);
    if let Ok(mut log) = side.log.lock() {
        log.clear();
    }
    let mut session = spawn_session(&command, &args, &env, &cwd, Arc::clone(&side.log))?;
    session.cancel = Arc::clone(&side.cancel);
    // 发现阶段的总截止时间要盖得住握手：远端握手本身就能占掉三分钟。
    let connect_deadline =
        Instant::now() + Duration::from_secs(handshake_secs.saturating_add(60));

    // Handshake.
    let initialized = match session.request(
        "initialize",
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {
                "roots": {"listChanged": false}
            },
            "clientInfo": { "name": "Michael-IDE", "version": "1.0" }
        }),
        handshake_secs,
    ) {
        Ok(initialized) => initialized,
        Err(error) => {
            // 远端 + 超时 + 子进程还活着 = 它多半正开着浏览器等用户点同意。把会话**原样留在
            // 槽位里**（一旦让它掉出作用域就是 kill，OAuth 回调服务器随之关闭，令牌落不了盘），
            // 面板拿 mcp_pending_auth 轮询；用户点完之后重连，mcp-remote 直接读 ~/.mcp-auth
            // 里的令牌，那一次握手会很快过去。
            if remote
                && error.starts_with("MCP 请求超时")
                && matches!(session.child.try_wait(), Ok(None))
            {
                session.awaiting_auth = true;
                let old = active.replace(session);
                drop(active);
                drop(old);
                return Err(format!(
                    "MCP 服务「{}」正在等待浏览器授权：请在弹出的页面里完成登录，然后重新连接。",
                    key.name
                ));
            }
            return Err(error);
        }
    };
    // 版本对不上就在这里停：往下走一定会以别的方式失败，而那时候已经看不出真原因了。
    check_protocol_version(&initialized)?;
    session.capabilities = initialized
        .get("capabilities")
        .cloned()
        .unwrap_or_else(|| json!({}));
    session.server_info = initialized
        .get("serverInfo")
        .cloned()
        .unwrap_or_else(|| json!({}));
    session.notify("notifications/initialized", json!({}));
    // 服务**声明了** logging 才去设级别：没声明的会规规矩矩回一条 -32601，白多一次往返，
    // 还在诊断里留下一条与真实故障无关的报错。结果不看——设不上只是少一些诊断信息，
    // 不该让一次本来能成的连接失败。
    if session.capabilities.get("logging").is_some() {
        let _ = session.request("logging/setLevel", json!({"level": "info"}), 5);
    }

    let discovery = discover_capabilities(&mut session, connect_deadline)?;

    // Keep the stable per-key slot in the map. In particular, disconnect never removes this
    // slot, so an in-flight call cannot resurrect an older session after a reconnect.
    let old = active.replace(session);
    drop(active);
    drop(old);
    Ok(discovery)
}

/// 把服务现在声明的工具 / 资源 / 模板 / 提示词全列一遍。
///
/// 抽出来是因为 `mcp_rediscover` 要在**同一个已连接的会话**上重跑这一整段：服务发来
/// `*/list_changed` 之后，要拿到的正是这份新清单。两边共用一份实现，能力位的判断（服务没声明
/// tools 就不去问 tools/list，问了会拿到 -32601）也就不会长歪。
fn discover_capabilities(
    session: &mut Session,
    deadline: Instant,
) -> Result<McpDiscovery, String> {
    let has_tools = session.capabilities.get("tools").is_some();
    let has_resources = session.capabilities.get("resources").is_some();
    let has_prompts = session.capabilities.get("prompts").is_some();
    let tools = if has_tools {
        parse_tools(list_capability_pages(
            session,
            "tools/list",
            "tools",
            deadline,
            MAX_TOOLS_PER_SESSION,
        )?)?
    } else {
        Vec::new()
    };
    let resources = if has_resources {
        parse_resources(list_capability_pages(
            session,
            "resources/list",
            "resources",
            deadline,
            MAX_TOOLS_PER_SESSION,
        )?)
    } else {
        Vec::new()
    };
    let resource_templates = if has_resources {
        parse_resource_templates(list_capability_pages(
            session,
            "resources/templates/list",
            "resourceTemplates",
            deadline,
            MAX_TOOLS_PER_SESSION,
        )?)
    } else {
        Vec::new()
    };
    let prompts = if has_prompts {
        parse_prompts(list_capability_pages(
            session,
            "prompts/list",
            "prompts",
            deadline,
            MAX_TOOLS_PER_SESSION,
        )?)
    } else {
        Vec::new()
    };
    Ok(McpDiscovery {
        tools,
        resources,
        resource_templates,
        prompts,
        capabilities: session.capabilities.clone(),
        server_info: session.server_info.clone(),
    })
}

/// Launch an MCP server and return complete negotiated capabilities.
#[tauri::command]
pub async fn mcp_connect_full(
    window: tauri::WebviewWindow,
    name: String,
    root: String,
    command: String,
    args: Option<Vec<String>>,
    env: Option<HashMap<String, String>>,
    cwd: Option<String>,
    // 远端服务（`npx mcp-remote` 桥接）第一次连接要等用户在浏览器里授权，握手预算和本地
    // 完全不是一个量级。缺省 false = 按本地服务对待，前端不传也不会比现在差。
    remote: Option<bool>,
) -> Result<McpDiscovery, String> {
    connect_full_at(
        session_key(&window, &root, &name),
        command,
        args,
        env,
        cwd,
        remote.unwrap_or(false),
    )
    .await
}

/// 窗口标签只能在**进 `spawn_blocking` 之前**取出来：`WebviewWindow` 不是 `Send`，把它带进
/// 闭包连编译都过不去。所有 MCP 命令都在这里把窗口收敛成一个 `String`。
fn session_key(window: &tauri::WebviewWindow, root: &str, name: &str) -> SessionKey {
    SessionKey::new(window.label(), root, name)
}

async fn connect_full_at(
    key: SessionKey,
    command: String,
    args: Option<Vec<String>>,
    env: Option<HashMap<String, String>>,
    cwd: Option<String>,
    remote: bool,
) -> Result<McpDiscovery, String> {
    tauri::async_runtime::spawn_blocking(move || {
        connect_full_blocking(key, command, args, env, cwd, remote)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Backward-compatible tool-only discovery used by older clients and tests.
#[tauri::command]
pub async fn mcp_connect(
    window: tauri::WebviewWindow,
    name: String,
    root: String,
    command: String,
    args: Option<Vec<String>>,
    env: Option<HashMap<String, String>>,
    cwd: Option<String>,
    remote: Option<bool>,
) -> Result<Vec<McpTool>, String> {
    Ok(connect_full_at(
        session_key(&window, &root, &name),
        command,
        args,
        env,
        cwd,
        remote.unwrap_or(false),
    )
    .await?
    .tools)
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

/// 这个错误意味着**通道本身没了**，而不是「服务答了，只是答得不合心意」。只有前者才该把
/// 整个会话连同子进程一起收掉。
///
/// 超时曾经也算在这里，是错的：超时只说明子进程**还没**答，它可能正在跑一个慢工具。于是
/// 一次慢调用就会顺手杀掉服务进程，用户眼里是「点了一下，服务就没了」，而且下一次调用还得
/// 从头 spawn + 握手。真的退出了有更硬的证据——`child.try_wait()` 拿得到退出码；真的卡死
/// 不回话则由 `consecutive_timeouts` 数到 `MAX_CONSECUTIVE_TIMEOUTS` 收场。
fn transport_session_error(error: &str) -> bool {
    error.starts_with("MCP stdout protocol frame rejected:")
        || error.starts_with("MCP 服务已退出")
        || error.starts_with("写入 MCP 服务失败")
}

/// 一次工具调用「多久没动静就算它不干了」。
const TOOL_CALL_IDLE_SECS: u64 = 60;
/// 无论服务多勤快地报进度，一次调用最多占住这么久。
///
/// 没有这道硬顶的话，一个每秒报一次进度、永远不给结果的服务可以把这一轮永远吊住——用户既
/// 等不到结果，也等不到报错。
const TOOL_CALL_MAX_SECS: u64 = 600;

/// 一次工具调用的（空闲, 硬顶）预算。`MCP_TOOL_TIMEOUT` 只给硬顶。
fn tool_call_budget() -> (u64, u64) {
    match env_timeout_secs("MCP_TOOL_TIMEOUT") {
        // 空闲判定不能比硬顶还长：那样硬顶总是先到，"多久没动静就算它不干了"这条
        // 一次都不会触发，用户把上限调小反而要等得更久。
        Some(max) => (TOOL_CALL_IDLE_SECS.min(max), max),
        None => (TOOL_CALL_IDLE_SECS, TOOL_CALL_MAX_SECS),
    }
}

fn call_on_session(key: &SessionKey, tool: String, args: Option<Value>) -> Result<Value, String> {
    let (idle_secs, max_secs) = tool_call_budget();
    on_session(key, |session| {
        session.request_with_progress(
            "tools/call",
            json!({ "name": tool, "arguments": args.unwrap_or(json!({})) }),
            idle_secs,
            max_secs,
        )
    })
}

fn request_on_session(
    key: &SessionKey,
    method: &str,
    params: Value,
    timeout_secs: u64,
) -> Result<Value, String> {
    on_session(key, |session| {
        session.request(method, params, timeout_secs)
    })
}

/// 在一个已连接的会话上做一件事，并按同一套判据决定要不要把它收掉。
///
/// 抽成闭包版是因为「重新发现」（`mcp_rediscover`）要连发好几条请求，而收会话的判据必须和
/// 单条请求完全一致——复制一份的话，两边迟早会长歪，而长歪的那一半是「什么时候杀用户的服务
/// 进程」。
fn on_session<T>(
    key: &SessionKey,
    run: impl FnOnce(&mut Session) -> Result<T, String>,
) -> Result<T, String> {
    let slot = find_session_slot(key)?.ok_or_else(|| key.missing())?;
    let (response, retired) = {
        let mut active = slot.lock().map_err(|_| "MCP session state poisoned")?;
        let session = active.as_mut().ok_or_else(|| key.missing())?;
        // 握手都还没完成的会话（正等浏览器授权）不能当正常会话用：发过去只会一路等到超时，
        // 而攒够三次超时就会把 mcp-remote 连同它的 OAuth 回调服务器一起杀掉——那正是
        // 用户马上要用来完成授权的东西。
        if session.awaiting_auth {
            return Err(format!(
                "MCP 服务「{}」正在等待浏览器授权，完成后请重新连接",
                key.name
            ));
        }
        let response = run(session);
        let dead_process = response.is_err() && matches!(session.child.try_wait(), Ok(Some(_)));
        let unhealthy = response
            .as_ref()
            .err()
            .is_some_and(|error| transport_session_error(error));
        // 单次超时留着不动（服务多半只是慢），但连着这么多次一个字都没回来，就不能再等了，
        // 否则「不因超时收会话」会把一个真卡死的服务永远留在会话表里。
        let wedged = session.consecutive_timeouts >= MAX_CONSECUTIVE_TIMEOUTS;
        let retired = (dead_process || unhealthy || wedged)
            .then(|| active.take())
            .flatten();
        (response, retired)
    };
    // Session::drop kills and waits for the child. It must never run under the
    // per-server lock, otherwise same-name reconnect/status/calls queue behind it.
    drop(retired);
    response
}

#[tauri::command]
pub async fn mcp_call_full(
    window: tauri::WebviewWindow,
    name: String,
    root: String,
    tool: String,
    args: Option<Value>,
) -> Result<McpCallResult, String> {
    call_full_at(session_key(&window, &root, &name), tool, args).await
}

async fn call_full_at(
    key: SessionKey,
    tool: String,
    args: Option<Value>,
) -> Result<McpCallResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        call_on_session(&key, tool, args).map(structured_call_result)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mcp_read_resource(
    window: tauri::WebviewWindow,
    name: String,
    root: String,
    uri: String,
) -> Result<Value, String> {
    let key = session_key(&window, &root, &name);
    tauri::async_runtime::spawn_blocking(move || {
        request_on_session(&key, "resources/read", json!({"uri": uri}), 60)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mcp_get_prompt(
    window: tauri::WebviewWindow,
    name: String,
    root: String,
    prompt: String,
    args: Option<Value>,
) -> Result<Value, String> {
    get_prompt_at(session_key(&window, &root, &name), prompt, args).await
}

async fn get_prompt_at(
    key: SessionKey,
    prompt: String,
    args: Option<Value>,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        request_on_session(
            &key,
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
pub async fn mcp_call(
    window: tauri::WebviewWindow,
    name: String,
    root: String,
    tool: String,
    args: Option<Value>,
) -> Result<String, String> {
    call_text_at(session_key(&window, &root, &name), tool, args).await
}

async fn call_text_at(
    key: SessionKey,
    tool: String,
    args: Option<Value>,
) -> Result<String, String> {
    let result = call_full_at(key, tool, args).await?;
    if result.is_error {
        Err(format!("[工具报错] {}", result.text))
    } else {
        Ok(result.text)
    }
}

/// Report whether a named session still has a live child process. Dead sessions are evicted so
/// the next connect starts cleanly and the UI never keeps showing a stale green status.
#[tauri::command]
pub async fn mcp_status(
    window: tauri::WebviewWindow,
    name: String,
    root: String,
) -> Result<bool, String> {
    status_at(session_key(&window, &root, &name)).await
}

async fn status_at(key: SessionKey) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<bool, String> {
        let Some(slot) = find_session_slot(&key)? else {
            return Ok(false);
        };
        let (running, retired) = {
            /*
             * try_lock，拿不到就答「在」。
             *
             * 这把锁被在飞的请求握着（工具调用最长 600 秒），而前端每一轮开工前都要对
             * 每个已连接服务 await 一次 mcp_status，还是 Promise.all——用阻塞锁的话，
             * 一个正在干活的服务会把**下一轮的工具准备**整个吊住，用户看到的是"发了一句
             * 话，界面卡住不动"。
             *
             * 拿不到锁 = 有请求正在这个会话上跑 = 子进程活着并且在应答，这比下面那次 ping
             * 是更强的存活证据，不是更弱。所以 WouldBlock 映射成 true 不是猜，是事实。
             * （映射成 false 才是错的：那会把一个正忙的服务判成死的，接着被回收。）
             *
             * Poisoned 仍然照旧报错——那是真的坏了，和"忙"不是一回事。
             */
            let mut active = match slot.try_lock() {
                Ok(guard) => guard,
                Err(std::sync::TryLockError::WouldBlock) => return Ok(true),
                Err(std::sync::TryLockError::Poisoned(_)) => return Err("MCP session state poisoned".to_string()),
            };
            let Some(session) = active.as_mut() else {
                return Ok(false);
            };
            // 等授权的会话直接算「在」：子进程活着，只是握手还没做完。这里**不能**去 ping
            // 它——ping 不会有人答，攒够三次超时就把 mcp-remote 和它的 OAuth 回调服务器一起
            // 杀了，用户点完浏览器回来发现要重头再来。面板另有 mcp_pending_auth 区分这两种
            // 「在」。
            if session.awaiting_auth {
                return Ok(matches!(session.child.try_wait(), Ok(None)));
            }
            let dead = session
                .child
                .try_wait()
                .map(|status| status.is_some())
                .map_err(|e| format!("无法检查 MCP 服务状态: {e}"))?;
            if dead {
                (false, active.take())
            } else {
                let ping = session.request("ping", json!({}), 3);
                // **`ping` 回错误 ≠ 服务死了。** `ping` 在 MCP 里不是必须实现的方法，一个完全
                // 正常的服务合规地回 `-32601 Method not found` 是常见的；旧代码把任何 `Err`
                // 都当成死亡判据，于是这批服务会在用户用得好好的时候突然掉线——会话被收走、
                // 子进程被杀——而用户什么都没做，也看不出原因。只认「通道没了」。
                let unhealthy = ping
                    .as_ref()
                    .err()
                    .is_some_and(|error| transport_session_error(error));
                // 上面那次 try_wait 发生在请求**之前**：子进程完全可能在这几秒里退出，
                // 只查一次会把一个已经死掉的服务报成健康的，UI 就一直亮着绿灯。
                let dead_now = matches!(session.child.try_wait(), Ok(Some(_)));
                // 活着但连着这么多次不回话，和死了对用户是一回事（见 MAX_CONSECUTIVE_TIMEOUTS）。
                let wedged = session.consecutive_timeouts >= MAX_CONSECUTIVE_TIMEOUTS;
                if unhealthy || dead_now || wedged {
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

/// 放弃当前这条在飞的请求，**但把服务留着**。
///
/// 两个坑，踩中任何一个这条命令就是废的：
///
/// 一、前端给不出请求 id——id 是 `Session` 私有的，前端从来看不到，也不该看到。所以能取消的
/// 只有「此刻在飞的那一条」。判断依据是槽位的锁：请求从头到尾握着它，所以**锁拿得到就说明
/// 没有请求在飞**，这时候把标志立起来只会误伤下一次调用（用户点了停，然后下一次点什么都直接
/// 失败）。
///
/// 二、标志本身不能放在会话那把锁后面（见 `Session::cancel`）。
///
/// 返回值是「有没有真的取消到东西」，面板据此决定要不要给用户一句反馈。
#[tauri::command]
pub async fn mcp_cancel(
    window: tauri::WebviewWindow,
    name: String,
    root: String,
) -> Result<bool, String> {
    cancel_at(session_key(&window, &root, &name)).await
}

async fn cancel_at(key: SessionKey) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<bool, String> {
        let Some(slot) = find_session_slot(&key)? else {
            return Ok(false);
        };
        if slot.try_lock().is_ok() {
            return Ok(false);
        }
        side_channel(&key)?.cancel.store(true, Ordering::SeqCst);
        Ok(true)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 服务宣告过的清单变更，读一次就清空。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpChanges {
    tools: bool,
    resources: bool,
    prompts: bool,
}

/// 取走并清空「服务说它的清单变了」这几个标志。
///
/// 这些标志只可能在**某次请求顺路读到通知**时被立起来：`Session` 独占那唯一一个
/// `Receiver`，再开一条后台读线程会把在飞的响应偷走，所以没有别的排空方式。每轮的
/// `mcp_status` ping 就是那个排空动作。
///
/// 没有这个会话时回全 false，而不是报错：前端每轮都会问一次，让「还没连」变成一次假失败
/// 只会逼它在外面包一层 try/catch。
#[tauri::command]
pub async fn mcp_take_changes(
    window: tauri::WebviewWindow,
    name: String,
    root: String,
) -> Result<McpChanges, String> {
    take_changes_at(session_key(&window, &root, &name)).await
}

async fn take_changes_at(key: SessionKey) -> Result<McpChanges, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<McpChanges, String> {
        let none = McpChanges {
            tools: false,
            resources: false,
            prompts: false,
        };
        let Some(slot) = find_session_slot(&key)? else {
            return Ok(none);
        };
        /*
         * try_lock，拿不到就当「没变化」——**这是无损的**，和 status_at 那次改动理由相同
         * 但道理不同：那边拿不到锁必须答「在」（答错会触发整根重连），这边拿不到锁只是
         * 晚一轮再取，std::mem::take 还没执行，标志原封不动躺在会话里。
         *
         * 不能用阻塞锁：前端每开一轮都会对每个已连服务问一次，而这把锁被一次 tools/call
         * 握着最长 600 秒。阻塞的话这些查询全堆在在飞的调用后面——status_at 刚为此改成
         * try_lock，这条路再用阻塞锁就是把同一个拥塞从另一扇门放回来。更糟的是前端那层
         * _invokeCapped 只 reject 它自己的 promise，Rust 这边的 spawn_blocking 线程照样
         * 停在锁上，每轮再来一次就多停一条。
         */
        let mut active = match slot.try_lock() {
            Ok(guard) => guard,
            Err(std::sync::TryLockError::WouldBlock) => return Ok(none),
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err("MCP session state poisoned".to_string())
            }
        };
        let Some(session) = active.as_mut() else {
            return Ok(none);
        };
        Ok(McpChanges {
            tools: std::mem::take(&mut session.tools_changed),
            resources: std::mem::take(&mut session.resources_changed),
            prompts: std::mem::take(&mut session.prompts_changed),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 在**同一个**会话上重新列一遍清单。
///
/// 绝不重启服务：清单会变，往往正是因为服务里刚发生了状态变化（登录成功了、切了仓库、装了
/// 插件）。重启会把那个状态连同变化本身一起抹掉，用户看到的是「刷新了一下，新工具反而没了」。
#[tauri::command]
pub async fn mcp_rediscover(
    window: tauri::WebviewWindow,
    name: String,
    root: String,
) -> Result<McpDiscovery, String> {
    rediscover_at(session_key(&window, &root, &name)).await
}

async fn rediscover_at(key: SessionKey) -> Result<McpDiscovery, String> {
    tauri::async_runtime::spawn_blocking(move || {
        on_session(&key, |session| {
            // 重新发现本身就是排空：拿到的就是最新清单，标志再留着只会让面板多刷一次。
            session.tools_changed = false;
            session.resources_changed = false;
            session.prompts_changed = false;
            discover_capabilities(session, Instant::now() + Duration::from_secs(60))
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 这个服务自己说过的话：stderr 尾巴 + `notifications/message` 诊断，按到达顺序。
///
/// 没有这个会话时回空数组而不是报错——面板会在服务连不上的时候来问这个（那正是最需要看它的
/// 时候），报错只会把真正的失败盖掉。
#[tauri::command]
pub async fn mcp_server_log(
    window: tauri::WebviewWindow,
    name: String,
    root: String,
) -> Result<Vec<String>, String> {
    server_log_at(session_key(&window, &root, &name)).await
}

async fn server_log_at(key: SessionKey) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<String>, String> {
        // 走 SIDE_CHANNELS 而不是会话槽：服务卡住时才最需要看这份日志，而那时候会话锁正被
        // 卡住的那条请求握着，从槽里读会一直等到它结束——等于「越是要看越看不到」。
        let Some(side) = find_side_channel(&key) else {
            return Ok(Vec::new());
        };
        Ok(side
            .log
            .lock()
            .map(|log| log.iter().cloned().collect())
            .unwrap_or_default())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 这个服务是不是正卡在「等用户去浏览器里授权」上。
///
/// 面板靠它把两种「在」分开：一种是连上了能用，一种是进程活着但握手还没做完（见
/// `REMOTE_HANDSHAKE_SECS`）。没有它的话，用户只会看到一个既不报错也不能用的服务。
#[tauri::command]
pub async fn mcp_pending_auth(
    window: tauri::WebviewWindow,
    name: String,
    root: String,
) -> Result<bool, String> {
    pending_auth_at(session_key(&window, &root, &name)).await
}

async fn pending_auth_at(key: SessionKey) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<bool, String> {
        let Some(slot) = find_session_slot(&key)? else {
            return Ok(false);
        };
        // try_lock，不是 lock。这把锁在握手（远端预算 180 秒）和工具调用（600 秒）全程
        // 被握着，而前端渲染"已配置服务"列表之前会对每个服务 await 一次这个查询——
        // 用阻塞锁的话，用户配好远程服务点了连接、再打开 MCP 面板想看看卡在哪，
        // 列表最长 180 秒根本不渲染，正好卡在它要报告的那段时间里。
        //
        // 拿不到锁 = 有请求在飞，而 awaiting_auth 只在超时分支赋值、且要等 drop(active)
        // 之后才对外可见；status_at / on_session 又都对这种会话短路。所以"长时间持锁"
        // 与"awaiting_auth == true"不可能同时成立：拿不到锁就是 false。
        let active = match slot.try_lock() {
            Ok(guard) => guard,
            Err(std::sync::TryLockError::WouldBlock) => return Ok(false),
            Err(std::sync::TryLockError::Poisoned(_)) => return Err("MCP session state poisoned".to_string()),
        };
        Ok(active.as_ref().is_some_and(|session| session.awaiting_auth))
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

/// 从自有配置文本里摘出服务表。`mcpServers` 是面板写的那个键；`servers` 一并认，
/// 是因为手写这份文件的人可能照着 VS Code 的写法来，认一下不花什么代价。
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

/// 这份文本**解析不了**时的报错原文（解析得了就是 `None`）。
///
/// `servers_subtree` / `disabled_subtree` 都把解析失败吞成空表，这对「读」是对的（一份手写坏
/// 的配置不该把整个面板打崩），但那个空表和「用户还没配过任何服务」长得一模一样。前端保存时
/// 提交的又是从这份投影重建出来的整份文档——于是漏了一个逗号的配置，会在用户在面板里点一下
/// 之后，被 `{}` 原子替换掉：服务没了，填在里面的 API Key 也没了，全程无提示、不可撤销。
/// 所以「读不懂」必须作为一个**单独的事实**传到前端，而不是伪装成「空」。
fn config_parse_error(text: &str) -> Option<String> {
    serde_json::from_str::<Value>(text)
        .err()
        .map(|error| error.to_string())
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
    servers: Value,
    /// 用户在本 IDE 里停用的服务名。这里记的是**仓库自带**的 `.mcp.json` 里那些服务——
    /// 它们跟着 git clone 来，想少加载一个不该去改仓库里的文件，所以停用记在自己这儿。
    disabled: Value,
    /// 文件在、但读不懂时的报错。**「读不懂」和「是空的」必须能分开**——见
    /// `config_parse_error`。面板拿到它就该显示原文并停手，而不是照着一份空投影去保存。
    parse_error: Option<String>,
}

/// 本 IDE 自己那份配置的投影。路径作参数的理由和 `save_user_config_at` 一样：只为了能在
/// 测试里对着临时目录验，命令本身不接受调用方给路径。
fn own_user_config(path: &std::path::Path) -> McpUserConfig {
    let text = read_capped(path);
    McpUserConfig {
        path: path.to_string_lossy().into_owned(),
        servers: text
            .as_deref()
            .map(servers_subtree)
            .unwrap_or_else(|| json!({})),
        disabled: text
            .as_deref()
            .map(disabled_subtree)
            .unwrap_or_else(|| json!([])),
        // 文件**不存在**照旧不算错（下面那段注释说的第一次使用：servers 为空、可写、无报错）。
        // 只有「文件在、但解析不了」才报——那正是空投影会骗人的那一种。
        parse_error: text.as_deref().and_then(config_parse_error),
    }
}

/// 用户级 MCP 配置：本 IDE 自己的 `~/.michael-ide/mcp.json`，**只有这一份**。
///
/// 这里曾经还会去读 Claude Code / Cursor / Codex / Claude Desktop 的全局配置，把它们的
/// 服务一并合进来。按用户要求去掉了：别的客户端的配置文件不是他的目录，不该在这个 IDE
/// 里生效。想用那边配过的服务，在 MCP 面板里加一遍——那一次是明确的采纳，而不是默默继承。
///
/// 文件还不存在也照样返回（servers 为空）：面板要靠它知道"保存会写到哪里"。
#[tauri::command]
pub fn mcp_user_config() -> Result<McpUserConfig, String> {
    Ok(own_user_config(&user_config_path()?))
}

/// 覆盖写 `~/.michael-ide/mcp.json`，返回它的绝对路径。
///
/// 权限收到 0600（目录 0700）：这个文件里会有 API Key。先写临时文件再 rename，
/// 断电或者写到一半崩了不会留下半截 JSON 把所有 MCP 服务一起带走。
///
/// `force` 缺省即 false：面板正常保存不传，撞上「磁盘上那份读不懂」时会被拒绝并拿到报错；
/// 用户看过报错、确认要用面板里这份覆盖掉它，前端才带 `force: true` 再来一次。
#[tauri::command]
pub fn mcp_save_user_config(text: String, force: Option<bool>) -> Result<String, String> {
    save_user_config_at(&user_config_path()?, &text, force.unwrap_or(false))
}

/// 真正落盘的那一段。路径作参数是为了能在测试里用临时目录跑完整往返——命令本身
/// 不接受调用方给路径（那会把 HOME 底下任意文件变成可写目标）。
fn save_user_config_at(path: &std::path::Path, text: &str, force: bool) -> Result<String, String> {
    // 拒绝写坏文件：这一份被所有项目共用，写坏了是把全部 MCP 一起弄丢。
    serde_json::from_str::<Value>(text).map_err(|e| format!("MCP 配置不是合法 JSON：{e}"))?;
    // 反过来的那一半：**要盖掉的那份**也得先看懂。前端提交的是「面板里现在这一屏」，而那一屏
    // 是从解析结果重建的——磁盘上这份此刻解析不了（手改时漏了个逗号、别的进程写了一半），
    // 面板看到的就是空表，一次保存把用户全部服务和 API Key 原子换成 `{}`。
    //
    // 这道闸放在这里，是因为它是所有调用方的必经之处：前端那边加多少判断都可能有下一个入口
    // 绕过去，落盘只有这一条路。先备份再拒绝——备份是给「原文已经没法在面板里显示」的用户
    // 留一条自己回去补逗号的路。
    if !force {
        // 空文件（`touch` 出来的、上一次崩在半路的）里没有任何用户数据，拦下来只会变成一个
        // 谁也修不了的死结，所以它不算「读不懂」。
        let existing = std::fs::read_to_string(path)
            .ok()
            .filter(|on_disk| !on_disk.trim().is_empty());
        if let Some(existing) = existing {
            if let Some(reason) = config_parse_error(&existing) {
                let mut backup = path.as_os_str().to_os_string();
                backup.push(".bak");
                let backup = std::path::PathBuf::from(backup);
                std::fs::write(&backup, existing.as_bytes())
                    .map_err(|e| format!("无法备份 {}：{e}", path.display()))?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    // 备份里同样有 API Key，权限跟正本一致。
                    let _ =
                        std::fs::set_permissions(&backup, std::fs::Permissions::from_mode(0o600));
                }
                return Err(format!(
                    "{} 现在不是合法 JSON（{reason}），已备份到 {}。\
                     直接保存会用面板里这份把它整个替换掉，里面的服务和 API Key 会一起丢失，\
                     所以这次没有写入。请先修好那个文件，或确认要覆盖后再保存一次。",
                    path.display(),
                    backup.display()
                ));
            }
        }
    }
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

/// 读一份用户文档（rules / habits），**文件不在就先建一个空的**。
///
/// 这两份是"自带的"文档：用户从菜单点进去，是在编辑器里打开一个标签页，所以它必须是磁盘上
/// 真实存在的文件，而不是一个概念。默认空白——内容全由用户自己写。
///
/// 它们住在 `~/.michael-ide/` 而**不在 app 包里**，所以升级、重装、换版本都碰不到它们；
/// 用户写进去的东西不会被更新覆盖掉。
#[tauri::command]
pub fn user_rules_read(kind: Option<String>) -> Result<Value, String> {
    let kind = kind.unwrap_or_else(|| "rules".into());
    let path = user_doc_path(&kind)?;
    if !path.exists() {
        write_user_doc(&kind, "")?;
    }
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
    write_user_doc(&kind, &text)
}

/// 落盘。**清空不删文件**——这两份现在是编辑器里打开的标签页，删掉的话用户全选删除再保存，
/// 手上那个标签就指向一个不存在的文件了（编辑器会报"文件已被外部删除"）。空文件是正常状态。
fn write_user_doc(kind: &str, text: &str) -> Result<String, String> {
    let path = user_doc_path(kind)?;
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
    let temp = dir.join(format!("{}.tmp", user_doc_file(kind)?));
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
pub async fn mcp_disconnect(
    window: tauri::WebviewWindow,
    name: String,
    root: String,
) -> Result<(), String> {
    disconnect_at(session_key(&window, &root, &name)).await
}

async fn disconnect_at(key: SessionKey) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let Some(slot) = find_session_slot(&key)? else {
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

    /// 测试里的默认键：单窗口、无工作区。命令那一层的窗口标签由 Tauri 注入，测试构造不出
    /// `WebviewWindow`，所以一律走命令内部那半（`*_at`）——被测的是同一段代码。
    fn key(session_name: &str) -> SessionKey {
        SessionKey::new("main", "", session_name)
    }

    async fn connect_fixture_at(
        key: &SessionKey,
        cwd: &str,
        env: Option<HashMap<String, String>>,
    ) -> Result<Vec<McpTool>, String> {
        Ok(connect_full_at(
            key.clone(),
            "node".into(),
            Some(vec![fixture_path()]),
            env,
            Some(cwd.to_string()),
            false,
        )
        .await?
        .tools)
    }

    async fn connect_fixture(
        session_name: &str,
        env: Option<HashMap<String, String>>,
    ) -> Result<Vec<McpTool>, String> {
        connect_fixture_at(&key(session_name), env!("CARGO_MANIFEST_DIR"), env).await
    }

    /// 会话槽里那个子进程的 pid。用来分辨「服务还是原来那个」和「它被杀了、又新起了一个」。
    fn session_child_pid(session_name: &str) -> Option<u32> {
        slot_child_pid(&key(session_name))
    }

    fn slot_child_pid(key: &SessionKey) -> Option<u32> {
        let slot = find_session_slot(key).unwrap()?;
        let guard = slot.lock().unwrap();
        guard.as_ref().map(|session| session.child.id())
    }

    /// 下一条请求会用到的 JSON-RPC id。
    fn session_next_id(session_name: &str) -> Option<u64> {
        let slot = find_session_slot(&key(session_name)).unwrap()?;
        let guard = slot.lock().unwrap();
        guard.as_ref().map(|session| session.next_id)
    }

    async fn connect_fixture_full(session_name: &str) -> Result<McpDiscovery, String> {
        connect_full_at(
            key(session_name),
            "node".into(),
            Some(vec![fixture_path()]),
            None,
            Some(env!("CARGO_MANIFEST_DIR").into()),
            false,
        )
        .await
    }

    // ── 命令的测试替身 ─────────────────────────────────────────────────────────
    //
    // 下面这几个和 `#[tauri::command]` 同名，是**故意的**：命令那一层现在的第一个参数是
    // Tauri 注入的 `WebviewWindow`，测试里造不出来（也不该造——那一层做的事只有
    // 「window.label() → 键」）。同名局部定义会盖住 `use super::*` 带进来的那个，于是既有的
    // 断言一个字都不用改，跑的还是命令内部那半真代码。
    async fn mcp_connect(
        session_name: String,
        command: String,
        args: Option<Vec<String>>,
        env: Option<HashMap<String, String>>,
        cwd: Option<String>,
    ) -> Result<Vec<McpTool>, String> {
        Ok(connect_full_at(key(&session_name), command, args, env, cwd, false)
            .await?
            .tools)
    }

    async fn mcp_connect_full(
        session_name: String,
        command: String,
        args: Option<Vec<String>>,
        env: Option<HashMap<String, String>>,
        cwd: Option<String>,
    ) -> Result<McpDiscovery, String> {
        connect_full_at(key(&session_name), command, args, env, cwd, false).await
    }

    async fn mcp_status(session_name: String) -> Result<bool, String> {
        status_at(key(&session_name)).await
    }

    async fn mcp_disconnect(session_name: String) -> Result<(), String> {
        disconnect_at(key(&session_name)).await
    }

    async fn mcp_call(
        session_name: String,
        tool: String,
        args: Option<Value>,
    ) -> Result<String, String> {
        call_text_at(key(&session_name), tool, args).await
    }

    async fn mcp_call_full(
        session_name: String,
        tool: String,
        args: Option<Value>,
    ) -> Result<McpCallResult, String> {
        call_full_at(key(&session_name), tool, args).await
    }

    async fn mcp_read_resource(session_name: String, uri: String) -> Result<Value, String> {
        let key = key(&session_name);
        tauri::async_runtime::spawn_blocking(move || {
            request_on_session(&key, "resources/read", json!({"uri": uri}), 60)
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn mcp_get_prompt(
        session_name: String,
        prompt: String,
        args: Option<Value>,
    ) -> Result<Value, String> {
        get_prompt_at(key(&session_name), prompt, args).await
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
    fn clearing_user_rules_keeps_the_file() {
        // 这两份现在是编辑器里打开的标签页。清空就删文件的话，用户全选删除再保存，手上那个
        // 标签立刻指向一个不存在的文件（编辑器会报"已被外部删除"）。空文件是正常状态。
        let dir = std::env::temp_dir().join("michael-rules-clear");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rules.md");
        std::fs::write(&path, "回答用中文。").unwrap();
        std::fs::write(&path, "").unwrap();
        assert!(path.exists(), "清空之后文件必须还在");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reading_absent_user_rules_is_empty_not_an_error() {
        // "还没写过"是常态，不是错误——报错的话前端每次打开都要处理一次假失败。
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

    /// 用户级配置**只有自己那一份**。
    ///
    /// 这里曾经会把 Claude Code / Cursor / Codex / Claude Desktop 的全局配置一并读进来。
    /// 按用户要求去掉了——别的客户端的配置文件不是他的目录。这条守卫不看函数名（那种断言
    /// 换个名字就废了），而是把四份别的客户端的配置**真的写到磁盘上**，再看它们的服务有
    /// 没有混进返回值：任何一条回来，这条就红。
    #[test]
    fn only_this_ides_own_config_is_read_never_another_clients() {
        let _serial = ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let home = std::env::temp_dir().join(format!("michael-own-only-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);

        let write = |rel: &str, body: &str| {
            let path = home.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, body).unwrap();
        };
        write(".michael-ide/mcp.json", r#"{"mcpServers":{"我自己的":{"command":"mine"}}}"#);
        // 别的客户端各写一份，连 Claude Code 的 local 作用域（projects["<cwd>"]）也写上。
        write(
            ".claude.json",
            r#"{"mcpServers":{"claude-全局":{"command":"x"}},
                "projects":{"/w/mine":{"mcpServers":{"claude-项目":{"command":"x"}}}}}"#,
        );
        write(".cursor/mcp.json", r#"{"mcpServers":{"cursor-的":{"command":"x"}}}"#);
        write(".codex/mcp.json", r#"{"mcpServers":{"codex-的":{"command":"x"}}}"#);
        write(
            "Library/Application Support/Claude/claude_desktop_config.json",
            r#"{"mcpServers":{"桌面版的":{"command":"x"}}}"#,
        );

        // HOME 是进程级的，改完必须还原——包括断言失败 panic 出去的那条路。
        struct RestoreHome(Option<String>);
        impl Drop for RestoreHome {
            fn drop(&mut self) {
                unsafe {
                    match self.0.take() {
                        Some(old) => std::env::set_var("HOME", old),
                        None => std::env::remove_var("HOME"),
                    }
                }
            }
        }
        let _restore = RestoreHome(std::env::var("HOME").ok());
        unsafe { std::env::set_var("HOME", &home) };

        let config = mcp_user_config().expect("读自有配置");
        let names: Vec<&String> = config
            .servers
            .as_object()
            .expect("servers 应当是个对象")
            .keys()
            .collect();
        assert_eq!(names, vec!["我自己的"], "混进了别的客户端的服务：{names:?}");
        // 整份返回值里也不该出现它们的任何痕迹（路径、服务名都算）。
        let dumped = serde_json::to_string(&config.servers).unwrap() + &config.path;
        for leaked in ["claude-全局", "claude-项目", "cursor-的", "codex-的", "桌面版的", ".cursor", ".codex", ".claude"] {
            assert!(!dumped.contains(leaked), "{leaked} 还在返回值里：{dumped}");
        }
        let _ = std::fs::remove_dir_all(&home);
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
        save_user_config_at(&path, r#"{"mcpServers":{"a":{"command":"x"}}}"#, false).unwrap();
        let before = std::fs::read_to_string(&path).unwrap();

        let error = save_user_config_at(&path, "{ 半截", false).unwrap_err();
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
        let saved = save_user_config_at(&path, written, false).unwrap();
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
        save_user_config_at(&path, r#"{"mcpServers":{}}"#, false).unwrap();
        assert_eq!(servers_subtree(&read_capped(&path).unwrap()), json!({}));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 一份读不懂的配置**不能**长得和「还没配过」一样。
    ///
    /// 前端保存时提交的是从这份投影重建出来的整份文档：解析失败静默变成 `{}` 的话，用户在
    /// 面板上点一下，磁盘上的服务连同 API Key 就被 `{}` 原子替换掉了，无提示、不可撤销。
    #[test]
    fn an_unparsable_user_config_reports_the_error_instead_of_looking_empty() {
        let dir = std::env::temp_dir().join("michael-mcp-usercfg-broken");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mcp.json");
        // 手写配置最常见的坏法：多一个逗号。
        std::fs::write(&path, r#"{"mcpServers":{"a":{}},}"#).unwrap();

        let projected = own_user_config(&path);
        assert_eq!(projected.servers, json!({}), "解析失败时投影本来就是空的");
        assert!(
            projected.parse_error.is_some(),
            "空投影必须带着「读不懂」这个事实，否则前端分不清它和「用户还没配过」"
        );

        // 文件不存在是**第一次使用**，不是错误——面板要靠这条知道保存会写到哪里。
        let fresh = own_user_config(&dir.join("never-written.json"));
        assert_eq!(fresh.servers, json!({}));
        assert_eq!(fresh.disabled, json!([]));
        assert!(fresh.parse_error.is_none(), "文件不存在不该报错");

        // 正常的一份同样不报错。
        std::fs::write(&path, r#"{"mcpServers":{"a":{"command":"x"}}}"#).unwrap();
        assert!(own_user_config(&path).parse_error.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 落盘这一条是所有调用方的必经之处，所以最后一道闸放在这儿：要盖掉的那份读不懂时，
    /// 先备份、再拒绝。前端加多少判断都可能有下一个入口绕过去，这里绕不过去。
    #[test]
    fn saving_over_an_unreadable_config_backs_it_up_and_refuses_without_force() {
        let dir = std::env::temp_dir().join("michael-mcp-usercfg-clobber");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mcp.json");
        let broken = r#"{"mcpServers":{"tavily":{"env":{"TAVILY_API_KEY":"秘密"}}},}"#;
        std::fs::write(&path, broken).unwrap();

        // 面板正常保存（不带 force）提交的正是那份空投影重建出来的文档。
        let error = save_user_config_at(&path, r#"{"mcpServers":{}}"#, false).unwrap_err();
        assert!(error.contains("不是合法 JSON"), "{error}");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            broken,
            "被拒绝的保存一个字节都不该改到磁盘上的配置"
        );
        let backup = dir.join("mcp.json.bak");
        assert!(backup.is_file(), "拒绝之前必须先留一份备份：{error}");
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), broken);
        assert!(error.contains("mcp.json.bak"), "报错要告诉用户备份在哪：{error}");

        // 用户看过报错、确认要覆盖：force 放行。
        save_user_config_at(&path, r#"{"mcpServers":{"a":{"command":"x"}}}"#, true).unwrap();
        assert_eq!(servers_subtree(&read_capped(&path).unwrap())["a"]["command"], "x");

        // 磁盘上那份能读懂时，保存照常，不该平白多出备份文件。
        let _ = std::fs::remove_file(&backup);
        save_user_config_at(&path, r#"{"mcpServers":{"b":{"command":"y"}}}"#, false).unwrap();
        assert!(!backup.exists(), "正常保存不该留下 .bak");
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

    /// 握手是一次协商，服务回的版本不一定是我们报的那个。以前这个回值一个字都没看过，
    /// 于是版本对不上时故障全推迟到后面某次 tools/call，报出来的话看不出真原因。
    #[test]
    fn an_unsupported_protocol_version_stops_the_handshake_with_the_real_reason() {
        // 我们自己报的那个版本必须在支持表里，否则连"和自己握手"都过不了。
        assert!(SUPPORTED_PROTOCOL_VERSIONS.contains(&PROTOCOL_VERSION));
        for good in SUPPORTED_PROTOCOL_VERSIONS {
            assert!(check_protocol_version(&json!({"protocolVersion": good})).is_ok(), "{good}");
        }
        let error = check_protocol_version(&json!({"protocolVersion": "1999-01-01"}))
            .expect_err("不认的版本必须拦下来");
        // 报错要同时说清两边各要什么——只说"版本不对"的话用户不知道该改哪个数字。
        assert!(error.contains("1999-01-01"), "{error}");
        assert!(error.contains(PROTOCOL_VERSION), "{error}");
        // 不声明版本的照旧放行：确实有服务不回，这不构成拒绝一次能用的连接的理由。
        assert!(check_protocol_version(&json!({})).is_ok());
        assert!(check_protocol_version(&json!({"protocolVersion": "  "})).is_ok());
    }

    /// 慢服务以前没有任何退路：45 秒握不完手就是连不上，改不了。`MCP_TIMEOUT` /
    /// `MCP_TOOL_TIMEOUT` 是 Claude Code 那两个变量，单位是**毫秒**——同名不同单位的话，
    /// 用户照着文档填的 30000 在这边会变成八小时。
    #[test]
    fn timeout_env_vars_follow_claude_codes_names_and_milliseconds() {
        let _serial = ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let _clean = ClearTimeoutEnv;   // 失败也要还原，见它的注释
        assert_eq!(handshake_budget(true), REMOTE_HANDSHAKE_SECS);
        assert_eq!(handshake_budget(false), LOCAL_HANDSHAKE_SECS);
        assert_eq!(tool_call_budget(), (TOOL_CALL_IDLE_SECS, TOOL_CALL_MAX_SECS));

        unsafe { std::env::set_var("MCP_TIMEOUT", "300000") };
        assert_eq!(handshake_budget(false), 300, "30 万毫秒 = 300 秒，不是 30 万秒");
        assert_eq!(handshake_budget(true), 300, "显式设过就该盖住远端那个默认值");

        unsafe { std::env::set_var("MCP_TOOL_TIMEOUT", "30000") };
        let (idle, max) = tool_call_budget();
        assert_eq!(max, 30);
        // 空闲判定不能比硬顶还长，否则硬顶先到、"没动静"这条一次都不触发。
        assert!(idle <= max, "空闲 {idle}s 超过了硬顶 {max}s");

        for junk in ["", "  ", "abc", "0", "-5", "12.5"] {
            unsafe { std::env::set_var("MCP_TIMEOUT", junk) };
            assert_eq!(
                handshake_budget(false),
                LOCAL_HANDSHAKE_SECS,
                "{junk:?} 该当没设过；0 尤其不能认——那是「立刻超时」，每次请求当场失败",
            );
        }
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
        let slot = find_session_slot(&key(&session_name)).unwrap().unwrap();
        assert!(slot.lock().unwrap().is_none());
    }

    /// 一个**从不回话**的服务最终还是要被收掉，只是不能第一次就收。
    ///
    /// 这条以前叫 failed_status_ping_evicts_...，断言的是「ping 失败 = 立刻收会话」。那条
    /// 判据太粗，把下面那条测试里的正常服务也一起误杀了（见
    /// `a_ping_answered_with_method_not_found_keeps_the_session_alive`）。现在的判据是连续
    /// 超时次数：前几次留着（服务可能只是忙），到 MAX_CONSECUTIVE_TIMEOUTS 才认定卡死。
    #[tokio::test]
    async fn a_server_that_never_answers_ping_is_evicted_only_after_repeated_timeouts() {
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

        for attempt in 1..MAX_CONSECUTIVE_TIMEOUTS {
            assert!(
                mcp_status(session_name.clone()).await.unwrap(),
                "第 {attempt} 次超时就收会话的话，一个只是慢的服务会在用户眼前掉线"
            );
        }
        assert!(!mcp_status(session_name.clone()).await.unwrap());
        let slot = find_session_slot(&key(&session_name)).unwrap().unwrap();
        assert!(slot.lock().unwrap().is_none());
    }

    /// `ping` 在 MCP 里不是必须实现的方法：合规的服务回 `-32601 Method not found` 很常见。
    /// 那是**服务答了**，不是通道断了——旧代码把任何 `Err` 都当成死亡判据，于是这批服务会在
    /// 用户用得好好的时候突然掉线（会话被收走、子进程被杀），而用户什么都没做。
    #[tokio::test]
    async fn a_ping_answered_with_method_not_found_keeps_the_session_alive() {
        let session_name = format!("fixture-ping-unsupported-{}", std::process::id());
        connect_fixture(
            &session_name,
            Some(HashMap::from([(
                "MCP_FIXTURE_PING_UNSUPPORTED".to_string(),
                "1".to_string(),
            )])),
        )
        .await
        .unwrap();

        assert!(
            mcp_status(session_name.clone()).await.unwrap(),
            "服务回了 -32601，说明它还在听——不该报成掉线"
        );
        let slot = find_session_slot(&key(&session_name)).unwrap().unwrap();
        assert!(
            slot.lock().unwrap().is_some(),
            "会话槽被清空了：下一次调用要重新 spawn + 握手，用户看到的是服务自己没了"
        );
        // 还能正常用：证明子进程也没被顺手杀掉。
        let echo = mcp_call(session_name.clone(), "echo".into(), Some(json!({"text":"活着"})))
            .await
            .unwrap();
        assert_eq!(echo, "活着");
        mcp_disconnect(session_name).await.unwrap();
    }

    /// 一次慢调用不能把服务进程杀掉。
    ///
    /// 超时只说明子进程**还没**答（慢工具、装依赖、加载大索引都会这样）。以前 `MCP 请求超时`
    /// 被算作传输层错误，于是一次超时顺手 kill 掉整个服务：用户下一次调用得从 spawn + 握手
    /// 重来一遍，而且服务里攒的状态全没了。
    #[tokio::test]
    async fn a_call_that_outruns_its_budget_leaves_the_same_child_usable() {
        let session_name = format!("fixture-slow-call-{}", std::process::id());
        connect_fixture(&session_name, None).await.unwrap();
        let pid_before = session_child_pid(&session_name).expect("刚连上就该有子进程");

        // 预算 1 秒、服务 1.5 秒才回：走的是和 mcp_call 同一条 request_on_session，只是把
        // 那 60 秒的预算换成测试等得起的长度。
        let error = request_on_session(
            &key(&session_name),
            "tools/call",
            json!({"name":"delay_echo","arguments":{"text":"慢","delay_ms":1_500}}),
            1,
        )
        .expect_err("超过预算的调用应当返回超时");
        assert!(error.contains("MCP 请求超时"), "{error}");

        assert!(
            mcp_status(session_name.clone()).await.unwrap(),
            "一次超时之后服务必须还活着"
        );
        assert_eq!(
            session_child_pid(&session_name),
            Some(pid_before),
            "子进程被换掉了 = 原来那个被杀了，服务里的状态全丢"
        );

        let echo = mcp_call(session_name.clone(), "echo".into(), Some(json!({"text":"第二次"})))
            .await
            .expect("超时之后同一个会话应当还能继续用");
        assert_eq!(echo, "第二次");
        assert_eq!(session_child_pid(&session_name), Some(pid_before));
        mcp_disconnect(session_name).await.unwrap();
    }

    /// 放弃等待之后要告诉服务「这条别做了」。
    ///
    /// 少了这一条，客户端这边已经超时返回、服务那边还在跑：一次误点的慢查询会继续占着它的
    /// 连接和配额，而结果回来时我们只会当成陌生 id 丢掉。
    #[tokio::test]
    async fn a_timed_out_request_tells_the_server_to_drop_it() {
        // 规范禁止取消 initialize。这条只有 45 秒握手超时才走得到，靠集成测试等于没有守卫。
        assert!(!cancellable_request("initialize"));
        assert!(cancellable_request("tools/call") && cancellable_request("ping"));

        let session_name = format!("fixture-cancel-{}", std::process::id());
        let log_path = std::env::temp_dir().join(format!("mcp-cancel-log-{session_name}"));
        let _ = std::fs::remove_file(&log_path);
        connect_fixture(
            &session_name,
            Some(HashMap::from([(
                "MCP_FIXTURE_CANCEL_LOG".to_string(),
                log_path.to_string_lossy().into_owned(),
            )])),
        )
        .await
        .unwrap();

        let doomed_id = session_next_id(&session_name).expect("刚连上就该有会话");
        request_on_session(
            &key(&session_name),
            "tools/call",
            json!({"name":"delay_echo","arguments":{"text":"来不及了","delay_ms":3_000}}),
            1,
        )
        .expect_err("超过预算的调用应当返回超时");

        tokio::time::timeout(Duration::from_secs(3), async {
            while std::fs::read_to_string(&log_path).unwrap_or_default().trim().is_empty() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("超时之后服务应当收到 notifications/cancelled");

        let logged = std::fs::read_to_string(&log_path).unwrap();
        assert_eq!(
            logged.trim().lines().collect::<Vec<_>>(),
            vec![doomed_id.to_string()],
            "取消的必须正是超时的那条请求"
        );
        mcp_disconnect(session_name).await.unwrap();
        let _ = std::fs::remove_file(&log_path);
    }

    /// 但真卡死的必须还是收得掉，否则「不因超时收会话」就变成了永远不收。
    #[tokio::test]
    async fn a_session_that_times_out_repeatedly_is_finally_retired() {
        let session_name = format!("fixture-wedged-{}", std::process::id());
        connect_fixture(&session_name, None).await.unwrap();

        for attempt in 1..=MAX_CONSECUTIVE_TIMEOUTS {
            let error = request_on_session(
                &key(&session_name),
                "tools/call",
                json!({"name":"delay_echo","arguments":{"text":"卡住","delay_ms":5_000}}),
                1,
            )
            .expect_err("超过预算的调用应当返回超时");
            assert!(error.contains("MCP 请求超时"), "{error}");
            let slot = find_session_slot(&key(&session_name)).unwrap().unwrap();
            let still_there = slot.lock().unwrap().is_some();
            if attempt < MAX_CONSECUTIVE_TIMEOUTS {
                assert!(still_there, "第 {attempt} 次超时还不该收会话");
            } else {
                assert!(!still_there, "连续 {attempt} 次不回话就该收掉了");
            }
        }
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
        let before = find_session_slot(&key(&session_name)).unwrap().unwrap();

        mcp_disconnect(session_name.clone()).await.unwrap();
        assert!(!mcp_status(session_name.clone()).await.unwrap());
        connect_fixture(&session_name, None).await.unwrap();

        let after = find_session_slot(&key(&session_name)).unwrap().unwrap();
        assert!(Arc::ptr_eq(&before, &after));
        assert!(mcp_status(session_name.clone()).await.unwrap());
        mcp_disconnect(session_name).await.unwrap();
    }

    /// 两个项目里叫同一个名字的服务，必须是两个各不相干的服务。
    ///
    /// `filesystem` / `github` / `memory` 这些名字，两个项目十有八九取得一模一样。以前只用名字
    /// 做键：打开第二个项目 = 第一个项目的服务进程被杀，而它那边在飞的调用会落到第二个项目的
    /// 进程上——别人的 cwd、别人的 env、别人的数据。这条按 pid 验，因为「哪个进程真的执行了这次
    /// 调用」是唯一不会骗人的证据。
    #[tokio::test]
    async fn one_server_name_in_two_projects_is_two_independent_servers() {
        let name = format!("filesystem-{}", std::process::id());
        let first = SessionKey::new("main", "/tmp/michael-project-a", &name);
        let second = SessionKey::new("project-2", "/tmp/michael-project-b", &name);
        connect_fixture_at(&first, env!("CARGO_MANIFEST_DIR"), None)
            .await
            .unwrap();
        connect_fixture_at(&second, env!("CARGO_MANIFEST_DIR"), None)
            .await
            .unwrap();

        let first_pid = slot_child_pid(&first).expect("第一个项目的会话被第二个顶掉了");
        let second_pid = slot_child_pid(&second).expect("第二个项目应当有自己的会话");
        assert_ne!(
            first_pid, second_pid,
            "两个项目共用了同一个子进程：一个项目的调用会读到另一个项目的数据"
        );

        let says_first = call_text_at(first.clone(), "echo".into(), Some(json!({"pid": true})))
            .await
            .unwrap();
        let says_second = call_text_at(second.clone(), "echo".into(), Some(json!({"pid": true})))
            .await
            .unwrap();
        assert_eq!(says_first, first_pid.to_string(), "调用落到了别的项目的进程上");
        assert_eq!(says_second, second_pid.to_string(), "调用落到了别的项目的进程上");

        // 关掉一个项目不该把另一个一起关掉——这正是用户开第二个项目标签页时看到的症状。
        disconnect_at(first.clone()).await.unwrap();
        assert!(!status_at(first).await.unwrap());
        assert!(
            status_at(second.clone()).await.unwrap(),
            "断开一个项目的服务，把另一个项目的也带走了"
        );
        assert_eq!(
            call_text_at(second.clone(), "echo".into(), Some(json!({"pid": true})))
                .await
                .unwrap(),
            second_pid.to_string()
        );
        disconnect_at(second).await.unwrap();
    }

    /// 关掉一个窗口，只收这个窗口的服务。
    ///
    /// lib.rs 以前一个窗口事件都不监听：关掉副窗口，它的 MCP 子进程活到整个 App 退出。按窗口
    /// 分区之后这个泄漏是成倍的（每个窗口一整套），所以补上的收尸点必须**只**收这个标签下的。
    #[tokio::test]
    async fn closing_one_window_reaps_only_that_windows_servers() {
        let name = format!("memory-{}", std::process::id());
        let doomed = SessionKey::new("window-closing", "", &name);
        let kept = SessionKey::new("window-staying", "", &name);
        connect_fixture_at(&doomed, env!("CARGO_MANIFEST_DIR"), None)
            .await
            .unwrap();
        connect_fixture_at(&kept, env!("CARGO_MANIFEST_DIR"), None)
            .await
            .unwrap();

        stop_window("window-closing");

        assert!(
            find_session_slot(&doomed).unwrap().is_none(),
            "槽位还在会话表里 = 这个窗口的子进程会一直活到退出 App"
        );
        assert!(
            status_at(kept.clone()).await.unwrap(),
            "别的窗口的同名服务被一起收掉了"
        );
        assert_eq!(
            call_text_at(kept.clone(), "echo".into(), Some(json!({"text":"还在"})))
                .await
                .unwrap(),
            "还在"
        );
        disconnect_at(kept).await.unwrap();
    }

    /// 一直在报进度的长任务，不该被「多久没动静」的预算掐掉。
    ///
    /// 以前 `Session::request` 只认两种帧（我们的回应、服务反过来问我们的请求），进度通知被
    /// 直接丢掉——于是一次装依赖、跑测试、爬网页的调用，只要超过那个固定预算就一律算失败，
    /// 哪怕服务一秒一条地在报「我还在跑」。
    ///
    /// 时长按比例缩过（每秒一次进度、总共 9 秒、静默预算 3 秒）：被验的是「进度把预算往后推」
    /// 这条性质，而不是某个具体秒数，放大十倍只会让整套测试每次都多等一分半。
    #[tokio::test]
    async fn progress_notifications_keep_a_long_call_from_timing_out() {
        let session_name = format!("fixture-progress-{}", std::process::id());
        connect_fixture(&session_name, None).await.unwrap();

        let started = Instant::now();
        let result = on_session(&key(&session_name), |session| {
            session.request_with_progress(
                "tools/call",
                json!({"name":"delay_echo","arguments":{
                    "text":"跑完了","delay_ms":9_000,"progress_ms":1_000
                }}),
                3,
                30,
            )
        })
        .expect("一直在报进度的调用不该被静默预算掐掉");
        assert_eq!(flatten_tool_content(&result), "跑完了");
        assert!(
            started.elapsed() >= Duration::from_secs(8),
            "根本没等满就返回了，说明这条根本没走到进度那条路上"
        );

        // 同一个会话、同一段等待，**不报进度**的时候必须照旧超时——否则上面那条只是把预算
        // 调宽了，和进度没关系。
        let error = on_session(&key(&session_name), |session| {
            session.request_with_progress(
                "tools/call",
                json!({"name":"delay_echo","arguments":{"text":"没人报进度","delay_ms":9_000}}),
                3,
                30,
            )
        })
        .expect_err("没有进度的话，3 秒静默预算就该到头");
        assert!(error.contains("MCP 请求超时"), "{error}");
        mcp_disconnect(session_name).await.unwrap();
    }

    /// 但话痨也不能把一轮永远吊住：总时长是硬的。
    #[tokio::test]
    async fn an_endless_progress_stream_still_hits_the_absolute_deadline() {
        let session_name = format!("fixture-progress-forever-{}", std::process::id());
        connect_fixture(&session_name, None).await.unwrap();

        let started = Instant::now();
        let error = on_session(&key(&session_name), |session| {
            session.request_with_progress(
                "tools/call",
                json!({"name":"delay_echo","arguments":{
                    "text":"永远不会到","delay_ms":60_000,"progress_ms":200
                }}),
                2,
                4,
            )
        })
        .expect_err("每 200 毫秒报一次进度的服务能把静默预算无限往后推，总时长必须能兜住");
        assert!(error.contains("MCP 请求超时"), "{error}");
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "等了 {:?}：总时长这道硬顶没起作用",
            started.elapsed()
        );
        mcp_disconnect(session_name).await.unwrap();
    }

    /// 点「停」要真的停，而且**只**停这一条——服务得留着继续用。
    #[tokio::test]
    async fn cancelling_an_in_flight_call_keeps_the_server_usable() {
        let session_name = format!("fixture-cancel-inflight-{}", std::process::id());
        let session_key = key(&session_name);
        let log_path = std::env::temp_dir().join(format!("mcp-cancel-inflight-{session_name}"));
        let started_path = std::env::temp_dir().join(format!("mcp-cancel-started-{session_name}"));
        let _ = std::fs::remove_file(&log_path);
        let _ = std::fs::remove_file(&started_path);
        connect_fixture(
            &session_name,
            Some(HashMap::from([(
                "MCP_FIXTURE_CANCEL_LOG".to_string(),
                log_path.to_string_lossy().into_owned(),
            )])),
        )
        .await
        .unwrap();
        let pid_before = session_child_pid(&session_name).unwrap();
        let doomed_id = session_next_id(&session_name).unwrap();

        // 手上没有请求的时候点取消：标志绝不能留下来，否则下一次调用会当场被取消，
        // 而用户完全不知道自己什么时候「取消」过它。
        assert!(
            !cancel_at(session_key.clone()).await.unwrap(),
            "没有请求在飞的时候不该报告「取消到了」"
        );

        let in_flight = {
            let key = session_key.clone();
            let started = started_path.to_string_lossy().into_owned();
            tokio::task::spawn_blocking(move || {
                call_on_session(
                    &key,
                    "delay_echo".into(),
                    Some(json!({"text":"来不及","delay_ms":30_000,"started_path":started})),
                )
            })
        };
        tokio::time::timeout(Duration::from_secs(5), async {
            while !started_path.is_file() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("fixture 应当已经收到那条慢调用");

        assert!(
            cancel_at(session_key.clone()).await.unwrap(),
            "取消时会话那把锁正被在飞的请求握着——去抢锁只会等到它自己结束，等于没取消"
        );
        let error = in_flight
            .await
            .unwrap()
            .expect_err("被取消的调用必须返回错误，而不是继续等");
        assert!(error.contains(CANCELLED_ERROR), "{error}");
        assert!(
            !transport_session_error(&error),
            "取消的错误串撞上了传输层前缀：那会连服务一起收掉，而取消的意义正是把服务留着"
        );

        // 服务被告知了，且告知的正是那一条。
        tokio::time::timeout(Duration::from_secs(3), async {
            while std::fs::read_to_string(&log_path).unwrap_or_default().trim().is_empty() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("取消之后服务应当收到 notifications/cancelled");
        assert_eq!(
            std::fs::read_to_string(&log_path).unwrap().trim(),
            doomed_id.to_string()
        );

        assert!(mcp_status(session_name.clone()).await.unwrap());
        assert_eq!(session_child_pid(&session_name), Some(pid_before), "服务被换掉了");
        let echo = mcp_call(session_name.clone(), "echo".into(), Some(json!({"text":"下一条"})))
            .await
            .expect("取消之后同一个会话应当照常能用");
        assert_eq!(echo, "下一条");

        mcp_disconnect(session_name).await.unwrap();
        let _ = std::fs::remove_file(&log_path);
        let _ = std::fs::remove_file(&started_path);
    }

    /// 服务说「我的清单变了」之后，要能拿到新清单——而且是在**同一个进程**上。
    #[tokio::test]
    async fn a_list_changed_notice_survives_until_the_next_ping_and_rediscovers_in_place() {
        let session_name = format!("fixture-list-changed-{}", std::process::id());
        let session_key = key(&session_name);
        let tools = connect_fixture(
            &session_name,
            Some(HashMap::from([(
                "MCP_FIXTURE_LIST_CHANGED".to_string(),
                "1".to_string(),
            )])),
        )
        .await
        .unwrap();
        assert!(!tools.iter().any(|tool| tool.name == "late_bloomer"));
        let pid_before = session_child_pid(&session_name).unwrap();

        mcp_call(session_name.clone(), "echo".into(), Some(json!({"text":"触发"})))
            .await
            .unwrap();
        // 通知是在回应**之后**发出来的，所以此刻还躺在管道里：调用那一层拿到回应就返回了。
        // 每轮的 mcp_status ping 就是排空它的那一下（没有后台读线程能代劳——那个 Receiver
        // 归 Session 独占，另起一条会把在飞的响应偷走）。
        assert!(mcp_status(session_name.clone()).await.unwrap());

        let changes = take_changes_at(session_key.clone()).await.unwrap();
        assert!(changes.tools, "服务宣告过工具清单变了，这里必须看得到");
        assert!(
            !take_changes_at(session_key.clone()).await.unwrap().tools,
            "取走之后必须清空，否则面板会一直重复刷新"
        );

        let again = rediscover_at(session_key.clone()).await.unwrap();
        assert!(
            again.tools.iter().any(|tool| tool.name == "late_bloomer"),
            "重新发现没拿到新工具：{:?}",
            again.tools.iter().map(|t| t.name.as_str()).collect::<Vec<_>>()
        );
        assert_eq!(
            session_child_pid(&session_name),
            Some(pid_before),
            "重新发现把服务重启了：清单会变往往正是因为服务里刚有状态变化，重启会把它抹掉"
        );
        mcp_disconnect(session_name).await.unwrap();
    }

    /// 服务自己报的诊断（`notifications/message`）要能被用户看到；而 `logging/setLevel`
    /// **只在服务声明了 logging 能力时**才发。
    #[tokio::test]
    async fn a_server_that_declares_logging_gets_its_diagnostics_surfaced() {
        let logging_name = format!("fixture-logging-{}", std::process::id());
        connect_fixture(
            &logging_name,
            Some(HashMap::from([(
                "MCP_FIXTURE_LOGGING".to_string(),
                "1".to_string(),
            )])),
        )
        .await
        .unwrap();
        let log = server_log_at(key(&logging_name)).await.unwrap();
        assert!(
            log.iter().any(|line| line.contains("level set to info")),
            "服务的诊断没进到那条尾巴里：{log:?}"
        );
        assert!(
            log.iter().any(|line| line.contains("fixture")),
            "诊断要带上是谁说的：{log:?}"
        );

        // 没声明 logging 的服务不该收到 setLevel：它会规规矩矩回一条 -32601，白多一次往返，
        // 还在诊断里留下一条与真实故障无关的报错。
        let plain_name = format!("fixture-no-logging-{}", std::process::id());
        connect_fixture(&plain_name, None).await.unwrap();
        let plain_log = server_log_at(key(&plain_name)).await.unwrap();
        assert!(
            !plain_log.iter().any(|line| line.contains("level set to")),
            "服务没声明 logging，客户端却发了 setLevel：{plain_log:?}"
        );

        // 会话不在时回空数组而不是报错——服务连不上的时候正是最想看这份日志的时候。
        assert!(server_log_at(key("从来没连过的服务")).await.unwrap().is_empty());

        mcp_disconnect(logging_name).await.unwrap();
        mcp_disconnect(plain_name).await.unwrap();
    }

    /// 服务卡住的时候，它的日志必须还看得见。
    ///
    /// 这份日志的用处几乎全在「出事的时候」，而出事最常见的样子就是某次调用一直不回来——那时
    /// 会话锁正被那条调用握着。从会话槽里去读的话，越是要看越看不到：查日志这个动作会一直等到
    /// 那条卡住的调用自己结束。所以它挂在 `SIDE_CHANNELS` 上，不经过会话锁。
    #[tokio::test]
    async fn the_server_log_is_readable_while_a_call_is_stuck() {
        let session_name = format!("fixture-log-while-stuck-{}", std::process::id());
        let session_key = key(&session_name);
        let started_path = std::env::temp_dir().join(format!("mcp-stuck-started-{session_name}"));
        let _ = std::fs::remove_file(&started_path);
        connect_fixture(
            &session_name,
            Some(HashMap::from([(
                "MCP_FIXTURE_LOGGING".to_string(),
                "1".to_string(),
            )])),
        )
        .await
        .unwrap();

        let stuck = {
            let key = session_key.clone();
            let started = started_path.to_string_lossy().into_owned();
            tokio::task::spawn_blocking(move || {
                call_on_session(
                    &key,
                    "delay_echo".into(),
                    Some(json!({"text":"卡着","delay_ms":30_000,"started_path":started})),
                )
            })
        };
        tokio::time::timeout(Duration::from_secs(5), async {
            while !started_path.is_file() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("fixture 应当已经收到那条慢调用");

        let log = tokio::time::timeout(Duration::from_secs(2), server_log_at(session_key.clone()))
            .await
            .expect("查日志被那条卡住的调用挡住了——而那正是最需要看日志的时刻")
            .unwrap();
        assert!(
            log.iter().any(|line| line.contains("level set to info")),
            "{log:?}"
        );

        cancel_at(session_key.clone()).await.unwrap();
        let _ = stuck.await;
        mcp_disconnect(session_name).await.unwrap();
        let _ = std::fs::remove_file(&started_path);
    }

    /// 一个正在干活的服务，不该把下一轮的工具准备整个吊住。
    ///
    /// status_at 以前用阻塞锁，而这把锁被在飞的请求握着（工具调用最长 600 秒）；前端每轮
    /// 开工前都要对每个已连接服务 await 一次 mcp_status，还是 Promise.all。于是用户发一句
    /// 话，界面就停在"准备工具"那步不动——而服务其实好好的，只是忙。
    #[tokio::test]
    async fn status_answers_immediately_while_the_session_is_busy() {
        let session_name = format!("fixture-status-while-busy-{}", std::process::id());
        let session_key = key(&session_name);
        let started_path = std::env::temp_dir().join(format!("mcp-busy-started-{session_name}"));
        let _ = std::fs::remove_file(&started_path);
        connect_fixture(&session_name, None).await.unwrap();

        let stuck = {
            let key = session_key.clone();
            let started = started_path.to_string_lossy().into_owned();
            tokio::task::spawn_blocking(move || {
                call_on_session(
                    &key,
                    "delay_echo".into(),
                    Some(json!({"text":"忙着","delay_ms":30_000,"started_path":started})),
                )
            })
        };
        tokio::time::timeout(Duration::from_secs(5), async {
            while !started_path.is_file() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("fixture 应当已经收到那条慢调用");

        let alive = tokio::time::timeout(Duration::from_secs(2), status_at(session_key.clone()))
            .await
            .expect("忙碌会话把 mcp_status 吊住了——下一轮的工具准备会跟着一起卡")
            .unwrap();
        assert!(
            alive,
            "锁被在飞的请求握着，正说明它活着；这时候答「不在」会把一个正忙的服务当死的回收"
        );

        cancel_at(session_key.clone()).await.unwrap();
        let _ = stuck.await;
        mcp_disconnect(session_name).await.unwrap();
    }

    /// 服务回的 JSON-RPC 错误要把 `code` 和 `data` 一起带出来。
    ///
    /// 只取 `message` 的话，`-32602` 那一类的具体原因（哪个参数不对）全在 `data` 里被扔掉，
    /// 用户和模型看到的只有一句「Invalid arguments」，没法据此改。
    #[tokio::test]
    async fn a_server_error_keeps_its_code_and_data() {
        let session_name = format!("fixture-error-detail-{}", std::process::id());
        connect_fixture(&session_name, None).await.unwrap();
        let error = mcp_call(session_name.clone(), "no_such_tool".into(), None)
            .await
            .expect_err("未知工具应当报错");
        assert!(error.contains("unknown tool"), "{error}");
        assert!(error.contains("-32602"), "错误码丢了：{error}");
        assert!(error.contains("no_such_tool"), "data 里的细节丢了：{error}");
        assert!(
            !transport_session_error(&error),
            "服务答了话就不算通道断了，这个错误串不能撞上传输层前缀"
        );
        assert!(mcp_status(session_name.clone()).await.unwrap());
        mcp_disconnect(session_name).await.unwrap();
    }

    /// 版本对不上要在**握手那一刻**停下来，而不是揣着一个谈不拢的会话往下走。
    ///
    /// 单验 `check_protocol_version` 是不够的：把握手里那句改成 `let _ = ...`（＝根本不看
    /// 它的结论），只验函数的那条守卫照样绿。这条走的是 connect_full_within——和运行时
    /// 同一条路，钉的是"这个判断真的挂在链路上"。
    #[tokio::test]
    async fn the_handshake_actually_refuses_a_version_it_cannot_speak() {
        let session_name = format!("fixture-badproto-{}", std::process::id());
        let error = connect_full_within(
            key(&session_name),
            "node".into(),
            Some(vec![fixture_path()]),
            Some(HashMap::from([(
                "MCP_FIXTURE_PROTOCOL_VERSION".to_string(),
                "1999-01-01".to_string(),
            )])),
            Some(env!("CARGO_MANIFEST_DIR").into()),
            false,
            5,
        )
        .expect_err("服务回了一个客户端不认的协议版本，这次连接不该算成功");
        assert!(error.contains("1999-01-01"), "报错里没说服务要哪个版本：{error}");
        assert!(error.contains(PROTOCOL_VERSION), "报错里没说客户端支持哪些：{error}");

        // 反过来：服务回一个我们支持的旧版本，连接照常成立——这道闸不能顺手把
        // 还在 2024-11-05 上的服务全部拒掉。
        let ok_name = format!("fixture-oldproto-{}", std::process::id());
        connect_full_within(
            key(&ok_name),
            "node".into(),
            Some(vec![fixture_path()]),
            Some(HashMap::from([(
                "MCP_FIXTURE_PROTOCOL_VERSION".to_string(),
                "2024-11-05".to_string(),
            )])),
            Some(env!("CARGO_MANIFEST_DIR").into()),
            false,
            5,
        )
        .expect("2024-11-05 在支持表里，不该被拒");
    }

    /// 远端服务在等浏览器授权时，会话必须**留着**。
    ///
    /// mcp-remote 第一次连接会开浏览器等用户点同意，这期间 initialize 没有回音。旧行为是超时
    /// 即弃：`Session::drop` 杀掉 mcp-remote，它正开着的 OAuth 回调服务器随之关闭，用户点完
    /// 「同意」，回调打到一个已经没人听的端口，令牌永远写不进 ~/.mcp-auth——于是这个服务永远
    /// 连不上，而且每次失败的样子都一模一样。
    ///
    /// 握手预算在这里传 1 秒：真实值是 180 秒（`REMOTE_HANDSHAKE_SECS`），靠等满它才能走到
    /// 这条路的测试等于没有测试。
    #[tokio::test]
    async fn a_remote_server_waiting_for_browser_auth_is_parked_not_killed() {
        // 这两条断言读的是进程级环境变量，和改它的那条用例要排队，见 ENV_LOCK。
        let _serial = ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        assert_eq!(handshake_budget(true), REMOTE_HANDSHAKE_SECS);
        assert_eq!(handshake_budget(false), LOCAL_HANDSHAKE_SECS);

        let session_name = format!("fixture-oauth-{}", std::process::id());
        let session_key = key(&session_name);
        let error = connect_full_within(
            session_key.clone(),
            "node".into(),
            Some(vec![fixture_path()]),
            Some(HashMap::from([(
                "MCP_FIXTURE_IGNORE_INITIALIZE".to_string(),
                "1".to_string(),
            )])),
            Some(env!("CARGO_MANIFEST_DIR").into()),
            true,
            1,
        )
        .expect_err("握手没完成，连接本身当然算失败");
        assert!(error.contains("等待浏览器授权"), "{error}");

        let slot = find_session_slot(&session_key).unwrap().unwrap();
        assert!(
            slot.lock().unwrap().is_some(),
            "会话被丢掉了 = mcp-remote 被杀 = 用户点完同意也拿不到令牌"
        );
        assert!(pending_auth_at(session_key.clone()).await.unwrap());
        assert!(
            status_at(session_key.clone()).await.unwrap(),
            "进程还活着就该报「在」；这里去 ping 它反而会攒够超时把它收掉"
        );
        // 这种会话不能当正常会话用：直接说清楚，而不是让用户等满一次调用预算。
        let refused = call_on_session(&session_key, "echo".into(), None)
            .expect_err("握手还没完成的会话不该接受调用");
        assert!(refused.contains("等待浏览器授权"), "{refused}");

        // 本地服务同样超时的话，照旧是失败——「等授权」这个说法只对远端成立。
        let local_name = format!("fixture-local-nohandshake-{}", std::process::id());
        let local_error = connect_full_within(
            key(&local_name),
            "node".into(),
            Some(vec![fixture_path()]),
            Some(HashMap::from([(
                "MCP_FIXTURE_IGNORE_INITIALIZE".to_string(),
                "1".to_string(),
            )])),
            Some(env!("CARGO_MANIFEST_DIR").into()),
            false,
            1,
        )
        .expect_err("本地服务握手超时就是失败");
        assert!(local_error.contains("MCP 请求超时"), "{local_error}");
        assert!(!pending_auth_at(key(&local_name)).await.unwrap());

        disconnect_at(session_key).await.unwrap();
        disconnect_at(key(&local_name)).await.unwrap();
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


#[cfg(test)]
mod live_user_config_tests {
    /// 用 IDE 自己的客户端，按用户真实配置连一次、列一次工具。
    ///
    /// 「配置写进去了」和「IDE 真的能用它」是两回事：GUI 启动的 App 拿到的 PATH 很窄，
    /// 裸 npx 常常 spawn 不起来；配置形状写错也只会在运行时静默变成"没有工具"。
    /// 这条测试走的是 connect_full_blocking——和运行时同一条路。
    ///
    /// 标 ignore：它会真的拉起子进程、走网络装包，不适合进常规 CI。
    /// 手动跑：cargo test --release live_user_config -- --ignored --nocapture
    #[test]
    #[ignore]
    fn 用户配置里的服务能真连上并列出工具() {
        let cfg = super::mcp_user_config().expect("读用户 MCP 配置");
        let mut checked = 0;
        {
            let Some(servers) = cfg.servers.as_object() else { return };
            for (name, spec) in servers {
                let command = spec.get("command").and_then(|v| v.as_str()).unwrap_or("");
                if command.is_empty() {
                    continue;
                }
                let args: Vec<String> = spec
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
                    .unwrap_or_default();
                println!("  连接 {name}: {command} {}", args.join(" "));
                let out = super::connect_full_blocking(
                    super::SessionKey::new("main", "", name),
                    command.to_string(),
                    Some(args),
                    None,
                    None,
                    false,
                );
                match out {
                    Ok(d) => {
                        println!("    ✅ 工具 {} 个: {}", d.tools.len(),
                            d.tools.iter().map(|t| t.name.as_str())
                                .collect::<Vec<_>>().join(", "));
                        assert!(!d.tools.is_empty(), "{name} 连上了但一个工具都没有");
                        checked += 1;
                    }
                    Err(e) => panic!("{name} 连接失败：{e}"),
                }
            }
        }
        assert!(checked > 0, "用户配置里一个可连的 MCP 服务都没有");
    }
}
