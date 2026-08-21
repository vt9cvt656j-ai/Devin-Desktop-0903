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

    /// 按应用名找进程号。activate_window 里那段遍历做的是同一件事。
unsafe fn pid_of_app(title: &str) -> Option<i32> {
    let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
    let apps: id = msg_send![workspace, runningApplications];
    let count: usize = msg_send![apps, count];
    for i in 0..count {
        let app: id = msg_send![apps, objectAtIndex: i];
        let app_name: id = msg_send![app, localizedName];
        if app_name == nil {
            continue;
        }
        let ptr: *const i8 = msg_send![app_name, UTF8String];
        let name = std::ffi::CStr::from_ptr(ptr).to_string_lossy();
        if name.contains(title) {
            let pid: i32 = msg_send![app, processIdentifier];
            return Some(pid);
        }
    }
    None
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
    /// 枚举**真窗口**，不是运行中的应用。
    ///
    /// 原来这里走 `NSWorkspace.runningApplications`——那是应用列表，不是窗口列表：
    /// x/y/width/height 全部硬写 0，还会混进 universalaccessd / talagentd 这类
    /// 根本没有窗口的后台守护进程（实测本机 99 条，几何全 0）。
    /// 而工具描述教模型「先用 window.list 找到窗口，再把它前置/按坐标点进去」——
    /// 拿到的坐标永远是 0,0,0×0，回执里一句说明都没有。
    ///
    /// 改用 CGWindowListCopyWindowInfo（core-graphics 已经是依赖，同文件 150 行
    /// 就在用 CGDisplay）：只取屏幕上真实存在的窗口层，带真实几何。
    fn enumerate_windows(&self) -> Result<Vec<WindowInfo>> {
        use core_foundation::base::{CFType, TCFType};
        use core_foundation::dictionary::CFDictionary;
        use core_foundation::number::CFNumber;
        use core_foundation::string::CFString;
        use core_graphics::window::{
            copy_window_info, kCGNullWindowID, kCGWindowListExcludeDesktopElements,
            kCGWindowListOptionOnScreenOnly,
        };

        let frontmost = unsafe {
            let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
            let app: id = msg_send![workspace, frontmostApplication];
            if app == nil {
                String::new()
            } else {
                let n: id = msg_send![app, localizedName];
                if n == nil {
                    String::new()
                } else {
                    let ptr: *const i8 = msg_send![n, UTF8String];
                    std::ffi::CStr::from_ptr(ptr).to_string_lossy().to_string()
                }
            }
        };

        let mut windows = Vec::new();
        let Some(list) = copy_window_info(
            kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
            kCGNullWindowID,
        ) else {
            return Ok(windows);
        };

        for item in list.iter() {
            let dict: CFDictionary<CFString, CFType> =
                unsafe { CFDictionary::wrap_under_get_rule(*item as *const _) };
            let s_of = |key: &str| -> String {
                dict.find(&CFString::new(key))
                    .and_then(|v| v.downcast::<CFString>())
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            };
            let n_of = |key: &str| -> f64 {
                dict.find(&CFString::new(key))
                    .and_then(|v| v.downcast::<CFNumber>())
                    .and_then(|v| v.to_f64())
                    .unwrap_or(0.0)
            };
            // layer != 0 是菜单栏 / Dock / 悬浮面板这类系统层，不是可操作的应用窗口。
            if n_of("kCGWindowLayer") != 0.0 {
                continue;
            }
            // kCGWindowBounds 是个嵌套字典；CFDictionary 没实现 ConcreteCFType，
            // 不能走 downcast，按引用重新包一层。
            let (x, y, w, h) = dict
                .find(&CFString::new("kCGWindowBounds"))
                .map(|v| unsafe {
                    let b: CFDictionary<CFString, CFType> =
                        CFDictionary::wrap_under_get_rule(v.as_CFTypeRef() as *const _);
                    let g = |k: &str| {
                        b.find(&CFString::new(k))
                            .and_then(|n| n.downcast::<CFNumber>())
                            .and_then(|n| n.to_f64())
                            .unwrap_or(0.0)
                    };
                    (g("X"), g("Y"), g("Width"), g("Height"))
                })
                .unwrap_or((0.0, 0.0, 0.0, 0.0));
            // 2×2 以下是阴影 / 输入法之类的附属层，点不到也没意义。
            if w < 2.0 || h < 2.0 {
                continue;
            }
            let owner = s_of("kCGWindowOwnerName");
            if owner.is_empty() {
                continue;
            }
            let title = s_of("kCGWindowName");
            windows.push(WindowInfo {
                title: if title.is_empty() { owner.clone() } else { title },
                process_name: owner.clone(),
                x: x as i32,
                y: y as i32,
                width: w.max(0.0) as u32,
                height: h.max(0.0) as u32,
                is_visible: true,
                is_frontmost: !frontmost.is_empty() && owner == frontmost,
                is_minimized: false,
            });
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
    
    fn minimize_window(&self, title: &str) -> Result<()> {
        // 以前这里是个只会返回 UnsupportedPlatform 的空实现，而 window.minimize
        // 就写在工具目录的 enum 里——模型照着调必然报错，等于清单在说谎。
        // AX 侧本来就有 AXMinimized 这个可写属性，实现在 macos_tree.rs。
        let pid = unsafe { pid_of_app(title) }
            .ok_or_else(|| Error::ElementNotFound(format!("没找到叫「{title}」的应用")))?;
        crate::platform::macos_tree::set_minimized(pid, true)
            .map(|_| ())
            .map_err(Error::System)
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

#[cfg(test)]
mod window_enumeration_tests {
    /// window.list 原来枚举的是 `NSWorkspace.runningApplications`——**应用**不是窗口：
    /// x/y/width/height 全部硬写 0，还混进 universalaccessd / talagentd 这类根本没有
    /// 窗口的后台守护进程（实测本机 99 条，几何全 0）。而工具描述教模型「先用
    /// window.list 找到窗口，再按坐标点进去」，拿到的坐标永远是 0,0,0×0。
    /// 换成 CGWindowListCopyWindowInfo 之后实测 2 个真窗口、几何全部真实。
    #[test]
    fn enumerates_real_windows_not_running_applications() {
        let src = include_str!("macos.rs");
        let at = src
            .find("fn enumerate_windows(&self) -> Result<Vec<WindowInfo>>")
            .expect("enumerate_windows 不见了");
        let end = src[at..].find("\n    fn ").map(|e| at + e).unwrap_or(src.len());
        let body: String = src[at..end]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            body.contains("copy_window_info"),
            "又回到枚举应用了 —— 拿到的几何会全是 0，模型按坐标点会点到屏幕外"
        );
        assert!(
            !body.contains("runningApplications"),
            "还在用 runningApplications 枚举「窗口」"
        );
        // 几何必须来自 kCGWindowBounds，不能再硬写 0。
        assert!(body.contains("kCGWindowBounds"), "没读真实窗口几何");
        assert!(
            !body.contains("x: 0,\n                    y: 0,"),
            "几何又被硬写成 0 了"
        );
        // 系统层和附属层要滤掉，否则列表里全是菜单栏和阴影。
        assert!(body.contains("kCGWindowLayer"), "没滤掉菜单栏 / Dock 这类系统层");
        assert!(body.contains("w < 2.0 || h < 2.0"), "没滤掉 2x2 以下的附属层");
    }
}
