//! 子进程要用的**当前**环境，以及"用哪个解释器跑命令"的决定。
//!
//! # 这个模块解决的两个问题
//!
//! **一、外部改了环境变量，进程里看不到。** 一个进程的 environment block 是 exec 那一刻的
//! 快照。用户在别的终端 `export`、改 `.zshrc`、用系统属性面板加一个变量，都不会改写已经在跑
//! 的这个 app。macOS 上唯一的真值来源是重新跑一次登录 shell（见 `process_util`）；Windows 上
//! 则是注册表——`setx` 和系统属性面板写的就是注册表，然后广播一条 `WM_SETTINGCHANGE`。
//! 我们不去挂那条消息（Tauri v2 没有绑定，要自己做 HWND subclass），直接每次起子进程前读一遍
//! 注册表：那是微秒级的操作，比消息循环便宜，而且不会漏掉 app 没获得焦点时发生的变更。
//!
//! **二、模型写的是 POSIX bash，Windows 上却由 cmd.exe 执行。** cmd 下最要命的不是"命令不
//! 存在"（那会报 9009，模型能自纠），而是**静默错**：`;` 不是命令分隔符、`$VAR` 不展开、
//! `'单引号'` 不是引号、`$(…)` 不做替换——退出码 0，结果错，模型没有任何信号。所以这里在
//! Windows 上优先找 Git Bash，让模型写的 bash 就是对的；找不到才降级到 cmd，而且降级要**说
//! 出来**，不能像以前那样静悄悄地把 POSIX 喂进去。
//!
//! # 为什么大部分代码不带 `cfg`
//!
//! 合并、展开、挑选这些逻辑全是纯函数，不带平台条件——于是 Windows 的绝大部分行为可以在一台
//! Mac 上 `cargo test` 直接测到。只有真正碰注册表和真正探活的那几十行是 `#[cfg(windows)]`，
//! 那部分靠 `cargo check --target x86_64-pc-windows-msvc` 守。

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// 纯函数部分：不带 cfg，两个平台上都能测
// ---------------------------------------------------------------------------

/// 合并 Windows 的两份 PATH：系统在前、用户在后，**大小写不敏感**去重（Windows 自己的规则）。
///
/// 尾部反斜杠也要一起归一，否则 `C:\Tools` 和 `C:\tools\` 会被当成两个目录，PATH 里塞满
/// 只是大小写不同的重复项——真实机器上这一段能涨到几千字符，而 `CreateProcess` 对环境块
/// 有大小上限。
pub fn merge_path(system: &str, user: &str) -> String {
    let mut seen: Vec<String> = Vec::new();
    let mut out: Vec<String> = Vec::new();
    for part in system.split(';').chain(user.split(';')) {
        let p = part.trim().trim_end_matches('\\');
        if p.is_empty() {
            continue;
        }
        let key = p.to_ascii_lowercase();
        if seen.iter().any(|s| *s == key) {
            continue;
        }
        seen.push(key);
        out.push(p.to_string());
    }
    out.join(";")
}

/// 展开 `%VAR%`，一轮、不递归；**未定义的原样保留**——和 cmd 命令行的行为一致。
///
/// 注册表里存的常常是 `REG_EXPAND_SZ`，值本身就写着 `%SystemRoot%\system32`。不展开就等于
/// 把一个字面量 `%SystemRoot%` 塞进子进程的 PATH，那条目录名不存在，于是整条 PATH 少一段。
pub fn expand_percent(value: &str, table: &HashMap<String, String>) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut out = String::with_capacity(value.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' {
            if let Some(end) = chars[i + 1..].iter().position(|c| *c == '%') {
                let name: String = chars[i + 1..i + 1 + end].iter().collect();
                if let Some(v) = table.get(&name.to_ascii_uppercase()) {
                    out.push_str(v);
                    i += end + 2;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// 系统表 + 用户表 → 一份可以直接喂给子进程的环境。
///
/// 用户覆盖系统，**PATH 例外**：那一条是合并而不是覆盖，这也是 Windows 自己的规则。
/// `base` 是展开 `%VAR%` 时的底表（`%SystemRoot%` 之类不在这两个注册表键下面，得从进程环境取）。
pub fn merge_env(
    system: &[(String, String)],
    user: &[(String, String)],
    base: &HashMap<String, String>,
) -> Vec<(String, String)> {
    let mut map: HashMap<String, String> = HashMap::new();
    for (k, v) in system {
        map.insert(k.to_ascii_uppercase(), v.clone());
    }
    for (k, v) in user {
        let ku = k.to_ascii_uppercase();
        if ku == "PATH" {
            let sys = map.get("PATH").cloned().unwrap_or_default();
            map.insert(ku, merge_path(&sys, v));
        } else {
            map.insert(ku, v.clone());
        }
    }
    let mut table = base.clone();
    for (k, v) in &map {
        table.insert(k.clone(), v.clone());
    }
    map.into_iter()
        .map(|(k, v)| {
            let e = expand_percent(&v, &table);
            (k, e)
        })
        .collect()
}

/// `WindowsApps` 下的 `bash.exe` 是**应用执行别名**——一个引导你去商店装 WSL 的桩，不是 shell。
/// 用它当解释器，每条命令都会变成一个商店弹窗。
pub fn is_windows_apps_stub(path: &str) -> bool {
    path.replace('/', "\\")
        .split('\\')
        .any(|s| s.eq_ignore_ascii_case("WindowsApps"))
}

/// 纯排序器：按给定顺序挑第一个既不是商店桩、又**真能跑起来**的候选。
///
/// `runs` 由调用方注入，所以这个决定过程可以在没有 Windows 的机器上测。
pub fn pick_shell<F: Fn(&str) -> bool>(candidates: &[String], runs: F) -> Option<String> {
    candidates
        .iter()
        .find(|c| !is_windows_apps_stub(c) && runs(c))
        .cloned()
}

/// 一次"用什么跑命令"的完整决定。
#[derive(Clone, Debug, serde::Serialize)]
pub struct ShellPlan {
    /// 解释器的绝对路径（macOS/Linux 上是 `$SHELL`）。
    pub program: String,
    /// 一次性执行用的参数，命令字符串追加在这些参数之后。
    pub oneshot: Vec<String>,
    /// 交互式 PTY 用的参数。
    pub interactive: Vec<String>,
    /// `"posix"` | `"cmd"`。模型和界面都按这个字段分支，不要去猜 program 的文件名。
    pub kind: String,
    /// 给人看、也给模型看的名字：bash / zsh / cmd.exe。
    pub label: String,
}

/// 从解释器路径推一个短名字（`/bin/zsh` → `zsh`，`C:\...\bash.exe` → `bash`）。
pub fn shell_label(program: &str) -> String {
    let base = program
        .rsplit(|c| c == '/' || c == '\\')
        .next()
        .unwrap_or(program);
    base.strip_suffix(".exe")
        .or_else(|| base.strip_suffix(".EXE"))
        .unwrap_or(base)
        .to_string()
}

// ---------------------------------------------------------------------------
// 平台相关部分
// ---------------------------------------------------------------------------

/// 读注册表里的**当前**环境。只有这一段是 Windows-only。
#[cfg(windows)]
pub fn registry_env() -> Vec<(String, String)> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;
    let mut sys = Vec::new();
    let mut usr = Vec::new();
    if let Ok(k) = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment")
    {
        for (n, v) in k.enum_values().flatten() {
            sys.push((n, v.to_string()));
        }
    }
    if let Ok(k) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Environment") {
        for (n, v) in k.enum_values().flatten() {
            usr.push((n, v.to_string()));
        }
    }
    let base: HashMap<String, String> = std::env::vars()
        .map(|(k, v)| (k.to_ascii_uppercase(), v))
        .collect();
    merge_env(&sys, &usr, &base)
}

/// 非 Windows 上没有这个概念——环境就是进程环境，PATH 由 `process_util` 的登录 shell 探测负责。
#[cfg(not(windows))]
pub fn registry_env() -> Vec<(String, String)> {
    Vec::new()
}

/// 探活：**不能只 stat，也不能只看 `--version`**。
///
/// 只跑内建命令的探测在开了系统级强制 ASLR（ForceRelocateImages）的机器上会通过——bash 起得来、
/// 版本号也打得出来，然后**每一条真命令都 fork 失败**。所以必须真的去 fork 一个外部程序。
#[cfg(windows)]
fn bash_runs(path: &str) -> bool {
    crate::process_util::command(path)
        .args([
            "--noprofile",
            "--norc",
            "-c",
            "/usr/bin/true; /usr/bin/cat --version >/dev/null",
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Windows 上按可靠性排序的 Git Bash 候选。
#[cfg(windows)]
fn bash_candidates() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    // 1. 显式覆盖，给装在非常规位置的人留的口子。
    if let Ok(p) = std::env::var("MRDAY_GIT_BASH") {
        if !p.trim().is_empty() {
            out.push(p);
        }
    }
    // 2. 从 git.exe 反推安装根：<root>\cmd\git.exe 或 <root>\bin\git.exe → <root>\bin\bash.exe
    if let Ok(o) = crate::process_util::command("where.exe").arg("git").output() {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            let git = line.trim();
            if git.is_empty() {
                continue;
            }
            let p = std::path::Path::new(git);
            if let Some(root) = p.parent().and_then(|d| d.parent()) {
                out.push(root.join("bin").join("bash.exe").to_string_lossy().into());
            }
        }
    }
    // 3. 默认安装位置。
    out.push(r"C:\Program Files\Git\bin\bash.exe".into());
    out.push(r"C:\Program Files (x86)\Git\bin\bash.exe".into());
    // 4. PATH 上的 bash（可能是商店桩，pick_shell 会滤掉）。
    out.push("bash.exe".into());
    out
}

#[cfg(windows)]
fn compute_plan() -> ShellPlan {
    if let Some(bash) = pick_shell(&bash_candidates(), |c| bash_runs(c)) {
        return ShellPlan {
            label: shell_label(&bash),
            program: bash,
            oneshot: vec!["-c".into()],
            interactive: vec!["-i".into(), "-l".into()],
            kind: "posix".into(),
        };
    }
    let comspec = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into());
    ShellPlan {
        label: shell_label(&comspec),
        program: comspec,
        oneshot: vec!["/C".into()],
        interactive: vec!["/K".into()],
        kind: "cmd".into(),
    }
}

#[cfg(not(windows))]
fn compute_plan() -> ShellPlan {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    ShellPlan {
        label: shell_label(&shell),
        program: shell,
        // `-lc`：login 才会 source ~/.zprofile（macOS 上 brew shellenv、path_helper 都在那儿）。
        oneshot: vec!["-lc".into()],
        interactive: vec!["-l".into()],
        kind: "posix".into(),
    }
}

static PLAN_CACHE: std::sync::RwLock<Option<ShellPlan>> = std::sync::RwLock::new(None);

/// 当前的解释器决定。结果缓存，由 [`invalidate_plan`] 失效——用户装完 Git for Windows 之后
/// 不用重启 app，下一次刷新就能捡到。
pub fn plan() -> ShellPlan {
    if let Ok(g) = PLAN_CACHE.read() {
        if let Some(p) = g.as_ref() {
            return p.clone();
        }
    }
    let fresh = compute_plan();
    if let Ok(mut g) = PLAN_CACHE.write() {
        *g = Some(fresh.clone());
    }
    fresh
}

/// 丢掉缓存的解释器决定。PATH 变了就该重挑一次。
pub fn invalidate_plan() {
    if let Ok(mut g) = PLAN_CACHE.write() {
        *g = None;
    }
}

/// Git Bash 需要的三个环境变量。少一个都会出问题，而且都是静默的：
/// - `MSYS_NO_PATHCONV=1`：否则 Git Bash 会把原生工具的开关（`/FO`、`/TN`）当成路径改写成
///   `C:/Program Files/Git/FO`。
/// - `MSYS2_ARG_CONV_EXCL=*`：真 MSYS2 只认这一个，不认上面那个。
/// - `MSYSTEM=`：Git Bash 启动器无条件注入 `MSYSTEM=MINGW64`，会让原生工具链选错平台。
pub fn posix_shim_env(kind: &str, program: &str) -> Vec<(&'static str, String)> {
    if kind != "posix" || !program.to_ascii_lowercase().ends_with(".exe") {
        return Vec::new();
    }
    vec![
        ("MSYS_NO_PATHCONV", "1".into()),
        ("MSYS2_ARG_CONV_EXCL", "*".into()),
        ("MSYSTEM", String::new()),
    ]
}

#[derive(serde::Serialize)]
pub struct EnvRefresh {
    /// PATH 与上一次相比变了。false 也可能是"第一次调用"，前端据此不弹提示。
    pub changed: bool,
    pub path: String,
    pub shell: ShellPlan,
}

/// 重新读一遍环境，并报告 PATH 是否变化。前端在窗口获得焦点时调它。
#[tauri::command(async)]
pub fn env_refresh() -> EnvRefresh {
    #[cfg(not(windows))]
    crate::process_util::refresh_login_shell_path(false);
    #[cfg(not(windows))]
    let path = crate::process_util::augmented_path(None);
    // Windows 上 augmented_path 的基底是 std::env::var("PATH")——进程启动时的快照，
    // 而 Windows **永远不会**改写运行中进程的环境块。也就是说这里每次算出来的都是
    // 同一个字符串，changed 恒为 false，invalidate_plan() 一次也不会触发：
    // 用户装完 Git、setx 完 PATH，解释器缓存到进程退出都不会失效——而这个函数
    // 存在的全部意义就是发现这件事。
    //
    // 注册表才是 Windows 上环境变化的真实来源，tasks.rs 已经在读它了。
    #[cfg(windows)]
    let path = {
        let reg_path = registry_env()
            .into_iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("PATH"))
            .map(|(_, v)| v);
        crate::process_util::augmented_path_over(reg_path, None)
    };
    static LAST: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
    let mut g = LAST.lock().unwrap_or_else(|e| e.into_inner());
    let changed = !g.is_empty() && *g != path;
    if changed {
        // PATH 变了，刚装的 git 可能就在里面——重新挑一次解释器。
        invalidate_plan();
    }
    *g = path.clone();
    EnvRefresh { changed, path, shell: plan() }
}

/// 当前解释器决定，给前端用来告诉模型"这台机器上 run_cmd 到底由谁执行"。
#[tauri::command(async)]
pub fn shell_plan() -> ShellPlan {
    plan()
}

#[cfg(test)]
mod one_shell_decision {
    /// 「用哪个解释器」这个决定**只许有一处**。
    ///
    /// 这条守的是一个实际发生过的 bug：终端另起炉灶读 COMSPEC，而 run_cmd 走 plan()。
    /// 装了 Git for Windows 的机器上 run_cmd 跑 bash、终端跑 cmd.exe，而每轮注入给
    /// 模型的平台说明只说一句「两者都由 bash 执行」——对前者是真的，对后者是假的。
    /// 模型据此往终端里写 POSIX 语法，而本该保护它的那段警告正好被关掉。
    #[test]
    fn only_shell_env_decides_the_interpreter() {
        for (name, src) in [
            ("terminal.rs", include_str!("terminal.rs")),
            ("tasks.rs", include_str!("tasks.rs")),
        ] {
            let code: String = src
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                !code.contains("COMSPEC"),
                "{name} 又自己去读 COMSPEC 挑解释器了——这个决定只该由 shell_env::plan() 做"
            );
            assert!(
                code.contains("shell_env::plan"),
                "{name} 应该走 shell_env::plan()"
            );
        }
    }
}

#[cfg(test)]
mod tests {

    /// 反漂移：Windows 上 env_refresh 必须拿**注册表**的 PATH 作基底。
    ///
    /// 用进程环境（std::env::var("PATH")）的话它是个常量——Windows 永远不改写
    /// 运行中进程的环境块。于是 changed 恒 false、invalidate_plan() 一次不触发，
    /// 用户装完 Git、setx 完 PATH，解释器缓存到进程退出都不失效。而这个函数
    /// 存在的全部意义就是发现这件事。
    ///
    /// 断言源码（Windows 分支在 mac 上不参与编译），先剥注释——上面这段解释里
    /// 就引用了旧写法，不剥的话断言会被自己喂饱。
    #[test]
    fn windows_env_refresh_reads_the_registry_not_the_process_snapshot() {
        let src: String = include_str!("shell_env.rs")
            .lines()
            .filter(|l| {
                let t = l.trim_start();
                !t.starts_with("//") && !t.starts_with("///")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let at = src
            .find("pub fn env_refresh()")
            .expect("找不到 env_refresh");
        let body = &src[at..at + 1400];
        assert!(
            body.contains("registry_env()"),
            "env_refresh 的 Windows 分支没有读注册表"
        );
        assert!(
            body.contains("augmented_path_over"),
            "env_refresh 没有把注册表 PATH 当作基底传下去"
        );
        // 无平台门的那个调用不该还在：它会让 Windows 也走进程快照。
        let bare = format!("let path = crate::process_util::{}(None);", "augmented_path");
        let guarded = format!("#[cfg(not(windows))]\n    {bare}");
        assert_eq!(
            body.matches(&bare).count(),
            1,
            "augmented_path(None) 应该只剩被 cfg(not(windows)) 门住的那一处"
        );
        assert!(body.contains(&guarded), "那一处没有被 cfg(not(windows)) 门住");
    }
    use super::*;

    fn table(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_ascii_uppercase(), v.to_string()))
            .collect()
    }

    #[test]
    fn merge_path_dedupes_case_insensitively_and_keeps_system_first() {
        // Windows 的路径比较是大小写不敏感的；不按这个规则去重，PATH 里会塞满只有大小写
        // 不同的重复项，而环境块是有大小上限的。
        assert_eq!(
            merge_path(r"C:\a;C:\b", r"C:\B;C:\c"),
            r"C:\a;C:\b;C:\c",
            "C:\\B 和 C:\\b 是同一个目录"
        );
        // 尾部反斜杠同样要归一。
        assert_eq!(merge_path(r"C:\a\", r"C:\a"), r"C:\a");
        // 空段（`;;` 或首尾分号）丢掉，不然子进程会拿到一个空目录项。
        assert_eq!(merge_path(r";C:\a;;", r""), r"C:\a");
    }

    #[test]
    fn expand_percent_leaves_undefined_names_alone() {
        let t = table(&[("SYSTEMROOT", r"C:\Windows")]);
        assert_eq!(expand_percent(r"%SystemRoot%\system32", &t), r"C:\Windows\system32");
        // 大小写不敏感——注册表里写的是 %SystemRoot%，环境里的键是 SYSTEMROOT。
        assert_eq!(expand_percent("%systemroot%", &t), r"C:\Windows");
        // 未定义的原样留着，和 cmd 一致；换成空串会把一整段路径吃掉。
        assert_eq!(expand_percent(r"%NOPE%\x", &t), r"%NOPE%\x");
        // 落单的 % 不该把后面的内容吞掉。
        assert_eq!(expand_percent("50% done", &t), "50% done");
    }

    #[test]
    fn merge_env_lets_user_override_but_merges_path() {
        let base = table(&[("SYSTEMROOT", r"C:\Windows")]);
        let sys = vec![
            ("Path".to_string(), r"%SystemRoot%\system32".to_string()),
            ("FOO".to_string(), "sys".to_string()),
        ];
        let usr = vec![
            ("Path".to_string(), r"C:\Users\m\bin".to_string()),
            ("FOO".to_string(), "user".to_string()),
        ];
        let out: HashMap<String, String> = merge_env(&sys, &usr, &base).into_iter().collect();
        // 普通变量：用户覆盖系统。
        assert_eq!(out.get("FOO").unwrap(), "user");
        // PATH：合并而不是覆盖，且 %SystemRoot% 已展开——不展开的话子进程的 PATH 里会有一段
        // 字面量 %SystemRoot%\system32，那个目录并不存在。
        assert_eq!(out.get("PATH").unwrap(), r"C:\Windows\system32;C:\Users\m\bin");
    }

    #[test]
    fn windows_apps_bash_is_a_store_stub_not_a_shell() {
        // 这个"bash.exe"是应用执行别名，跑它只会弹商店去装 WSL。
        assert!(is_windows_apps_stub(
            r"C:\Users\m\AppData\Local\Microsoft\WindowsApps\bash.exe"
        ));
        assert!(is_windows_apps_stub(
            "C:/Users/m/AppData/Local/Microsoft/WindowsApps/bash.exe"
        ));
        assert!(!is_windows_apps_stub(r"C:\Program Files\Git\bin\bash.exe"));
    }

    #[test]
    fn pick_shell_skips_stubs_and_dead_candidates_in_order() {
        let cands: Vec<String> = vec![
            r"C:\Users\m\AppData\Local\Microsoft\WindowsApps\bash.exe".into(),
            r"C:\Program Files\Git\bin\bash.exe".into(),
            r"C:\Program Files (x86)\Git\bin\bash.exe".into(),
        ];
        // 商店桩即使"能跑"也要跳过。
        let picked = pick_shell(&cands, |_| true).unwrap();
        assert_eq!(picked, r"C:\Program Files\Git\bin\bash.exe");
        // 探活失败的往后顺延——这条正是为强制 ASLR 下"起得来但 fork 不了"准备的。
        let picked = pick_shell(&cands, |c| c.contains("(x86)")).unwrap();
        assert_eq!(picked, r"C:\Program Files (x86)\Git\bin\bash.exe");
        // 一个都不行就是 None，调用方据此降级到 cmd。
        assert!(pick_shell(&cands, |_| false).is_none());
    }

    #[test]
    fn shell_label_is_the_bare_name() {
        assert_eq!(shell_label("/bin/zsh"), "zsh");
        assert_eq!(shell_label(r"C:\Program Files\Git\bin\bash.exe"), "bash");
        assert_eq!(shell_label(r"C:\WINDOWS\system32\cmd.exe"), "cmd");
    }

    #[test]
    fn msys_shims_only_apply_to_a_windows_bash() {
        // macOS 的 /bin/zsh 不该被塞 MSYS 变量。
        assert!(posix_shim_env("posix", "/bin/zsh").is_empty());
        // cmd 兜底也不需要。
        assert!(posix_shim_env("cmd", r"C:\WINDOWS\system32\cmd.exe").is_empty());
        let shims = posix_shim_env("posix", r"C:\Program Files\Git\bin\bash.exe");
        let keys: Vec<&str> = shims.iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, vec!["MSYS_NO_PATHCONV", "MSYS2_ARG_CONV_EXCL", "MSYSTEM"]);
        // MSYSTEM 必须是空串——Git Bash 启动器会注入 MINGW64，留着会让原生工具链选错平台。
        assert_eq!(shims[2].1, "");
    }

    #[test]
    fn the_plan_is_coherent_on_this_machine() {
        let p = plan();
        assert!(!p.program.is_empty());
        assert!(!p.oneshot.is_empty(), "一次性执行必须有参数，否则命令会被当成脚本文件名");
        assert!(p.kind == "posix" || p.kind == "cmd");
        #[cfg(not(windows))]
        {
            assert_eq!(p.kind, "posix");
            // -l 是关键：macOS 上 brew shellenv 和 path_helper 都在 ~/.zprofile 里，
            // 非 login shell 拿不到它们。
            assert!(p.oneshot.iter().any(|a| a.contains('l')), "一次性执行也要走 login");
        }
    }
}
