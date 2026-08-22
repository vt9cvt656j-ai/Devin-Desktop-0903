//! AI Agent 统一接口 - 封装所有自动化能力（修复版）

use crate::error::{Error, Result};
use crate::types::*;
use crate::platform::desktop_element::{DesktopElement, DesktopElementControl};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(feature = "browser")]
use crate::browser::BrowserAutomation;

#[cfg(feature = "system")]
use crate::system::SystemAutomation;

#[cfg(all(feature = "system", target_os = "windows"))]
use crate::platform::windows_ui_automation::WindowsUIAutomation;

#[cfg(all(feature = "system", target_os = "macos"))]
use crate::platform::macos_accessibility::MacOSAccessibility;

/// macOS 会静默丢弃未授权进程发出的合成事件：enigo 照样返回 Ok，于是每一次点击、
/// 每一次按键都"成功"了，但屏幕上什么都没发生。这是最坏的一种失败——调用方据此
/// 继续推进，越走越偏，还查不出原因。宁可在入口就报错，并且把补救路径写清楚。
#[cfg(all(feature = "system", target_os = "macos"))]
pub fn input_permission_granted() -> bool {
    // 每次真查，**不缓存**。
    //
    // 这里原来把「拿到过授权」用 static AtomicBool 缓存到进程死亡，依据是「授权一旦拿到
    // 就不会再撤销」——可 sidecar 随 IDE 常驻几个小时，用户中途在系统设置里取消勾选
    // （或 tccutil reset）之后，这个缓存让 ensure_input_permission 永远放行，macOS 则
    // 静默丢掉每一个合成事件：回执全是 ok、screen.info 还报 granted，模型照着「已点击 /
    // 已输入」继续推进——正是上面那段头注说要杜绝的最坏失败。IDE 侧同一个 API
    // （src-tauri permissions.rs）就是每次查的，这个调用本身很便宜。
    unsafe { accessibility_sys::AXIsProcessTrusted() }
}

#[cfg(all(feature = "system", not(target_os = "macos")))]
pub fn input_permission_granted() -> bool {
    true
}

#[cfg(feature = "system")]
fn ensure_input_permission() -> Result<()> {
    if input_permission_granted() {
        return Ok(());
    }
    Err(Error::System(
        "macOS 未授予辅助功能（Accessibility）权限，鼠标与键盘事件会被系统静默丢弃，\
         看起来成功其实没有任何效果。请到 系统设置 → 隐私与安全性 → 辅助功能 勾选本应用，\
         然后完全退出并重新打开它。在此之前请改用不需要该权限的手段：浏览器自动化、\
         Shell 命令、或直接读写文件。"
            .to_string(),
    ))
}

pub struct Agent {
    #[cfg(feature = "browser")]
    browser: Option<Arc<Mutex<BrowserAutomation>>>,
    
    #[cfg(feature = "system")]
    system: Option<Arc<Mutex<SystemAutomation>>>,
    
    #[cfg(all(feature = "system", target_os = "windows"))]
    desktop: Option<WindowsUIAutomation>,
    
    #[cfg(all(feature = "system", target_os = "macos"))]
    desktop: Option<MacOSAccessibility>,
    
    runtime: Option<tokio::runtime::Runtime>,
}

impl Agent {
    pub fn new() -> Result<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| Error::Other(anyhow::anyhow!("Failed to create runtime: {}", e)))?;
        
        Ok(Self {
            #[cfg(feature = "browser")]
            browser: None,
            
            #[cfg(feature = "system")]
            system: None,
            
            #[cfg(all(feature = "system", target_os = "windows"))]
            desktop: None,
            
            #[cfg(all(feature = "system", target_os = "macos"))]
            desktop: None,
            
            runtime: Some(runtime),
        })
    }
    
    // ==================== 系统自动化 ====================
    
    #[cfg(feature = "system")]
    pub fn system_init(&mut self) -> Result<()> {
        // 所有注入型操作（鼠标/键盘）都以这里为唯一入口；剪贴板走 arboard、
        // screen.info 走 platform，都不经过这里，所以只在这里预检不会误伤只读能力。
        ensure_input_permission()?;
        self.system_init_readonly()
    }

    /// 只读能力用的初始化：**不查辅助功能权限**。
    ///
    /// 上面那句注释写着「只在这里预检不会误伤只读能力」，可截屏偏偏走的就是 system_init——
    /// 于是只授予了「屏幕录制」而没给「辅助功能」的机器上，智能体**根本看不到屏幕**，
    /// 而且报错还指向错误的那一格设置。截屏需要的是屏幕录制，和鼠标键盘注入是两回事。
    #[cfg(feature = "system")]
    pub fn system_init_readonly(&mut self) -> Result<()> {
        if self.system.is_none() {
            let sys = SystemAutomation::new()?;
            self.system = Some(Arc::new(Mutex::new(sys)));
        }
        Ok(())
    }
    
    /// 移动指针，并且**等它真的到位**再返回。
    ///
    /// CGEvent 是异步投递的。enigo 的 button() 会先读当前指针位置、再把点击事件发到
    /// 那个位置上——移动刚发出、指针还没落位时点击，事件就带着旧坐标发出去：不但点错
    /// 地方，那条 mouse-down 还会把指针拽回旧位置。整条链路一路返回 ok，屏幕上什么
    /// 都没发生。所以"移动"必须是同步语义，不能是"已发出移动请求"。
    #[cfg(feature = "system")]
    pub fn mouse_move(&mut self, x: i32, y: i32) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.move_mouse(x, y)?;
        // 轮询到位。上限 200ms：到不了就说明目标被系统夹住了（越界、被其他进程抢走），
        // 与其假装成功，不如让调用方拿到真实落点自己判断。
        let mut last = (x, y);
        for _ in 0..40 {
            match sys.mouse_location() {
                Ok((cx, cy)) if (cx - x).abs() <= 2 && (cy - y).abs() <= 2 => return Ok(()),
                Ok(p) => last = p,
                // 读不到当前位置就不能判定，按到位处理（老行为），别凭空拒绝执行。
                Err(_) => return Ok(()),
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        // 200ms 还没落位 = 没到位。原来这里直接 Ok(())，于是"指针根本没过去"被报成成功，
        // 紧接着的点击就点在旧位置上——而调用方拿到的是一路 ok，屏幕上什么都没发生。
        // 最常见的原因是坐标越界（多显示器、或者拿像素当点用，Retina 上正好差一倍）。
        // 宁可拒绝执行也不要假装成功：把真实落点和目标一起说出来，模型自己能判断。
        Err(Error::System(format!(
            "指针没能移动到 ({x}, {y})：200ms 后它停在 ({}, {})。本次不执行后续点击。\
最常见的原因是坐标超出了可用范围——先调 screen.info 看屏幕的**点**尺寸；\
如果坐标是从截图上量的，注意 Retina 截图是像素、这里收的是点，两者差一倍。",
            last.0, last.1
        )))
    }
    
    #[cfg(feature = "system")]
    pub fn mouse_location(&mut self) -> Result<(i32, i32)> {
        self.system_init()?;
        let sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.mouse_location()
    }

    /// 拍屏幕。`region` 为 None 就是整屏。返回 PNG 的 data URL。
    #[cfg(feature = "system")]
    pub fn screen_capture(&mut self, region: Option<(i32, i32, i32, i32)>) -> Result<(String, Option<String>)> {
        // 截屏是只读能力，不需要辅助功能权限（那是给鼠标键盘注入用的）。
        self.system_init_readonly()?;
        let sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.screen_capture(region)
    }

    #[cfg(feature = "system")]
    pub fn mouse_click(&mut self, button: Option<&str>) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        let btn = match button.unwrap_or("left") {
            "left" => MouseButton::Left,
            "right" => MouseButton::Right,
            "middle" => MouseButton::Middle,
            _ => MouseButton::Left,
        };
        sys.click(btn)
    }
    
    #[cfg(feature = "system")]
    pub fn mouse_double_click(&mut self, button: Option<&str>) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        let btn = match button.unwrap_or("left") {
            "left" => MouseButton::Left,
            "right" => MouseButton::Right,
            "middle" => MouseButton::Middle,
            _ => MouseButton::Left,
        };
        sys.double_click(btn)
    }
    
    #[cfg(feature = "system")]
    pub fn mouse_drag(&mut self, from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.drag(from_x, from_y, to_x, to_y)
    }
    
    /// 滚动。参数与 `SystemAutomation::scroll` 一致：(delta_x, delta_y)。
    /// 此前只收一个 `amount` 且误传成了水平分量——调用方以为在垂直滚动，实际页面纹丝不动。
    #[cfg(feature = "system")]
    pub fn mouse_scroll(&mut self, delta_x: i32, delta_y: i32) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.scroll(delta_x, delta_y)
    }
    
    #[cfg(feature = "system")]
    pub fn keyboard_type(&mut self, text: &str) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.type_text(text)
    }
    
    #[cfg(feature = "system")]
    pub fn keyboard_press(&mut self, key: &str) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        let k = parse_key(key)?;
        sys.press_key(k)
    }
    
    #[cfg(feature = "system")]
    /// 按下但不松开 / 松开。成对使用就能表达「按住 Shift 再点几下」「按住 Cmd
    /// 拖拽」这类操作——底层一直有 key_down/key_up，只是从没暴露出去，于是路线 B
    /// 里这类操作根本没法表达。松开务必配对，否则修饰键会一直卡住。
    #[cfg(feature = "system")]
    pub fn keyboard_down(&mut self, key: &str) -> Result<()> {
        self.system_init()?;
        let k = parse_key(key)?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.key_down(k)
    }

    #[cfg(feature = "system")]
    pub fn keyboard_up(&mut self, key: &str) -> Result<()> {
        self.system_init()?;
        let k = parse_key(key)?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.key_up(k)
    }

    pub fn keyboard_combo(&mut self, keys: Vec<&str>) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        
        let key_vec: Result<Vec<Key>> = keys.iter().map(|k| parse_key(k)).collect();
        let parsed_keys = key_vec?;
        
        for key in &parsed_keys {
            sys.key_down(key.clone())?;
        }
        std::thread::sleep(Duration::from_millis(50));
        for key in parsed_keys.iter().rev() {
            sys.key_up(key.clone())?;
        }
        Ok(())
    }
    
    // ==================== 剪贴板 ====================
    
    #[cfg(feature = "system")]
    pub fn clipboard_get_text(&self) -> Result<String> {
        use arboard::Clipboard;
        let mut clipboard = Clipboard::new()
            .map_err(|e| Error::System(format!("剪贴板初始化失败: {}", e)))?;
        clipboard.get_text()
            .map_err(|e| Error::System(format!("读取剪贴板失败: {}", e)))
    }
    
    #[cfg(feature = "system")]
    pub fn clipboard_set_text(&self, text: &str) -> Result<()> {
        use arboard::Clipboard;
        let mut clipboard = Clipboard::new()
            .map_err(|e| Error::System(format!("剪贴板初始化失败: {}", e)))?;
        clipboard.set_text(text)
            .map_err(|e| Error::System(format!("写入剪贴板失败: {}", e)))
    }
    
    #[cfg(feature = "system")]
    pub fn copy_selection(&mut self) -> Result<()> {
        #[cfg(target_os = "macos")]
        self.keyboard_combo(vec!["cmd", "c"])?;
        #[cfg(not(target_os = "macos"))]
        self.keyboard_combo(vec!["ctrl", "c"])?;
        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    }
    
    #[cfg(feature = "system")]
    pub fn paste_from_clipboard(&mut self) -> Result<()> {
        #[cfg(target_os = "macos")]
        self.keyboard_combo(vec!["cmd", "v"])?;
        #[cfg(not(target_os = "macos"))]
        self.keyboard_combo(vec!["ctrl", "v"])?;
        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    }
    
    #[cfg(feature = "system")]
    pub fn quick_paste(&mut self, text: &str) -> Result<()> {
        // 粘贴要借道剪贴板，但剪贴板是用户的东西。原来这里直接覆盖且从不还原——
        // 用户复制了一段待用的内容，被自动化悄悄换掉，事后完全无从察觉。
        // 先存后还，还原前多等一会儿：目标应用是在处理 Cmd+V 时才去读剪贴板的，
        // 还得太快会把粘贴本身弄坏（paste_from_clipboard 已经等了 100ms）。
        let saved = self.clipboard_get_text().ok();
        self.clipboard_set_text(text)?;
        let pasted = self.paste_from_clipboard();
        if let Some(prev) = saved {
            std::thread::sleep(Duration::from_millis(250));
            // 还原失败不该盖掉粘贴本身的结果
            let _ = self.clipboard_set_text(&prev);
        }
        pasted
    }
    
    // ==================== 浏览器自动化 ====================

    /// 取浏览器锁：未启动 / Mutex 中毒都返回错误而不是 panic——否则一次中毒
    /// 会连带杀死整个常驻的 automation-server。
    #[cfg(feature = "browser")]
    fn browser_lock(&self) -> Result<std::sync::MutexGuard<'_, BrowserAutomation>> {
        self.browser
            .as_ref()
            .ok_or_else(|| Error::Browser("浏览器未启动".to_string()))?
            .lock()
            .map_err(|e| Error::Browser(format!("Mutex 中毒: {}", e)))
    }

    #[cfg(feature = "browser")]
    fn rt(&self) -> Result<&tokio::runtime::Runtime> {
        self.runtime
            .as_ref()
            .ok_or_else(|| Error::Other(anyhow::anyhow!("Runtime not initialized")))
    }

    #[cfg(feature = "browser")]
    pub fn browser_start(&mut self, headless: bool) -> Result<()> {
        self.browser_start_with_profile(headless, crate::browser::BrowserProfile::Isolated)
    }

    /// 起浏览器，并指定用哪套身份。
    ///
    /// Isolated：每次全新，抓公开页面/测自己的站点用。
    /// Session：专用持久 profile，保留登录态——任务需要用户已登录的身份时用这个，
    /// 否则一定撞登录墙。它和用户自己的 Chrome 互不干扰，可以同时开着。
    #[cfg(feature = "browser")]
    pub fn browser_start_with_profile(
        &mut self,
        headless: bool,
        profile: crate::browser::BrowserProfile,
    ) -> Result<()> {
        if self.browser.is_some() {
            return Ok(());
        }
        
        let runtime = self.runtime.as_ref()
            .ok_or_else(|| Error::Other(anyhow::anyhow!("Runtime not initialized")))?;
        
        let browser = runtime.block_on(async {
            {
                BrowserAutomation::new_with_profile(headless, profile).await
            }
        })?;
        
        std::thread::sleep(Duration::from_millis(1500));
        self.browser = Some(Arc::new(Mutex::new(browser)));
        Ok(())
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_goto(&self, url: &str) -> Result<()> {
        let mut browser = self.browser_lock()?;
        self.rt()?.block_on(browser.navigate(url))
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_click(&self, selector: &str) -> Result<()> {
        let mut browser = self.browser_lock()?;
        self.rt()?.block_on(browser.click_element(selector))
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_type(&self, selector: &str, text: &str) -> Result<()> {
        let mut browser = self.browser_lock()?;
        self.rt()?.block_on(browser.type_text(selector, text))
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_wait(&self, selector: &str, timeout_ms: u64) -> Result<()> {
        let mut browser = self.browser_lock()?;
        self.rt()?.block_on(browser.wait_for_element(selector, timeout_ms))
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_eval(&self, script: &str) -> Result<serde_json::Value> {
        let mut browser = self.browser_lock()?;
        self.rt()?.block_on(browser.execute_script(script))
    }
    
    #[cfg(feature = "browser")]
    /// 返回 PNG 字节——**不要再丢掉它**。
    ///
    /// 原来这里是 `browser.screenshot(path)?; Ok(())`：底层 browser.rs 明明
    /// `Ok(screenshot)` 返回了 PNG，这一层直接扔了，RPC 再回一句 `{"status":"ok"}`。
    /// 模型调 browser.screenshot 的唯一目的就是看一眼页面，结果什么都没看到，
    /// 却被告知成功了——它会照着「我看过了」继续往下推。
    /// 对照 screen.capture：那条一直是回 data_url 的。
    pub fn browser_screenshot(&self, path: Option<&str>) -> Result<Vec<u8>> {
        let mut browser = self.browser_lock()?;
        self.rt()?.block_on(browser.screenshot(path))
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_content(&self) -> Result<String> {
        let mut browser = self.browser_lock()?;
        self.rt()?.block_on(browser.get_content())
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_close(&mut self) -> Result<()> {
        let Some(browser_arc) = self.browser.take() else { return Ok(()) };
        // 直接 drop 会杀掉 Chrome，cookie 来不及落盘——持久 profile 下等于登录白登。
        // 先走 CDP 的优雅关闭再放手。
        if let Some(runtime) = self.runtime.as_ref() {
            runtime.block_on(async {
                if let Ok(mut guard) = browser_arc.lock() {
                    let _ = guard.close_gracefully().await;
                }
            });
        }
        drop(browser_arc);
        Ok(())
    }
    
    // ==================== 桌面元素操作 ====================
    
    #[cfg(all(feature = "system", target_os = "windows"))]
    pub fn desktop_init(&mut self) -> Result<()> {
        if self.desktop.is_none() {
            self.desktop = Some(WindowsUIAutomation::new()?);
        }
        Ok(())
    }
    
    #[cfg(all(feature = "system", target_os = "macos"))]
    pub fn desktop_init(&mut self) -> Result<()> {
        if self.desktop.is_none() {
            self.desktop = Some(MacOSAccessibility::new()?);
        }
        Ok(())
    }
    
    #[cfg(feature = "system")]
    pub fn desktop_find_element(&mut self, window_title: &str, element_name: &str) -> Result<Option<DesktopElement>> {
        self.desktop_init()?;
        
        #[cfg(target_os = "windows")]
        {
            self.desktop.as_ref().unwrap().find_element_by_name(window_title, element_name)
        }
        
        #[cfg(target_os = "macos")]
        {
            self.desktop.as_ref().unwrap().find_element_by_name(window_title, element_name)
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            Err(Error::System("桌面元素操作仅支持 Windows 和 macOS".to_string()))
        }
    }
    
    #[cfg(feature = "system")]
    pub fn desktop_find_elements_by_type(&mut self, window_title: &str, element_type: &str) -> Result<Vec<DesktopElement>> {
        self.desktop_init()?;
        
        #[cfg(target_os = "windows")]
        {
            self.desktop.as_ref().unwrap().find_elements_by_type(window_title, element_type)
        }
        
        #[cfg(target_os = "macos")]
        {
            self.desktop.as_ref().unwrap().find_elements_by_type(window_title, element_type)
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            Err(Error::System("桌面元素操作仅支持 Windows 和 macOS".to_string()))
        }
    }
    
    #[cfg(feature = "system")]
    pub fn desktop_click_element(&self, element: &DesktopElement) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            self.desktop.as_ref()
                .ok_or_else(|| Error::System("桌面控制器未初始化".to_string()))?
                .click_element(element)
        }
        
        #[cfg(target_os = "macos")]
        {
            self.desktop.as_ref()
                .ok_or_else(|| Error::System("桌面控制器未初始化".to_string()))?
                .click_element(element)
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            Err(Error::System("桌面元素操作仅支持 Windows 和 macOS".to_string()))
        }
    }
    
    #[cfg(feature = "system")]
    pub fn desktop_type_into(&self, element: &DesktopElement, text: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            self.desktop.as_ref()
                .ok_or_else(|| Error::System("桌面控制器未初始化".to_string()))?
                .type_into_element(element, text)
        }
        
        #[cfg(target_os = "macos")]
        {
            self.desktop.as_ref()
                .ok_or_else(|| Error::System("桌面控制器未初始化".to_string()))?
                .type_into_element(element, text)
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            Err(Error::System("桌面元素操作仅支持 Windows 和 macOS".to_string()))
        }
    }
    
    #[cfg(feature = "system")]
    pub fn desktop_get_text(&self, element: &DesktopElement) -> Result<String> {
        #[cfg(target_os = "windows")]
        {
            self.desktop.as_ref()
                .ok_or_else(|| Error::System("桌面控制器未初始化".to_string()))?
                .get_element_text(element)
        }
        
        #[cfg(target_os = "macos")]
        {
            self.desktop.as_ref()
                .ok_or_else(|| Error::System("桌面控制器未初始化".to_string()))?
                .get_element_text(element)
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            Err(Error::System("桌面元素操作仅支持 Windows 和 macOS".to_string()))
        }
    }
}

/// 只给测试用的出口。修饰键映射错了在 Windows 上是**静默**按下 Win 键，
/// 三层回执还都说成功——这种东西必须有测试钉着。
#[cfg(test)]
pub fn parse_key_for_test(key: &str) -> Result<Key> {
    parse_key(key)
}

fn parse_key(key: &str) -> Result<Key> {
    match key.to_lowercase().as_str() {
        "return" | "enter" => Ok(Key::Return),
        "tab" => Ok(Key::Tab),
        "space" => Ok(Key::Space),
        "backspace" => Ok(Key::Backspace),
        "escape" | "esc" => Ok(Key::Escape),
        "spacebar" => Ok(Key::Space),
        // browser 工具那边教的写法是 ArrowDown / Escape / Meta 这一套（DOM 键名），
        // 而模型是从**同一份工具目录**里学到的，不认这套等于逼它猜。全都收下。
        "up" | "uparrow" | "arrowup" => Ok(Key::UpArrow),
        "down" | "downarrow" | "arrowdown" => Ok(Key::DownArrow),
        "left" | "leftarrow" | "arrowleft" => Ok(Key::LeftArrow),
        "right" | "rightarrow" | "arrowright" => Ok(Key::RightArrow),
        // **cmd 在 Windows 上必须是 Ctrl，不是 Win 键。**
        //
        // 原来一律映射成 Key::Meta，而 enigo 在 Windows 上把 Meta 编成 VK_LWIN。
        // 于是模型照工具描述里的示例发 ["cmd","s"] 想保存文件，Windows 上实际按下的是
        // Win+S——打开系统搜索框，紧接着 keyboard.type 把内容打进了搜索框而不是目标应用。
        // 而 RPC 回 {"status":"ok"} 还带 delivered_to，三层都在说成功，模型收不到任何
        // 失败信号，会把「已保存」当既成事实继续往下做。Win+R / Win+D / Win+E 同理。
        //
        // 所以 cmd/command/mod/primary 一律解释成「这个平台的主修饰键」，
        // 而 win/super 保留给真正的 Windows 键——想按它的人还有话可说。
        #[cfg(target_os = "macos")]
        "cmd" | "command" | "mod" | "primary" => Ok(Key::Meta),
        #[cfg(not(target_os = "macos"))]
        "cmd" | "command" | "mod" | "primary" => Ok(Key::Control),
        "meta" | "super" | "win" => Ok(Key::Meta),
        "ctrl" | "control" => Ok(Key::Control),
        "alt" | "option" => Ok(Key::Alt),
        "shift" => Ok(Key::Shift),
        // 下面这些原来一个都不认，于是「按 F5 刷新」「按 Delete 删掉」「翻页」
        // 这类再普通不过的操作，在路线 B 里直接报 Unknown key —— 底层
        // （enigo）明明全都支持，只是这张白名单没写。
        "delete" | "del" | "forwarddelete" => Ok(Key::Delete),
        "home" => Ok(Key::Home),
        "end" => Ok(Key::End),
        "pageup" | "pgup" => Ok(Key::PageUp),
        "pagedown" | "pgdn" => Ok(Key::PageDown),
        "f1" => Ok(Key::F1),
        "f2" => Ok(Key::F2),
        "f3" => Ok(Key::F3),
        "f4" => Ok(Key::F4),
        "f5" => Ok(Key::F5),
        "f6" => Ok(Key::F6),
        "f7" => Ok(Key::F7),
        "f8" => Ok(Key::F8),
        "f9" => Ok(Key::F9),
        "f10" => Ok(Key::F10),
        "f11" => Ok(Key::F11),
        "f12" => Ok(Key::F12),
        s if s.len() == 1 => {
            let ch = s.chars().next().unwrap();
            Ok(Key::Character(ch))
        }
        _ => Err(Error::System(format!("Unknown key: {}", key))),
    }
}

#[cfg(test)]
mod input_permission_tests {
    /// 辅助功能授权**不能正向缓存到进程死亡**。
    ///
    /// sidecar 随 IDE 常驻几个小时；用户中途在系统设置里取消勾选（或 tccutil reset）后，
    /// 缓存让 ensure_input_permission 永远放行，macOS 静默丢弃全部合成键鼠事件——每层
    /// 回执都是 ok、screen.info 也报 granted，模型照着「已点击 / 已输入」继续推进。
    /// IDE 侧同一个 API（src-tauri permissions.rs）就是每次查的。
    /// 只扫生产代码里 macOS 那个变体的函数体，并先剥掉注释行——注释里会引用旧代码。
    #[test]
    fn accessibility_check_is_not_cached() {
        let src = include_str!("agent.rs");
        let prod = src.split("\n#[cfg(test)]").next().unwrap_or(src);
        let at = prod
            .find("pub fn input_permission_granted() -> bool {")
            .expect("input_permission_granted 不见了");
        let end = prod[at..].find("\n}").map(|e| at + e).unwrap_or(prod.len());
        let body: String = prod[at..end]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(body.contains("AXIsProcessTrusted()"), "macOS 分支必须每次真查 AXIsProcessTrusted: {body}");
        for bad in ["static ", "AtomicBool", "OnceLock", "OnceCell", "thread_local", "Mutex"] {
            assert!(
                !body.contains(bad),
                "又把授权结果缓存起来了（{bad}）——用户中途撤销后会永远放行: {body}"
            );
        }
    }
}
