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

// ─────────────────── 登录 shell 的**整份**环境（不只是 PATH） ───────────────────
//
// 从 Claude Code / Cursor 导进来的 MCP 配置里，密钥基本都写成引用而不是明文：
//     "env": {"GITHUB_PERSONAL_ACCESS_TOKEN": "${GITHUB_TOKEN}"}
// 展开这些 `${VAR}` 不能拿本进程的环境去查：从 Finder / Dock 启动的 macOS 应用**一个 shell
// 导出都继承不到**，`std::env::var("GITHUB_TOKEN")` 对着的正好是这批人——把导出写在
// `.zshrc`/`.zprofile` 里、因此才需要这个功能的人——永远是空。展开成空串比展开失败更糟：
// 服务照样起来，只是带着一个空 token，报出来的是「401」这种完全看不出原因的错。

/// 从探测输出里抠出 `env -0` 的结果。
///
/// 两处细节都是被真实数据逼出来的：
///   - **按 NUL 切，不按行切。** 环境变量的值里可以有换行（多行的 SSH key、粘进来的证书），
///     按行切会把它们切成一堆不成对的碎片。NUL 是值里唯一不可能出现的字节。
///   - **marker 包裹。** `-lic` 会 source 用户的 rc 文件，里面的 `echo` / MOTD / 版本管理器
///     的提示会先打到同一个 stdout 上，不剥掉就会和第一条 `KEY=VALUE` 粘在一起。
///
/// 解析不出任何变量返回 `None`：真实环境一定有 PATH / HOME，一个都没有意味着这次探测没成功
/// （`env -0` 不被支持、rc 文件把 stdout 吞了），而不是「用户的环境是空的」。
#[cfg(not(windows))]
fn parse_env0_output(raw: &[u8]) -> Option<std::collections::HashMap<String, String>> {
    const MARKER: &[u8] = b"__WE__";
    let start = raw.windows(MARKER.len()).position(|w| w == MARKER)?;
    let end = raw.windows(MARKER.len()).rposition(|w| w == MARKER)?;
    if end <= start {
        return None;
    }
    let mut map = std::collections::HashMap::new();
    for entry in raw[start + MARKER.len()..end].split(|byte| *byte == 0) {
        let Ok(text) = std::str::from_utf8(entry) else {
            continue;
        };
        let Some((key, value)) = text.split_once('=') else {
            continue; // 没有 '=' 的不是环境变量（rc 文件漏出来的噪声）
        };
        if !key.is_empty() {
            map.insert(key.to_string(), value.to_string());
        }
    }
    if map.is_empty() {
        None
    } else {
        Some(map)
    }
}

/// 真跑一次 `$SHELL -lic 'env -0'` 把用户登录 shell 的整份环境问出来。
///
/// 纪律和上面那个 PATH 探测完全一致：跑在独立线程里并设预算（一个病态的 rc 文件永远卡不死
/// app），**失败返回 `None`，绝不返回空表**——空表会被上层当成「用户就是没有这些变量」，
/// 于是 `${GITHUB_TOKEN}` 被展开成空串写进配置，比直接失败难查得多。
///
/// 这里**不做缓存**，所以 PATH 那条「绝不缓存失败」的规矩在这儿是天然成立的。不缓存是故意的：
/// 这张表装的是用户的真密钥，多留一份就多一份泄漏面，而它只在展开配置时才被调一次。
#[cfg(not(windows))]
fn probe_login_shell_env(budget_ms: u64) -> Option<std::collections::HashMap<String, String>> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let out = std::process::Command::new(&shell)
            // `command` 绕开被别名/函数改写过的 printf 和 env。
            .args(["-lic", "command printf '__WE__'; command env -0; command printf '__WE__'"])
            .output();
        let _ = tx.send(out.map(|o| o.stdout).ok());
    });
    let raw = rx
        .recv_timeout(std::time::Duration::from_millis(budget_ms))
        .ok()
        .flatten()?;
    parse_env0_output(&raw)
}

/// 用户登录 shell 里的环境变量表。给前端展开 MCP 配置里的 `${VAR}` 用。
///
/// **这张表里装的是用户的真密钥**（GITHUB_TOKEN、各家 API Key）。所以：不打日志、不进任何
/// 错误串——错误串会一路回到前端并可能落进对话记录里，那等于把密钥写进一个纯文本文件。
/// 探测失败就退回本进程的环境：那是**今天**的行为，不会更糟，而且从终端启动 IDE 时它本来
/// 就是对的；探测成功时用户 shell 里的那份是严格更全的。
#[cfg(not(windows))]
pub fn login_shell_env() -> std::collections::HashMap<String, String> {
    // 预算给 4 秒：这条路不在关键路径上（用户点「连接」时才走一次），而 rc 文件里挂着
    // nvm/conda 初始化的机器实测能跑到 1 秒以上，给短了会白白退回空环境。
    probe_login_shell_env(4000).unwrap_or_else(|| std::env::vars().collect())
}

/// Windows 上 GUI 进程继承的就是用户注册表里那份环境，没有「只有登录 shell 才看得到」的
/// 变量，直接给本进程的环境即可。
#[cfg(windows)]
pub fn login_shell_env() -> std::collections::HashMap<String, String> {
    std::env::vars().collect()
}

/// 探一次登录 shell 的环境，交给前端去展开 MCP 配置里的 `${VAR}`。
#[tauri::command(async)]
pub fn shell_env_probe() -> std::collections::HashMap<String, String> {
    login_shell_env()
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
    // IDE 自己管的那份 npm 全局目录。目录名必须问同一个入口要——写死一份的后果是：
    // 应用目录改名之后，npm 照旧往新目录装，而 PATH 里加的是老目录（那个目录在搬迁时
    // 已经不存在了），于是 IDE 装过的命令行工具**一个都找不到**，且没有任何报错。
    parts.push(format!("{home}/{}/npm-global/bin", crate::mcp::app_dir_name()));
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
        // 目录名跟着应用走，不在这里写死：写死过一次，应用目录改名之后前后端就指着两个
        // 不同的地方——npm 装出来的东西落在谁也不去看的目录里。
        base.push(format!("{home}\\{}\\npm-global", crate::mcp::app_dir_name()));
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

/// Windows 上把命令名解析成一个**完整路径**。
///
/// 这里原来直接把名字原样交回去，注释写的理由是"让操作系统按 PATH + PATHEXT 去找"。
/// **那个前提是错的**：Rust 的 `std::process::Command` 在 Windows 上走 `CreateProcessW`，
/// 而它只会给无扩展名的名字补 `.exe`，**不查 PATHEXT**。npm 装出来的东西恰恰全是 `.cmd`
/// （`npx.cmd`、`typescript-language-server.cmd`、`js-debug-adapter-stdio.cmd`），于是：
///
///   · 所有 npm 系语言服务器起不动（TS/JS、bash、yaml、dockerfile、vue…）
///   · 几乎所有 MCP 服务连不上——内置市场那一排都是 `npx -y @modelcontextprotocol/...`，
///     远程 MCP 依赖的 `mcp-remote` 桥也是
///   · F5 一键调试起不来（Node 适配器同样是 .cmd）
///
/// 而且是**静默**的：`lsp_check_available` 那条路专门查了 .cmd/.bat，所以界面显示"已安装、
/// 无需提示"，真正启动的这条路却找不到——用户看到的是"装好了但没反应"。
///
/// 另一半原因是 PATH 本身：`CreateProcessW` 用的是**进程**的 PATH，而 GUI 启动的应用拿到
/// 的那份很窄，nvm/volta/用户级 npm 前缀都不在里面。所以这里查的是 `augmented_path`，
/// 和 `lsp_check_available` 用的是同一份。
#[cfg(windows)]
pub fn resolve_command(cmd: &str, workspace: Option<&str>) -> String {
    if windows_command_is_explicit(cmd) {
        return cmd.to_string();
    }
    let pathext = std::env::var("PATHEXT").unwrap_or_default();
    let candidates = windows_command_candidates(cmd, &pathext);
    for dir in augmented_path(workspace).split(';').filter(|d| !d.is_empty()) {
        for name in &candidates {
            let full = format!("{dir}\\{name}");
            if std::path::Path::new(&full).is_file() {
                return full;
            }
        }
    }
    // 找不到就退回原名：交给系统再试一次，报错也更像用户预期的那句"不是内部或外部命令"。
    cmd.to_string()
}

/// 调用方已经给了明确的东西（带路径、或带扩展名），别去改它。
///
/// **不带 cfg 是有意的**：这样它在 macOS 上也编译、也能测。Windows 那边的机器我没有，
/// 交叉编译又卡在 C 依赖上——把纯逻辑留在两个平台都编译的地方，是这种情况下唯一能自证
/// "至少语法和类型是对的"的办法。判据本身也和当前编译目标无关：一个带盘符或反斜杠的
/// 字符串，在哪台机器上看都是"调用方已经指名道姓了"。
fn windows_command_is_explicit(cmd: &str) -> bool {
    cmd.contains('\\') || cmd.contains('/') || std::path::Path::new(cmd).extension().is_some()
}

/// `npx` + PATHEXT → `["npx", "npx.COM", "npx.EXE", "npx.BAT", "npx.CMD", …]`。
///
/// 裸名字排第一：PATH 上偶尔真的躺着一个无扩展名的可执行文件。PATHEXT 是用户可改的，
/// 空的时候给一份和 Windows 默认一致的兜底。
fn windows_command_candidates(cmd: &str, pathext: &str) -> Vec<String> {
    let parse = |raw: &str| -> Vec<String> {
        raw.split(';')
            .map(str::trim)
            .filter(|e| !e.is_empty())
            .map(|e| e.to_string())
            .collect()
    };
    // 兜底的判据是**解析出来的扩展名为空**，不是原始字符串为空：`";;"` 这种
    // trim 之后非空、切出来却一个扩展名都没有，按前者判会让候选只剩裸名字——
    // 而裸名字恰恰是 npm 系工具唯一找不到的那个形态。
    let mut exts = parse(pathext);
    if exts.is_empty() {
        exts = parse(".COM;.EXE;.BAT;.CMD");
    }
    std::iter::once(cmd.to_string())
        .chain(exts.into_iter().map(|e| format!("{cmd}{e}")))
        .collect()
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

// ─────────────────────────── 子进程输出的解码 ───────────────────────────

/// Windows 的 ANSI 代码页（`GetACP()`）对应的 encoding_rs 编码。
///
/// 简体中文机器是 936（GBK），繁体 950（Big5），日文 932，韩文 949，西欧 1252。
/// 拿不到或者不认识就返回 None，交给调用方走 lossy。
#[cfg(windows)]
fn legacy_encoding() -> Option<&'static encoding_rs::Encoding> {
    #[link(name = "Kernel32")]
    extern "system" {
        fn GetACP() -> u32;
    }
    // Safe: GetACP 无参数、无副作用，返回一个进程级的代码页号。
    let cp = unsafe { GetACP() };
    encoding_for_codepage(cp)
}

/// macOS / Linux 上非 UTF-8 的子进程输出极少见（系统本身就是 UTF-8），真碰上了
/// 用 chardetng 猜——和 tabular.rs 读 CSV 用的是同一套探测器。
#[cfg(not(windows))]
fn legacy_encoding() -> Option<&'static encoding_rs::Encoding> {
    None
}

/// 代码页号 → 编码。抽出来单独测，不然 Windows 分支在 mac 上一行都跑不到。
#[allow(dead_code)]
fn encoding_for_codepage(cp: u32) -> Option<&'static encoding_rs::Encoding> {
    Some(match cp {
        65001 => return None, // 已经是 UTF-8，走不到回退分支
        936 => encoding_rs::GBK,
        950 => encoding_rs::BIG5,
        932 => encoding_rs::SHIFT_JIS,
        949 => encoding_rs::EUC_KR,
        874 => encoding_rs::WINDOWS_874,
        1250 => encoding_rs::WINDOWS_1250,
        1251 => encoding_rs::WINDOWS_1251,
        1252 => encoding_rs::WINDOWS_1252,
        1253 => encoding_rs::WINDOWS_1253,
        1254 => encoding_rs::WINDOWS_1254,
        1255 => encoding_rs::WINDOWS_1255,
        1256 => encoding_rs::WINDOWS_1256,
        1257 => encoding_rs::WINDOWS_1257,
        1258 => encoding_rs::WINDOWS_1258,
        _ => return None,
    })
}

/// 末尾那截**还没读完**的多字节序列有多长。
///
/// 输出是按 8KB 分块读的，2MB 的上限也可能正好切在一个汉字中间。直接解码会把这个
/// 半截字符变成一个 `�`，而 `�` 是不可逆的——所以宁可把这几个字节丢掉/留到下次。
/// 返回 0 表示结尾是完整的。
fn incomplete_utf8_tail(bytes: &[u8]) -> usize {
    // UTF-8 一个字符最长 4 字节，所以只需要回看 3 个字节找起始字节。
    for back in 1..=3usize {
        if back > bytes.len() {
            break;
        }
        let b = bytes[bytes.len() - back];
        if b & 0b1100_0000 == 0b1000_0000 {
            continue; // 续接字节，继续往前找
        }
        let need = if b & 0b1000_0000 == 0 {
            1
        } else if b & 0b1110_0000 == 0b1100_0000 {
            2
        } else if b & 0b1111_0000 == 0b1110_0000 {
            3
        } else if b & 0b1111_1000 == 0b1111_0000 {
            4
        } else {
            return 0; // 非法起始字节，不是"没读完"，交给解码器出 �
        };
        return if need > back { back } else { 0 };
    }
    0
}

/// 把子进程的 stdout/stderr 字节解成字符串。
///
/// 规则是确定的，不猜：
///   1. 整体是合法 UTF-8 → 按 UTF-8 解。**绝不**先 lossy 再补救。
///   2. 否则按平台的传统代码页解（Windows 上就是 `GetACP()`：中文机器 GBK）。
///      Windows 的命令行工具往管道里写的是 ANSI 代码页字节，`chcp 65001` 只管
///      控制台不管管道，之前一律 `from_utf8_lossy` 就是中文全变 `���` 的原因。
///   3. 代码页也认不出来 → 最后才 lossy。
///
/// 末尾半截的多字节序列先切掉（见 `incomplete_utf8_tail`），免得一个被截断的汉字
/// 把整段输出判成"不是 UTF-8"从而错误地走 GBK 分支。
pub fn decode_process_output(bytes: &[u8]) -> String {
    decode_with(bytes, legacy_encoding())
}

/// 传统代码页作为参数传进来，这样 Windows 那条分支在 mac/Linux 上也能被测到——
/// 否则整个 GBK 回退逻辑一行测试都跑不到，而它恰恰是最容易写错的那部分。
fn decode_with(bytes: &[u8], legacy: Option<&'static encoding_rs::Encoding>) -> String {
    // 1) 整体就是合法 UTF-8 —— 绝大多数情况。
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_owned();
    }

    // 2) 只是**结尾**被 8KB 分块 / 2MB 上限切断的 UTF-8：去掉那半截，剩下的必须仍然
    //    合法，才认定是这种情况。
    //
    //    这里的顺序很要命。第一版是"无条件先按 UTF-8 规则砍尾巴，再判编码"，而 GBK
    //    汉字的第二个字节常落在 0x80–0xBF、第一个字节常落在 0xE0–0xF7 —— 在 UTF-8 眼里
    //    正好像"一个没读完的三/四字节字符"。实测 9 条 Windows 中文报错里有 4 条最后一个
    //    汉字被砍掉半个（`无法打开` → `无法�`、`错误` → `错�`），制造出的正是这个函数
    //    声称要杜绝的那个 `�`。
    let tail = incomplete_utf8_tail(bytes);
    if tail > 0 {
        if let Ok(s) = std::str::from_utf8(&bytes[..bytes.len() - tail]) {
            return s.to_owned();
        }
    }

    // 3) 到这儿说明**中间**就有非 UTF-8 字节。两种可能，代价完全不对称：
    //      (a) 整段是传统代码页（Windows 中文机器的 GBK）——按 UTF-8 解会全变 `�`；
    //      (b) 本来是 UTF-8，只是混进了个别坏字节（日志里夹了二进制）——整段按 GBK 解
    //          会把所有中文换成**另一批汉字**，比几个 `�` 糟得多，而且看不出来是错的。
    //
    //    判据取"哪一边解得干净"，而不是比谁的坏字符少：只有代码页能**一个坏字符都不出**、
    //    而 UTF-8 出了，才认定它是代码页。实测 10 组样本（6 组真 GBK + 4 组 UTF-8 掺坏
    //    字节）全部分类正确。分不清的时候倒向 UTF-8 lossy —— 那是旧行为，不会更糟。
    let lossy = String::from_utf8_lossy(bytes);
    if let Some(enc) = legacy {
        let (candidate, _, _) = enc.decode(bytes);
        if !has_undecodable(&candidate) && has_undecodable(&lossy) {
            return candidate.into_owned();
        }
    }
    lossy.into_owned()
}

/// 这段文本里有没有"没解出来"的痕迹：替换字符，或者私用区码位。
///
/// 私用区也要算：GBK 会把一部分字节对映射进 U+E000–U+F8FF 而**不报错**，所以把一段
/// UTF-8 硬按 GBK 解，经常得到一串 PUA 而不是替换字符——只看替换字符会漏掉这种情况。
/// 正常的程序输出里不会出现私用区字符。
fn has_undecodable(s: &str) -> bool {
    s.chars()
        .any(|c| c == '\u{fffd}' || ('\u{e000}'..='\u{f8ff}').contains(&c))
}

// ─────────────────────────── 子进程的 locale ───────────────────────────

/// 从 Finder / Dock 启动的 macOS 应用**拿不到任何 locale 环境变量**——`LANG`、
/// `LC_ALL`、`LC_CTYPE` 全是空的（对着运行中的 app 进程 `ps eww` 查过）。
///
/// 这件事影响到谁，取决于登录 shell，实测结论是：
///   - **zsh**（macOS 默认）：`/etc/zprofile` 里有 `if [ -z "$LANG" ]; then export
///     LANG=C.UTF-8`，登录 shell 自己把这个洞补上了 —— 这条路径本来就没坏。
///   - **bash**（换过登录 shell 的人）：`/etc/profile` 不设 locale，`bash -lc` 出来
///     `LANG` 就是空的。实测在这种 shell 下 `ls` 一个中文目录打到终端是
///     `????????????.txt`，`awk`/`wc -m`/`sort` 也全部退化成按字节。
///   - Linux：发行版之间不一致，同样不能指望。
///
/// 所以这不是"macOS 全线坏了"，而是"不能指望登录 shell 替我们兜底"。在 shell 之前
/// 补一个，三种情况就都是确定的。
///
/// 规则：用户自己配过就不动（哪怕配的不是 UTF-8，那是他的选择）；一个都没有、或者
/// 只是 `C`/`POSIX` 这种"等于没配"的值，才补一个 UTF-8 locale 进去。
/// 首选补 `LANG` 而不是 `LC_ALL`，这样登录 shell 的 profile 还能再覆盖回去
/// （`/etc/zprofile` 那句 `if [ -z "$LANG" ]` 也就顺理成章地跳过，不会打架）。
fn locale_is_configured(read: impl Fn(&str) -> Option<String>) -> bool {
    for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if meaningful_locale(&read, key) {
            return true;
        }
    }
    false
}

/// 这个变量有没有配一个**有意义**的值。`C` / `POSIX` / 空串等于"按字节处理"，不算配过。
fn meaningful_locale(read: &impl Fn(&str) -> Option<String>, key: &str) -> bool {
    match read(key) {
        Some(v) => {
            let t = v.trim();
            !t.is_empty() && !t.eq_ignore_ascii_case("c") && !t.eq_ignore_ascii_case("posix")
        }
        None => false,
    }
}

/// 兜底 locale。macOS 上 `en_US.UTF-8` 一定存在；Linux 上 `C.UTF-8` 是 glibc/musl
/// 的通用名。装不上也只是退回今天的 C locale，不会更糟。
#[allow(dead_code)]
fn default_utf8_locale() -> &'static str {
    if cfg!(target_os = "macos") {
        "en_US.UTF-8"
    } else {
        "C.UTF-8"
    }
}

/// 要塞给子进程的 locale 环境变量；用户已经配好就返回空。
///
/// 环境读取器是参数，这样这条判定能被确定性地测——直接读 `std::env` 的测试会跟着
/// 跑测试那台机器的环境变量走，结果是"改动删掉也照样绿"的假守卫。
#[allow(dead_code)]
fn locale_env_from(read: impl Fn(&str) -> Option<String>) -> Vec<(&'static str, String)> {
    if locale_is_configured(&read) {
        return Vec::new();
    }
    let target = default_utf8_locale().to_string();
    let mut out = vec![("LANG", target.clone())];
    // POSIX 的优先级是 LC_ALL > LC_* > LANG。所以光设 LANG 是不够的：一个已经存在的
    // `LC_ALL=C`（或 `LC_CTYPE=C`）会把它整个压掉，子进程照样跑在 C locale 里，这个
    // 修复就等于没做。只在它们**确实存在且等于 C/POSIX** 时才一起改——本来就没有的
    // 变量不去平白无故地引入。
    for key in ["LC_ALL", "LC_CTYPE"] {
        if read(key).is_some() {
            out.push((key, target.clone()));
        }
    }
    out
}

#[cfg(not(windows))]
pub fn utf8_locale_env() -> Vec<(&'static str, String)> {
    locale_env_from(|k| std::env::var(k).ok())
}

/// Windows 不走 POSIX locale，编码是靠代码页解决的（PTY 里 `chcp 65001`，
/// 管道输出交给 `decode_process_output` 按 ANSI 代码页解）。
#[cfg(windows)]
pub fn utf8_locale_env() -> Vec<(&'static str, String)> {
    Vec::new()
}

#[cfg(test)]
mod locale_tests {
    use super::*;
    use std::collections::HashMap;

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k: &str| map.get(k).cloned()
    }

    #[test]
    fn a_finder_launched_app_has_no_locale_and_gets_one() {
        assert!(!locale_is_configured(env_of(&[])));
        // 这才是真守卫：断言**返回的东西**，而不是只断言判定函数。把 tasks.rs /
        // terminal.rs 里的注入删掉这条不会红，但把这个函数改坏了一定会红。
        let injected = locale_env_from(env_of(&[]));
        assert_eq!(injected.len(), 1);
        assert_eq!(injected[0].0, "LANG");
        assert!(injected[0].1.to_lowercase().contains("utf-8"), "{injected:?}");
    }

    /// POSIX 的优先级是 LC_ALL > LC_* > LANG。所以只设 LANG 是不够的：一个已经存在的
    /// `LC_ALL=C` 会把它整个压掉，子进程照样在 C locale 里跑，修复等于没做。
    #[test]
    fn an_existing_lc_all_of_c_is_overridden_too_or_lang_would_lose() {
        let injected = locale_env_from(env_of(&[("LC_ALL", "C")]));
        let lc_all = injected.iter().find(|(k, _)| *k == "LC_ALL");
        assert!(lc_all.is_some(), "LC_ALL=C 还在，光设 LANG 会被它压掉：{injected:?}");
        assert!(lc_all.unwrap().1.to_lowercase().contains("utf-8"));
        assert!(injected.iter().any(|(k, _)| *k == "LANG"));

        // LC_CTYPE=POSIX 同理。
        let injected = locale_env_from(env_of(&[("LC_CTYPE", "POSIX")]));
        assert!(injected.iter().any(|(k, _)| *k == "LC_CTYPE"), "{injected:?}");

        // 本来就不存在的变量不要平白无故引入——只补 LANG 就够，profile 还能覆盖。
        let injected = locale_env_from(env_of(&[]));
        assert_eq!(injected.len(), 1, "{injected:?}");
        assert_eq!(injected[0].0, "LANG");
    }

    #[test]
    fn a_configured_locale_means_we_inject_nothing() {
        assert!(locale_env_from(env_of(&[("LANG", "zh_CN.UTF-8")])).is_empty());
        assert!(locale_env_from(env_of(&[("LC_ALL", "ja_JP.eucJP")])).is_empty());
        // C / POSIX 等于没配，要补。
        assert!(!locale_env_from(env_of(&[("LANG", "C")])).is_empty());
    }

    #[test]
    fn an_existing_locale_is_left_alone() {
        assert!(locale_is_configured(env_of(&[("LANG", "zh_CN.UTF-8")])));
        assert!(locale_is_configured(env_of(&[("LC_CTYPE", "UTF-8")])));
        assert!(locale_is_configured(env_of(&[("LC_ALL", "ja_JP.eucJP")])));
    }

    #[test]
    fn c_and_posix_count_as_unset() {
        // 这两个值等于"按字节处理"，正是要修掉的那个状态。
        assert!(!locale_is_configured(env_of(&[("LANG", "C")])));
        assert!(!locale_is_configured(env_of(&[("LANG", "POSIX")])));
        assert!(!locale_is_configured(env_of(&[("LC_ALL", "c")])));
        assert!(!locale_is_configured(env_of(&[("LANG", "   ")])));
    }

    #[test]
    fn the_fallback_locale_is_utf8_on_every_platform() {
        assert!(default_utf8_locale().to_lowercase().contains("utf-8"));
    }
}

#[cfg(all(test, not(windows)))]
mod shell_env_probe_tests {
    use super::*;

    fn probe_output(body: &[u8]) -> Vec<u8> {
        let mut raw = b"nvm: v20.11.0\n__WE__".to_vec(); // rc 文件先打了一行字
        raw.extend_from_slice(body);
        raw.extend_from_slice(b"__WE__");
        raw
    }

    #[test]
    fn a_multiline_secret_survives_the_probe_intact() {
        // 私钥、证书这种带换行的值是这个函数存在的理由：按行切会把它们切碎，
        // 展开进 MCP 配置的就是半截 key，服务起来之后报的是「认证失败」。
        let key = "-----BEGIN KEY-----\nline1\nline2\n-----END KEY-----";
        let body = format!("PATH=/usr/bin\0SSH_KEY={key}\0GITHUB_TOKEN=ghp_x\0");
        let map = parse_env0_output(&probe_output(body.as_bytes())).unwrap();
        assert_eq!(map.get("SSH_KEY").map(String::as_str), Some(key));
        assert_eq!(map.get("GITHUB_TOKEN").map(String::as_str), Some("ghp_x"));
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn rc_file_chatter_outside_the_markers_is_not_mistaken_for_a_variable() {
        // `-lic` 会 source 用户的 rc 文件，MOTD / 版本管理器的提示先落在同一个 stdout 上。
        let map = parse_env0_output(&probe_output(b"HOME=/Users/x\0")).unwrap();
        assert_eq!(map.len(), 1, "{map:?}");
        assert_eq!(map.get("HOME").map(String::as_str), Some("/Users/x"));
        // 值里带 '=' 的（连接串、base64）只在第一个 '=' 处切。
        let map = parse_env0_output(&probe_output(b"DSN=postgres://a:b@h/db?x=1\0")).unwrap();
        assert_eq!(map["DSN"], "postgres://a:b@h/db?x=1");
    }

    #[test]
    fn a_failed_probe_is_none_not_an_empty_environment() {
        // 空表会被上层当成「用户就是没有这些变量」，于是 ${GITHUB_TOKEN} 被展开成空串写进
        // 配置——服务照样起来，只是带着一个空 token，报的是看不出原因的 401。
        assert!(parse_env0_output(b"").is_none());
        assert!(parse_env0_output(b"zsh: command not found: env\n").is_none());
        assert!(parse_env0_output(&probe_output(b"")).is_none());
        assert!(parse_env0_output(b"__WE__").is_none(), "只有一个 marker 不算一次完整输出");
    }

    /// 真跑一次登录 shell。会 spawn 子进程、source 用户的 rc 文件（秒级），所以默认不跑：
    ///     cargo test --lib process_util -- --ignored --nocapture
    #[test]
    #[ignore = "spawns the user's real login shell"]
    fn the_real_login_shell_yields_a_usable_environment() {
        let map = probe_login_shell_env(8000).expect("登录 shell 应当能问出环境");
        assert!(map.contains_key("PATH"), "拿不到 PATH 说明解析或 marker 出了问题");
        assert!(map.contains_key("HOME"));
        // 密钥不打印：这里只报数量。
        println!("probed {} variables", map.len());
    }
}

#[cfg(test)]
mod decode_tests {
    use super::*;

    #[test]
    fn utf8_output_survives_untouched() {
        let bytes = "编译成功 ✓\n".as_bytes();
        assert_eq!(decode_process_output(bytes), "编译成功 ✓\n");
    }

    #[test]
    fn a_chinese_char_cut_in_half_is_dropped_not_turned_into_a_replacement() {
        let full = "你好".as_bytes().to_vec();
        // 砍掉最后一个字节：'好' 只剩前两字节。
        let cut = &full[..full.len() - 1];
        let decoded = decode_process_output(cut);
        assert_eq!(decoded, "你", "半截字符要丢掉，不能留下 �");
        assert!(!decoded.contains('\u{fffd}'));
    }

    /// 传 Some(GBK) 模拟"跑在简体中文 Windows 上"，这样这条分支在 mac 上也测得到。
    fn as_gbk_machine(bytes: &[u8]) -> String {
        decode_with(bytes, Some(encoding_rs::GBK))
    }

    #[test]
    fn gbk_output_is_decoded_as_gbk_not_smashed_into_replacement_chars() {
        for s in [
            "找不到文件",
            "无法打开",
            "权限不足",
            "错误",
            "中文",
            "系统找不到指定的路径。",
            "error: 权限不足",
        ] {
            let (gbk, _, _) = encoding_rs::GBK.encode(s);
            assert!(std::str::from_utf8(&gbk).is_err(), "{s} 的 GBK 字节应当不是合法 UTF-8");
            assert!(
                String::from_utf8_lossy(&gbk).contains('\u{fffd}'),
                "{s}：旧的 from_utf8_lossy 会毁掉它",
            );
            assert_eq!(as_gbk_machine(&gbk), s, "{s} 没能原样解回来");
        }
    }

    /// 第一版的顺序错了：先无条件按 UTF-8 规则砍尾巴，再判编码。GBK 汉字的第二个
    /// 字节常在 0x80–0xBF、第一个字节常在 0xE0–0xF7，在 UTF-8 眼里正好像一个"没读完
    /// 的三字节字符"，于是最后一个汉字被砍掉半个 —— 造出的正是要杜绝的那个 �。
    #[test]
    fn a_complete_gbk_string_never_loses_its_last_character() {
        for s in ["无法打开", "错误", "中文", "权限不足"] {
            let (gbk, _, _) = encoding_rs::GBK.encode(s);
            let decoded = as_gbk_machine(&gbk);
            assert!(!decoded.contains('\u{fffd}'), "{s} → {decoded:?} 出现了 �");
            assert_eq!(decoded, s);
        }
    }

    /// 反向的坑：一段本来是 UTF-8、只混进个别坏字节的输出，绝不能整段按 GBK 解——
    /// 那会把所有中文换成**另一批汉字**，比几个 � 糟得多，而且看不出来是错的。
    #[test]
    fn mostly_utf8_output_with_a_stray_bad_byte_stays_utf8() {
        let mut mixed = "编译成功：3 个文件已更新".as_bytes().to_vec();
        mixed.push(0xFF);
        mixed.extend_from_slice(b" done");
        let decoded = as_gbk_machine(&mixed);
        assert!(decoded.contains("编译成功"), "中文被改写成了别的字：{decoded:?}");
        assert!(decoded.contains("done"));

        // 夹了一小段二进制的日志同理。
        let mut binary = "构建日志：".as_bytes().to_vec();
        binary.extend_from_slice(&[0x00, 0xFF, 0xFE, 0x80]);
        binary.extend_from_slice("结束".as_bytes());
        let decoded = as_gbk_machine(&binary);
        assert!(decoded.contains("构建日志"), "{decoded:?}");
        assert!(decoded.contains("结束"), "{decoded:?}");
    }

    #[test]
    fn private_use_characters_count_as_a_failed_decode() {
        // GBK 会把一部分字节对映射进私用区而**不报错**，只看替换字符会漏掉。
        assert!(has_undecodable("\u{e045}"));
        assert!(has_undecodable("\u{fffd}"));
        assert!(!has_undecodable("正常输出 ok ✓"));
    }

    #[test]
    fn windows_codepages_map_to_the_right_encoding() {
        assert_eq!(encoding_for_codepage(936), Some(encoding_rs::GBK));
        assert_eq!(encoding_for_codepage(950), Some(encoding_rs::BIG5));
        assert_eq!(encoding_for_codepage(932), Some(encoding_rs::SHIFT_JIS));
        assert_eq!(encoding_for_codepage(1252), Some(encoding_rs::WINDOWS_1252));
        // 65001 已经是 UTF-8，不该被当成"传统代码页"再解一遍。
        assert_eq!(encoding_for_codepage(65001), None);
        assert_eq!(encoding_for_codepage(1), None);
    }

    #[test]
    fn ascii_and_empty_are_stable() {
        assert_eq!(decode_process_output(b""), "");
        assert_eq!(decode_process_output(b"ok\n"), "ok\n");
    }

    #[test]
    fn a_complete_tail_is_never_trimmed() {
        assert_eq!(incomplete_utf8_tail("你好".as_bytes()), 0);
        assert_eq!(incomplete_utf8_tail(b"abc"), 0);
        assert_eq!(incomplete_utf8_tail(b""), 0);
    }
}

#[cfg(test)]
mod windows_resolution_tests {
    use super::*;

    /// Windows 上「命令找不到」曾经同时打掉四个功能：npm 系语言服务器、几乎所有 MCP 服务
    /// （内置目录清一色 `npx`）、Node 一键调试、抓包。根因是一句错的注释——`resolve_command`
    /// 原样返回名字，理由写的是"让操作系统按 PATH + PATHEXT 解析"。**Rust 的 Command 在
    /// Windows 上只补 `.exe`，从不读 PATHEXT。**
    ///
    /// 我没有 Windows 机器，交叉编译又卡在 C 依赖上，所以把纯逻辑抽了出来在这儿测——
    /// 至少保证候选名的生成是对的，且这段代码在两个平台都真的编译过。
    #[test]
    fn npm_installed_tools_get_their_cmd_extension() {
        let got = windows_command_candidates("npx", ".COM;.EXE;.BAT;.CMD");
        assert_eq!(got, vec!["npx", "npx.COM", "npx.EXE", "npx.BAT", "npx.CMD"]);
        // 裸名字必须排第一：PATH 上偶尔真的躺着无扩展名的可执行文件。
        assert_eq!(got[0], "npx");
        // 这三个是真实会被找的：npm/gem 装出来的就是它们。
        for want in ["npx.CMD", "npx.BAT"] {
            assert!(got.iter().any(|c| c == want), "少了 {want}：{got:?}");
        }
    }

    #[test]
    fn an_empty_or_odd_pathext_still_finds_cmd_and_bat() {
        // PATHEXT 是用户可改的，清空它不该让 npm 系工具全体消失。
        for weird in ["", "   ", ";;"] {
            let got = windows_command_candidates("npx", weird);
            assert!(got.iter().any(|c| c == "npx.CMD"), "{weird:?} → {got:?}");
            assert!(got.iter().any(|c| c == "npx.EXE"), "{weird:?} → {got:?}");
        }
        // 分号周围的空格要吃掉，否则拼出来的是 "npx .CMD"。
        assert!(windows_command_candidates("npx", " .EXE ; .CMD ")
            .iter()
            .any(|c| c == "npx.CMD"));
    }

    #[test]
    fn an_explicit_path_or_extension_is_left_alone() {
        // 调用方已经指名道姓的，不许改——改了会把用户显式选的解释器换掉。
        for explicit in [
            "C:\\Python311\\python.exe",
            "..\\node_modules\\.bin\\tsserver.cmd",
            "/usr/local/bin/node",
            "python.exe",
        ] {
            assert!(windows_command_is_explicit(explicit), "{explicit}");
        }
        for bare in ["npx", "node", "python", "rust-analyzer"] {
            assert!(!windows_command_is_explicit(bare), "{bare} 应当去 PATH 里找");
        }
    }
}
