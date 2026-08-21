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

/// 当前前台应用名。切前台失败时光说「没成功」没法排查，得说出是谁占着前台。
fn frontmost_app_name() -> Option<String> {
    unsafe {
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let app: id = msg_send![workspace, frontmostApplication];
        if app == nil {
            return None;
        }
        let name: id = msg_send![app, localizedName];
        if name == nil {
            return None;
        }
        let ptr: *const i8 = msg_send![name, UTF8String];
        Some(std::ffi::CStr::from_ptr(ptr).to_string_lossy().to_string())
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
                    is_frontmost: is_active,
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
                    // 激活是异步的，而合成按键和点击只会投给**当前**前台应用。
                    // 发完就回 Ok 等于把「请求已发出」当成「已经切过去了」——冷启动、
                    // 跨 Space、被对话框截胡时它根本没切成，而后面每一次 keyboard.type
                    // 都打进上一个应用，并且一路返回成功。所以这里必须回读。
                    let deadline =
                        std::time::Instant::now() + std::time::Duration::from_millis(2500);
                    loop {
                        let active: bool = msg_send![app, isActive];
                        if active {
                            return Ok(());
                        }
                        if std::time::Instant::now() >= deadline {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(60));
                    }
                    let front = frontmost_app_name()
                        .unwrap_or_else(|| "（读不到）".to_string());
                    return Err(Error::Timeout(format!(
                        "已向「{}」发出激活请求，但 2.5 秒后它仍不在前台，当前前台是「{}」。\
合成按键和点击只进前台应用，此刻继续 keyboard.type / mouse.click 会打进「{}」。\
先处理挡在前面的东西（对话框、权限提示、另一个 Space），或改用 system open。",
                        name, front, front
                    )));
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
