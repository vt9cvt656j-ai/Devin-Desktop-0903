//! macOS 隐私授权的**真实**状态，以及"开关明明是开的却不管用"的解释。
//!
//! 起因：用户在 系统设置 → 隐私与安全性 里看到 Mr. Day One 的开关是打开的，
//! 自动化却一直报缺权限。查下来两条都成立，而且不矛盾：
//!
//!   - TCC 数据库里 `ai.devin.ide` 的辅助功能与录屏确实都是"允许"；
//!   - 但那条授权的**指定要求(designated requirement)**是 `cdhash H"…"` ——
//!     它把权限钉死在**某一次构建**上。本项目用 ad-hoc 签名
//!     (`tauri.conf.json` 里 `signingIdentity: "-"`)，每重新构建一次 cdhash 就变一次，
//!     于是授权对不上号而失效，**而系统设置里的开关仍然亮着**。
//!
//! 所以判断"有没有权限"绝不能看系统设置的界面，只能问系统 API。而当 API 说没有、
//! 界面却说有时，用户需要的不是"请去勾选"，是"请移除再重新添加"。这个模块负责把
//! 这件事讲清楚。

use serde::Serialize;
use std::sync::OnceLock;

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
}

#[cfg(target_os = "macos")]
#[repr(C)]
struct AEDesc {
    descriptor_type: u32,
    data_handle: *mut std::ffi::c_void,
}

#[cfg(target_os = "macos")]
#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    fn AECreateDesc(
        type_code: u32,
        data_ptr: *const std::ffi::c_void,
        data_size: isize,
        result: *mut AEDesc,
    ) -> i16;
    fn AEDisposeDesc(desc: *mut AEDesc) -> i16;
    fn AEDeterminePermissionToAutomateTarget(
        target: *const AEDesc,
        event_class: u32,
        event_id: u32,
        ask_user_if_needed: bool,
    ) -> i32;
}

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
}

/// 辅助功能（合成鼠标键盘、读 AX 树、驱动 System Events）是否真的可用。
///
/// 不缓存：用户可能刚在系统设置里改过。这个调用本身很便宜。
pub fn accessibility_granted() -> bool {
    #[cfg(target_os = "macos")]
    {
        unsafe { AXIsProcessTrusted() }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// 「自动化」权限：能不能给 System Events 发 Apple Event。
///
/// read_screen、ui_click、system.* 全都是通过 osascript 驱动 System Events 实现的，
/// 所以它们真正要的是 辅助功能 **加上** 这一项。而在此之前，全项目没有任何一处文案
/// 提到过「自动化」——用户被引去查辅助功能和录屏，唯独漏掉真正卡住的这一格。
pub fn apple_events_granted() -> bool {
    #[cfg(target_os = "macos")]
    unsafe {
        const TYPE_APPLICATION_BUNDLE_ID: u32 = u32::from_be_bytes(*b"bund");
        const TYPE_WILDCARD: u32 = u32::from_be_bytes(*b"****");
        const NO_ERR: i32 = 0;
        let bundle = b"com.apple.systemevents";
        let mut desc = AEDesc { descriptor_type: 0, data_handle: std::ptr::null_mut() };
        if AECreateDesc(
            TYPE_APPLICATION_BUNDLE_ID,
            bundle.as_ptr() as *const std::ffi::c_void,
            bundle.len() as isize,
            &mut desc,
        ) != 0
        {
            // 描述符都建不起来就别下结论了——报"有权限"，让真实调用去暴露问题，
            // 总好过凭一次失败的探测把用户引向一条错误的排查路径。
            return true;
        }
        // ask_user_if_needed = false：这里只是探测，不该在用户没主动要求时弹框。
        let status = AEDeterminePermissionToAutomateTarget(&desc, TYPE_WILDCARD, TYPE_WILDCARD, false);
        AEDisposeDesc(&mut desc);
        // -1743 = errAEEventNotPermitted（明确拒绝）。-600 之类是"System Events 没在跑"，
        // 那不是权限问题，不能算作拒绝。
        status == NO_ERR || status != -1743
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// 主动触发系统的辅助功能授权框。
///
/// 平时用的 `AXIsProcessTrusted()` 是**不提示**的变体：它从不弹框，也从不把本 App 写进
/// 系统设置的列表里。带 prompt 的这个变体会让 macOS 弹出标准授权框并把 App 加进列表——
/// 对"授权因为重新构建而失效"这种情况尤其有用，用户不必再手工移除重加。
///
/// 只应由用户的明确动作触发（点按钮），不要在后台自动调用。
pub fn prompt_for_accessibility() -> bool {
    #[cfg(target_os = "macos")]
    unsafe {
        use objc2_foundation::{NSDictionary, NSNumber, NSString};
        let key = NSString::from_str("AXTrustedCheckOptionPrompt");
        let value = NSNumber::new_bool(true);
        let options = NSDictionary::from_slices(&[&*key], &[&*value as &objc2::runtime::AnyObject]);
        AXIsProcessTrustedWithOptions(
            objc2::rc::Retained::as_ptr(&options) as *const std::ffi::c_void
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// 屏幕录制（截屏、OCR）是否可用。preflight 不会弹授权框。
pub fn screen_recording_granted() -> bool {
    #[cfg(target_os = "macos")]
    {
        unsafe { CGPreflightScreenCaptureAccess() }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// 本次构建的身份是不是"钉死在这一次构建上"。
///
/// 判据直接取自 `codesign -d -r-` 输出的指定要求：以 `cdhash ` 开头就说明系统认的是
/// 这一份二进制的哈希，换一次构建就换一个身份，此前的授权随之作废。证书签名的 App
/// 拿到的是 `identifier "…" and anchor apple generic and certificate leaf[subject.OU] = …`，
/// 跨构建稳定。
///
/// 结果缓存：要 fork 一个 codesign 进程，而同一次运行里这个答案不会变。
pub fn identity_pinned_to_build() -> bool {
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(|| {
        #[cfg(target_os = "macos")]
        {
            let Ok(exe) = std::env::current_exe() else {
                return false;
            };
            let out = std::process::Command::new("/usr/bin/codesign")
                .args(["-d", "-r-", "--"])
                .arg(&exe)
                .output();
            let Ok(out) = out else { return false };
            // codesign 把签名信息写到 stderr，指定要求写到 stdout，版本之间还不太一致，
            // 两边都看一遍最稳。
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            text.lines().any(|line| {
                let l = line.trim_start_matches('#').trim();
                l.strip_prefix("designated =>")
                    .map(|req| req.trim().starts_with("cdhash "))
                    .unwrap_or(false)
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    })
}

#[derive(Debug, Serialize)]
pub struct PermissionStatus {
    /// 辅助功能：合成鼠标键盘、读 AX 树、驱动 System Events 都靠它。
    pub accessibility: bool,
    /// 屏幕录制：截屏与 OCR 靠它。
    pub screen_recording: bool,
    /// 自动化(Apple Events)：驱动 System Events 靠它。read_screen / ui_click / system.* 都要。
    pub apple_events: bool,
    /// 身份是否钉死在本次构建（ad-hoc 签名的必然结果）。
    pub identity_pinned_to_build: bool,
    /// 直接说给用户听的一段话。缺什么、为什么系统设置里看着是开的、下一步怎么做。
    pub advice: String,
    /// 点一下就能跳到对应的系统设置面板。
    pub settings_url: String,
}

/// 打开系统设置里对应面板的 URL。缺辅助功能就直接落到辅助功能那一页。
fn settings_url_for(accessibility: bool, apple_events: bool) -> &'static str {
    if !accessibility {
        "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
    } else if !apple_events {
        "x-apple.systempreferences:com.apple.preference.security?Privacy_Automation"
    } else {
        "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
    }
}

/// 组织给用户看的说明。
///
/// 这里的分寸很重要：授权已失效但开关还亮着时，说"请去勾选"是错的——用户会看到它
/// 本来就勾着，然后认定是程序在骗人。必须直说"那个勾是上一次构建留下的"。
pub fn advice_text(
    accessibility: bool,
    screen_recording: bool,
    apple_events: bool,
    pinned: bool,
) -> String {
    if accessibility && screen_recording && apple_events {
        return String::new();
    }
    // 「自动化」这一项此前从没被任何文案提过，而 read_screen / ui_click / system.* 恰恰
    // 卡在它上面——漏掉它，用户会把辅助功能和录屏来回检查一遍还是找不出毛病。
    let mut names: Vec<&str> = Vec::new();
    if !accessibility {
        names.push("辅助功能");
    }
    if !apple_events {
        names.push("自动化");
    }
    if !screen_recording {
        names.push("屏幕录制");
    }
    let missing = names.join("、");
    if pinned {
        // 分段写。这段话会原样进对话框，糊成一坨的话用户根本读不完，
        // 而这里每一句都是他必须知道的——尤其"开关看着是开的但已经失效"那句。
        format!(
            "缺少「{missing}」权限。\n\n\
             系统设置里 Mr. Day One 的开关**看起来多半是打开的**，但它已经失效了：\
             授权绑定的是上一次构建的代码签名，而本地构建每编译一次就换一个签名。\
             重新勾一次没有用。\n\n\
             最省事的办法是点下面那个按钮，让 macOS 自己弹授权框——它会把当前这一版\
             加进列表。\n\n\
             如果弹框之后还是不行，就手工来一次：\n\
             1. 打开 系统设置 → 隐私与安全性 → {missing}\n\
             2. 选中 Mr. Day One，点「−」移除\n\
             3. 点「+」重新添加 /Applications/Mr. Day One.app\n\
             4. 完全退出 Mr. Day One 再重新打开\n\n\
             想彻底不再出这个问题，需要给 App 换成固定的代码签名身份\
             （Apple 开发者证书），换过之后每次更新都不会再掉权限。"
        )
    } else {
        format!(
            "缺少「{missing}」权限。\n\n\
             到 系统设置 → 隐私与安全性 → {missing} 里打开 Mr. Day One 的开关，\
             然后完全退出 App 再重新打开——授权要重启后才生效。"
        )
    }
}

#[tauri::command]
pub fn permission_status() -> PermissionStatus {
    let accessibility = accessibility_granted();
    let screen_recording = screen_recording_granted();
    let apple_events = apple_events_granted();
    let pinned = identity_pinned_to_build();
    PermissionStatus {
        accessibility,
        screen_recording,
        apple_events,
        identity_pinned_to_build: pinned,
        advice: advice_text(accessibility, screen_recording, apple_events, pinned),
        settings_url: settings_url_for(accessibility, apple_events).to_string(),
    }
}

/// 只针对**这次失败的操作真正需要的那几项**给建议。
///
/// 为什么不能直接用 permission_status().advice：那份是全量的，只要三项里缺任何一项就出文案。
/// 而 read_screen / ui_click 根本不需要屏幕录制。于是一个「ref 已过期（位置从 100,200
/// 移到 300,400）」这种完全正常的界面变动，会被贴上一整段「缺少『屏幕录制』权限，去系统
/// 设置里移除再重加、然后完全退出重开」。用户照着做一遍，问题当然还在——真因是要重读一次屏。
/// 这是这个仓库最忌讳的形态：一个**权威、具体、可执行、而且完全错误**的指示。
///
/// 做法是把不相关的那几项当成"已授权"喂进去，让 advice_text 自己判空。
#[tauri::command]
pub fn permission_advice(scope: String) -> String {
    let accessibility = accessibility_granted();
    let screen_recording = screen_recording_granted();
    let apple_events = apple_events_granted();
    let pinned = identity_pinned_to_build();
    // 每一项列出**谁真的会卡在它上面**，改的时候照着这个改，别凭印象。
    let (need_ax, need_capture, need_events) = match scope.as_str() {
        // 截屏、录屏：只卡屏幕录制。
        "capture" => (false, true, false),
        // 合成鼠标键盘：只卡辅助功能。没有 AppleEvents 也照样能注入事件。
        "input" => (true, false, false),
        // 读屏 / 按 ref 操作 / system.*：卡辅助功能（AX 树）；JXA 那条兜底路还要 AppleEvents。
        _ => (true, false, true),
    };
    advice_text(
        accessibility || !need_ax,
        screen_recording || !need_capture,
        apple_events || !need_events,
        pinned,
    )
}

/// 弹出系统的辅助功能授权框，并返回弹框之后的状态。
///
/// 这是"授权因重新构建而失效"的最省事出路：带 prompt 的检查会让 macOS 自己把本 App
/// 写进系统设置的列表，用户不用手工移除再重加。只在用户点击时调用。
#[tauri::command]
pub fn request_accessibility() -> PermissionStatus {
    let _ = prompt_for_accessibility();
    permission_status()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 都齐全时不啰嗦() {
        assert_eq!(advice_text(true, true, true, true), "");
        assert_eq!(advice_text(true, true, true, false), "");
    }

    #[test]
    fn 授权失效时不能叫用户去勾一个已经勾着的开关() {
        let text = advice_text(false, true, true, true);
        assert!(text.contains("移除"), "必须告诉用户移除再重新添加：{text}");
        assert!(text.contains("重新添加"), "{text}");
        // 这是关键：不能只说"请打开开关"——用户看到的就是它已经开着。
        // 钉性质不钉措辞：必须同时讲清"看着是开的"和"其实已经失效"，改文案不该让这条误红。
        assert!(text.contains("看起来"), "必须点出开关看着是开的：{text}");
        assert!(text.contains("失效"), "必须点明它已经失效：{text}");
        assert!(text.contains("辅助功能"), "要说清缺的是哪一个：{text}");
        assert!(!text.contains("屏幕录制"), "不缺的权限不要拉进来：{text}");
    }

    #[test]
    fn 身份稳定时给的是普通指引() {
        let text = advice_text(false, true, true, false);
        assert!(!text.contains("移除"), "身份稳定就不该让用户做移除重加：{text}");
        assert!(text.contains("打开"), "{text}");
    }

    #[test]
    fn 自动化权限不再被漏掉() {
        // read_screen / ui_click / system.* 真正卡在这一项上，而此前全项目没有一处提过它。
        let text = advice_text(true, true, false, true);
        assert!(text.contains("自动化"), "缺自动化权限必须点名：{text}");
        assert!(!text.contains("辅助功能"), "没缺的别拉进来：{text}");
        assert!(settings_url_for(true, false).ends_with("Privacy_Automation"));
    }

    #[test]
    fn 缺两个时两个都点名() {
        let text = advice_text(false, false, true, true);
        assert!(text.contains("辅助功能") && text.contains("屏幕录制"), "{text}");
    }

    #[test]
    fn 面板链接指向真正缺的那一项() {
        assert!(settings_url_for(false, true).ends_with("Privacy_Accessibility"));
        assert!(settings_url_for(true, true).ends_with("Privacy_ScreenCapture"));
    }
}
