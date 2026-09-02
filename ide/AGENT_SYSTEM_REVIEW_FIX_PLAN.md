# Michael IDE 智能体系统审查报告（核实版）

> 审查范围：automation-framework（桌面自动化智能体）、src-tauri/src/ai.rs（AI 核心）、
> src-tauri/src/automation.rs（sidecar 桥接）、src/growth.js（成长系统）、src/main.js（后端抽象）。
> 本报告只保留**逐行核实过**的结论；所列修复均已落地并编译/测试通过。

---

## 一、结论先行

项目的智能体是**真实现，不是假的**：

- `ai.rs`：完整的流式工具调用循环、真正生效的取消机制（RAII CancelGuard）、
  michael-compression 前缀引用、连接池复用——工业级水准。
- `automation-framework`：真实的 CDP 浏览器驱动 + enigo 键鼠合成 + macOS Accessibility /
  Windows COM UIA 双平台实现；RPC 层带常量时间 token 比较与浏览器指纹头拒绝（防网页 RCE），
  安全设计相当扎实。
- `growth.js`：真实的 Bayesian Knowledge Tracing 学习者模型 + Open Learner Model，
  不是摆设进度条；同步 localStorage 写入是注释里写明理由的刻意设计（防丢最后一轮信号），不改。
- `main.js` 的 `mockBackend()` 是**浏览器预览的刻意降级**（浏览器摸不到本地文件系统），
  AI 走真实网关；`net.rs` 里的 "mockup" 字样是单元测试的测试数据。两者都不是假智能体内容。

**但确实存在真 bug**，集中在 automation-framework，以下全部已修复。

---

## 二、已修复的真实 Bug

### 1. 垂直滚动从未生效（业务逻辑错误）🔴
`agent.rs::mouse_scroll(amount)` 把唯一参数传给了 `SystemAutomation::scroll(delta_x, delta_y)`
的**水平分量**（`sys.scroll(amount, 0)`）。RPC 层 `mouse.scroll` 要求调用方必传 `delta_y`、
读了 `delta_x` 又丢弃——净效果：智能体请求垂直滚动，页面纹丝不动；水平滚动被静默吞掉。
`AI_AGENT_GUIDE.md` 文档写的还是双参数签名，文档与代码互相矛盾。

**修复**：`mouse_scroll(delta_x, delta_y)` 双参数直通，RPC 两个分量都真实传递；与文档一致。

### 2. 双击按键参数读了不用（死参数）🟡
`rpc.rs` 的 `mouse.double_click` 解析了 `button` 参数却调用无参的
`mouse_double_click()`（硬编码左键）。**修复**：参数真实传递，右键/中键双击可用。

### 3. shell 注入（安全）🔴
`task.rs::open_and_type` 用 `sh -c "open -a '{app_name}'"` 拼接；app_name 含引号即可注入任意命令。
`screenshot_region` 同样经 `sh -c` 拼接。
**修复**：全部改为 `Command::new("open").arg("-a").arg(app_name)` 直接传参，不经过 shell；
并补上退出码检查（此前 `open` 失败也照样往下盲打键盘）。

### 4. 除零 panic 🔴
`task.rs::monitor_web_changes` 里 `duration_secs / interval_secs`，`interval_secs=0` 直接 panic
拉死常驻 server。**修复**：入参校验，返回错误结果而非崩溃。

### 5. JS 选择器注入 / 脚本撞坏 🟡
`task.rs` 三处把 CSS 选择器裸拼进 `querySelectorAll('{}')`——选择器含 `'` 或 `\` 时脚本直接
语法错误（如 `a[title='x']`）。**修复**：选择器经 `serde_json::json!` 编码为合法 JS 字符串字面量。

### 6. 浏览器方法 `.lock().unwrap()` panic 链 🟡
`agent.rs` 七个浏览器方法全用 `.lock().unwrap()` + `.runtime.as_ref().unwrap()`；一次 Mutex
中毒会连环 panic 杀死整个 automation-server（系统方法早就正确处理了中毒，浏览器方法没跟上）。
**修复**：抽出 `browser_lock()` / `rt()` 辅助函数，所有路径返回错误而非 panic，顺带消掉了
七份复制粘贴的样板（这部分才是真正的"垃圾代码"）。

### 7. Windows UIA 句柄悬垂（use-after-free）🔴
`windows_ui_automation.rs::find_element_by_name` 把 COM 裸指针存进 `DesktopElement` 后，
本体 `elem` 立刻 drop（Release）——引用计数归零，后续对该句柄 `click_element` 就是访问已释放
对象。姊妹函数 `find_elements_by_type` 补了 `AddRef` 而这里漏了。**修复**：补齐 `AddRef`。

### 8. `http_server_demo` 必然编译失败（假示例）🟡
示例引用 `HttpServer`（在 `server` feature 后面），但 Cargo.toml 只给它声明了
`required-features = ["system", "browser"]`——任何人跑这个示例都编译失败。
**修复**：补上 `server` feature；`--features full --examples` 全量编译通过。

---

## 三、验证记录

```
cargo check --features "system browser"                  ✅ 通过
cargo check --features "system browser" --bins --examples ✅ 通过
cargo check --features full --bins --examples             ✅ 通过
cargo test  --features "system browser" --lib             ✅ 28 passed, 0 failed
cargo build --release --bin automation-server             ✅ 已重建
src-tauri/binaries/automation-server-aarch64-apple-darwin ✅ sidecar 已同步为修复后的二进制
```

注：Windows UIA 修复（第 7 条）在 macOS 上无法编译验证，属 `cfg(target_os = "windows")`
专属代码；修复方式与同文件已验证的姊妹函数完全一致。

## 四、明确不改的地方（以免"修复"变破坏）

- `growth.js` 同步 `save()`：注释写明 payload 极小、按用户动作触发、debounce 反而会在关窗时
  丢最后一轮信号。设计正确。
- `main.js mockBackend()`：浏览器构建的文件系统/终端降级，注释清晰、与真实后端接口对齐。
- `net.rs` 测试里的 example.com：`#[test]` 数据，不进生产路径。
- RPC 对外契约（method 名、参数名）零变更，IDE 侧 `automation_call` 无需任何改动。
