//! JSON-RPC 服务层 - 让 AI agent 可以通过 HTTP 调用自动化框架
//! 
//! 这个模块提供了一个简单的 HTTP 服务器，接收 JSON-RPC 格式的请求

use crate::agent::Agent;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
    pub id: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

/// JSON-RPC 服务器
pub struct RpcServer {
    agent: Arc<Mutex<Agent>>,
    port: u16,
    /// 与父进程共享的一次性密钥（`MICHAEL_AUTOMATION_TOKEN`）。
    ///
    /// None 表示未配置：此时保持旧行为（方便单独跑 sidecar 调试），但浏览器来源的请求
    /// 依然一律拒绝。
    token: Option<String>,
}

/// 常量时间比较，避免用响应时间逐字节试出 token。
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

impl RpcServer {
    /// 创建新的 RPC 服务器
    pub fn new(port: u16) -> Result<Self> {
        let agent = Agent::new()?;

        Ok(Self {
            agent: Arc::new(Mutex::new(agent)),
            port,
            token: std::env::var("MICHAEL_AUTOMATION_TOKEN")
                .ok()
                .filter(|t| !t.trim().is_empty()),
        })
    }
    
    /// 处理 RPC 请求
    pub fn handle_request(&self, req: RpcRequest) -> RpcResponse {
        let result = self.execute_method(&req.method, req.params);
        
        match result {
            Ok(value) => RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(value),
                error: None,
                id: req.id,
            },
            Err(e) => RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError {
                    code: -32603,
                    message: e.to_string(),
                }),
                id: req.id,
            },
        }
    }
    
    fn execute_method(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let mut agent = self.agent.lock().unwrap();
        
        match method {
            // 浏览器方法
            #[cfg(feature = "browser")]
            "browser.start" => {
                let headless = params.get("headless").and_then(|v| v.as_bool()).unwrap_or(false);
                agent.browser_start(headless)?;
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            #[cfg(feature = "browser")]
            "browser.goto" => {
                let url = params.get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'url' parameter")))?;
                agent.browser_goto(url)?;
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            #[cfg(feature = "browser")]
            "browser.click" => {
                let selector = params.get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'selector' parameter")))?;
                agent.browser_click(selector)?;
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            #[cfg(feature = "browser")]
            "browser.type" => {
                let selector = params.get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'selector' parameter")))?;
                let text = params.get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'text' parameter")))?;
                agent.browser_type(selector, text)?;
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            #[cfg(feature = "browser")]
            "browser.wait" => {
                let selector = params.get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'selector' parameter")))?;
                let timeout = params.get("timeout")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5000);
                agent.browser_wait(selector, timeout)?;
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            #[cfg(feature = "browser")]
            "browser.eval" => {
                let script = params.get("script")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'script' parameter")))?;
                let result = agent.browser_eval(script)?;
                Ok(result)
            }
            
            #[cfg(feature = "browser")]
            "browser.screenshot" => {
                let path = params.get("path").and_then(|v| v.as_str());
                agent.browser_screenshot(path)?;
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            #[cfg(feature = "browser")]
            "browser.content" => {
                let content = agent.browser_content()?;
                Ok(serde_json::json!({"content": content}))
            }
            
            #[cfg(feature = "browser")]
            "browser.close" => {
                agent.browser_close()?;
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            // 系统方法
            #[cfg(feature = "system")]
            "system.init" => {
                agent.system_init()?;
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            #[cfg(feature = "system")]
            "mouse.move" => {
                let x = params.get("x")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'x' parameter")))? as i32;
                let y = params.get("y")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'y' parameter")))? as i32;
                agent.mouse_move(x, y)?;
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            #[cfg(feature = "system")]
            "mouse.click" => {
                let button = params.get("button").and_then(|v| v.as_str());
                agent.mouse_click(button)?;
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            #[cfg(feature = "system")]
            "mouse.double_click" => {
                let button = params.get("button").and_then(|v| v.as_str());
                agent.mouse_double_click(button)?;
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            #[cfg(feature = "system")]
            "mouse.drag" => {
                let from_x = params.get("from_x")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'from_x' parameter")))? as i32;
                let from_y = params.get("from_y")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'from_y' parameter")))? as i32;
                let to_x = params.get("to_x")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'to_x' parameter")))? as i32;
                let to_y = params.get("to_y")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'to_y' parameter")))? as i32;
                agent.mouse_drag(from_x, from_y, to_x, to_y)?;
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            #[cfg(feature = "system")]
            "mouse.scroll" => {
                let delta_x = params.get("delta_x")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
                let delta_y = params.get("delta_y")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'delta_y' parameter")))? as i32;
                agent.mouse_scroll(delta_x, delta_y)?;
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            #[cfg(feature = "system")]
            "keyboard.type" => {
                let text = params.get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'text' parameter")))?;
                agent.keyboard_type(text)?;
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            #[cfg(feature = "system")]
            "keyboard.press" => {
                let key = params.get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'key' parameter")))?;
                agent.keyboard_press(key)?;
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            #[cfg(feature = "system")]
            "keyboard.combo" => {
                let keys = params.get("keys")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'keys' parameter (array)")))?;
                let key_strs: Vec<&str> = keys.iter()
                    .filter_map(|v| v.as_str())
                    .collect();
                agent.keyboard_combo(key_strs)?;
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            #[cfg(feature = "system")]
            "sleep" => {
                let ms = params.get("ms")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'ms' parameter")))?;
                drop(agent);
                std::thread::sleep(Duration::from_millis(ms));
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            // ── 录制回放：一条 recording = 一串 {method, params} 步骤，回放即逐条 re-dispatch ──
            "recorder.save" => {
                let name = params.get("name").and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'name'")))?;
                let steps = params.get("steps").cloned().unwrap_or_else(|| serde_json::json!([]));
                drop(agent);
                let dir = recordings_dir();
                std::fs::create_dir_all(&dir).ok();
                let path = dir.join(format!("{}.json", sanitize_name(name)));
                let doc = serde_json::json!({ "name": name, "steps": steps });
                std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap_or_default())
                    .map_err(|e| Error::Other(anyhow::anyhow!("save recording failed: {}", e)))?;
                Ok(serde_json::json!({ "status": "ok", "path": path.to_string_lossy() }))
            }
            "recorder.list" => {
                drop(agent);
                let mut names = vec![];
                if let Ok(rd) = std::fs::read_dir(recordings_dir()) {
                    for e in rd.flatten() {
                        if let Some(n) = e.path().file_stem().and_then(|s| s.to_str()) { names.push(n.to_string()); }
                    }
                }
                Ok(serde_json::json!({ "recordings": names }))
            }
            "recorder.replay" => {
                // 回放：优先用传入的 steps，否则按 name 从盘上加载；逐条 re-dispatch，步间可选延时。
                drop(agent); // 释放锁，下面每步 execute_method 会重新加锁
                let steps: Vec<serde_json::Value> = if let Some(arr) = params.get("steps").and_then(|v| v.as_array()) {
                    arr.clone()
                } else if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
                    let path = recordings_dir().join(format!("{}.json", sanitize_name(name)));
                    let data = std::fs::read_to_string(&path)
                        .map_err(|e| Error::Other(anyhow::anyhow!("load recording '{}' failed: {}", name, e)))?;
                    serde_json::from_str::<serde_json::Value>(&data).ok()
                        .and_then(|v| v.get("steps").and_then(|s| s.as_array()).cloned())
                        .ok_or_else(|| Error::Other(anyhow::anyhow!("recording '{}' has no steps", name)))?
                } else {
                    return Err(Error::Other(anyhow::anyhow!("recorder.replay needs 'steps' or 'name'")));
                };
                let delay = params.get("delay_ms").and_then(|v| v.as_u64()).unwrap_or(250);
                let mut done = 0u64;
                for step in &steps {
                    let m = step.get("method").and_then(|v| v.as_str()).unwrap_or("");
                    if m.is_empty() { continue; }
                    let p = step.get("params").cloned().unwrap_or_else(|| serde_json::json!({}));
                    self.execute_method(m, p)?; // 复用同一套分发
                    done += 1;
                    if delay > 0 { std::thread::sleep(Duration::from_millis(delay)); }
                }
                Ok(serde_json::json!({ "status": "ok", "replayed": done }))
            }

            // ── 窗口/屏幕：平台层早就有（enumerate/activate/screen_info），此前一直没暴露给 RPC，
            //    AI 桌面自动化最需要的"激活目标应用再操作"因此做不了。补齐。 ──
            #[cfg(feature = "system")]
            "window.list" => {
                drop(agent);
                let ctrl = crate::platform::get_window_controller();
                let wins = ctrl.enumerate_windows()?;
                let list: Vec<serde_json::Value> = wins.iter().map(|w| serde_json::json!({
                    "title": w.title, "process": w.process_name,
                    "x": w.x, "y": w.y, "width": w.width, "height": w.height,
                    "visible": w.is_visible, "minimized": w.is_minimized,
                })).collect();
                Ok(serde_json::json!({ "windows": list }))
            }
            #[cfg(feature = "system")]
            "window.activate" => {
                let title = params.get("title").and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'title' parameter")))?;
                drop(agent);
                crate::platform::get_window_controller().activate_window(title)?;
                Ok(serde_json::json!({ "status": "ok" }))
            }
            #[cfg(feature = "system")]
            "window.minimize" => {
                let title = params.get("title").and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'title' parameter")))?;
                drop(agent);
                crate::platform::get_window_controller().minimize_window(title)?;
                Ok(serde_json::json!({ "status": "ok" }))
            }
            #[cfg(feature = "system")]
            "screen.info" => {
                drop(agent);
                let info = crate::platform::get_window_controller().get_screen_info()?;
                Ok(serde_json::json!({ "width": info.width, "height": info.height, "scale_factor": info.scale_factor }))
            }

            // ── 剪贴板：agent.rs 早就实现（clipboard_get/set、quick_paste 粘贴长文本比逐键快百倍），补暴露。 ──
            #[cfg(feature = "system")]
            "clipboard.get" => {
                let text = agent.clipboard_get_text()?;
                Ok(serde_json::json!({ "text": text }))
            }
            #[cfg(feature = "system")]
            "clipboard.set" => {
                let text = params.get("text").and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'text' parameter")))?;
                agent.clipboard_set_text(text)?;
                Ok(serde_json::json!({ "status": "ok" }))
            }
            #[cfg(feature = "system")]
            "keyboard.paste" => {
                let text = params.get("text").and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'text' parameter")))?;
                agent.quick_paste(text)?;
                Ok(serde_json::json!({ "status": "ok" }))
            }
            _ => Err(Error::Other(anyhow::anyhow!("Unknown method: {}", method))),
        }
    }

    /// 启动 HTTP-RPC 服务器（axum）——把这一个**有状态**的 RpcServer 通过 `POST /rpc` 暴露出去。
    /// 浏览器会话 + 录制状态在整个进程生命周期常驻；任何自动化引擎都能 POST /rpc 调它。
    /// 极简**单线程阻塞式** HTTP-RPC 服务（std only）。Agent 含 macOS !Send 句柄，必须全程钉在
    /// 一条线程上；自动化本就串行，单线程正合适。`POST /rpc` body=JSON-RPC → JSON-RPC 响应；`GET /health`→ok。
    pub fn serve_http_blocking(&self) -> Result<()> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = std::net::TcpListener::bind(&addr)
            .map_err(|e| Error::Other(anyhow::anyhow!("bind {} failed: {}", addr, e)))?;
        eprintln!("🚀 automation server on http://{}/rpc", addr);
        for stream in listener.incoming() {
            match stream {
                Ok(mut s) => { let _ = self.handle_conn(&mut s); }
                Err(_) => continue,
            }
        }
        Ok(())
    }

    fn handle_conn(&self, stream: &mut std::net::TcpStream) -> std::io::Result<()> {
        use std::io::{Read, Write, BufRead, BufReader};
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let mut parts = line.split_whitespace();
        let http_method = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("").to_string();
        // 读到空行为止，抓 Content-Length + 鉴权/来源判定所需的头
        let mut content_len = 0usize;
        let mut token: Option<String> = None;
        let mut browser_origin = false;
        loop {
            let mut h = String::new();
            let n = reader.read_line(&mut h)?;
            if n == 0 || h == "\r\n" || h == "\n" { break; }
            let low = h.to_ascii_lowercase();
            if let Some(v) = low.strip_prefix("content-length:") {
                content_len = v.trim().parse().unwrap_or(0);
            }
            if let Some(v) = low.strip_prefix("x-automation-token:") {
                token = Some(v.trim().to_string());
            }
            // 浏览器一定会带上这些头之一；本地父进程（reqwest）一个都不带。
            if low.starts_with("origin:")
                || low.starts_with("referer:")
                || low.starts_with("sec-fetch-site:")
                || low.starts_with("sec-fetch-mode:")
            {
                browser_origin = true;
            }
        }
        let body = if content_len > 0 {
            let mut buf = vec![0u8; content_len];
            reader.read_exact(&mut buf)?;
            buf
        } else { Vec::new() };

        // ── 鉴权 ──────────────────────────────────────────────────────────────
        //
        // 这个服务能合成**真实的鼠标键盘事件**（mouse.click / keyboard.type /
        // keyboard.combo），也就是说能打开终端敲任意命令。它监听 127.0.0.1 上一个固定
        // 端口、随签名安装包分发，且此前对每个响应都回 `Access-Control-Allow-Origin: *`。
        //
        // 后果：用户只要用过一次桌面自动化，之后在**普通浏览器里打开的任意网页**（包括
        // 第三方广告 iframe）就能 fetch 到这里 —— `text/plain` 属于 CORS 安全列表内容
        // 类型、不触发预检，请求直达，零用户交互拿到本机代码执行。
        //
        // 两道闸：
        // 1. 共享密钥走**自定义请求头**。自定义头会强制浏览器发 CORS 预检，而我们不响应
        //    OPTIONS —— 于是浏览器永远发不出这个头，网页被物理挡在门外；本地父进程
        //    （reqwest）不受影响。
        // 2. 只要出现任何浏览器指纹头（Origin / Referer / Sec-Fetch-*）就直接拒绝。
        //
        // ACAO 头也一并删掉：它此前额外解锁了「读回响应」的能力（browser.content 等），
        // 让攻击从盲写升级成可读写。注意只删 ACAO 不能修掉 RCE —— 写侧根本不需要读响应。
        let authed = match (&self.token, &token) {
            (Some(expected), Some(got)) => constant_time_eq(expected.as_bytes(), got.as_bytes()),
            // 没配 token 时保持旧行为（本地开发直接跑 sidecar），但依然拒绝浏览器来源。
            (None, _) => true,
            (Some(_), None) => false,
        };
        if !authed || browser_origin {
            let body = b"{\"error\":\"unauthorized\"}";
            let header = format!(
                "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(header.as_bytes())?;
            stream.write_all(body)?;
            stream.flush()?;
            return Ok(());
        }

        let resp_body: Vec<u8> = if path == "/health" {
            b"ok".to_vec()
        } else if http_method == "POST" && path == "/rpc" {
            match serde_json::from_slice::<RpcRequest>(&body) {
                Ok(req) => serde_json::to_vec(&self.handle_request(req)).unwrap_or_default(),
                Err(e) => serde_json::to_vec(&serde_json::json!({
                    "jsonrpc": "2.0", "id": serde_json::Value::Null,
                    "error": { "code": -32700, "message": format!("parse error: {}", e) }
                })).unwrap_or_default(),
            }
        } else {
            b"{\"error\":\"not found\"}".to_vec()
        };

        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            resp_body.len()
        );
        stream.write_all(header.as_bytes())?;
        stream.write_all(&resp_body)?;
        stream.flush()?;
        Ok(())
    }
}

/// 录制文件目录：~/.michael-automation/recordings/
fn recordings_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(home).join(".michael-automation").join("recordings")
}

/// 清洗录制名为安全文件名（防路径穿越）。
fn sanitize_name(name: &str) -> String {
    let s: String = name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    let s = s.trim_matches('_').to_string();
    if s.is_empty() { "recording".into() } else { s.chars().take(80).collect() }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rpc_request_parsing() {
        let json = r#"{
            "jsonrpc": "2.0",
            "method": "browser.goto",
            "params": {"url": "https://example.com"},
            "id": 1
        }"#;
        
        let req: RpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "browser.goto");
        assert_eq!(req.params["url"], "https://example.com");
    }
}
