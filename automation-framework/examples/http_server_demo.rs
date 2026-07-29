//! HTTP 服务器演示
//! 
//! 启动本地 HTTP 服务，AI agent 通过 HTTP API 调用自动化能力

use rust_automation_framework::HttpServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 启动 Rust 自动化框架 HTTP 服务器...\n");
    
    let server = HttpServer::new(3030)?;
    
    println!("📝 使用示例:");
    println!("  curl http://localhost:3030/health");
    println!("  curl -X POST http://localhost:3030/api/system/init");
    println!("  curl -X POST http://localhost:3030/api/mouse/move -H 'Content-Type: application/json' -d '{{\"x\":500,\"y\":300}}'");
    println!("  curl -X POST http://localhost:3030/api/keyboard/type -H 'Content-Type: application/json' -d '{{\"text\":\"Hello from AI!\"}}'");
    println!("  curl -X POST http://localhost:3030/api/task/web_search -H 'Content-Type: application/json' -d '{{\"query\":\"Rust programming\",\"engine\":\"duckduckgo\"}}'");
    println!();
    
    server.start().await?;
    
    Ok(())
}
