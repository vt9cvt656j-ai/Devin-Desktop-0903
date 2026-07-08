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
        if self.system.is_none() {
            let sys = SystemAutomation::new()?;
            self.system = Some(Arc::new(Mutex::new(sys)));
        }
        Ok(())
    }
    
    #[cfg(feature = "system")]
    pub fn mouse_move(&mut self, x: i32, y: i32) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.move_mouse(x, y)
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
    pub fn mouse_double_click(&mut self) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.double_click(MouseButton::Left)
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
    
    #[cfg(feature = "system")]
    pub fn mouse_scroll(&mut self, amount: i32) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.scroll(amount, 0)
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
        let browser = self.browser.as_ref()
            .ok_or_else(|| Error::Other(anyhow::anyhow!("Browser not started")))?;
        let mut browser = browser.lock().unwrap();
        let runtime = self.runtime.as_ref().unwrap();
        runtime.block_on(browser.navigate(url))
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_click(&self, selector: &str) -> Result<()> {
        let browser = self.browser.as_ref()
            .ok_or_else(|| Error::Other(anyhow::anyhow!("Browser not started")))?;
        let mut browser = browser.lock().unwrap();
        let runtime = self.runtime.as_ref().unwrap();
        runtime.block_on(browser.click_element(selector))
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_type(&self, selector: &str, text: &str) -> Result<()> {
        let browser = self.browser.as_ref()
            .ok_or_else(|| Error::Other(anyhow::anyhow!("Browser not started")))?;
        let mut browser = browser.lock().unwrap();
        let runtime = self.runtime.as_ref().unwrap();
        runtime.block_on(browser.type_text(selector, text))
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_wait(&self, selector: &str, timeout_ms: u64) -> Result<()> {
        let browser = self.browser.as_ref()
            .ok_or_else(|| Error::Other(anyhow::anyhow!("Browser not started")))?;
        let mut browser = browser.lock().unwrap();
        let runtime = self.runtime.as_ref().unwrap();
        runtime.block_on(browser.wait_for_element(selector, timeout_ms))
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_eval(&self, script: &str) -> Result<serde_json::Value> {
        let browser = self.browser.as_ref()
            .ok_or_else(|| Error::Other(anyhow::anyhow!("Browser not started")))?;
        let mut browser = browser.lock().unwrap();
        let runtime = self.runtime.as_ref().unwrap();
        runtime.block_on(browser.execute_script(script))
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_screenshot(&self, path: Option<&str>) -> Result<()> {
        let browser = self.browser.as_ref()
            .ok_or_else(|| Error::Other(anyhow::anyhow!("Browser not started")))?;
        let mut browser = browser.lock().unwrap();
        let runtime = self.runtime.as_ref().unwrap();
        runtime.block_on(browser.screenshot(path))?;
        Ok(())
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_content(&self) -> Result<String> {
        let browser = self.browser.as_ref()
            .ok_or_else(|| Error::Other(anyhow::anyhow!("Browser not started")))?;
        let mut browser = browser.lock().unwrap();
        let runtime = self.runtime.as_ref().unwrap();
        runtime.block_on(browser.get_content())
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
