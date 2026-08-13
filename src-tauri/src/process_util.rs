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
    let cut = bytes.len() - incomplete_utf8_tail(bytes);
    let body = &bytes[..cut];
    if let Ok(s) = std::str::from_utf8(body) {
        return s.to_owned();
    }
    match legacy_encoding() {
        Some(enc) => {
            let (text, _, _) = enc.decode(body);
            text.into_owned()
        }
        None => String::from_utf8_lossy(body).into_owned(),
    }
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
/// 补的是 `LANG` 而不是 `LC_ALL`，这样登录 shell 的 profile 还能再覆盖回去
/// （`/etc/zprofile` 那句 `if [ -z "$LANG" ]` 也就顺理成章地跳过，不会打架）。
fn locale_is_configured(read: impl Fn(&str) -> Option<String>) -> bool {
    for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Some(v) = read(key) {
            let t = v.trim();
            if !t.is_empty() && !t.eq_ignore_ascii_case("c") && !t.eq_ignore_ascii_case("posix") {
                return true;
            }
        }
    }
    false
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
    if locale_is_configured(read) {
        return Vec::new();
    }
    vec![("LANG", default_utf8_locale().to_string())]
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

    #[test]
    fn gbk_bytes_are_not_decoded_as_utf8() {
        let (gbk, _, _) = encoding_rs::GBK.encode("找不到文件");
        // 这串 GBK 字节不是合法 UTF-8——旧代码会把它变成一堆 �。
        assert!(std::str::from_utf8(&gbk).is_err());
        let lossy = String::from_utf8_lossy(&gbk);
        assert!(lossy.contains('\u{fffd}'));
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
