//! 系统级自动化模块
//!
//! 跨平台鼠标、键盘和窗口控制

use crate::error::{Error, Result};
use crate::types::*;
use enigo::{
    Button as EnigoButton, Coordinate, Direction, Enigo, Keyboard, Mouse, 
    Settings as EnigoSettings,
};
use tracing::{debug, info};

/// 系统自动化控制器
pub struct SystemAutomation {
    enigo: Enigo,
}

impl SystemAutomation {
    /// 创建新的系统自动化实例
    pub fn new() -> Result<Self> {
        info!("初始化系统自动化");
        let enigo = Enigo::new(&EnigoSettings::default())
            .map_err(|e| Error::System(format!("初始化失败: {:?}", e)))?;
        
        Ok(Self { enigo })
    }

    /// 移动鼠标到指定位置
    pub fn move_mouse(&mut self, x: i32, y: i32) -> Result<()> {
        debug!("移动鼠标到 ({}, {})", x, y);
        self.enigo
            .move_mouse(x, y, Coordinate::Abs)
            .map_err(|e| Error::System(format!("移动鼠标失败: {:?}", e)))?;
        Ok(())
    }

    /// 相对移动鼠标
    pub fn move_mouse_relative(&mut self, dx: i32, dy: i32) -> Result<()> {
        debug!("相对移动鼠标 ({}, {})", dx, dy);
        self.enigo
            .move_mouse(dx, dy, Coordinate::Rel)
            .map_err(|e| Error::System(format!("相对移动鼠标失败: {:?}", e)))?;
        Ok(())
    }

    /// 鼠标点击
    pub fn click(&mut self, button: MouseButton) -> Result<()> {
        debug!("点击鼠标按钮: {:?}", button);
        let enigo_button = Self::convert_button(button);
        self.enigo
            .button(enigo_button, Direction::Click)
            .map_err(|e| Error::System(format!("点击失败: {:?}", e)))?;
        Ok(())
    }

    /// 鼠标按下
    pub fn mouse_down(&mut self, button: MouseButton) -> Result<()> {
        debug!("鼠标按下: {:?}", button);
        let enigo_button = Self::convert_button(button);
        self.enigo
            .button(enigo_button, Direction::Press)
            .map_err(|e| Error::System(format!("鼠标按下失败: {:?}", e)))?;
        Ok(())
    }

    /// 鼠标释放
    pub fn mouse_up(&mut self, button: MouseButton) -> Result<()> {
        debug!("鼠标释放: {:?}", button);
        let enigo_button = Self::convert_button(button);
        self.enigo
            .button(enigo_button, Direction::Release)
            .map_err(|e| Error::System(format!("鼠标释放失败: {:?}", e)))?;
        Ok(())
    }

    /// 鼠标双击
    pub fn double_click(&mut self, button: MouseButton) -> Result<()> {
        debug!("双击: {:?}", button);
        self.click(button)?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        self.click(button)?;
        Ok(())
    }

    /// 鼠标滚动
    pub fn scroll(&mut self, delta_x: i32, delta_y: i32) -> Result<()> {
        debug!("滚动: x={}, y={}", delta_x, delta_y);
        if delta_y != 0 {
            self.enigo
                .scroll(delta_y, enigo::Axis::Vertical)
                .map_err(|e| Error::System(format!("垂直滚动失败: {:?}", e)))?;
        }
        if delta_x != 0 {
            self.enigo
                .scroll(delta_x, enigo::Axis::Horizontal)
                .map_err(|e| Error::System(format!("水平滚动失败: {:?}", e)))?;
        }
        Ok(())
    }

    /// 拖拽操作
    pub fn drag(&mut self, from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> Result<()> {
        debug!("拖拽: ({}, {}) -> ({}, {})", from_x, from_y, to_x, to_y);
        
        self.move_mouse(from_x, from_y)?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        self.mouse_down(MouseButton::Left)?;
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        self.move_mouse(to_x, to_y)?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        self.mouse_up(MouseButton::Left)?;
        
        Ok(())
    }

    /// 输入文本
    pub fn type_text(&mut self, text: &str) -> Result<()> {
        debug!("输入文本: {} 字符", text.len());
        self.enigo
            .text(text)
            .map_err(|e| Error::System(format!("输入文本失败: {:?}", e)))?;
        Ok(())
    }

    /// 按下并释放按键
    pub fn press_key(&mut self, key: Key) -> Result<()> {
        debug!("按键: {:?}", key);
        let enigo_key = Self::convert_key(&key)?;
        self.enigo
            .key(enigo_key, Direction::Click)
            .map_err(|e| Error::System(format!("按键失败: {:?}", e)))?;
        Ok(())
    }

    /// 按下按键
    pub fn key_down(&mut self, key: Key) -> Result<()> {
        debug!("按下按键: {:?}", key);
        let enigo_key = Self::convert_key(&key)?;
        self.enigo
            .key(enigo_key, Direction::Press)
            .map_err(|e| Error::System(format!("按下按键失败: {:?}", e)))?;
        Ok(())
    }

    /// 释放按键
    pub fn key_up(&mut self, key: Key) -> Result<()> {
        debug!("释放按键: {:?}", key);
        let enigo_key = Self::convert_key(&key)?;
        self.enigo
            .key(enigo_key, Direction::Release)
            .map_err(|e| Error::System(format!("释放按键失败: {:?}", e)))?;
        Ok(())
    }

    /// 组合键（如 Ctrl+C）
    pub fn key_combination(&mut self, keys: Vec<Key>) -> Result<()> {
        debug!("组合键: {:?}", keys);
        
        // 按下所有键
        for key in &keys {
            self.key_down(key.clone())?;
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        // 释放所有键（逆序）
        for key in keys.iter().rev() {
            self.key_up(key.clone())?;
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        
        Ok(())
    }

    /// 执行鼠标操作
    pub fn execute_mouse_action(&mut self, action: MouseAction) -> Result<ExecutionResult> {
        match action {
            MouseAction::Move { x, y, mode } => {
                match mode {
                    CoordinateMode::Absolute => self.move_mouse(x, y)?,
                    CoordinateMode::Relative => self.move_mouse_relative(x, y)?,
                }
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("鼠标移动到 ({}, {})", x, y)),
                    data: None,
                })
            }
            MouseAction::Click { button } => {
                self.click(button)?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("点击 {:?}", button)),
                    data: None,
                })
            }
            MouseAction::DoubleClick { button } => {
                self.double_click(button)?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("双击 {:?}", button)),
                    data: None,
                })
            }
            MouseAction::Down { button } => {
                self.mouse_down(button)?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("按下 {:?}", button)),
                    data: None,
                })
            }
            MouseAction::Up { button } => {
                self.mouse_up(button)?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("释放 {:?}", button)),
                    data: None,
                })
            }
            MouseAction::Scroll { delta_x, delta_y } => {
                self.scroll(delta_x, delta_y)?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("滚动 x={}, y={}", delta_x, delta_y)),
                    data: None,
                })
            }
            MouseAction::Drag { from_x, from_y, to_x, to_y } => {
                self.drag(from_x, from_y, to_x, to_y)?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("拖拽完成")),
                    data: None,
                })
            }
        }
    }

    /// 执行键盘操作
    pub fn execute_keyboard_action(&mut self, action: KeyboardAction) -> Result<ExecutionResult> {
        match action {
            KeyboardAction::Text(text) => {
                self.type_text(&text)?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("输入文本: {} 字符", text.len())),
                    data: None,
                })
            }
            KeyboardAction::Press(key) => {
                self.press_key(key.clone())?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("按键: {:?}", key)),
                    data: None,
                })
            }
            KeyboardAction::Down(key) => {
                self.key_down(key.clone())?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("按下: {:?}", key)),
                    data: None,
                })
            }
            KeyboardAction::Up(key) => {
                self.key_up(key.clone())?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("释放: {:?}", key)),
                    data: None,
                })
            }
            KeyboardAction::Combination(keys) => {
                self.key_combination(keys.clone())?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("组合键: {:?}", keys)),
                    data: None,
                })
            }
        }
    }

    // 辅助函数：转换按钮类型
    fn convert_button(button: MouseButton) -> EnigoButton {
        match button {
            MouseButton::Left => EnigoButton::Left,
            MouseButton::Right => EnigoButton::Right,
            MouseButton::Middle => EnigoButton::Middle,
        }
    }

    // 辅助函数：转换按键类型
    fn convert_key(key: &Key) -> Result<enigo::Key> {
        use enigo::Key as EK;
        
        let enigo_key = match key {
            Key::Character(c) => EK::Unicode(*c),
            Key::String(_) => return Err(Error::System(
                "字符串类型应使用 type_text 方法".to_string()
            )),
            Key::Return => EK::Return,
            Key::Tab => EK::Tab,
            Key::Space => EK::Space,
            Key::Backspace => EK::Backspace,
            Key::Escape => EK::Escape,
            Key::Delete => EK::Delete,
            Key::Home => EK::Home,
            Key::End => EK::End,
            Key::PageUp => EK::PageUp,
            Key::PageDown => EK::PageDown,
            Key::LeftArrow => EK::LeftArrow,
            Key::RightArrow => EK::RightArrow,
            Key::UpArrow => EK::UpArrow,
            Key::DownArrow => EK::DownArrow,
            Key::F1 => EK::F1,
            Key::F2 => EK::F2,
            Key::F3 => EK::F3,
            Key::F4 => EK::F4,
            Key::F5 => EK::F5,
            Key::F6 => EK::F6,
            Key::F7 => EK::F7,
            Key::F8 => EK::F8,
            Key::F9 => EK::F9,
            Key::F10 => EK::F10,
            Key::F11 => EK::F11,
            Key::F12 => EK::F12,
            Key::Control => EK::Control,
            Key::Shift => EK::Shift,
            Key::Alt => EK::Alt,
            Key::Meta => EK::Meta,
        };
        
        Ok(enigo_key)
    }
}
