//! 桌面应用元素操作（跨平台抽象）

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// 桌面元素信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopElement {
    /// 元素名称/标签
    pub name: String,
    /// 元素类型（按钮/输入框/文本/菜单项等）
    pub element_type: String,
    /// 元素位置（屏幕坐标）
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// 是否可交互
    pub is_enabled: bool,
    /// 是否可见
    pub is_visible: bool,
    /// 坐标是否有效（用于判断是否能回退到坐标点击）
    pub position_valid: bool,
    /// 平台特定句柄（不透明）
    #[serde(skip)]
    pub native_handle: Option<NativeHandle>,
}

impl Drop for DesktopElement {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        {
            if let Some(NativeHandle::MacOS(handle)) = self.native_handle {
                unsafe {
                    use core_foundation::base::CFRelease;
                    CFRelease(handle as *const std::ffi::c_void);
                }
            }
        }
    }
}

/// 平台特定的元素句柄
#[derive(Debug)]
pub enum NativeHandle {
    #[cfg(target_os = "windows")]
    Windows(isize), // IUIAutomationElement pointer
    #[cfg(target_os = "macos")]
    MacOS(usize), // AXUIElementRef as usize
}

// 禁止自动 Clone，防止 double-free
impl Clone for NativeHandle {
    fn clone(&self) -> Self {
        match self {
            #[cfg(target_os = "windows")]
            NativeHandle::Windows(handle) => {
                // Windows COM 对象需要 AddRef
                #[cfg(target_os = "windows")]
                {
                    use windows::core::{IUnknown, Interface};
                    unsafe {
                        let raw = *handle as *mut std::ffi::c_void;
                        if !raw.is_null() {
                            // windows 0.58 不再暴露 AddRef：借出指针后 clone 一次让计数 +1，
                            // forget 掉借用的那一份，净效果与 AddRef 相同。
                            let borrowed = IUnknown::from_raw(raw);
                            std::mem::forget(borrowed.clone());
                            std::mem::forget(borrowed);
                        }
                    }
                }
                NativeHandle::Windows(*handle)
            }
            #[cfg(target_os = "macos")]
            NativeHandle::MacOS(handle) => {
                // macOS CFRetain
                unsafe {
                    use core_foundation::base::CFRetain;
                    CFRetain(*handle as *const std::ffi::c_void);
                }
                NativeHandle::MacOS(*handle)
            }
        }
    }
}

/// 桌面元素控制器（平台特定实现）
pub trait DesktopElementControl {
    /// 在指定窗口中查找元素（按名称/标签）
    fn find_element_by_name(&self, window_title: &str, element_name: &str) -> Result<Option<DesktopElement>>;
    
    /// 查找所有匹配的元素
    fn find_elements_by_name(&self, window_title: &str, element_name: &str) -> Result<Vec<DesktopElement>>;
    
    /// 查找指定类型的元素（如"button", "edit", "menuitem"）
    fn find_elements_by_type(&self, window_title: &str, element_type: &str) -> Result<Vec<DesktopElement>>;
    
    /// 点击元素
    fn click_element(&self, element: &DesktopElement) -> Result<()>;
    
    /// 向输入框输入文本
    fn type_into_element(&self, element: &DesktopElement, text: &str) -> Result<()>;
    
    /// 获取元素文本内容
    fn get_element_text(&self, element: &DesktopElement) -> Result<String>;
}
