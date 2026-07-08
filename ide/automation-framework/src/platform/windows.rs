//! Windows 平台特定实现

use crate::error::{Error, Result};
use crate::platform::WindowControl;
use crate::types::{ScreenInfo, WindowInfo};
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Graphics::Gdi::*;
use std::mem;

pub struct WindowsControl;

impl WindowsControl {
    pub fn new() -> Self {
        Self
    }
}

impl WindowControl for WindowsControl {
    fn enumerate_windows(&self) -> Result<Vec<WindowInfo>> {
        let mut windows = Vec::new();
        
        unsafe {
            EnumWindows(
                Some(enum_windows_callback),
                LPARAM(&mut windows as *mut _ as isize),
            )
            .map_err(|e| Error::System(format!("枚举窗口失败: {:?}", e)))?;
        }
        
        Ok(windows)
    }
    
    fn find_window(&self, title: &str) -> Result<Option<WindowInfo>> {
        let windows = self.enumerate_windows()?;
        Ok(windows.into_iter().find(|w| w.title.contains(title)))
    }
    
    fn activate_window(&self, title: &str) -> Result<()> {
        let window = self.find_window(title)?
            .ok_or_else(|| Error::ElementNotFound(format!("未找到窗口: {}", title)))?;
        
        let hwnd = find_hwnd_by_title(title)?;
        
        unsafe {
            // 如果窗口最小化，先恢复
            if IsIconic(hwnd).as_bool() {
                ShowWindow(hwnd, SW_RESTORE);
            }
            
            // 激活窗口
            SetForegroundWindow(hwnd)
                .map_err(|e| Error::System(format!("激活窗口失败: {:?}", e)))?;
        }
        
        Ok(())
    }
    
    fn minimize_window(&self, title: &str) -> Result<()> {
        let hwnd = find_hwnd_by_title(title)?;
        
        unsafe {
            ShowWindow(hwnd, SW_MINIMIZE);
        }
        
        Ok(())
    }
    
    fn maximize_window(&self, title: &str) -> Result<()> {
        let hwnd = find_hwnd_by_title(title)?;
        
        unsafe {
            ShowWindow(hwnd, SW_MAXIMIZE);
        }
        
        Ok(())
    }
    
    fn close_window(&self, title: &str) -> Result<()> {
        let hwnd = find_hwnd_by_title(title)?;
        
        unsafe {
            PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0))
                .map_err(|e| Error::System(format!("关闭窗口失败: {:?}", e)))?;
        }
        
        Ok(())
    }
    
    fn get_screen_info(&self) -> Result<ScreenInfo> {
        unsafe {
            let width = GetSystemMetrics(SM_CXSCREEN) as u32;
            let height = GetSystemMetrics(SM_CYSCREEN) as u32;
            
            // 获取 DPI 缩放
            let hdc = GetDC(HWND(0));
            let dpi_x = GetDeviceCaps(hdc, LOGPIXELSX);
            let scale_factor = dpi_x as f64 / 96.0;
            ReleaseDC(HWND(0), hdc);
            
            Ok(ScreenInfo {
                width,
                height,
                scale_factor,
            })
        }
    }
}

// 回调函数：枚举窗口
unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = &mut *(lparam.0 as *mut Vec<WindowInfo>);
    
    // 只收集可见的顶层窗口
    if !IsWindowVisible(hwnd).as_bool() {
        return true.into();
    }
    
    // 获取窗口标题
    let mut title_buf = [0u16; 512];
    let title_len = GetWindowTextW(hwnd, &mut title_buf);
    
    if title_len == 0 {
        return true.into();
    }
    
    let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);
    
    // 获取进程名
    let mut process_id = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    let process_name = format!("process_{}", process_id); // 简化实现
    
    // 获取窗口位置
    let mut rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);
    
    let is_minimized = IsIconic(hwnd).as_bool();
    
    windows.push(WindowInfo {
        title,
        process_name,
        x: rect.left,
        y: rect.top,
        width: (rect.right - rect.left) as u32,
        height: (rect.bottom - rect.top) as u32,
        is_visible: true,
        is_minimized,
    });
    
    true.into()
}

// 辅助函数：根据标题查找窗口句柄
fn find_hwnd_by_title(title: &str) -> Result<HWND> {
    let title_wide: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
    
    unsafe {
        let hwnd = FindWindowW(PCWSTR::null(), PCWSTR(title_wide.as_ptr()));
        
        if hwnd.0 == 0 {
            return Err(Error::ElementNotFound(format!("未找到窗口: {}", title)));
        }
        
        Ok(hwnd)
    }
}
