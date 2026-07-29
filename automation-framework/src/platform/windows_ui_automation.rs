//! Windows UI Automation 实现

use crate::error::{Error, Result};
use crate::platform::desktop_element::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Accessibility::*;
use std::mem;

pub struct WindowsUIAutomation {
    automation: Option<IUIAutomation>,
}

impl WindowsUIAutomation {
    pub fn new() -> Result<Self> {
        unsafe {
            // 初始化 COM
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .map_err(|e| Error::System(format!("COM 初始化失败: {:?}", e)))?;
            
            // 创建 UI Automation 实例
            let automation: IUIAutomation = CoCreateInstance(
                &CUIAutomation,
                None,
                CLSCTX_INPROC_SERVER,
            ).map_err(|e| Error::System(format!("创建 UI Automation 失败: {:?}", e)))?;
            
            Ok(Self {
                automation: Some(automation),
            })
        }
    }
    
    fn get_window_element(&self, window_title: &str) -> Result<IUIAutomationElement> {
        unsafe {
            let automation = self.automation.as_ref()
                .ok_or_else(|| Error::System("UI Automation 未初始化".to_string()))?;
            
            // 获取根元素
            let root = automation.GetRootElement()
                .map_err(|e| Error::System(format!("获取根元素失败: {:?}", e)))?;
            
            // 创建条件：查找窗口标题包含指定文本
            let title_bstr = windows::core::BSTR::from(window_title);
            let name_property = automation.CreatePropertyCondition(
                UIA_NamePropertyId,
                &windows::core::VARIANT::from(title_bstr),
            ).map_err(|e| Error::System(format!("创建条件失败: {:?}", e)))?;
            
            // 查找窗口
            let window = root.FindFirst(TreeScope_Children, &name_property)
                .map_err(|e| Error::ElementNotFound(format!("未找到窗口 '{}': {:?}", window_title, e)))?;
            
            Ok(window)
        }
    }
}

impl DesktopElementControl for WindowsUIAutomation {
    fn find_element_by_name(&self, window_title: &str, element_name: &str) -> Result<Option<DesktopElement>> {
        unsafe {
            let automation = self.automation.as_ref()
                .ok_or_else(|| Error::System("UI Automation 未初始化".to_string()))?;
            
            let window = self.get_window_element(window_title)?;
            
            // 创建名称匹配条件
            let name_bstr = windows::core::BSTR::from(element_name);
            let name_condition = automation.CreatePropertyCondition(
                UIA_NamePropertyId,
                &windows::core::VARIANT::from(name_bstr),
            ).map_err(|e| Error::System(format!("创建条件失败: {:?}", e)))?;
            
            // 在窗口及其子元素中查找
            let element = window.FindFirst(TreeScope_Descendants, &name_condition)
                .ok();
            
            if let Some(elem) = element {
                let name = elem.CurrentName()
                    .unwrap_or_default()
                    .to_string();
                
                let control_type = elem.CurrentControlType()
                    .unwrap_or(UIA_ButtonControlTypeId);
                
                let element_type = match control_type.0 {
                    50000 => "button",
                    50004 => "edit",
                    50009 => "menuitem",
                    50020 => "text",
                    _ => "unknown",
                }.to_string();
                
                let rect = elem.CurrentBoundingRectangle()
                    .unwrap_or_default();
                
                let is_enabled = elem.CurrentIsEnabled()
                    .unwrap_or(false)
                    .as_bool();
                
                let is_visible = !elem.CurrentIsOffscreen()
                    .unwrap_or(true)
                    .as_bool();
                
                let handle = elem.as_raw() as isize;
                elem.AddRef(); // 句柄要存进 DesktopElement，elem 本体马上 drop（Release）；不补引用计数就是悬垂指针
                
                Ok(Some(DesktopElement {
                    name,
                    element_type,
                    x: rect.left,
                    y: rect.top,
                    width: rect.right - rect.left,
                    height: rect.bottom - rect.top,
                    is_enabled,
                    is_visible,
                    position_valid: true,
                    native_handle: Some(NativeHandle::Windows(handle)),
                }))
            } else {
                Ok(None)
            }
        }
    }
    
    fn find_elements_by_name(&self, window_title: &str, element_name: &str) -> Result<Vec<DesktopElement>> {
        // 简化实现：只返回第一个匹配
        if let Some(elem) = self.find_element_by_name(window_title, element_name)? {
            Ok(vec![elem])
        } else {
            Ok(vec![])
        }
    }
    
    fn find_elements_by_type(&self, window_title: &str, element_type: &str) -> Result<Vec<DesktopElement>> {
        unsafe {
            let automation = self.automation.as_ref()
                .ok_or_else(|| Error::System("UI Automation 未初始化".to_string()))?;
            
            let window = self.get_window_element(window_title)?;
            
            let control_type_id = match element_type {
                "button" => UIA_ButtonControlTypeId,
                "edit" => UIA_EditControlTypeId,
                "menuitem" => UIA_MenuItemControlTypeId,
                "text" => UIA_TextControlTypeId,
                _ => return Ok(vec![]),
            };
            
            let type_condition = automation.CreatePropertyCondition(
                UIA_ControlTypePropertyId,
                &windows::core::VARIANT::from(control_type_id.0 as i32),
            ).map_err(|e| Error::System(format!("创建条件失败: {:?}", e)))?;
            
            let elements = window.FindAll(TreeScope_Descendants, &type_condition)
                .map_err(|e| Error::System(format!("查找元素失败: {:?}", e)))?;
            
            let count = elements.Length()
                .unwrap_or(0);
            
            let mut results = Vec::new();
            for i in 0..count {
                if let Ok(elem) = elements.GetElement(i) {
                    let name = elem.CurrentName()
                        .unwrap_or_default()
                        .to_string();
                    
                    let rect = elem.CurrentBoundingRectangle()
                        .unwrap_or_default();
                    
                    let is_enabled = elem.CurrentIsEnabled()
                        .unwrap_or(false)
                        .as_bool();
                    
                    let is_visible = !elem.CurrentIsOffscreen()
                        .unwrap_or(true)
                        .as_bool();
                    
                    let handle = elem.as_raw() as isize;
                    elem.AddRef(); // 增加引用计数，防止提前释放

                    results.push(DesktopElement {
                        name,
                        element_type: element_type.to_string(),
                        x: rect.left,
                        y: rect.top,
                        width: rect.right - rect.left,
                        height: rect.bottom - rect.top,
                        is_enabled,
                        is_visible,
                        position_valid: true,
                        native_handle: Some(NativeHandle::Windows(handle)),
                    });
                }
            }
            
            Ok(results)
        }
    }
    
    fn click_element(&self, element: &DesktopElement) -> Result<()> {
        if let Some(NativeHandle::Windows(handle)) = element.native_handle {
            unsafe {
                let elem = IUIAutomationElement::from_raw(handle as *mut _);
                elem.AddRef(); // 借用指针，增加引用计数
                
                // 尝试调用 Invoke 模式（适用于按钮）
                let invoke_pattern: Result<IUIAutomationInvokePattern, _> = elem.GetCurrentPatternAs(UIA_InvokePatternId);
                
                if let Ok(pattern) = invoke_pattern {
                    pattern.Invoke()
                        .map_err(|e| Error::System(format!("调用元素失败: {:?}", e)))?;
                    return Ok(());
                }
                
                // 备选：使用鼠标点击中心点
                Err(Error::System("元素不支持 Invoke 模式，请使用鼠标点击".to_string()))
            }
        } else {
            Err(Error::System("无效的元素句柄".to_string()))
        }
    }
    
    fn type_into_element(&self, element: &DesktopElement, text: &str) -> Result<()> {
        if let Some(NativeHandle::Windows(handle)) = element.native_handle {
            unsafe {
                let elem = IUIAutomationElement::from_raw(handle as *mut _);
                elem.AddRef(); // 借用指针，增加引用计数
                
                // 使用 Value 模式设置文本
                let value_pattern: IUIAutomationValuePattern = elem.GetCurrentPatternAs(UIA_ValuePatternId)
                    .map_err(|e| Error::System(format!("获取 Value 模式失败: {:?}", e)))?;
                
                let text_bstr = windows::core::BSTR::from(text);
                value_pattern.SetValue(&text_bstr)
                    .map_err(|e| Error::System(format!("设置文本失败: {:?}", e)))?;
                
                Ok(())
            }
        } else {
            Err(Error::System("无效的元素句柄".to_string()))
        }
    }
    
    fn get_element_text(&self, element: &DesktopElement) -> Result<String> {
        if let Some(NativeHandle::Windows(handle)) = element.native_handle {
            unsafe {
                let elem = IUIAutomationElement::from_raw(handle as *mut _);
                elem.AddRef(); // 借用指针，增加引用计数
                
                let name = elem.CurrentName()
                    .unwrap_or_default()
                    .to_string();
                
                Ok(name)
            }
        } else {
            Err(Error::System("无效的元素句柄".to_string()))
        }
    }
}

impl Drop for WindowsUIAutomation {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
