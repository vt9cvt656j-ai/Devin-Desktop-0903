//! macOS 平台特定实现

use crate::error::{Error, Result};
use crate::platform::WindowControl;
use crate::types::{ScreenInfo, WindowInfo};
use cocoa::base::{id, nil};
use core_graphics::display::CGDisplay;
use objc::{class, msg_send, sel, sel_impl};

pub struct MacOSControl;

impl MacOSControl {
    pub fn new() -> Self {
        Self
    }
}

impl WindowControl for MacOSControl {
    fn enumerate_windows(&self) -> Result<Vec<WindowInfo>> {
        let mut windows = Vec::new();
        
        unsafe {
            let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
            let apps: id = msg_send![workspace, runningApplications];
            let count: usize = msg_send![apps, count];
            
            for i in 0..count {
                let app: id = msg_send![apps, objectAtIndex: i];
                let app_name: id = msg_send![app, localizedName];
                
                if app_name == nil {
                    continue;
                }
                
                let name_ptr: *const i8 = msg_send![app_name, UTF8String];
                let name = std::ffi::CStr::from_ptr(name_ptr)
                    .to_string_lossy()
                    .to_string();
                
                let is_active: bool = msg_send![app, isActive];
                
                windows.push(WindowInfo {
                    title: name.clone(),
                    process_name: name,
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 0,
                    is_visible: is_active,
                    is_minimized: false,
                });
            }
        }
        
        Ok(windows)
    }
    
    fn find_window(&self, title: &str) -> Result<Option<WindowInfo>> {
        let windows = self.enumerate_windows()?;
        Ok(windows.into_iter().find(|w| w.title.contains(title)))
    }
    
    fn activate_window(&self, title: &str) -> Result<()> {
        unsafe {
            let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
            let apps: id = msg_send![workspace, runningApplications];
            let count: usize = msg_send![apps, count];
            
            for i in 0..count {
                let app: id = msg_send![apps, objectAtIndex: i];
                let app_name: id = msg_send![app, localizedName];
                
                if app_name == nil {
                    continue;
                }
                
                let name_ptr: *const i8 = msg_send![app_name, UTF8String];
                let name = std::ffi::CStr::from_ptr(name_ptr).to_string_lossy();
                
                if name.contains(title) {
                    let _: () = msg_send![app, activateWithOptions: 0];
                    return Ok(());
                }
            }
        }
        
        Err(Error::ElementNotFound(format!("未找到窗口: {}", title)))
    }
    
    fn minimize_window(&self, _title: &str) -> Result<()> {
        Err(Error::UnsupportedPlatform(
            "macOS 平台暂不支持最小化指定窗口".to_string()
        ))
    }
    
    fn maximize_window(&self, _title: &str) -> Result<()> {
        Err(Error::UnsupportedPlatform(
            "macOS 平台暂不支持最大化指定窗口".to_string()
        ))
    }
    
    fn close_window(&self, _title: &str) -> Result<()> {
        Err(Error::UnsupportedPlatform(
            "macOS 平台暂不支持关闭指定窗口".to_string()
        ))
    }
    
    fn get_screen_info(&self) -> Result<ScreenInfo> {
        let display = CGDisplay::main();
        let width = display.pixels_wide() as u32;
        let height = display.pixels_high() as u32;
        
        let scale_factor = unsafe {
            let screen: id = msg_send![class!(NSScreen), mainScreen];
            let backing_scale: f64 = msg_send![screen, backingScaleFactor];
            backing_scale
        };
        
        Ok(ScreenInfo {
            width,
            height,
            scale_factor,
        })
    }
}
