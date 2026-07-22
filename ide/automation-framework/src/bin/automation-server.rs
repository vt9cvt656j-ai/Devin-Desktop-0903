//! Michael IDE 自动化服务：把有状态的 RpcServer 通过极简阻塞 HTTP `POST /rpc` 暴露给 IDE 智能体。
//! Agent 含 macOS !Send 句柄，全程单线程；用法：`automation-server [port]` 或 `AUTOMATION_PORT=3037`。
use rust_automation_framework::RpcServer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .or_else(|| std::env::var("AUTOMATION_PORT").ok().and_then(|s| s.parse().ok()))
        .unwrap_or(3037);
    let server = RpcServer::new(port)?;
    server.serve_http_blocking()?;
    Ok(())
}
