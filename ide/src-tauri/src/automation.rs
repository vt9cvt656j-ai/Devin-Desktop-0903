//! 对接桌面自动化框架（~/Desktop/自动化工具框架）。
//!
//! 那个框架编成一个 `automation-server` 二进制：一个**有状态**的本地 HTTP-RPC 服务
//! （`POST /rpc`，浏览器会话 + 录制状态常驻），暴露 browser/mouse/keyboard/system/**录制回放**。
//! 这里 spawn 它、并给 IDE 智能体一个 `automation_call` 命令去驱动它——所有引擎都能用同一个服务。
//! （Agent 含 macOS !Send 句柄，服务端本身是单线程阻塞的，我们只是 HTTP 客户端，无所谓。）

use std::process::Child;
use std::sync::Mutex;
use std::time::Duration;

const PORT: u16 = 3037;
const MAX_RPC_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

/// spawn 出来的服务进程句柄，进程内唯一。
static CHILD: Mutex<Option<Child>> = Mutex::new(None);

fn packaged_server_filename(executable_suffix: &str) -> String {
    format!("automation-server{executable_suffix}")
}

/// 解析 automation-server 二进制路径：env 覆盖 → **打进 App 的 sidecar**（发行版所有用户都有）
/// → 开发期磁盘上的框架产物。sidecar 由 Tauri externalBin 放在主程序同目录（Contents/MacOS/）。
fn server_bin() -> Option<std::path::PathBuf> {
    if let Ok(p) = std::env::var("MICHAEL_AUTOMATION_BIN") {
        let pb = std::path::PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    // 打包版：Tauri 把 externalBin 放到主程序旁，并去掉目标三元组后缀。
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let side = dir.join(packaged_server_filename(std::env::consts::EXE_SUFFIX));
            if side.exists() {
                return Some(side);
            }
        }
    }
    // 开发期兜底：仓库内 / 桌面上的框架构建产物。
    let home = std::env::var("HOME").unwrap_or_default();
    for rel in [
        "Desktop/Michael-IDE/Devin-Desktop/ide/automation-framework/target/release/automation-server",
        "Desktop/自动化工具框架/target/release/automation-server",
        "Desktop/自动化工具框架/target/debug/automation-server",
    ] {
        let pb = std::path::PathBuf::from(&home).join(rel);
        if pb.exists() {
            return Some(pb);
        }
    }
    None
}

/// 端口上那个服务，是不是**我们自己刚起的那个 sidecar**。
///
/// 这里以前有两个错，合起来让「先占住 127.0.0.1:3037 就能接管全部桌面自动化」一直成立：
///
///   1. 只看 HTTP 200。冒充者不运行我们的代码，它想回什么状态码就回什么。
///      而当时的注释写着「冒充者不知道这个一次性 token，健康检查会失败」——
///      那句话描述的是一个从没实现过的校验。
///   2. **把 token 发给了尚未验明身份的一方**。请求头里带着 x-automation-token 去问
///      "你是谁"，等于先把一次性密钥交到冒充者手上，它随后可以拿着它满足任何后续校验。
///
/// 接管之后流过去的包括 keyboard.type 的正文（用户正在输入的密码）、clipboard.get、
/// browser.content —— 而这一层还能合成真实键鼠，也就是能开终端敲任意命令。
///
/// 现在改成挑战应答：我们发一个随机 nonce（明文，不是凭据），sidecar 回
/// SHA-256("<token>:<nonce>")。只有真持有 token 的一方算得出来，而 token 从不出本进程。
async fn health_ok() -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(600))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    let expected = health_challenge_response(AUTOMATION_TOKEN.as_str(), &nonce);
    let Ok(resp) = client
        .get(format!("http://127.0.0.1:{PORT}/health"))
        .header("x-automation-nonce", &nonce)
        .send()
        .await
    else {
        return false;
    };
    if !resp.status().is_success() {
        return false;
    }
    let Ok(body) = resp.text().await else { return false };
    // 常量时间比较：这条路每次失败都会重试，时序差异是可观测的。
    constant_time_eq(body.trim().as_bytes(), expected.as_bytes())
}

/// 和 sidecar 侧 `health_challenge_response` 必须逐字节一致（automation-framework/src/rpc.rs）。
fn health_challenge_response(token: &str, nonce: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hasher.update(b":");
    hasher.update(nonce.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// 与本次进程的 automation-server 共享的一次性密钥。
///
/// sidecar 能合成**真实的鼠标键盘事件**，也就是能打开终端敲任意命令；它监听固定端口
/// 127.0.0.1:3037 且随签名安装包分发。没有这道闸时，用户只要用过一次桌面自动化，
/// 之后在浏览器里打开的**任意网页**都能 fetch 到它、零交互拿到本机代码执行。
///
/// 用**自定义请求头**携带是关键：自定义头会强制浏览器发 CORS 预检，而 sidecar 不响应
/// OPTIONS —— 网页因此永远发不出这个头，被物理挡在门外；本进程不受影响。
static AUTOMATION_TOKEN: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| uuid::Uuid::new_v4().simple().to_string());

/// 确保服务在跑：健康就直接用；否则 spawn 一个、等它就绪（最多 ~4s）。
async fn ensure_server() -> Result<(), String> {
    if health_ok().await {
        return Ok(());
    }
    {
        // spawn 是同步且很快的；MutexGuard 不 Send，必须在任何 await 之前 drop。
        let mut guard = CHILD.lock().unwrap();
        if let Some(ch) = guard.as_mut() {
            if let Ok(Some(_)) = ch.try_wait() {
                *guard = None; // 死了的进程回收
            }
        }
        if guard.is_none() {
            let bin = server_bin().ok_or_else(|| {
                "找不到 automation-server：请先在 ~/Desktop/自动化工具框架 里 \
                 `cargo build --release --features 'system browser' --bin automation-server`，\
                 或用环境变量 MICHAEL_AUTOMATION_BIN 指定其路径。"
                    .to_string()
            })?;
            let mut command = crate::process_util::command(bin.to_string_lossy().as_ref());
            command
                .arg(PORT.to_string())
                .env("MICHAEL_AUTOMATION_TOKEN", AUTOMATION_TOKEN.as_str())
                .env("PATH", crate::process_util::augmented_path(None))
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                command.process_group(0);
            }
            let child = command
                .spawn()
                .map_err(|e| format!("启动 automation-server 失败: {e}"))?;
            *guard = Some(child);
        }
    }
    for _ in 0..40 {
        if health_ok().await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    // A process that never becomes healthy must not stay cached forever. Without
    // this cleanup every later call waits another four seconds on the same wedged
    // child, and the IDE also leaves that process behind on exit.
    if let Some(mut child) = take_child() {
        terminate_child(&mut child);
    }
    Err("automation-server 启动后未就绪（健康检查超时）".into())
}

fn take_child() -> Option<Child> {
    CHILD.lock().ok()?.take()
}

fn terminate_child(child: &mut Child) {
    #[cfg(unix)]
    unsafe {
        // The sidecar is a process-group leader, so this also stops browser and
        // recorder children it created instead of orphaning them on IDE exit.
        let _ = libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        let _ = crate::process_util::command("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn parse_rpc_response(
    status: reqwest::StatusCode,
    value: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if let Some(error) = value.get("error") {
        return Err(error
            .get("message")
            .and_then(|message| message.as_str())
            .unwrap_or("automation error")
            .to_string());
    }
    if !status.is_success() {
        return Err(format!("自动化服务返回 HTTP {}", status.as_u16()));
    }
    value
        .get("result")
        .cloned()
        .ok_or_else(|| "自动化服务响应缺少 result/error".to_string())
}

async fn read_rpc_response(mut response: reqwest::Response) -> Result<serde_json::Value, String> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RPC_RESPONSE_BYTES as u64)
    {
        return Err(format!(
            "自动化服务响应超过 {} 字节上限",
            MAX_RPC_RESPONSE_BYTES
        ));
    }

    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("读取自动化服务响应失败: {error}"))?
    {
        if body.len().saturating_add(chunk.len()) > MAX_RPC_RESPONSE_BYTES {
            return Err(format!(
                "自动化服务响应超过 {} 字节上限",
                MAX_RPC_RESPONSE_BYTES
            ));
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|error| format!("解析自动化服务响应失败: {error}"))
}

/// 智能体驱动桌面自动化框架的统一入口。method 例：
/// `browser.goto` / `browser.click` / `mouse.click` / `keyboard.type` / `system.init`
/// / `recorder.save` / `recorder.replay` / `recorder.list`。params 是对应的 JSON 参数。
#[tauri::command]
pub async fn automation_call(
    method: String,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    ensure_server().await?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1
    });
    let resp = client
        .post(format!("http://127.0.0.1:{PORT}/rpc"))
        .header("x-automation-token", AUTOMATION_TOKEN.as_str())
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("调用自动化服务失败: {e}"))?;
    let status = resp.status();
    let v = read_rpc_response(resp).await?;
    parse_rpc_response(status, v)
}

/// 停掉服务（IDE 退出时调，别留孤儿进程）。
pub fn stop() {
    if let Some(mut child) = take_child() {
        terminate_child(&mut child);
    }
}

#[cfg(test)]
mod tests {
    use super::{packaged_server_filename, parse_rpc_response, terminate_child};

    #[test]
    fn packaged_server_filename_uses_unix_executable_name() {
        assert_eq!(packaged_server_filename(""), "automation-server");
    }

    #[test]
    fn packaged_server_filename_uses_windows_executable_name() {
        assert_eq!(packaged_server_filename(".exe"), "automation-server.exe");
    }

    #[test]
    fn malformed_rpc_response_is_not_reported_as_success() {
        let error = parse_rpc_response(reqwest::StatusCode::OK, serde_json::json!({}))
            .expect_err("missing result/error must fail");
        assert!(error.contains("缺少 result/error"));
    }

    #[test]
    fn rpc_error_message_is_preserved() {
        let error = parse_rpc_response(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({"error": {"message": "sidecar failed"}}),
        )
        .expect_err("RPC error must fail");
        assert_eq!(error, "sidecar failed");
    }

    #[cfg(unix)]
    #[test]
    fn terminate_child_reaps_process() {
        use std::os::unix::process::CommandExt;

        let mut command = std::process::Command::new("/bin/sh");
        command.args(["-c", "sleep 30 & wait"]).process_group(0);
        let mut child = command.spawn().expect("test child should start");
        terminate_child(&mut child);
        assert!(child.try_wait().unwrap().is_some());
    }
}

#[cfg(test)]
mod health_probe_tests {
    /// 健康探测**不许把 token 发出去**，且必须校验应答内容而不是只看状态码。
    ///
    /// 这两点各自失守都足以让「先占住 127.0.0.1:3037」接管全部桌面自动化：
    ///   · 只看 200 —— 冒充者不运行我们的代码，想回什么状态码就回什么；
    ///   · 发 token —— 请求是发给尚未验明身份的一方的，等于把一次性密钥送上门。
    #[test]
    fn probe_never_sends_the_token_and_verifies_the_answer() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/automation.rs"))
            .expect("read automation.rs");
        let at = src.find("async fn health_ok()").expect("health_ok 改名了");
        let body: String = src[at..].chars().take(1_600).collect();
        // 剥注释：解释性注释里会提到 x-automation-token，不剥的话断言会被自己的注释喂到。
        let code: String = body
            .lines()
            .map(|l| match l.find("//") { Some(i) => &l[..i], None => l })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            !code.contains("x-automation-token"),
            "健康探测又把 token 发给未验明身份的一方了",
        );
        assert!(code.contains("x-automation-nonce"), "没有发送挑战用的 nonce");
        assert!(
            code.contains("health_challenge_response(AUTOMATION_TOKEN.as_str(), &nonce)"),
            "没有算出期望应答",
        );
        assert!(
            code.contains("constant_time_eq(body.trim().as_bytes(), expected.as_bytes())"),
            "没有校验应答内容——只看状态码等于没验",
        );
    }

    /// 两侧的构造必须逐字节一致，否则真 sidecar 也会被自己判成冒充者。
    #[test]
    fn challenge_matches_the_sidecar_implementation() {
        // SHA-256("tok:n1")，与 automation-framework/src/rpc.rs 的同名函数同源。
        let ours = super::health_challenge_response("tok", "n1");
        assert_eq!(ours.len(), 64);
        assert_ne!(ours, super::health_challenge_response("tok", "n2"));
        assert_ne!(ours, super::health_challenge_response("other", "n1"));
        assert!(super::constant_time_eq(ours.as_bytes(), ours.as_bytes()));
        assert!(!super::constant_time_eq(b"ok", ours.as_bytes()));
    }
}
