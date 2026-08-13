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
    use std::sync::atomic::{AtomicBool, Ordering};
    static GRANTED: AtomicBool = AtomicBool::new(false);
    // 授权一旦拿到，本进程生命周期内不会再撤销，缓存住即可；反过来没拿到时必须
    // 每次真查——用户可能刚在系统设置里勾上（虽然通常还需要重启应用才生效）。
    if GRANTED.load(Ordering::Relaxed) {
        return true;
    }
    let trusted = unsafe { accessibility_sys::AXIsProcessTrusted() };
    if trusted {
        GRANTED.store(true, Ordering::Relaxed);
    }
    trusted
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
        for _ in 0..40 {
            match sys.mouse_location() {
                Ok((cx, cy)) if (cx - x).abs() <= 1 && (cy - y).abs() <= 1 => return Ok(()),
                Ok(_) => {}
                Err(_) => return Ok(()),
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
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
        self.clipboard_set_text(text)?;
        self.paste_from_clipboard()
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
        if self.browser.is_some() {
            return Ok(());
        }
        
        let runtime = self.runtime.as_ref()
            .ok_or_else(|| Error::Other(anyhow::anyhow!("Runtime not initialized")))?;
        
        let browser = runtime.block_on(async {
            if headless {
                BrowserAutomation::new().await
            } else {
                BrowserAutomation::new_headed().await
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
    pub fn browser_screenshot(&self, path: Option<&str>) -> Result<()> {
        let mut browser = self.browser_lock()?;
        self.rt()?.block_on(browser.screenshot(path))?;
        Ok(())
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_content(&self) -> Result<String> {
        let mut browser = self.browser_lock()?;
        self.rt()?.block_on(browser.get_content())
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_close(&mut self) -> Result<()> {
        if let Some(browser_arc) = self.browser.take() {
            drop(browser_arc);
        }
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

fn parse_key(key: &str) -> Result<Key> {
    match key.to_lowercase().as_str() {
        "return" | "enter" => Ok(Key::Return),
        "tab" => Ok(Key::Tab),
        "space" => Ok(Key::Space),
        "backspace" => Ok(Key::Backspace),
        "escape" | "esc" => Ok(Key::Escape),
        "up" | "uparrow" => Ok(Key::UpArrow),
        "down" | "downarrow" => Ok(Key::DownArrow),
        "left" | "leftarrow" => Ok(Key::LeftArrow),
        "right" | "rightarrow" => Ok(Key::RightArrow),
        "cmd" | "command" | "meta" => Ok(Key::Meta),
        "ctrl" | "control" => Ok(Key::Control),
        "alt" | "option" => Ok(Key::Alt),
        "shift" => Ok(Key::Shift),
        s if s.len() == 1 => {
            let ch = s.chars().next().unwrap();
            Ok(Key::Character(ch))
        }
        _ => Err(Error::System(format!("Unknown key: {}", key))),
    }
}
