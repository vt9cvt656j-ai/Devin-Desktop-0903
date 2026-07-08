//! macOS Accessibility API 实现 - 使用原生 CoreFoundation

#[cfg(target_os = "macos")]
use crate::error::{Error, Result};
use crate::platform::desktop_element::{DesktopElement, DesktopElementControl, NativeHandle};
use core_foundation::base::{CFRelease, CFRetain, TCFType};
use core_foundation::string::{CFString, CFStringRef};
use core_foundation::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
use core_foundation::number::CFNumber;
use core_foundation::dictionary::CFDictionary;
use std::ptr;

#[repr(C)]
struct __AXUIElement(std::ffi::c_void);
type AXUIElementRef = *const __AXUIElement;

extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut *const std::ffi::c_void,
    ) -> i32;
    fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> i32;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *const std::ffi::c_void,
    ) -> i32;
}

// 直接定义常量字符串，避免链接外部符号
const AX_CHILDREN: &str = "AXChildren";
const AX_TITLE: &str = "AXTitle";
const AX_VALUE: &str = "AXValue";
const AX_PRESS: &str = "AXPress";
const AX_POSITION: &str = "AXPosition";
const AX_SIZE: &str = "AXSize";

pub struct MacOSAccessibility;

impl MacOSAccessibility {
    pub fn new() -> Result<Self> {
        Ok(MacOSAccessibility)
    }

    unsafe fn get_running_app_pid(name: &str) -> Option<i32> {
        // 转义双引号防止命令注入
        let escaped_name = name.replace("\"", "\\\"");
        std::process::Command::new("osascript")
            .arg("-e")
            .arg(format!("tell application \"{}\" to get id", escaped_name))
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse::<i32>().ok())
    }

    unsafe fn find_element_recursive(
        element: AXUIElementRef,
        target_name: &str,
        depth: u32,
    ) -> Option<AXUIElementRef> {
        const MAX_DEPTH: u32 = 50; // 提高到 50 以支持深层 UI（IDE/DevTools）
        if depth > MAX_DEPTH {
            return None;
        }

        // 获取元素标题
        let mut title_val: *const std::ffi::c_void = ptr::null();
        let title_attr = CFString::new(AX_TITLE);
        if AXUIElementCopyAttributeValue(element, title_attr.as_concrete_TypeRef(), &mut title_val) == 0
            && !title_val.is_null()
        {
            let title_str = CFString::wrap_under_create_rule(title_val as CFStringRef);
            if title_str.to_string().contains(target_name) {
                CFRetain(element as *const std::ffi::c_void);
                return Some(element);
            }
        }

        // 递归子元素
        let mut children_val: *const std::ffi::c_void = ptr::null();
        let children_attr = CFString::new(AX_CHILDREN);
        if AXUIElementCopyAttributeValue(element, children_attr.as_concrete_TypeRef(), &mut children_val) == 0
            && !children_val.is_null()
        {
            let children_array = children_val as CFArrayRef;
            let count = CFArrayGetCount(children_array);
            
            for i in 0..count {
                let child_ref = CFArrayGetValueAtIndex(children_array, i) as AXUIElementRef;
                
                if let Some(found) = Self::find_element_recursive(child_ref, target_name, depth + 1) {
                    // 找到后释放 children_array 再返回
                    CFRelease(children_val);
                    return Some(found);
                }
            }
            
            // 未找到也要释放
            CFRelease(children_val);
        }

        None
    }

    unsafe fn get_element_position(element: AXUIElementRef) -> Option<(i32, i32)> {
        let mut pos_val: *const std::ffi::c_void = ptr::null();
        let pos_attr = CFString::new(AX_POSITION);
        
        if AXUIElementCopyAttributeValue(element, pos_attr.as_concrete_TypeRef(), &mut pos_val) == 0
            && !pos_val.is_null()
        {
            // AXPosition 返回 CGPoint 结构体，在 macOS 中是 CFDictionary
            use core_foundation::base::CFType;
            let cf_type = CFType::wrap_under_create_rule(pos_val as *const _);
            if let Some(dict) = cf_type.downcast_into::<CFDictionary>() {
                let x_key = CFString::new("X");
                let y_key = CFString::new("Y");
                
                if let Some(x_val) = dict.find(x_key.as_CFTypeRef()) {
                    if let Some(y_val) = dict.find(y_key.as_CFTypeRef()) {
                        let x_num = CFNumber::wrap_under_get_rule(*x_val as *const _);
                        let y_num = CFNumber::wrap_under_get_rule(*y_val as *const _);
                        
                        if let (Some(x), Some(y)) = (x_num.to_i32(), y_num.to_i32()) {
                            return Some((x, y));
                        }
                    }
                }
            }
        }
        
        None
    }

    unsafe fn get_element_size(element: AXUIElementRef) -> Option<(i32, i32)> {
        let mut size_val: *const std::ffi::c_void = ptr::null();
        let size_attr = CFString::new(AX_SIZE);
        
        if AXUIElementCopyAttributeValue(element, size_attr.as_concrete_TypeRef(), &mut size_val) == 0
            && !size_val.is_null()
        {
            use core_foundation::base::CFType;
            let cf_type = CFType::wrap_under_create_rule(size_val as *const _);
            if let Some(dict) = cf_type.downcast_into::<CFDictionary>() {
                let w_key = CFString::new("Width");
                let h_key = CFString::new("Height");
                
                if let Some(w_val) = dict.find(w_key.as_CFTypeRef()) {
                    if let Some(h_val) = dict.find(h_key.as_CFTypeRef()) {
                        let w_num = CFNumber::wrap_under_get_rule(*w_val as *const _);
                        let h_num = CFNumber::wrap_under_get_rule(*h_val as *const _);
                        
                        if let (Some(w), Some(h)) = (w_num.to_i32(), h_num.to_i32()) {
                            return Some((w, h));
                        }
                    }
                }
            }
        }
        
        None
    }
}

impl DesktopElementControl for MacOSAccessibility {
    fn find_element_by_name(&self, window: &str, name: &str) -> Result<Option<DesktopElement>> {
        unsafe {
            let pid = Self::get_running_app_pid(window)
                .ok_or_else(|| Error::ElementNotFound(format!("应用 {} 未运行或无法访问", window)))?;

            let app_element = AXUIElementCreateApplication(pid);
            if app_element.is_null() {
                return Err(Error::System("无法获取应用 UI 元素".to_string()));
            }

            let result = if let Some(element) = Self::find_element_recursive(app_element, name, 0) {
                // 获取实际位置和大小
                let pos = Self::get_element_position(element);
                let (x, y) = pos.unwrap_or((0, 0));
                let (width, height) = Self::get_element_size(element).unwrap_or((100, 30));

                Ok(Some(DesktopElement {
                    name: name.to_string(),
                    element_type: "unknown".to_string(),
                    x,
                    y,
                    width,
                    height,
                    is_visible: true,
                    is_enabled: true,
                    position_valid: pos.is_some(),
                    native_handle: Some(NativeHandle::MacOS(element as usize)),
                }))
            } else {
                Ok(None)
            };

            // 修复问题1：释放 app_element
            CFRelease(app_element as *const std::ffi::c_void);
            
            result
        }
    }

    fn find_elements_by_name(&self, window: &str, name: &str) -> Result<Vec<DesktopElement>> {
        self.find_element_by_name(window, name)?
            .map(|e| vec![e])
            .ok_or_else(|| Error::System("未找到元素".to_string()))
    }

    fn find_elements_by_type(&self, _window: &str, _element_type: &str) -> Result<Vec<DesktopElement>> {
        Err(Error::UnsupportedPlatform("macOS AX API 不支持按类型查找元素".to_string()))
    }

    fn click_element(&self, element: &DesktopElement) -> Result<()> {
        unsafe {
            if let Some(NativeHandle::MacOS(handle)) = &element.native_handle {
                let ax_element = *handle as AXUIElementRef;
                let press_action = CFString::new(AX_PRESS);
                let result = AXUIElementPerformAction(ax_element, press_action.as_concrete_TypeRef());
                if result == 0 {
                    return Ok(());
                }
                
                // AXPress 失败，静默回退到坐标点击（某些元素如静态文本不支持 AXPress）
            }
            
            // 检查坐标有效性再回退
            if !element.position_valid {
                return Err(Error::System("元素无有效坐标，且 AXPress 不可用".to_string()));
            }
            
            // 回退到坐标点击
            use crate::system::SystemAutomation;
            let mut system = SystemAutomation::new()?;
            system.move_mouse(element.x + element.width / 2, element.y + element.height / 2)?;
            std::thread::sleep(std::time::Duration::from_millis(100));
            system.click(crate::types::MouseButton::Left)?;
            Ok(())
        }
    }

    fn type_into_element(&self, element: &DesktopElement, text: &str) -> Result<()> {
        unsafe {
            if let Some(NativeHandle::MacOS(handle)) = &element.native_handle {
                let ax_element = *handle as AXUIElementRef;
                let cf_text = CFString::new(text);
                let value_attr = CFString::new(AX_VALUE);
                let result = AXUIElementSetAttributeValue(
                    ax_element,
                    value_attr.as_concrete_TypeRef(),
                    cf_text.as_concrete_TypeRef() as *const _,
                );
                if result == 0 {
                    return Ok(());
                }
            }
        }

        // 回退到键盘输入
        self.click_element(element)?;
        std::thread::sleep(std::time::Duration::from_millis(100));
        use crate::system::SystemAutomation;
        let mut system = SystemAutomation::new()?;
        system.type_text(text)?;
        Ok(())
    }

    fn get_element_text(&self, element: &DesktopElement) -> Result<String> {
        unsafe {
            if let Some(NativeHandle::MacOS(handle)) = &element.native_handle {
                let ax_element = *handle as AXUIElementRef;
                let mut value_val: *const std::ffi::c_void = ptr::null();
                let value_attr = CFString::new(AX_VALUE);
                
                if AXUIElementCopyAttributeValue(ax_element, value_attr.as_concrete_TypeRef(), &mut value_val) == 0
                    && !value_val.is_null()
                {
                    let value_str = CFString::wrap_under_create_rule(value_val as CFStringRef);
                    return Ok(value_str.to_string());
                }
            }
        }

        Err(Error::System("无法获取元素文本".to_string()))
    }
}
