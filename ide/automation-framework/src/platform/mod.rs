//! 平台特定功能模块

pub mod desktop_element;

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub mod windows_ui_automation;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
pub mod macos_accessibility;
/// 前台应用可访问性树的**原生**快照。JXA 那条路每读一个属性就是一次 Apple Event
/// 往返，实测真实窗口下 500 个元素要 95 秒，而读屏上限是 6 秒——必然超时。
#[cfg(target_os = "macos")]
pub mod macos_tree;

use crate::error::Result;
use crate::types::{ScreenInfo, WindowInfo};

/// 平台特定窗口操作接口
pub trait WindowControl {
    /// 枚举所有窗口
    fn enumerate_windows(&self) -> Result<Vec<WindowInfo>>;
    
    /// 查找窗口（根据标题）
    fn find_window(&self, title: &str) -> Result<Option<WindowInfo>>;
    
    /// 激活窗口
    fn activate_window(&self, title: &str) -> Result<()>;
    
    /// 最小化窗口
    fn minimize_window(&self, title: &str) -> Result<()>;
    
    /// 最大化窗口
    fn maximize_window(&self, title: &str) -> Result<()>;
    
    /// 关闭窗口
    fn close_window(&self, title: &str) -> Result<()>;
    
    /// 获取屏幕信息
    fn get_screen_info(&self) -> Result<ScreenInfo>;
}

/// 获取平台特定的窗口控制器
#[cfg(target_os = "windows")]
pub fn get_window_controller() -> Box<dyn WindowControl> {
    Box::new(windows::WindowsControl::new())
}

#[cfg(target_os = "macos")]
pub fn get_window_controller() -> Box<dyn WindowControl> {
    Box::new(macos::MacOSControl::new())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn get_window_controller() -> Box<dyn WindowControl> {
    compile_error!("当前平台不支持窗口控制功能");
}
