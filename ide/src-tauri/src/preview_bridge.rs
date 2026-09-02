//! 把预览调试桥注入到**每一个帧**。
//!
//! 实时预览用 iframe 直嵌本地 dev server。iframe 跨源，应用读不到里面的 console 和 DOM，
//! 所以需要页面自己把这些送出来——也就是往被预览的页面里塞一段脚本。
//!
//! 上一版是让 IDE 自带的那个 Python 预览服务把 `<script src>` 插进它服务的 HTML。
//! 问题是：只有那一个服务起的预览才有桥。用户拿 vite / next / django 起的服务
//! 一律注入不上，「指元素」永远弹一个「这个页面还没接调试桥」——那不叫能用。
//!
//! 这里改用 WebView 的原生能力：`js_init_script_on_all_frames` 一路落到
//! `WKUserScript(forMainFrameOnly: false)`（macOS）/ WebView2 的 AddScriptToExecuteOnDocumentCreated
//! （Windows 那边本来就总是注入子帧），于是**任何**被嵌的页面在文档解析之前就拿到了它，
//! 不需要用户改自己的项目一行代码。
//!
//! 脚本正文和前端共用同一个文件（`ide/src/preview-bridge.js`），`include_str!` 在编译期
//! 读进来。两边读同一份，不存在「改了一头忘了另一头」——test/live-preview.test.mjs 钉着这一点。

/// 桥的正文。**不要在这里另抄一份**：前端也读同一个文件。
const PREVIEW_BRIDGE_JS: &str = include_str!("../../src/preview-bridge.js");

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("preview-bridge")
        .js_init_script_on_all_frames(PREVIEW_BRIDGE_JS)
        .build()
}
