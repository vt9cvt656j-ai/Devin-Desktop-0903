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
/// `/health` 挑战应答：SHA-256("<token>:<nonce>") 的十六进制。
///
/// 用意是让**持有 token 的一方证明自己持有它**，而调用方不必先把 token 交出去。
/// nonce 每次现生成，所以观察到一次应答也没法拿去冒充下一次。
pub fn health_challenge_response(token: &str, nonce: &str) -> String {
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

/// 合成按键只会投给**当前前台应用**，而前台随时可能被用户、对话框或另一个
/// 应用抢走。回执里不带这个，一次打进错误窗口和一次正常输入长得一模一样——
/// 调用方拿到的都是 {"status":"ok"}。
#[cfg(feature = "system")]
fn frontmost_now() -> Option<String> {
    crate::platform::get_window_controller()
        .enumerate_windows()
        .ok()?
        .into_iter()
        .find(|w| w.is_visible)
        .map(|w| w.title)
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
    
    /// 带了 {x,y} 就先把指针移过去再点。
    ///
    /// 之所以做成"有就用、没有就保持原样"，是因为 mouse.click 的老语义是"在当前位置点"，
    /// 有脚本依赖它；而两个工具 schema 又都对外声明可以带坐标。两边都认账。
    #[cfg(feature = "system")]
    fn click_at_if_given(
        agent: &mut crate::agent::Agent,
        params: &serde_json::Value,
    ) -> Result<()> {
        let x = Self::coord(params, "x");
        let y = Self::coord(params, "y");
        if let (Some(x), Some(y)) = (x, y) {
            agent.mouse_move(x as i32, y as i32)?;
        }
        Ok(())
    }

    /// 坐标解析。工具 schema 里坐标是 number，模型给 100.0 或 "100" 都合法，
    /// 而 as_i64() 对这两种都返回 None——于是坐标被当成"没传"，静默丢弃。
    #[cfg(feature = "system")]
    fn coord(params: &serde_json::Value, key: &str) -> Option<f64> {
        let v = params.get(key)?;
        v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim().parse::<f64>().ok()))
    }

    /// 回执带上动作后的真实落点。只回 {"status":"ok"} 的话，"坐标被忽略"这类 bug
    /// 在调用方眼里完全不可见——它正是这么潜伏下来的。
    #[cfg(feature = "system")]
    fn acted_at(agent: &mut crate::agent::Agent) -> serde_json::Value {
        match agent.mouse_location() {
            Ok((x, y)) => serde_json::json!({"status": "ok", "x": x, "y": y, "coordinate_space": "screen_points_top_left"}),
            Err(_) => serde_json::json!({"status": "ok"}),
        }
    }

    fn execute_method(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let mut agent = self.agent.lock().unwrap();
        
        match method {
            // 浏览器方法
            #[cfg(feature = "browser")]
            "browser.start" => {
                let headless = params.get("headless").and_then(|v| v.as_bool()).unwrap_or(false);
                // profile 决定这次自动化用谁的身份。默认隔离（干净、安全）；
                // 任务需要用户已登录的会话时传 session——否则必定撞登录墙。
                let profile = match params.get("profile").and_then(|v| v.as_str()).unwrap_or("isolated") {
                    "session" | "user" | "persistent" | "logged_in" => crate::browser::BrowserProfile::Session,
                    _ => crate::browser::BrowserProfile::Isolated,
                };
                agent.browser_start_with_profile(headless, profile)?;
                Ok(serde_json::json!({
                    "status": "ok",
                    "profile": if profile == crate::browser::BrowserProfile::Session { "session" } else { "isolated" },
                    "note": if profile == crate::browser::BrowserProfile::Session {
                        "Persistent profile: cookies and logins from earlier runs are available, and anything you sign into here stays signed in. If a site still shows a login wall, start again with headless=false so the user can sign in once."
                    } else {
                        "Clean isolated profile: no cookies, no logins. If the task needs the user's own account, restart with profile=session."
                    }
                }))
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
            
            // 屏幕像素。这是整套系统里唯一能"看一眼真实桌面"的通路 —— 在它之前，
            // screenshot 工具只会用无头浏览器渲染一个网址，模型对任何原生应用都是全盲的，
            // 而工具描述还在教它"用 screenshot 验证结果"。
            #[cfg(feature = "system")]
            "screen.capture" => {
                let num = |k: &str| params.get(k).and_then(|v| v.as_f64()).map(|n| n as i32);
                // 四个都给才算区域；给一半是参数写错了，宁可报错也别悄悄拍整屏。
                let region = match (num("x"), num("y"), num("width"), num("height")) {
                    (Some(x), Some(y), Some(w), Some(h)) => Some((x, y, w, h)),
                    (None, None, None, None) => None,
                    _ => return Err(crate::error::Error::System(
                        "区域截图要同时给 x/y/width/height 四个参数；一个都不给就是整屏".into(),
                    )),
                };
                let (data_url, scale_note) = agent.screen_capture(region)?;
                // 坐标系和 mouse.move / screen.info 是同一套（屏幕点、左上原点），
                // 一并回给调用方，免得它再去猜要不要乘 scale_factor。
                Ok(serde_json::json!({
                    "data_url": data_url,
                    "region": region.map(|(x, y, w, h)| serde_json::json!({"x": x, "y": y, "width": w, "height": h})),
                    "coordinate_space": "screen_points_top_left",
                    // Retina 上图是像素、鼠标收的是点，比值在这里说清楚，
                    // 免得模型拿图上量到的坐标直接去点，点到屏幕外。
                    "scale_note": scale_note
                }))
            }

            #[cfg(feature = "system")]
            "mouse.position" => {
                let (x, y) = agent.mouse_location()?;
                Ok(serde_json::json!({"x": x, "y": y, "coordinate_space": "screen_points_top_left"}))
            }

            #[cfg(feature = "system")]
            "mouse.move" => {
                let x = Self::coord(&params, "x")
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'x' parameter")))? as i32;
                let y = Self::coord(&params, "y")
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'y' parameter")))? as i32;
                agent.mouse_move(x, y)?;
                Ok(Self::acted_at(&mut agent))
            }
            
            #[cfg(feature = "system")]
            "mouse.click" => {
                // 两个工具入口（automation / computer）都对外声明可以传 {x,y}，而这里以前只读
                // button——多余的键被 serde 静默忽略，于是"带坐标的点击"实际点在光标上次停留的
                // 位置上，还回一个 ok。不传坐标时保持原地点击的老行为。
                Self::click_at_if_given(&mut agent, &params)?;
                let button = params.get("button").and_then(|v| v.as_str());
                agent.mouse_click(button)?;
                Ok(Self::acted_at(&mut agent))
            }
            
            #[cfg(feature = "system")]
            "mouse.double_click" => {
                Self::click_at_if_given(&mut agent, &params)?;
                let button = params.get("button").and_then(|v| v.as_str());
                agent.mouse_double_click(button)?;
                Ok(Self::acted_at(&mut agent))
            }
            
            #[cfg(feature = "system")]
            "mouse.drag" => {
                let from_x = Self::coord(&params, "from_x")
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'from_x' parameter")))? as i32;
                let from_y = Self::coord(&params, "from_y")
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'from_y' parameter")))? as i32;
                let to_x = Self::coord(&params, "to_x")
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'to_x' parameter")))? as i32;
                let to_y = Self::coord(&params, "to_y")
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'to_y' parameter")))? as i32;
                agent.mouse_drag(from_x, from_y, to_x, to_y)?;
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            #[cfg(feature = "system")]
            "mouse.scroll" => {
                let delta_x = params.get("delta_x")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
                let delta_y = Self::coord(&params, "delta_y")
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
                drop(agent);
                Ok(serde_json::json!({"status": "ok", "delivered_to": frontmost_now()}))
            }
            
            #[cfg(feature = "system")]
            "keyboard.press" => {
                let key = params.get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'key' parameter")))?;
                agent.keyboard_press(key)?;
                drop(agent);
                Ok(serde_json::json!({"status": "ok", "delivered_to": frontmost_now()}))
            }
            
            #[cfg(feature = "system")]
            "keyboard.down" => {
                let key = params.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'key' parameter")))?;
                agent.keyboard_down(key)?;
                drop(agent);
                Ok(serde_json::json!({"status": "ok", "delivered_to": frontmost_now(),
                    "note": "这个键现在是按住状态，用完必须 keyboard.up 松开，否则它会一直卡住影响之后所有输入。"}))
            }
            #[cfg(feature = "system")]
            "keyboard.up" => {
                let key = params.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Missing 'key' parameter")))?;
                agent.keyboard_up(key)?;
                drop(agent);
                Ok(serde_json::json!({"status": "ok", "delivered_to": frontmost_now()}))
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
                drop(agent);
                Ok(serde_json::json!({"status": "ok", "delivered_to": frontmost_now()}))
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
                    // is_visible 里装的其实是 NSRunningApplication.isActive，也就是
                    // 「它是不是前台」。名字对不上语义，模型没法知道能拿它回答
                    // 「现在谁在前台」——而这正是合成按键会打进谁的唯一判据。
                    "frontmost": w.is_visible,
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
                // 到这里才代表**回读确认过**目标真在前台了（见 platform 层的轮询）。
                // 把实际前台一并回出去，调用方不必再信一个光秃秃的 ok。
                Ok(serde_json::json!({ "status": "ok", "frontmost": frontmost_now() }))
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
            // screen.info 报的 width/height 来自 CGDisplay::pixels_wide()——名字叫 pixels，
            // 返回的其实是**点**（本机实测 1728x1117，而物理像素是 3456x2234）。合成事件的
            // enigo、read_screen 的 AX 树、window.list 用的也都是点。曾经的提示词教模型
            // "坐标要乘 scale_factor"，于是每一次坐标点击都打在两倍的位置上。
            // 让接口自己把坐标空间说清楚，比在提示词里反复叮嘱可靠。
            "screen.info" => {
                drop(agent);
                let info = crate::platform::get_window_controller().get_screen_info()?;
                Ok(serde_json::json!({
                    "width": info.width,
                    "height": info.height,
                    "scale_factor": info.scale_factor,
                    // 自描述：别让调用方猜。width/height 和 mouse.move / read_screen /
                    // window.list 是同一套坐标；scale_factor 只说明像素密度，不是换算系数。
                    "coordinate_space": "screen_points_top_left",
                    "note": "width/height are already in the units mouse.move accepts. Never multiply coordinates by scale_factor.",
                    // 先告诉调用方能不能注入事件，省得它点了十次才发现系统压根没收到
                    "input_permission": if crate::agent::input_permission_granted() { "granted" } else { "denied" },
                    "input_permission_hint": if crate::agent::input_permission_granted() {
                        ""
                    } else {
                        "Accessibility permission is missing: synthetic mouse/keyboard events are discarded by macOS. Use browser automation, shell, or file tools instead, and tell the user to grant it in System Settings > Privacy & Security > Accessibility, then fully restart the app."
                    }
                }))
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
                drop(agent);
                Ok(serde_json::json!({ "status": "ok", "delivered_to": frontmost_now() }))
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
        // 读写都要超时：这个服务在**一条线程上串行**处理连接（自动化本就串行），
        // 一个连上来却不发数据的连接会把它整个堵死——后面所有自动化调用只能干等到
        // IDE 侧那 120 秒超时。本机任意进程都能连上 127.0.0.1，成本极低。
        let io_timeout = std::time::Duration::from_secs(15);
        let _ = stream.set_read_timeout(Some(io_timeout));
        let _ = stream.set_write_timeout(Some(io_timeout));
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let mut parts = line.split_whitespace();
        let http_method = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("").to_string();
        // 读到空行为止，抓 Content-Length + 鉴权/来源判定所需的头
        let mut content_len = 0usize;
        let mut token: Option<String> = None;
        let mut nonce: Option<String> = None;
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
            // 挑战应答的随机数（见下面 /health）。它不是凭据，明文即可。
            if let Some(v) = low.strip_prefix("x-automation-nonce:") {
                nonce = Some(v.trim().to_string());
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
        // `/health` 是**挑战应答**，不要求调用方出示 token —— 恰恰相反，它必须**不出示**。
        //
        // 这个端点的用途是回答「占着这个端口的，是不是我自己刚起的那个 sidecar」。
        // 原来的做法是客户端把 token 放在请求头里发过去、只看 HTTP 200 —— 两头都错：
        //   · 只看 200：冒充者不运行我们的代码，它想回什么状态码就回什么；
        //   · 发 token：请求是发给**尚未验明身份**的一方的，等于把一次性密钥直接
        //     交到冒充者手上，它随后可以拿着这个 token 去满足任何后续校验。
        // 于是「先占住 127.0.0.1:3037 就能接管全部桌面自动化」这条路一直是通的，
        // 而后续流过去的包括 keyboard.type 的正文（用户密码）、剪贴板、网页正文。
        //
        // 改成：客户端发一个随机 nonce（明文，不是凭据），我们回 SHA-256(token:nonce)。
        // 只有真的持有 token 的一方算得出来，而 token 从不离开本进程。
        let health_probe = path == "/health";
        let authed = if health_probe {
            true
        } else {
            match (&self.token, &token) {
                (Some(expected), Some(got)) => constant_time_eq(expected.as_bytes(), got.as_bytes()),
                // 没配 token 时保持旧行为（本地开发直接跑 sidecar），但依然拒绝浏览器来源。
                (None, _) => true,
                (Some(_), None) => false,
            }
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

        // 鉴权之后才读 body。原来读在鉴权**之前**：一个未鉴权的连接报一个撒谎的
        // Content-Length，就能让我们在验明身份前先按它说的大小申请内存；而这个服务是
        // 单线程串行处理的，一个不发数据的连接还能把后面所有自动化调用一起堵死。
        const MAX_BODY: usize = 8 * 1024 * 1024;
        if content_len > MAX_BODY {
            let body = b"{\"error\":\"payload too large\"}";
            let header = format!(
                "HTTP/1.1 413 Payload Too Large\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(header.as_bytes())?;
            stream.write_all(body)?;
            stream.flush()?;
            return Ok(());
        }
        let body = if content_len > 0 {
            let mut buf = vec![0u8; content_len];
            reader.read_exact(&mut buf)?;
            buf
        } else { Vec::new() };

        let resp_body: Vec<u8> = if health_probe {
            // 持有 token 才算得出这个值；没配 token（本地开发）时维持旧的 "ok"。
            match (&self.token, &nonce) {
                (Some(tok), Some(n)) => health_challenge_response(tok, n).into_bytes(),
                _ => b"ok".to_vec(),
            }
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

#[cfg(all(test, feature = "system"))]
mod coord_tests {
    use super::RpcServer;

    // 工具 schema 把坐标声明成 number，模型给 100.0 完全合法；而 as_i64() 对它返回 None，
    // 于是坐标被当成"根本没传"——RPC 照样回 ok，点击落在光标上次停留的地方。
    #[test]
    fn 小数与字符串坐标不再被当成没传() {
        let p = serde_json::json!({"x": 100.0, "y": 200.5, "sx": "512", "bad": "abc", "nil": null});
        assert_eq!(RpcServer::coord(&p, "x"), Some(100.0));
        assert_eq!(RpcServer::coord(&p, "y"), Some(200.5));
        assert_eq!(RpcServer::coord(&p, "sx"), Some(512.0));
        assert_eq!(RpcServer::coord(&p, "bad"), None, "非数字文本不能被当成坐标");
        assert_eq!(RpcServer::coord(&p, "nil"), None);
        assert_eq!(RpcServer::coord(&p, "missing"), None, "真的没传要如实报缺参");
    }

    #[test]
    fn 整数坐标保持原样() {
        let p = serde_json::json!({"x": 0, "y": -3, "big": 1728});
        assert_eq!(RpcServer::coord(&p, "x"), Some(0.0));
        assert_eq!(RpcServer::coord(&p, "y"), Some(-3.0), "负坐标是合法的多显示器坐标");
        assert_eq!(RpcServer::coord(&p, "big"), Some(1728.0));
    }
}

#[cfg(test)]
mod health_challenge_tests {
    use super::health_challenge_response;

    /// `/health` 必须是**挑战应答**，不能只回一个固定串。
    ///
    /// 修复前客户端只看 HTTP 200，而且还把 token 放在请求头里发给尚未验明身份的一方。
    /// 于是本机任意进程抢先占住 127.0.0.1:3037，对任何请求回 200 就能冒充 sidecar，
    /// 接管之后流过去的包括 keyboard.type 的正文（用户密码）、剪贴板、网页正文——
    /// 而这一层能合成真实键鼠，也就是能开终端敲任意命令。
    #[test]
    fn response_depends_on_both_token_and_nonce() {
        let a = health_challenge_response("tok", "n1");
        // 同 token 换 nonce → 不同（否则观察一次应答就能重放）
        assert_ne!(a, health_challenge_response("tok", "n2"), "换了 nonce 应答却不变，可被重放");
        // 同 nonce 换 token → 不同（否则不知道 token 的人也能算出来）
        assert_ne!(a, health_challenge_response("other", "n1"), "换了 token 应答却不变");
        // 稳定可复现（客户端和服务端各算一次要对得上）
        assert_eq!(a, health_challenge_response("tok", "n1"));
        // 就是 SHA-256("tok:n1") 的十六进制，两侧实现必须逐字节一致
        assert_eq!(a.len(), 64, "应当是 32 字节 SHA-256 的十六进制");
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        // 固定串（冒充者最容易回的那个）绝不可能命中
        assert_ne!(a, "ok");
    }
}
