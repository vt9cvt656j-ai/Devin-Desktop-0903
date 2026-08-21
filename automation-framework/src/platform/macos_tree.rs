//! 前台应用的可访问性树快照——**原生 AX API 版本**。
//!
//! 为什么要有这个：原来读屏走的是 JXA / System Events，而 JXA 每访问一次属性就是
//! 一次独立的 Apple Event 往返。实测（本机 M 芯片、Chrome 一个真实窗口）：
//!   · entireContents() 拿到 6701 个元素，2.0 秒
//!   · 只读其中 500 个元素的 5 个属性：**95 秒**，平均每个元素 190 毫秒
//! 而读屏的超时上限是 6 秒——也就是说在任何真实应用上它**必然超时**。用户看到的
//! 「read_screen 一直超时、智能体在那儿发呆」就是这么来的。
//!
//! 换成原生 AX：AXUIElementCopyAttributeValue 是进程内的 C 调用，同一棵树几十毫秒。
//!
//! 两个坑写在这里，省得下次再踩：
//!   1. AXPosition / AXSize 返回的是 **AXValueRef**，不是 CFDictionary。必须用
//!      AXValueGetValue 按 CGPoint / CGSize 取出来，downcast 成字典会静默失败。
//!   2. AX 调用会阻塞在没响应的应用上。AXUIElementSetMessagingTimeout 必须设，
//!      否则一个卡死的窗口能把整次读取拖死。

#![cfg(target_os = "macos")]

use core_foundation::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
use core_foundation::base::{CFRelease, CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::string::{CFString, CFStringRef};
use std::ptr;

#[repr(C)]
struct __AXUIElement(std::ffi::c_void);
type AXUIElementRef = *const __AXUIElement;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGPoint {
    x: f64,
    y: f64,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGSize {
    width: f64,
    height: f64,
}

const K_AXVALUE_CGPOINT: u32 = 1;
const K_AXVALUE_CGSIZE: u32 = 2;

extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn AXUIElementSetMessagingTimeout(element: AXUIElementRef, timeout: f32) -> i32;
    fn AXValueGetValue(value: CFTypeRef, the_type: u32, out: *mut std::ffi::c_void) -> bool;
}

/// 一个可访问性元素。字段名和 JXA 那版保持一致，下游不用改。
#[derive(Debug, Clone, serde::Serialize)]
pub struct AxNode {
    pub role: String,
    pub text: String,
    pub value: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub enabled: bool,
}

unsafe fn copy_attr(el: AXUIElementRef, name: &str) -> Option<CFType> {
    let key = CFString::new(name);
    let mut out: CFTypeRef = ptr::null();
    if AXUIElementCopyAttributeValue(el, key.as_concrete_TypeRef(), &mut out) == 0 && !out.is_null()
    {
        Some(CFType::wrap_under_create_rule(out))
    } else {
        None
    }
}

unsafe fn attr_string(el: AXUIElementRef, name: &str) -> Option<String> {
    let v = copy_attr(el, name)?;
    v.downcast::<CFString>().map(|s| s.to_string())
}

unsafe fn attr_bool(el: AXUIElementRef, name: &str) -> Option<bool> {
    let v = copy_attr(el, name)?;
    v.downcast::<CFBoolean>().map(|b| b.into())
}

unsafe fn attr_point(el: AXUIElementRef, name: &str) -> Option<(i32, i32)> {
    let v = copy_attr(el, name)?;
    let mut p = CGPoint::default();
    if AXValueGetValue(
        v.as_CFTypeRef(),
        K_AXVALUE_CGPOINT,
        &mut p as *mut _ as *mut std::ffi::c_void,
    ) {
        Some((p.x as i32, p.y as i32))
    } else {
        None
    }
}

unsafe fn attr_size(el: AXUIElementRef, name: &str) -> Option<(i32, i32)> {
    let v = copy_attr(el, name)?;
    let mut s = CGSize::default();
    if AXValueGetValue(
        v.as_CFTypeRef(),
        K_AXVALUE_CGSIZE,
        &mut s as *mut _ as *mut std::ffi::c_void,
    ) {
        Some((s.width as i32, s.height as i32))
    } else {
        None
    }
}

/// 元素的可读文本。AXTitle 最准，没有就退到 AXDescription，再退到 AXValue 的字符串形态。
unsafe fn node_text(el: AXUIElementRef) -> String {
    for k in ["AXTitle", "AXDescription", "AXLabel"] {
        if let Some(s) = attr_string(el, k) {
            if !s.trim().is_empty() {
                return s.chars().take(120).collect();
            }
        }
    }
    String::new()
}

unsafe fn node_value(el: AXUIElementRef) -> String {
    match copy_attr(el, "AXValue") {
        Some(v) => {
            if let Some(s) = v.clone().downcast::<CFString>() {
                s.to_string().chars().take(140).collect()
            } else if let Some(b) = v.downcast::<CFBoolean>() {
                let on: bool = b.into();
                (if on { "true" } else { "false" }).to_string()
            } else {
                String::new()
            }
        }
        None => String::new(),
    }
}

/// 深度优先遍历。cap 是硬上限；深度也限，防止病态深树。
///
/// 子元素**就地递归**，不把引用带出数组的生命周期：CFArrayGetValueAtIndex 返回的是
/// 借用引用，数组一释放它们就悬空。要带出去就得逐个 CFRetain，而那既麻烦又容易漏放。
unsafe fn walk(
    el: AXUIElementRef,
    depth: usize,
    cap: usize,
    out: &mut Vec<AxNode>,
    handles: &mut Vec<(u32, AXUIElementRef, AxNode)>,
) {
    if out.len() >= cap || depth > 24 {
        return;
    }
    let role = attr_string(el, "AXRole").unwrap_or_default();
    let (x, y) = attr_point(el, "AXPosition").unwrap_or((0, 0));
    let (w, h) = attr_size(el, "AXSize").unwrap_or((0, 0));
    // 尺寸退化的元素点不到，收进来只会挤掉真能点的（末尾有 cap 截断）。
    if w >= 2 && h >= 2 && !role.is_empty() {
        let node = AxNode {
            role: role.trim_start_matches("AX").to_string(),
            text: node_text(el),
            value: node_value(el),
            x,
            y,
            w,
            h,
            enabled: attr_bool(el, "AXEnabled").unwrap_or(true),
        };
        // ref 就是它在这一份结果里的序号。句柄一起留下来，点的时候直接用，
        // 不必重跑一遍枚举——那正是老路又慢又会下标错位的原因。
        //
        // **必须在这里 retain**：CFArrayGetValueAtIndex 给的是借用引用，出了这一层
        // 数组的作用域（下面那句 CFRelease）它就是野指针。等遍历结束再统一 retain
        // 会 retain 到已释放的对象上——直接 SIGTRAP（踩过一次）。
        core_foundation::base::CFRetain(el as CFTypeRef);
        let id = out.len() as u32 + 1;
        handles.push((id, el, node.clone()));
        out.push(node);
    }
    let key = CFString::new("AXChildren");
    let mut raw: CFTypeRef = ptr::null();
    if AXUIElementCopyAttributeValue(el, key.as_concrete_TypeRef(), &mut raw) == 0 && !raw.is_null()
    {
        let arr = raw as CFArrayRef;
        let n = CFArrayGetCount(arr);
        for i in 0..n {
            let c = CFArrayGetValueAtIndex(arr, i) as AXUIElementRef;
            if !c.is_null() {
                walk(c, depth + 1, cap, out, handles);
            }
            if out.len() >= cap {
                break;
            }
        }
        CFRelease(raw);
    }
}

/// 当前前台应用的进程号。调用方不必先跑一次 window.list 再把 pid 传回来——
/// 少一次往返，而「读前台」正是这个方法九成的用法。
pub fn frontmost_pid() -> Option<i32> {
    use cocoa::base::{id, nil};
    use objc::{class, msg_send, sel, sel_impl};
    unsafe {
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let app: id = msg_send![workspace, frontmostApplication];
        if app == nil {
            return None;
        }
        let pid: i32 = msg_send![app, processIdentifier];
        if pid > 0 { Some(pid) } else { None }
    }
}

/// 拍一份前台应用的可访问性树。
///
/// 只走用户真看得见、真点得到的窗口：跳过最小化的和尺寸退化的（浏览器会挂 1x1 的
/// 隐藏工具窗），主窗口排最前——被 cap 截断时先留它。
pub fn snapshot(pid: i32, cap: usize) -> Vec<AxNode> {
    let mut out = Vec::new();
    let mut handles: Vec<(u32, AXUIElementRef, AxNode)> = Vec::new();
    unsafe {
        let app = AXUIElementCreateApplication(pid);
        if app.is_null() {
            return out;
        }
        // 卡死的应用不能把整次读取拖死。
        AXUIElementSetMessagingTimeout(app, 2.0);

        // 窗口引用同样是数组的借用引用，所以整段筛选 + 遍历都在数组存活期内做完。
        let key = CFString::new("AXWindows");
        let mut raw: CFTypeRef = ptr::null();
        if AXUIElementCopyAttributeValue(app, key.as_concrete_TypeRef(), &mut raw) == 0
            && !raw.is_null()
        {
            let arr = raw as CFArrayRef;
            let n = CFArrayGetCount(arr);
            // 先按「主窗口优先」排出下标顺序，再按这个顺序遍历。
            let mut order: Vec<isize> = Vec::new();
            for i in 0..n {
                let w = CFArrayGetValueAtIndex(arr, i) as AXUIElementRef;
                if w.is_null() {
                    continue;
                }
                let (ww, wh) = attr_size(w, "AXSize").unwrap_or((0, 0));
                if ww < 40 || wh < 40 {
                    continue;
                }
                if attr_bool(w, "AXMinimized").unwrap_or(false) {
                    continue;
                }
                if attr_bool(w, "AXMain").unwrap_or(false) {
                    order.insert(0, i);
                } else {
                    order.push(i);
                }
            }
            // 一个都没留下时不要直接交空。
            //
            // 有的应用（托盘类的、比如 Clash Verge）会把自己**看得见的**窗口报成
            // AXMinimized=true——JXA 那边读到的也是 true，所以这不是读错，是这个
            // 应用就这么报的。但据此返回空，下游只有一条解释路径，会说成
            // 「这个应用不暴露可访问性树」，模型于是断定它没法自动化。
            // 宁可退回去读所有尺寸够大的窗口：坐标可能不准，但至少是真的有东西，
            // 而「点了没反应」比「这应用没法自动化」好排查得多。
            if order.is_empty() {
                for i in 0..n {
                    let w = CFArrayGetValueAtIndex(arr, i) as AXUIElementRef;
                    if w.is_null() {
                        continue;
                    }
                    let (ww, wh) = attr_size(w, "AXSize").unwrap_or((0, 0));
                    if ww >= 40 && wh >= 40 {
                        order.push(i);
                    }
                }
            }
            for i in order.into_iter().take(5) {
                let w = CFArrayGetValueAtIndex(arr, i) as AXUIElementRef;
                if !w.is_null() {
                    walk(w, 0, cap, &mut out, &mut handles);
                }
                if out.len() >= cap {
                    break;
                }
            }
            CFRelease(raw);
        }
        CFRelease(app as CFTypeRef);
    }
    store_handles(pid, handles);
    out
}

#[cfg(test)]
mod tests {
    /// 手动基准：对着一个真实运行的应用测一次，和 JXA 那条路对照。
    /// 默认 ignore——它依赖本机有那个应用在跑，且需要辅助功能权限。
    ///   cargo test --all-features bench_snapshot -- --ignored --nocapture
    #[test]
    #[ignore]
    fn bench_snapshot() {
        let pid: i32 = std::env::var("AX_PID").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        assert!(pid > 0, "用 AX_PID=<进程号> 指定目标");
        let t = std::time::Instant::now();
        let nodes = super::snapshot(pid, 500);
        let ms = t.elapsed().as_millis();
        println!("原生 AX：{} 个元素，{} 毫秒", nodes.len(), ms);
        let mut roles: std::collections::BTreeMap<&str, usize> = Default::default();
        for n in &nodes { *roles.entry(n.role.as_str()).or_default() += 1; }
        println!("角色分布：{:?}", roles);
        println!("有没有 WebArea：{}", nodes.iter().any(|n| n.role == "WebArea"));
        for n in nodes.iter().filter(|n| n.role == "Link" || n.role == "Button").take(4) {
            println!("  {} @{},{}  {}", n.role, n.x, n.y, n.text);
        }
    }
}

// ── 元素句柄表：读和点必须共用同一批句柄 ──────────────────────────────────
//
// 原来那条 JXA 路是「点的时候重跑一遍枚举，再按下标取第 N 个」。这有两个后果：
// 一是点一次和读一次一样贵（同样几十秒），二是两次枚举之间界面只要动过，下标就错位。
// 它靠元素签名比对来兜底，所以不会点错，但会直接失败。
//
// 原生这条把句柄本身留下来（CFRetain），点的时候直接对着那个元素发动作——不用重枚举，
// 快得多，也不存在下标错位。签名仍然存：界面变了要能说出「这个 ref 过期了，重读」，
// 而不是闷头点一个已经变成别的东西的位置。
use std::collections::HashMap;
use std::sync::Mutex;

extern "C" {
    fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> i32;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> i32;
}

struct Held {
    el: AXUIElementRef,
    sig: AxNode,
    pid: i32,
}
// AXUIElementRef 是不可变的 CF 对象，跨线程持有是安全的；这里用互斥量保证表本身的独占。
unsafe impl Send for Held {}

static HANDLES: Mutex<Option<HashMap<u32, Held>>> = Mutex::new(None);

/// 句柄在 walk 里就已经 retain 过了（那时数组还活着），这里只负责换表和放掉旧的。
fn store_handles(pid: i32, items: Vec<(u32, AXUIElementRef, AxNode)>) {
    let mut map = HashMap::new();
    for (id, el, sig) in items {
        map.insert(id, Held { el, sig, pid });
    }
    if let Ok(mut g) = HANDLES.lock() {
        if let Some(old) = g.take() {
            for (_, h) in old {
                unsafe { CFRelease(h.el as CFTypeRef) };
            }
        }
        *g = Some(map);
    }
}

/// 签名变了就说它变了，不要闷头去点。
fn signature_drift(a: &AxNode, b: &AxNode) -> Option<String> {
    if a.role != b.role {
        return Some(format!("role {} → {}", a.role, b.role));
    }
    if a.text != b.text {
        return Some("文案变了".into());
    }
    if (a.x - b.x).abs() > 4 || (a.y - b.y).abs() > 4 {
        return Some(format!("位置从 {},{} 移到 {},{}", a.x, a.y, b.x, b.y));
    }
    None
}

/// 对一个 ref 执行 AX 动作。press / focus / set_value 三种。
pub fn act(reference: u32, action: &str, value: Option<&str>) -> Result<serde_json::Value, String> {
    let g = HANDLES.lock().map_err(|_| "句柄表不可用".to_string())?;
    let map = g.as_ref().ok_or("还没有读过屏；先调 screen.elements")?;
    let held = map
        .get(&reference)
        .ok_or_else(|| format!("ref {reference} 不在最近一次读屏结果里；重新读一次"))?;

    unsafe {
        // 先确认它还是原来那个东西。
        let mut now = Vec::new();
        walk_one(held.el, &mut now);
        let live = now
            .into_iter()
            .next()
            .ok_or("这个元素已经不存在了；重新读一次屏")?;
        if let Some(d) = signature_drift(&held.sig, &live) {
            return Err(format!(
                "ref {reference} 已经过期（{d}）——界面变过了。重新 screen.elements 再操作。"
            ));
        }

        match action {
            "press" => {
                for name in ["AXPress", "AXOpen", "AXPick", "AXConfirm"] {
                    let a = CFString::new(name);
                    if AXUIElementPerformAction(held.el, a.as_concrete_TypeRef()) == 0 {
                        return Ok(serde_json::json!({
                            "ok": true, "action": "press", "used": name,
                            "role": live.role, "text": live.text,
                        }));
                    }
                }
                Err(format!(
                    "「{}」不响应 press/open/pick（role={}）",
                    live.text, live.role
                ))
            }
            "focus" => {
                let attr = CFString::new("AXFocused");
                let t = CFBoolean::true_value();
                let rc = AXUIElementSetAttributeValue(
                    held.el,
                    attr.as_concrete_TypeRef(),
                    t.as_CFTypeRef(),
                );
                if rc != 0 {
                    return Err(format!("聚焦被拒（AXError {rc}）"));
                }
                // 赋值不抛错 != 焦点真的到了。回读。
                let got = attr_bool(held.el, "AXFocused").unwrap_or(false);
                if !got {
                    return Err("赋值被接受，但焦点没落到这个元素上".into());
                }
                Ok(serde_json::json!({"ok": true, "action": "focus", "role": live.role}))
            }
            "set_value" => {
                let v = value.ok_or("set_value 需要 value")?;
                let attr = CFString::new("AXValue");
                let s = CFString::new(v);
                let rc = AXUIElementSetAttributeValue(
                    held.el,
                    attr.as_concrete_TypeRef(),
                    s.as_CFTypeRef(),
                );
                if rc != 0 {
                    return Err(format!("写入被拒（AXError {rc}）"));
                }
                let back = node_value(held.el);
                if back != v {
                    return Err(format!("写进去了但读回来是「{back}」，不是要写的值"));
                }
                Ok(serde_json::json!({"ok": true, "action": "set_value", "value": back}))
            }
            other => Err(format!(
                "不支持的动作「{other}」；可用：press / focus / set_value"
            )),
        }
    }
}

/// 只读**这一个**元素的签名，不递归。
unsafe fn walk_one(el: AXUIElementRef, out: &mut Vec<AxNode>) {
    let role = attr_string(el, "AXRole").unwrap_or_default();
    if role.is_empty() {
        return;
    }
    let (x, y) = attr_point(el, "AXPosition").unwrap_or((0, 0));
    let (w, h) = attr_size(el, "AXSize").unwrap_or((0, 0));
    out.push(AxNode {
        role: role.trim_start_matches("AX").to_string(),
        text: node_text(el),
        value: node_value(el),
        x,
        y,
        w,
        h,
        enabled: attr_bool(el, "AXEnabled").unwrap_or(true),
    });
}

/// 按应用名找进程号。window.restore 要用，和 macos.rs 里那个是同一件事。
pub fn pid_of(title: &str) -> Option<i32> {
    use cocoa::base::{id, nil};
    use objc::{class, msg_send, sel, sel_impl};
    unsafe {
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let apps: id = msg_send![workspace, runningApplications];
        let count: usize = msg_send![apps, count];
        for i in 0..count {
            let app: id = msg_send![apps, objectAtIndex: i];
            let name_obj: id = msg_send![app, localizedName];
            if name_obj == nil {
                continue;
            }
            let ptr: *const i8 = msg_send![name_obj, UTF8String];
            let name = std::ffi::CStr::from_ptr(ptr).to_string_lossy();
            if name.contains(title) {
                let pid: i32 = msg_send![app, processIdentifier];
                return Some(pid);
            }
        }
    }
    None
}

/// 最小化 / 还原某个应用的窗口。
///
/// 平台层那两个（minimize_window / maximize_window）在 macOS 上一直是只会返回
/// UnsupportedPlatform 的空实现，而 window.minimize 就写在工具目录的 enum 里——
/// 模型照着调必然报错。而 AX 侧本来就有 AXMinimized 这个可写属性，几行就能实现。
///
/// 按应用名匹配（和 window.activate 一致），只动第一个尺寸够大的窗口。
pub fn set_minimized(pid: i32, minimized: bool) -> Result<String, String> {
    unsafe {
        let app = AXUIElementCreateApplication(pid);
        if app.is_null() {
            return Err("拿不到这个应用的可访问性入口".into());
        }
        AXUIElementSetMessagingTimeout(app, 2.0);
        let key = CFString::new("AXWindows");
        let mut raw: CFTypeRef = ptr::null();
        let rc = AXUIElementCopyAttributeValue(app, key.as_concrete_TypeRef(), &mut raw);
        if rc != 0 || raw.is_null() {
            CFRelease(app as CFTypeRef);
            return Err(format!("这个应用没有交出窗口（AXError {rc}）"));
        }
        let arr = raw as CFArrayRef;
        let n = CFArrayGetCount(arr);
        let mut done: Option<String> = None;
        for i in 0..n {
            let w = CFArrayGetValueAtIndex(arr, i) as AXUIElementRef;
            if w.is_null() {
                continue;
            }
            // 尺寸过滤只在**最小化**时用（挑一个真窗口下手，别去动 1x1 的隐藏工具窗）。
            // 还原时不能用：已经最小化的窗口尺寸就是不正常的，按尺寸筛会把唯一那个
            // 要还原的窗口挡在外面——实测就是这么失败的（最小化成功、还原报「没找到」）。
            if minimized {
                let (ww, wh) = attr_size(w, "AXSize").unwrap_or((0, 0));
                if ww < 40 || wh < 40 {
                    continue;
                }
            } else if !attr_bool(w, "AXMinimized").unwrap_or(false) {
                // 还原时只找当前确实是最小化的那些。
                continue;
            }
            let attr = CFString::new("AXMinimized");
            let v = if minimized { CFBoolean::true_value() } else { CFBoolean::false_value() };
            let src = AXUIElementSetAttributeValue(w, attr.as_concrete_TypeRef(), v.as_CFTypeRef());
            if src != 0 {
                continue;
            }
            // 赋值不抛错 != 真的生效了。回读——这个项目里所有「发出请求」都要回读，
            // 否则就是又一个「一路 ok、屏幕上什么都没发生」。
            //
            // 要**轮询**不能只读一次：最小化那一下回读立刻就对，还原却有动画，
            // 立刻读到的还是 true。实测就是这么失败的（最小化成功、还原报没找到）。
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1500);
            let mut got = attr_bool(w, "AXMinimized").unwrap_or(!minimized);
            while got != minimized && std::time::Instant::now() < deadline {
                std::thread::sleep(std::time::Duration::from_millis(50));
                got = attr_bool(w, "AXMinimized").unwrap_or(!minimized);
            }
            if got == minimized {
                done = Some(attr_string(w, "AXTitle").unwrap_or_default());
                break;
            }
        }
        CFRelease(raw);
        CFRelease(app as CFTypeRef);
        match done {
            Some(t) => Ok(t),
            None => Err(format!(
                "没能{}任何窗口——可能这个应用不允许（AXMinimized 只读），或者它没有普通窗口",
                if minimized { "最小化" } else { "还原" }
            )),
        }
    }
}

#[cfg(test)]
mod act_tests {
    /// 句柄表和 ref 解析的端到端验证——不真的按下去，但把 act 在动作**之前**做的
    /// 每一步都跑一遍：查表、拿句柄、回读元素、比签名。
    ///
    /// 为什么值得单独测：这条路是「读一次留下句柄，点的时候直接用」，而老路是
    /// 「点的时候重跑一遍枚举按下标取」。下标那套一旦界面动过就错位，句柄这套不会——
    /// 但句柄如果没 retain 住，用的时候就是野指针（已经踩过一次 SIGTRAP）。
    ///   cargo test --all-features act_ref -- --ignored --nocapture
    #[test]
    #[ignore]
    fn act_ref_resolves_and_detects_staleness() {
        let pid: i32 = std::env::var("AX_PID").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        assert!(pid > 0, "用 AX_PID=<进程号> 指定目标");
        let nodes = super::snapshot(pid, 200);
        assert!(!nodes.is_empty(), "先得读到东西");

        // 不存在的 ref 要说清楚，而不是崩或者点到别的东西上。
        let bad = super::act(999_999, "press", None).unwrap_err();
        assert!(bad.contains("不在最近一次读屏结果里"), "越界 ref 的说法不对：{bad}");

        // 不支持的动作同样要点名可用的是哪几个。
        let wrong = super::act(1, "click", None).unwrap_err();
        assert!(wrong.contains("press") && wrong.contains("focus"), "不支持的动作没列出可用的：{wrong}");

        // 真正要验的：ref 1 的句柄还活着，回读得到、签名对得上。
        // 走 set_value 到一个多半不可写的元素上——它会在**签名比对之后**才失败，
        // 所以只要报的不是「过期」就说明句柄和签名这一段是通的。
        let r = super::act(1, "set_value", Some("__probe__"));
        match r {
            Ok(_) => println!("ref 1 可写，句柄链路通"),
            Err(e) => {
                assert!(!e.contains("已经过期"), "句柄没留住或签名对不上：{e}");
                assert!(!e.contains("不存在"), "句柄失效了：{e}");
                println!("ref 1 不可写（预期内），但签名比对通过：{e}");
            }
        }
        println!("句柄表 {} 个元素，ref 解析与签名比对正常", nodes.len());
    }
}

#[cfg(test)]
mod minimize_tests {
    /// 真的最小化再还原一次。这个项目里「发出请求」和「真发生了」是两回事，
    /// 而 minimize_window 以前就是个只会报 UnsupportedPlatform 的空实现，
    /// 却写在工具目录的 enum 里——清单在说谎。
    ///   AX_PID=<pid> cargo test --all-features minimize_roundtrip -- --ignored --nocapture
    #[test]
    #[ignore]
    fn minimize_roundtrip() {
        let pid: i32 = std::env::var("AX_PID").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        assert!(pid > 0, "用 AX_PID=<进程号> 指定目标");
        let title = super::set_minimized(pid, true).expect("最小化应当成功");
        println!("已最小化：{title}");
        std::thread::sleep(std::time::Duration::from_millis(600));
        let back = super::set_minimized(pid, false).expect("还原应当成功");
        println!("已还原：{back}");
    }
}
