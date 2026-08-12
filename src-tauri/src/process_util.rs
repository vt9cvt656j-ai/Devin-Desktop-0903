//! Shared utilities for LSP and DAP child process management.

/// The user's REAL login-shell PATH, captured ONCE (cached). A GUI-launched app inherits a
/// minimal PATH that misses everything a version manager sets up in `.zshrc`/`.zprofile` —
/// nvm's `~/.nvm/versions/node/<v>/bin`, volta, pyenv, asdf shims, a custom `npm -g` prefix,
/// pipx, etc. So a language server the user (or the AI) installed is invisible → false "缺少
/// 语言服务器" prompts and installs that "succeed" but still can't be found. Running the login
/// shell and reading its `$PATH` makes the IDE resolve exactly what the user's terminal does.
#[cfg(not(windows))]
struct PathCache {
    value: String,
    at: std::time::Instant,
}
#[cfg(not(windows))]
static PATH_CACHE: std::sync::RwLock<Option<PathCache>> = std::sync::RwLock::new(None);
/// 两次探测之间的最短间隔。alt-tab 连点不该刷出一串 zsh 进程。
#[cfg(not(windows))]
const PROBE_FLOOR: std::time::Duration = std::time::Duration::from_secs(5);

/// 从探测输出里抠出 PATH。marker 包裹是为了把 `.zshrc` 里的 echo / MOTD 噪声剥干净。
///
/// 抽成纯函数是为了能测：解析失败和探测失败必须是**两件事**，前者返回 `None` 让调用方保留
/// 上一次的好值，而不是把空串当成"用户的 PATH 就是空的"。
#[cfg(not(windows))]
fn parse_probe_output(raw: &str) -> Option<String> {
    match (raw.find("__WP__"), raw.rfind("__WP__")) {
        (Some(a), Some(b)) if b > a + 6 => {
            let p = &raw[a + 6..b];
            // 一个斜杠都没有的东西不可能是 PATH（多半是 rc 文件打的一行字）。
            if p.contains('/') { Some(p.to_string()) } else { None }
        }
        _ => None,
    }
}

/// 真跑一次 `$SHELL -lic` 把用户的真实 PATH 问出来。
///
/// **失败返回 `None`，绝不返回空串。** 旧实现把超时（`unwrap_or_default`）和解析失败
/// （`_ => String::new()`）都写进了 `OnceLock`：开机那一下系统忙、探测超过 4 秒，整个会话的
/// PATH 就被永久降级成下面那张硬编码目录表，nvm / pyenv / asdf 的 shim 从此全部失踪，而且
/// 重启 app 之前没有任何办法恢复。
#[cfg(not(windows))]
fn probe_login_shell_path(budget_ms: u64) -> Option<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    // 放在线程里跑并设预算，这样一个病态的 rc 文件永远卡不死 app。
    // `-lic` 同时 source login 和 interactive 的 rc（版本管理器就住在那儿）；
    // `command printf` 绕开被别名/函数改写过的 printf。
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let out = std::process::Command::new(&shell)
            .args(["-lic", "command printf '__WP__%s__WP__' \"$PATH\""])
            .output();
        let _ = tx.send(out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).ok());
    });
    let raw = rx
        .recv_timeout(std::time::Duration::from_millis(budget_ms))
        .ok()
        .flatten()?;
    parse_probe_output(&raw)
}

/// 重新探测用户的登录 shell PATH。`force` 跳过间隔下限。
///
/// 返回值表示"这次真的探测了"。探测失败时**保留上一次的好值**——宁可用旧的，也不要退回
/// 硬编码表。
#[cfg(not(windows))]
pub fn refresh_login_shell_path(force: bool) -> bool {
    if !force {
        if let Ok(g) = PATH_CACHE.read() {
            if let Some(c) = g.as_ref() {
                if c.at.elapsed() < PROBE_FLOOR {
                    return false;
                }
            }
        }
    }
    // 冷启动给 1.5 秒就够（实测 0.03–0.21s）；刷新时给 4 秒，反正不在关键路径上。
    let budget = if force { 4000 } else { 2500 };
    if let Some(p) = probe_login_shell_path(budget) {
        if let Ok(mut g) = PATH_CACHE.write() {
            *g = Some(PathCache { value: p, at: std::time::Instant::now() });
        }
        return true;
    }
    // 探测失败：只更新时间戳，避免每条命令都重试一次慢探测；旧值继续用。
    if let Ok(mut g) = PATH_CACHE.write() {
        if let Some(c) = g.as_mut() {
            c.at = std::time::Instant::now();
        }
    }
    false
}

#[cfg(not(windows))]
fn login_shell_path() -> String {
    if let Ok(g) = PATH_CACHE.read() {
        if let Some(c) = g.as_ref() {
            return c.value.clone();
        }
    }
    // 第一次：同步探一次（预算短），失败就返回空串走硬编码表——但**不缓存这个失败**，
    // 下一次调用还会再试，直到真的问出来为止。
    match probe_login_shell_path(1500) {
        Some(p) => {
            if let Ok(mut g) = PATH_CACHE.write() {
                *g = Some(PathCache { value: p.clone(), at: std::time::Instant::now() });
            }
            p
        }
        None => String::new(),
    }
}

/// Build a PATH that includes the workspace's `node_modules/.bin` + a Python venv + common
/// toolchain directories + the user's real login-shell PATH, so project-local and user-installed
/// tools resolve even when the app is launched from a GUI with a minimal PATH.
#[cfg(not(windows))]
pub fn augmented_path(workspace: Option<&str>) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut parts: Vec<String> = Vec::new();
    if let Some(ws) = workspace.filter(|w| !w.is_empty()) {
        parts.push(format!("{ws}/node_modules/.bin"));
        parts.push(format!("{ws}/.venv/bin")); // python venv (pyright/pylsp installed here)
        parts.push(format!("{ws}/venv/bin"));
    }
    parts.push(format!("{home}/.cargo/bin"));
    parts.push("/opt/homebrew/bin".into());
    parts.push("/usr/local/bin".into());
    parts.push(format!("{home}/go/bin"));
    parts.push(format!("{home}/.local/bin")); // pipx, pip --user
    parts.push(format!("{home}/.bun/bin"));
    parts.push(format!("{home}/.deno/bin"));
    parts.push(format!("{home}/.volta/bin"));
    parts.push(format!("{home}/.michael-ide/npm-global/bin")); // IDE-managed npm tools
    parts.push(format!("{home}/.npm-global/bin")); // common custom `npm config set prefix`
    parts.push("/usr/bin".into());
    parts.push("/bin".into());
    let extra = parts.join(":");
    // Append the user's real login-shell PATH (covers nvm/pyenv/asdf/custom prefixes we can't
    // hardcode), then the minimal inherited PATH as a final fallback.
    let mut all = extra;
    let shell = login_shell_path();
    if !shell.is_empty() {
        all = format!("{all}:{shell}");
    }
    if let Ok(p) = std::env::var("PATH") {
        if !p.is_empty() {
            all = format!("{all}:{p}");
        }
    }
    all
}

/// 解析一个**由 IDE 自己发起**的系统工具（git、node 之类），**不**把工作区目录算进来。
///
/// `augmented_path` 把 `{workspace}/node_modules/.bin` 放在最前面，这对项目自带的工具链
/// （项目自己的 eslint / TypeScript 版本）是正确且必要的行为。但它同时意味着：仓库里放一个
/// 可执行文件叫 `git`，IDE 一打开这个文件夹去查 git 状态，跑的就是攻击者的程序。
///
/// 区别在于「谁要求跑这个命令」：项目工具链是用户选的项目在提供，而 git 是 IDE 自己要用的
/// 基础设施——后者绝不能被仓库内容覆盖。
#[cfg(not(windows))]
pub fn resolve_system_command(cmd: &str) -> String {
    resolve_command(cmd, None)
}

#[cfg(windows)]
pub fn resolve_system_command(cmd: &str) -> String {
    resolve_command(cmd, None)
}

/// Resolve a command name to its full path using the augmented PATH.
/// Rust's `Command::new` only searches the *current* process PATH, which is
/// minimal when a Tauri app is launched from macOS Finder. This function
/// searches the augmented PATH so tools in `~/.local/bin`, `~/.cargo/bin`,
/// etc. are found even from a GUI launch.
#[cfg(not(windows))]
pub fn resolve_command(cmd: &str, workspace: Option<&str>) -> String {
    if cmd.contains('/') {
        return cmd.to_string();
    }
    let path = augmented_path(workspace);
    for dir in path.split(':') {
        let full = format!("{dir}/{cmd}");
        if std::path::Path::new(&full).exists() {
            return full;
        }
    }
    cmd.to_string()
}

/// Windows: prepend the workspace's `node_modules\.bin` + Python venv Scripts to the existing PATH.
#[cfg(windows)]
pub fn augmented_path(workspace: Option<&str>) -> String {
    let cur = std::env::var("PATH").unwrap_or_default();
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    let mut base = Vec::new();
    if !home.is_empty() {
        base.push(format!("{home}\\.michael-ide\\npm-global"));
    }
    match workspace.filter(|w| !w.is_empty()) {
        Some(ws) => {
            let mut parts = vec![format!("{ws}\\node_modules\\.bin")];
            for name in [".venv", "venv"] {
                let scripts = format!("{ws}\\{name}\\Scripts");
                if std::path::Path::new(&scripts).is_dir() {
                    parts.push(scripts);
                }
            }
            parts.extend(base);
            if !cur.is_empty() {
                parts.push(cur);
            }
            parts.join(";")
        }
        None => {
            if !cur.is_empty() {
                base.push(cur);
            }
            base.join(";")
        }
    }
}

/// Windows: let the OS resolve the command via PATH + PATHEXT (.exe/.cmd/.bat),
/// so just hand the name back unchanged.
#[cfg(windows)]
pub fn resolve_command(cmd: &str, _workspace: Option<&str>) -> String {
    cmd.to_string()
}

/// Maximum number of concurrent LSP or DAP processes allowed.
pub const MAX_CHILD_PROCESSES: usize = 16;

/// Build a `std::process::Command` that will **NOT pop a console window on
/// Windows** (sets `CREATE_NO_WINDOW`). No-op on macOS/Linux. Use this
/// EVERYWHERE instead of `Command::new` — every subprocess this app spawns
/// (git, LSP servers, shells, PowerShell, osascript, node, cargo…) otherwise
/// flashes a cmd/PowerShell black window on Windows, and the constant spawn
/// churn also wedges the UI. One helper → all spawns silent.
pub fn command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    #[allow(unused_mut)]
    let mut c = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}
