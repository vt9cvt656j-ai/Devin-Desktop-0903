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

async fn health_ok() -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(600))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    // 健康检查也带 token：它同时是「这个端口上的服务是不是**我们自己刚起的那个**」的
    // 判据。此前只要有任何进程在 3037 上回 200，ensure_server 就认它 —— 本机任意程序
    // 抢先占住这个固定端口即可冒充 sidecar，把后续所有自动化调用（含键鼠合成的参数）
    // 全部截走。冒充者不知道这个一次性 token，健康检查会失败。
    client
        .get(format!("http://127.0.0.1:{PORT}/health"))
        .header("x-automation-token", AUTOMATION_TOKEN.as_str())
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
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
