use serde::{Deserialize, Serialize};

/// 鼠标按钮
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// 坐标模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CoordinateMode {
    /// 绝对坐标
    Absolute,
    /// 相对当前位置
    Relative,
}

/// 鼠标操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseAction {
    /// 移动到指定位置
    Move { x: i32, y: i32, mode: CoordinateMode },
    /// 点击
    Click { button: MouseButton },
    /// 双击
    DoubleClick { button: MouseButton },
    /// 按下
    Down { button: MouseButton },
    /// 释放
    Up { button: MouseButton },
    /// 滚动
    Scroll { delta_x: i32, delta_y: i32 },
    /// 拖拽
    Drag { from_x: i32, from_y: i32, to_x: i32, to_y: i32 },
}

/// 键盘按键（常用键）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Key {
    Character(char),
    String(String),
    Return,
    Tab,
    Space,
    Backspace,
    Escape,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    LeftArrow,
    RightArrow,
    UpArrow,
    DownArrow,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    Control,
    Shift,
    Alt,
    Meta, // Windows键/Command键
}

/// 键盘操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyboardAction {
    /// 输入文本
    Text(String),
    /// 按键
    Press(Key),
    /// 按下
    Down(Key),
    /// 释放
    Up(Key),
    /// 组合键（如 Ctrl+C）
    Combination(Vec<Key>),
}

/// 浏览器操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserAction {
    /// 导航到 URL
    Navigate(String),
    /// 点击元素（CSS选择器）
    Click(String),
    /// 输入文本到元素
    Type { selector: String, text: String },
    /// 等待元素出现
    WaitForElement { selector: String, timeout_ms: u64 },
    /// 执行 JavaScript
    ExecuteScript(String),
    /// 截图
    Screenshot { path: Option<String> },
    /// 获取页面内容
    GetContent,
    /// 滚动
    Scroll { x: i32, y: i32 },
}

/// 系统窗口信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub title: String,
    pub process_name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_visible: bool,
    pub is_minimized: bool,
    /// 它**是不是当前前台**。
    ///
    /// 原来没有这个字段，两个平台都拿 is_visible 凑：macOS 塞的是 isActive（碰巧对），
    /// Windows 硬编码 true（永远错）。于是「谁在前台」在 Windows 上恒等于「枚举到的
    /// 第一个窗口」，而合成按键只会进入真正的前台应用——每条按键回执的 delivered_to、
    /// window.activate 的确认、window.list 的每一行，全都在说假话。
    #[serde(default)]
    pub is_frontmost: bool,
}

/// 屏幕信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenInfo {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

/// 自动化命令（可序列化的操作序列）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutomationCommand {
    Mouse(MouseAction),
    Keyboard(KeyboardAction),
    #[cfg(feature = "browser")]
    Browser(BrowserAction),
    /// 等待（毫秒）
    Wait(u64),
    /// 条件等待
    WaitUntil { condition: String, timeout_ms: u64 },
    /// 批量执行
    Batch(Vec<AutomationCommand>),
}

/// 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<serde_json::Value>,
}
