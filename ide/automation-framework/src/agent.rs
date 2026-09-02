//! AI Agent 统一接口 - 封装所有自动化能力（修复版）

use crate::error::{Error, Result};
use crate::types::*;
use crate::platform::desktop_element::{DesktopElement, DesktopElementControl};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(feature = "browser")]
use crate::browser::BrowserAutomation;

#[cfg(feature = "system")]
use crate::system::SystemAutomation;

#[cfg(all(feature = "system", target_os = "windows"))]
use crate::platform::windows_ui_automation::WindowsUIAutomation;

#[cfg(all(feature = "system", target_os = "macos"))]
use crate::platform::macos_accessibility::MacOSAccessibility;

/// macOS 会静默丢弃未授权进程发出的合成事件：enigo 照样返回 Ok，于是每一次点击、
/// 每一次按键都"成功"了，但屏幕上什么都没发生。这是最坏的一种失败——调用方据此
/// 继续推进，越走越偏，还查不出原因。宁可在入口就报错，并且把补救路径写清楚。
#[cfg(all(feature = "system", target_os = "macos"))]
pub fn input_permission_granted() -> bool {
    // 每次真查，**不缓存**。
    //
    // 这里原来把「拿到过授权」用 static AtomicBool 缓存到进程死亡，依据是「授权一旦拿到
    // 就不会再撤销」——可 sidecar 随 IDE 常驻几个小时，用户中途在系统设置里取消勾选
    // （或 tccutil reset）之后，这个缓存让 ensure_input_permission 永远放行，macOS 则
    // 静默丢掉每一个合成事件：回执全是 ok、screen.info 还报 granted，模型照着「已点击 /
    // 已输入」继续推进——正是上面那段头注说要杜绝的最坏失败。IDE 侧同一个 API
    // （src-tauri permissions.rs）就是每次查的，这个调用本身很便宜。
    unsafe { accessibility_sys::AXIsProcessTrusted() }
}

#[cfg(all(feature = "system", not(target_os = "macos")))]
pub fn input_permission_granted() -> bool {
    true
}

#[cfg(feature = "system")]
fn ensure_input_permission() -> Result<()> {
    if input_permission_granted() {
        return Ok(());
    }
    Err(Error::System(
        "macOS 未授予辅助功能（Accessibility）权限，鼠标与键盘事件会被系统静默丢弃，\
         看起来成功其实没有任何效果。请到 系统设置 → 隐私与安全性 → 辅助功能 勾选本应用，\
         然后完全退出并重新打开它。在此之前请改用不需要该权限的手段：浏览器自动化、\
         Shell 命令、或直接读写文件。"
            .to_string(),
    ))
}

/// browser.start 实际做了什么。
///
/// 回执必须由它生成，不能由请求参数生成：已经有实例在跑时 headless / profile 一个都不
/// 会生效，而模型正是照着工具描述「再 start 一次、这次带 profile=session / headless=false」
/// 走到这里的。给它一句 ok，它就会以为自己换到了持久身份、以为窗口已经弹出来让用户登录。
#[cfg(feature = "browser")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserStartOutcome {
    /// 本次真的起了一只新的，身份就是请求的身份。
    Started(crate::browser::BrowserIdentity),
    /// 已经有一只活着的，本次调用**什么都没改**。带的是那只的真实身份。
    AlreadyRunning(crate::browser::BrowserIdentity),
    /// 命中的那只已经不应答了（用户手关窗口 / Chrome 崩了）：残壳收掉，按请求重起。
    Restarted(crate::browser::BrowserIdentity),
}

/// browser.close 实际做了什么。flushed 是这里唯一重要的事实——见 close_gracefully。
#[cfg(feature = "browser")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserCloseOutcome {
    /// 本来就没有实例。这不是失败，但也不是「关掉了」。
    NotRunning,
    /// 关了。identity 为 None 只发生在连锁都拿不到的时候（那时也一定 flushed=false）。
    Closed {
        identity: Option<crate::browser::BrowserIdentity>,
        flushed: bool,
    },
}

/// 命中已有实例时该怎么办。
#[cfg(feature = "browser")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartDecision {
    /// 手上没有实例，正常起。
    Launch,
    /// 有，而且还活着：**不重启**。换身份必须先 close——一只跑着的浏览器改不了自己的
    /// profile 和有头无头，悄悄替用户杀掉它更不是这条命令该做的事。
    Reuse(crate::browser::BrowserIdentity),
    /// 有，但已经不应答了：收掉重起。原来这一支和 Reuse 走同一句无动作的 Ok，于是
    /// 「浏览器死了 → 每条 browser.* 报连接错误 → 模型调 browser.start 想重启 → 拿到 ok
    /// → 再调 browser.* 还是连接错误」成了一个没有出口的环。
    RestartDead(crate::browser::BrowserIdentity),
}

/// running: None = 手上没实例；Some((身份, 是否还应答))。
#[cfg(feature = "browser")]
pub fn decide_start(
    running: Option<(crate::browser::BrowserIdentity, bool)>,
) -> StartDecision {
    match running {
        None => StartDecision::Launch,
        Some((identity, true)) => StartDecision::Reuse(identity),
        Some((identity, false)) => StartDecision::RestartDead(identity),
    }
}

pub struct Agent {
    #[cfg(feature = "browser")]
    browser: Option<Arc<Mutex<BrowserAutomation>>>,
    
    #[cfg(feature = "system")]
    system: Option<Arc<Mutex<SystemAutomation>>>,
    
    #[cfg(all(feature = "system", target_os = "windows"))]
    desktop: Option<WindowsUIAutomation>,
    
    #[cfg(all(feature = "system", target_os = "macos"))]
    desktop: Option<MacOSAccessibility>,
    
    runtime: Option<tokio::runtime::Runtime>,
}

impl Agent {
    pub fn new() -> Result<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| Error::Other(anyhow::anyhow!("Failed to create runtime: {}", e)))?;
        
        Ok(Self {
            #[cfg(feature = "browser")]
            browser: None,
            
            #[cfg(feature = "system")]
            system: None,
            
            #[cfg(all(feature = "system", target_os = "windows"))]
            desktop: None,
            
            #[cfg(all(feature = "system", target_os = "macos"))]
            desktop: None,
            
            runtime: Some(runtime),
        })
    }
    
    // ==================== 系统自动化 ====================
    
    #[cfg(feature = "system")]
    pub fn system_init(&mut self) -> Result<()> {
        // 所有注入型操作（鼠标/键盘）都以这里为唯一入口；剪贴板走 arboard、
        // screen.info 走 platform，都不经过这里，所以只在这里预检不会误伤只读能力。
        ensure_input_permission()?;
        self.system_init_readonly()
    }

    /// 只读能力用的初始化：**不查辅助功能权限**。
    ///
    /// 上面那句注释写着「只在这里预检不会误伤只读能力」，可截屏偏偏走的就是 system_init——
    /// 于是只授予了「屏幕录制」而没给「辅助功能」的机器上，智能体**根本看不到屏幕**，
    /// 而且报错还指向错误的那一格设置。截屏需要的是屏幕录制，和鼠标键盘注入是两回事。
    #[cfg(feature = "system")]
    pub fn system_init_readonly(&mut self) -> Result<()> {
        if self.system.is_none() {
            let sys = SystemAutomation::new()?;
            self.system = Some(Arc::new(Mutex::new(sys)));
        }
        Ok(())
    }
    
    /// 移动指针，并且**等它真的到位**再返回。
    ///
    /// CGEvent 是异步投递的。enigo 的 button() 会先读当前指针位置、再把点击事件发到
    /// 那个位置上——移动刚发出、指针还没落位时点击，事件就带着旧坐标发出去：不但点错
    /// 地方，那条 mouse-down 还会把指针拽回旧位置。整条链路一路返回 ok，屏幕上什么
    /// 都没发生。所以"移动"必须是同步语义，不能是"已发出移动请求"。
    #[cfg(feature = "system")]
    pub fn mouse_move(&mut self, x: i32, y: i32) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.move_mouse(x, y)?;
        // 轮询到位。上限 200ms：到不了就说明目标被系统夹住了（越界、被其他进程抢走），
        // 与其假装成功，不如让调用方拿到真实落点自己判断。
        let mut last = (x, y);
        for _ in 0..40 {
            match sys.mouse_location() {
                Ok((cx, cy)) if (cx - x).abs() <= 2 && (cy - y).abs() <= 2 => return Ok(()),
                Ok(p) => last = p,
                // 读不到当前位置就不能判定，按到位处理（老行为），别凭空拒绝执行。
                Err(_) => return Ok(()),
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        // 200ms 还没落位 = 没到位。原来这里直接 Ok(())，于是"指针根本没过去"被报成成功，
        // 紧接着的点击就点在旧位置上——而调用方拿到的是一路 ok，屏幕上什么都没发生。
        // 最常见的原因是坐标越界（多显示器、或者拿像素当点用，Retina 上正好差一倍）。
        // 宁可拒绝执行也不要假装成功：把真实落点和目标一起说出来，模型自己能判断。
        Err(Error::System(format!(
            "指针没能移动到 ({x}, {y})：200ms 后它停在 ({}, {})。本次不执行后续点击。\
最常见的原因是坐标超出了可用范围——先调 screen.info 看屏幕的**点**尺寸；\
如果坐标是从截图上量的，注意 Retina 截图是像素、这里收的是点，两者差一倍。",
            last.0, last.1
        )))
    }
    
    #[cfg(feature = "system")]
    pub fn mouse_location(&mut self) -> Result<(i32, i32)> {
        self.system_init()?;
        let sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.mouse_location()
    }

    /// 拍屏幕。`region` 为 None 就是整屏。返回 PNG 的 data URL。
    #[cfg(feature = "system")]
    pub fn screen_capture(&mut self, region: Option<(i32, i32, i32, i32)>) -> Result<(String, Option<String>)> {
        // 截屏是只读能力，不需要辅助功能权限（那是给鼠标键盘注入用的）。
        self.system_init_readonly()?;
        let sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.screen_capture(region)
    }

    #[cfg(feature = "system")]
    /// 按钮名解析。原来在 mouse_click / mouse_double_click 里各抄了一份，
    /// 而 `_` 分支一律落回 Left —— 模型写 "secondary" 或 "Right" 就会**静默点成左键**，
    /// 回执照样 ok。这里认全常见写法，且大小写无关。
    fn button_of(name: Option<&str>) -> MouseButton {
        match name.unwrap_or("left").trim().to_ascii_lowercase().as_str() {
            "right" | "secondary" | "context" => MouseButton::Right,
            "middle" | "wheel" | "center" => MouseButton::Middle,
            _ => MouseButton::Left,
        }
    }

    pub fn mouse_click(&mut self, button: Option<&str>) -> Result<()> {
        self.mouse_click_times(button, 1)
    }

    /// 连点 n 次。1=单击、2=双击、3=三连击（整段选中）。
    #[cfg(feature = "system")]
    pub fn mouse_click_times(&mut self, button: Option<&str>, times: u32) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.click_times(Self::button_of(button), times)
    }

    /// 按住不放 / 松开。拖滑块、框选、拖放到另一个应用都靠这两条；
    /// 底层 system.rs 里早就写好了，只是一直没有任何生产调用点，也没暴露给 RPC。
    #[cfg(feature = "system")]
    pub fn mouse_button_down(&mut self, button: Option<&str>) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.mouse_down(Self::button_of(button))
    }

    #[cfg(feature = "system")]
    pub fn mouse_button_up(&mut self, button: Option<&str>) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.mouse_up(Self::button_of(button))
    }

    /// 按住一个键若干毫秒再松开。
    ///
    /// 没有它就做不了「长按」：keyboard.down 之后没有任何合法的等待手段——computer
    /// 拿不到 sleep，而两次 RPC 之间的间隔完全不可控（这个 sidecar 还是串行的）。
    /// 空格长按跳跃、方向键长按连续滚动、长按呼出菜单，全卡在这一条上。
    #[cfg(feature = "system")]
    pub fn keyboard_hold(&mut self, key: &str, millis: u64) -> Result<()> {
        // 上限 10 秒：按住不放期间这条线程不干别的，写成 5 分钟等于把服务挂起。
        let ms = millis.clamp(1, 10_000);
        self.keyboard_down(key)?;
        std::thread::sleep(std::time::Duration::from_millis(ms));
        self.keyboard_up(key)
    }

    /// 按住若干修饰键，执行一个动作，再松开。
    ///
    /// Shift+点击（连选）、Cmd+点击（多选 / 新标签页打开）、Alt+拖拽（复制）这些在
    /// 真实界面里是基本操作，而此前**做不到**：只能拆成 keyboard.down → mouse.click →
    /// keyboard.up 三次 RPC，而这个 sidecar 单线程串行，三次之间隔着完整的往返，
    /// 中途任何一次排队都会让修饰键在点击之前就松开——失败得毫无痕迹。
    ///
    /// 松开一定要执行：动作失败时若不松开，修饰键会**一直按着**，后面每一次输入都被污染。
    #[cfg(feature = "system")]
    pub fn with_modifiers<T>(
        &mut self,
        keys: &[String],
        f: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        for k in keys {
            self.keyboard_down(k)?;
        }
        let out = f(self);
        // 逆序松开，且不因为前一个失败就跳过后面的。
        for k in keys.iter().rev() {
            let _ = self.keyboard_up(k);
        }
        out
    }
    
    #[cfg(feature = "system")]
    pub fn mouse_double_click(&mut self, button: Option<&str>) -> Result<()> {
        self.mouse_click_times(button, 2)
    }
    
    #[cfg(feature = "system")]
    pub fn mouse_drag(&mut self, from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.drag(from_x, from_y, to_x, to_y)
    }
    
    /// 滚动。参数与 `SystemAutomation::scroll` 一致：(delta_x, delta_y)。
    /// 此前只收一个 `amount` 且误传成了水平分量——调用方以为在垂直滚动，实际页面纹丝不动。
    #[cfg(feature = "system")]
    pub fn mouse_scroll(&mut self, delta_x: i32, delta_y: i32) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.scroll(delta_x, delta_y)
    }
    
    #[cfg(feature = "system")]
    pub fn keyboard_type(&mut self, text: &str) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.type_text(text)
    }
    
    #[cfg(feature = "system")]
    pub fn keyboard_press(&mut self, key: &str) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        let k = parse_key(key)?;
        sys.press_key(k)
    }
    
    #[cfg(feature = "system")]
    /// 按下但不松开 / 松开。成对使用就能表达「按住 Shift 再点几下」「按住 Cmd
    /// 拖拽」这类操作——底层一直有 key_down/key_up，只是从没暴露出去，于是路线 B
    /// 里这类操作根本没法表达。松开务必配对，否则修饰键会一直卡住。
    #[cfg(feature = "system")]
    pub fn keyboard_down(&mut self, key: &str) -> Result<()> {
        self.system_init()?;
        let k = parse_key(key)?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.key_down(k)
    }

    #[cfg(feature = "system")]
    pub fn keyboard_up(&mut self, key: &str) -> Result<()> {
        self.system_init()?;
        let k = parse_key(key)?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        sys.key_up(k)
    }

    pub fn keyboard_combo(&mut self, keys: Vec<&str>) -> Result<()> {
        self.system_init()?;
        let mut sys = self.system.as_ref()
            .ok_or_else(|| Error::System("系统自动化未初始化".to_string()))?
            .lock()
            .map_err(|e| Error::System(format!("Mutex 中毒: {}", e)))?;
        
        let key_vec: Result<Vec<Key>> = keys.iter().map(|k| parse_key(k)).collect();
        let parsed_keys = key_vec?;
        
        for key in &parsed_keys {
            sys.key_down(key.clone())?;
        }
        std::thread::sleep(Duration::from_millis(50));
        for key in parsed_keys.iter().rev() {
            sys.key_up(key.clone())?;
        }
        Ok(())
    }
    
    // ==================== 剪贴板 ====================
    
    #[cfg(feature = "system")]
    pub fn clipboard_get_text(&self) -> Result<String> {
        use arboard::Clipboard;
        let mut clipboard = Clipboard::new()
            .map_err(|e| Error::System(format!("剪贴板初始化失败: {}", e)))?;
        clipboard.get_text()
            .map_err(|e| Error::System(format!("读取剪贴板失败: {}", e)))
    }
    
    #[cfg(feature = "system")]
    pub fn clipboard_set_text(&self, text: &str) -> Result<()> {
        use arboard::Clipboard;
        let mut clipboard = Clipboard::new()
            .map_err(|e| Error::System(format!("剪贴板初始化失败: {}", e)))?;
        clipboard.set_text(text)
            .map_err(|e| Error::System(format!("写入剪贴板失败: {}", e)))
    }
    
    #[cfg(feature = "system")]
    pub fn copy_selection(&mut self) -> Result<()> {
        #[cfg(target_os = "macos")]
        self.keyboard_combo(vec!["cmd", "c"])?;
        #[cfg(not(target_os = "macos"))]
        self.keyboard_combo(vec!["ctrl", "c"])?;
        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    }
    
    #[cfg(feature = "system")]
    pub fn paste_from_clipboard(&mut self) -> Result<()> {
        #[cfg(target_os = "macos")]
        self.keyboard_combo(vec!["cmd", "v"])?;
        #[cfg(not(target_os = "macos"))]
        self.keyboard_combo(vec!["ctrl", "v"])?;
        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    }
    
    #[cfg(feature = "system")]
    pub fn quick_paste(&mut self, text: &str) -> Result<()> {
        // 粘贴要借道剪贴板，但剪贴板是用户的东西。原来这里直接覆盖且从不还原——
        // 用户复制了一段待用的内容，被自动化悄悄换掉，事后完全无从察觉。
        // 先存后还，还原前多等一会儿：目标应用是在处理 Cmd+V 时才去读剪贴板的，
        // 还得太快会把粘贴本身弄坏（paste_from_clipboard 已经等了 100ms）。
        let saved = self.clipboard_get_text().ok();
        self.clipboard_set_text(text)?;
        let pasted = self.paste_from_clipboard();
        if let Some(prev) = saved {
            std::thread::sleep(Duration::from_millis(250));
            // 还原失败不该盖掉粘贴本身的结果
            let _ = self.clipboard_set_text(&prev);
        }
        pasted
    }
    
    // ==================== 浏览器自动化 ====================

    /// 取浏览器锁：未启动 / Mutex 中毒都返回错误而不是 panic——否则一次中毒
    /// 会连带杀死整个常驻的 automation-server。
    #[cfg(feature = "browser")]
    fn browser_lock(&self) -> Result<std::sync::MutexGuard<'_, BrowserAutomation>> {
        self.browser
            .as_ref()
            .ok_or_else(|| Error::Browser("浏览器未启动".to_string()))?
            .lock()
            .map_err(|e| Error::Browser(format!("Mutex 中毒: {}", e)))
    }

    #[cfg(feature = "browser")]
    fn rt(&self) -> Result<&tokio::runtime::Runtime> {
        self.runtime
            .as_ref()
            .ok_or_else(|| Error::Other(anyhow::anyhow!("Runtime not initialized")))
    }

    #[cfg(feature = "browser")]
    pub fn browser_start(&mut self, headless: bool) -> Result<BrowserStartOutcome> {
        self.browser_start_with_profile(headless, crate::browser::BrowserProfile::Isolated)
    }

    /// 起浏览器，并指定用哪套身份。
    ///
    /// Isolated：每次全新，抓公开页面/测自己的站点用。
    /// Session：专用持久 profile，保留登录态——任务需要用户已登录的身份时用这个，
    /// 否则一定撞登录墙。它和用户自己的 Chrome 互不干扰，可以同时开着。
    ///
    /// 返回的是**发生了什么**，不是「请求了什么」：已经有实例在跑时这两个参数一个都不
    /// 生效，调用方必须能看出这一点（见 BrowserStartOutcome）。命中的实例已经死了则
    /// 收掉残壳重起——原来无论如何都是一句无动作的 Ok，那是死循环的入口。
    #[cfg(feature = "browser")]
    pub fn browser_start_with_profile(
        &mut self,
        headless: bool,
        profile: crate::browser::BrowserProfile,
    ) -> Result<BrowserStartOutcome> {
        let requested = crate::browser::BrowserIdentity::new(headless, profile);

        // 命中缓存实例时先连接级探活。用 is_alive 不用 is_connected：后者在没开过页面时
        // 恒 true，正好覆盖「起完就被用户关掉」这个最常见的死法。
        let running = match self.browser.as_ref() {
            None => None,
            Some(arc) => {
                let guard = arc
                    .lock()
                    .map_err(|e| Error::Browser(format!("Mutex 中毒: {}", e)))?;
                let identity = guard.identity();
                let alive = self
                    .runtime
                    .as_ref()
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Runtime not initialized")))?
                    .block_on(guard.is_alive());
                drop(guard);
                Some((identity, alive))
            }
        };

        let restarted = match decide_start(running) {
            StartDecision::Reuse(identity) => return Ok(BrowserStartOutcome::AlreadyRunning(identity)),
            StartDecision::RestartDead(_) => {
                // 残壳该收就收：不收的话新实例起来后旧进程还挂着，而且 self.browser
                // 会被直接覆盖，旧的 Arc 连 Drop 的时机都不确定。这里不看它的 flushed——
                // 一只已经不应答的浏览器谈不上落盘，能报的事实在下面那条 Restarted 里。
                let _ = self.close_current();
                true
            }
            StartDecision::Launch => false,
        };

        let runtime = self.runtime.as_ref()
            .ok_or_else(|| Error::Other(anyhow::anyhow!("Runtime not initialized")))?;
        
        let browser = runtime.block_on(async {
            {
                BrowserAutomation::new_with_profile(headless, profile).await
            }
        })?;
        
        std::thread::sleep(Duration::from_millis(1500));
        self.browser = Some(Arc::new(Mutex::new(browser)));
        Ok(if restarted {
            BrowserStartOutcome::Restarted(requested)
        } else {
            BrowserStartOutcome::Started(requested)
        })
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_goto(&self, url: &str) -> Result<()> {
        let mut browser = self.browser_lock()?;
        self.rt()?.block_on(browser.navigate(url))
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_click(&self, selector: &str) -> Result<()> {
        let mut browser = self.browser_lock()?;
        self.rt()?.block_on(browser.click_element(selector))
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_type(&self, selector: &str, text: &str) -> Result<()> {
        let mut browser = self.browser_lock()?;
        self.rt()?.block_on(browser.type_text(selector, text))
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_wait(&self, selector: &str, timeout_ms: u64) -> Result<()> {
        let mut browser = self.browser_lock()?;
        self.rt()?.block_on(browser.wait_for_element(selector, timeout_ms))
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_eval(&self, script: &str) -> Result<serde_json::Value> {
        let mut browser = self.browser_lock()?;
        self.rt()?.block_on(browser.execute_script(script))
    }
    
    #[cfg(feature = "browser")]
    /// 返回 PNG 字节——**不要再丢掉它**。
    ///
    /// 原来这里是 `browser.screenshot(path)?; Ok(())`：底层 browser.rs 明明
    /// `Ok(screenshot)` 返回了 PNG，这一层直接扔了，RPC 再回一句 `{"status":"ok"}`。
    /// 模型调 browser.screenshot 的唯一目的就是看一眼页面，结果什么都没看到，
    /// 却被告知成功了——它会照着「我看过了」继续往下推。
    /// 对照 screen.capture：那条一直是回 data_url 的。
    pub fn browser_screenshot(&self, path: Option<&str>) -> Result<Vec<u8>> {
        let mut browser = self.browser_lock()?;
        self.rt()?.block_on(browser.screenshot(path))
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_content(&self) -> Result<String> {
        let mut browser = self.browser_lock()?;
        self.rt()?.block_on(browser.get_content())
    }
    
    #[cfg(feature = "browser")]
    pub fn browser_close(&mut self) -> Result<BrowserCloseOutcome> {
        Ok(self.close_current())
    }

    /// 收掉当前实例，并如实回答**它有没有来得及落盘**。
    ///
    /// 直接 drop 会杀掉 Chrome，cookie 来不及写进 profile——持久 profile 下等于登录白登。
    /// 所以先走 CDP 的优雅关闭再放手；而「等到了它自己退出」这件事本身必须回给调用方，
    /// 否则强杀和正常关闭在回执里长得一模一样。
    #[cfg(feature = "browser")]
    fn close_current(&mut self) -> BrowserCloseOutcome {
        let Some(browser_arc) = self.browser.take() else { return BrowserCloseOutcome::NotRunning };
        let Some(runtime) = self.runtime.as_ref() else {
            // 没有 runtime 就发不出 Browser.close，只能让 Drop 强杀：落盘没保证。
            return BrowserCloseOutcome::Closed { identity: None, flushed: false };
        };
        let (identity, flushed) = runtime.block_on(async {
            match browser_arc.lock() {
                Ok(mut guard) => {
                    let identity = guard.identity();
                    let flushed = guard.close_gracefully().await.unwrap_or(false);
                    (Some(identity), flushed)
                }
                // 拿不到锁就发不出 Browser.close，Drop 直接强杀——如实报 false，别说成成功。
                Err(_) => (None, false),
            }
        });
        drop(browser_arc);
        BrowserCloseOutcome::Closed { identity, flushed }
    }
    
    // ==================== 桌面元素操作 ====================
    
    #[cfg(all(feature = "system", target_os = "windows"))]
    pub fn desktop_init(&mut self) -> Result<()> {
        if self.desktop.is_none() {
            self.desktop = Some(WindowsUIAutomation::new()?);
        }
        Ok(())
    }
    
    #[cfg(all(feature = "system", target_os = "macos"))]
    pub fn desktop_init(&mut self) -> Result<()> {
        if self.desktop.is_none() {
            self.desktop = Some(MacOSAccessibility::new()?);
        }
        Ok(())
    }
    
    #[cfg(feature = "system")]
    pub fn desktop_find_element(&mut self, window_title: &str, element_name: &str) -> Result<Option<DesktopElement>> {
        self.desktop_init()?;
        
        #[cfg(target_os = "windows")]
        {
            self.desktop.as_ref().unwrap().find_element_by_name(window_title, element_name)
        }
        
        #[cfg(target_os = "macos")]
        {
            self.desktop.as_ref().unwrap().find_element_by_name(window_title, element_name)
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            Err(Error::System("桌面元素操作仅支持 Windows 和 macOS".to_string()))
        }
    }
    
    #[cfg(feature = "system")]
    pub fn desktop_find_elements_by_type(&mut self, window_title: &str, element_type: &str) -> Result<Vec<DesktopElement>> {
        self.desktop_init()?;
        
        #[cfg(target_os = "windows")]
        {
            self.desktop.as_ref().unwrap().find_elements_by_type(window_title, element_type)
        }
        
        #[cfg(target_os = "macos")]
        {
            self.desktop.as_ref().unwrap().find_elements_by_type(window_title, element_type)
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            Err(Error::System("桌面元素操作仅支持 Windows 和 macOS".to_string()))
        }
    }
    
    #[cfg(feature = "system")]
    pub fn desktop_click_element(&self, element: &DesktopElement) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            self.desktop.as_ref()
                .ok_or_else(|| Error::System("桌面控制器未初始化".to_string()))?
                .click_element(element)
        }
        
        #[cfg(target_os = "macos")]
        {
            self.desktop.as_ref()
                .ok_or_else(|| Error::System("桌面控制器未初始化".to_string()))?
                .click_element(element)
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            Err(Error::System("桌面元素操作仅支持 Windows 和 macOS".to_string()))
        }
    }
    
    #[cfg(feature = "system")]
    pub fn desktop_type_into(&self, element: &DesktopElement, text: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            self.desktop.as_ref()
                .ok_or_else(|| Error::System("桌面控制器未初始化".to_string()))?
                .type_into_element(element, text)
        }
        
        #[cfg(target_os = "macos")]
        {
            self.desktop.as_ref()
                .ok_or_else(|| Error::System("桌面控制器未初始化".to_string()))?
                .type_into_element(element, text)
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            Err(Error::System("桌面元素操作仅支持 Windows 和 macOS".to_string()))
        }
    }
    
    #[cfg(feature = "system")]
    pub fn desktop_get_text(&self, element: &DesktopElement) -> Result<String> {
        #[cfg(target_os = "windows")]
        {
            self.desktop.as_ref()
                .ok_or_else(|| Error::System("桌面控制器未初始化".to_string()))?
                .get_element_text(element)
        }
        
        #[cfg(target_os = "macos")]
        {
            self.desktop.as_ref()
                .ok_or_else(|| Error::System("桌面控制器未初始化".to_string()))?
                .get_element_text(element)
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            Err(Error::System("桌面元素操作仅支持 Windows 和 macOS".to_string()))
        }
    }
}

/// 只给测试用的出口。修饰键映射错了在 Windows 上是**静默**按下 Win 键，
/// 三层回执还都说成功——这种东西必须有测试钉着。
#[cfg(test)]
pub fn parse_key_for_test(key: &str) -> Result<Key> {
    parse_key(key)
}

fn parse_key(key: &str) -> Result<Key> {
    match key.to_lowercase().as_str() {
        "return" | "enter" => Ok(Key::Return),
        "tab" => Ok(Key::Tab),
        "space" => Ok(Key::Space),
        "backspace" => Ok(Key::Backspace),
        "escape" | "esc" => Ok(Key::Escape),
        "spacebar" => Ok(Key::Space),
        // browser 工具那边教的写法是 ArrowDown / Escape / Meta 这一套（DOM 键名），
        // 而模型是从**同一份工具目录**里学到的，不认这套等于逼它猜。全都收下。
        "up" | "uparrow" | "arrowup" => Ok(Key::UpArrow),
        "down" | "downarrow" | "arrowdown" => Ok(Key::DownArrow),
        "left" | "leftarrow" | "arrowleft" => Ok(Key::LeftArrow),
        "right" | "rightarrow" | "arrowright" => Ok(Key::RightArrow),
        // **cmd 在 Windows 上必须是 Ctrl，不是 Win 键。**
        //
        // 原来一律映射成 Key::Meta，而 enigo 在 Windows 上把 Meta 编成 VK_LWIN。
        // 于是模型照工具描述里的示例发 ["cmd","s"] 想保存文件，Windows 上实际按下的是
        // Win+S——打开系统搜索框，紧接着 keyboard.type 把内容打进了搜索框而不是目标应用。
        // 而 RPC 回 {"status":"ok"} 还带 delivered_to，三层都在说成功，模型收不到任何
        // 失败信号，会把「已保存」当既成事实继续往下做。Win+R / Win+D / Win+E 同理。
        //
        // 所以 cmd/command/mod/primary 一律解释成「这个平台的主修饰键」，
        // 而 win/super 保留给真正的 Windows 键——想按它的人还有话可说。
        #[cfg(target_os = "macos")]
        "cmd" | "command" | "mod" | "primary" => Ok(Key::Meta),
        #[cfg(not(target_os = "macos"))]
        "cmd" | "command" | "mod" | "primary" => Ok(Key::Control),
        "meta" | "super" | "win" => Ok(Key::Meta),
        "ctrl" | "control" => Ok(Key::Control),
        "alt" | "option" => Ok(Key::Alt),
        "shift" => Ok(Key::Shift),
        // 下面这些原来一个都不认，于是「按 F5 刷新」「按 Delete 删掉」「翻页」
        // 这类再普通不过的操作，在路线 B 里直接报 Unknown key —— 底层
        // （enigo）明明全都支持，只是这张白名单没写。
        "delete" | "del" | "forwarddelete" => Ok(Key::Delete),
        "home" => Ok(Key::Home),
        "end" => Ok(Key::End),
        "pageup" | "pgup" => Ok(Key::PageUp),
        "pagedown" | "pgdn" => Ok(Key::PageDown),
        "f1" => Ok(Key::F1),
        "f2" => Ok(Key::F2),
        "f3" => Ok(Key::F3),
        "f4" => Ok(Key::F4),
        "f5" => Ok(Key::F5),
        "f6" => Ok(Key::F6),
        "f7" => Ok(Key::F7),
        "f8" => Ok(Key::F8),
        "f9" => Ok(Key::F9),
        "f10" => Ok(Key::F10),
        "f11" => Ok(Key::F11),
        "f12" => Ok(Key::F12),
        // **符号键的带名写法。** "+" / "-" / "=" 这些本身走下面单字符那一支就通了，
        // 但模型也会写 "plus" / "minus"（DOM 键名和各家文档都这么教），认死一种等于逼它猜。
        // 映射到 Character 是安全的：所有平台都走同一条 Unicode 路径。
        "plus" | "add" => Ok(Key::Character('+')),
        "minus" | "subtract" | "dash" => Ok(Key::Character('-')),
        "equal" | "equals" => Ok(Key::Character('=')),
        "comma" => Ok(Key::Character(',')),
        "period" | "dot" => Ok(Key::Character('.')),
        "slash" => Ok(Key::Character('/')),
        "backslash" => Ok(Key::Character('\\')),
        "semicolon" => Ok(Key::Character(';')),
        "quote" | "apostrophe" => Ok(Key::Character('\'')),
        "bracketleft" | "leftbracket" => Ok(Key::Character('[')),
        "bracketright" | "rightbracket" => Ok(Key::Character(']')),
        "backquote" | "backtick" | "grave" => Ok(Key::Character('`')),
        // **Insert / CapsLock / Numlock / ScrollLock / Pause / Print 没有加。**
        // 不是忘了：enigo 0.2 把它们全部 cfg 成 `windows` 或 `unix && !macos`，
        // macOS 上那些变体根本不存在。要支持就得给本 crate 的 Key 加 cfg 变体、
        // 再给 convert_key 加 cfg 分支，而这条路**在这台机器上编译不出来也验不了**
        //（cfg 分支不参与本平台编译，mac 全绿说明不了 Windows 能编）。
        // 等真要做 Windows 那一轮时连 `cargo xwin check` 一起做，别在这里凭印象加。
        s if s.len() == 1 => {
            let ch = s.chars().next().unwrap();
            Ok(Key::Character(ch))
        }
        _ => Err(Error::System(format!("Unknown key: {}", key))),
    }
}

#[cfg(test)]
mod input_permission_tests {
    /// 辅助功能授权**不能正向缓存到进程死亡**。
    ///
    /// sidecar 随 IDE 常驻几个小时；用户中途在系统设置里取消勾选（或 tccutil reset）后，
    /// 缓存让 ensure_input_permission 永远放行，macOS 静默丢弃全部合成键鼠事件——每层
    /// 回执都是 ok、screen.info 也报 granted，模型照着「已点击 / 已输入」继续推进。
    /// IDE 侧同一个 API（src-tauri permissions.rs）就是每次查的。
    /// 只扫生产代码里 macOS 那个变体的函数体，并先剥掉注释行——注释里会引用旧代码。
    #[test]
    fn accessibility_check_is_not_cached() {
        let src = include_str!("agent.rs");
        let prod = src.split("\n#[cfg(test)]").next().unwrap_or(src);
        let at = prod
            .find("pub fn input_permission_granted() -> bool {")
            .expect("input_permission_granted 不见了");
        let end = prod[at..].find("\n}").map(|e| at + e).unwrap_or(prod.len());
        let body: String = prod[at..end]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(body.contains("AXIsProcessTrusted()"), "macOS 分支必须每次真查 AXIsProcessTrusted: {body}");
        for bad in ["static ", "AtomicBool", "OnceLock", "OnceCell", "thread_local", "Mutex"] {
            assert!(
                !body.contains(bad),
                "又把授权结果缓存起来了（{bad}）——用户中途撤销后会永远放行: {body}"
            );
        }
    }
}

#[cfg(all(test, feature = "browser"))]
mod browser_start_decision_tests {
    use super::{decide_start, StartDecision};
    use crate::browser::{BrowserIdentity, BrowserProfile};

    const ID: BrowserIdentity = BrowserIdentity {
        headless: true,
        profile: BrowserProfile::Isolated,
    };

    #[test]
    fn nothing_running_means_launch() {
        assert_eq!(decide_start(None), StartDecision::Launch);
    }

    /// 活着的实例不重启：换身份得先 close。悄悄替用户杀掉一只正开着的浏览器
    /// 不是 browser.start 该做的事，但回执必须说清参数没生效（见 rpc 那组用例）。
    #[test]
    fn a_live_browser_is_reused_and_carries_its_own_identity_out() {
        assert_eq!(decide_start(Some((ID, true))), StartDecision::Reuse(ID));
    }

    /// 死实例必须被换掉。
    ///
    /// 修复前这一支和「还活着」走同一句无动作的 Ok，于是用户手关了有头窗口之后：
    /// 每条 browser.* 抛底层连接错误 → 模型调 browser.start 想重启 → 拿到 ok →
    /// 再调 browser.* 还是连接错误。这个环没有出口。
    #[test]
    fn a_dead_browser_is_restarted_not_silently_accepted() {
        assert_eq!(decide_start(Some((ID, false))), StartDecision::RestartDead(ID));
        assert_ne!(
            decide_start(Some((ID, false))),
            decide_start(Some((ID, true))),
            "死的和活的走了同一条路——那就是那个没有出口的环"
        );
    }

    /// 只保留生产代码：test 模块里会引用被改掉的旧写法，扫整份文件会把断言喂饱自己。
    fn production_source() -> &'static str {
        let src = include_str!("agent.rs");
        let cut = ["\n#[cfg(test)]", "\n#[cfg(all(test"]
            .iter()
            .filter_map(|m| src.find(m))
            .min()
            .unwrap_or(src.len());
        &src[..cut]
    }

    /// 探活必须是**连接级**的，而且真的接在启动路径上。
    ///
    /// is_connected 回答不了「浏览器还在不在」：current_page 为 None 时它恒 true，
    /// 而那正好是「起完就被关掉、一个页面都没开过」这个最常见的死法。
    #[test]
    fn the_start_path_probes_the_connection_before_reusing() {
        let prod = production_source();
        let pat = "pub fn browser_start_with_profile(";
        let at = prod.find(pat).expect("browser_start_with_profile 不见了");
        let end = prod[at..]
            .find("\n    #[cfg(feature = \"browser\")]\n    pub fn browser_goto")
            .map(|e| at + e)
            .expect("browser_goto 不在它后面了，切不出函数体");
        let body: String = prod[at..end]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !body.contains("if self.browser.is_some()"),
            "又回到「已经有实例就无条件回 Ok」了: {body}"
        );
        assert!(body.contains("is_alive()"), "启动路径没做连接级探活: {body}");
        assert!(!body.contains("is_connected()"), "拿页面级探活当浏览器存活判据了: {body}");
        assert!(body.contains("decide_start("), "复用/重启的判定没走同一个判据: {body}");
    }

    /// 关闭路径不许再把「有没有等到进程自己退出」吞掉。
    #[test]
    fn the_close_path_keeps_the_flush_result() {
        let prod = production_source();
        let at = prod.find("fn close_current(").expect("close_current 不见了");
        let end = prod[at..].find("\n    // ====").map(|e| at + e).unwrap_or(prod.len());
        let body: String = prod[at..end]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !body.contains("let _ = guard.close_gracefully()"),
            "又把落盘结果吞掉了: {body}"
        );
        assert!(body.contains("close_gracefully().await"), "{body}");
    }
}
