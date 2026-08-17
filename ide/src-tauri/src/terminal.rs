//! Integrated terminal backed by a real PTY.
//!
//! Each terminal owns a pseudo-terminal running the user's login shell. Bytes
//! the shell writes are streamed to the frontend over a Tauri [`Channel`]; the
//! frontend (xterm.js) sends keystrokes back via [`term_write`] and window
//! resizes via [`term_resize`].

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;

/// Events streamed to the frontend for a single terminal session.
#[derive(Serialize, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TermEvent {
    /// A chunk of output from the shell, UTF-8 decoded on complete-char
    /// boundaries (multibyte chars split across PTY reads are reassembled).
    Data { data: String },
    /// The shell process exited; the terminal is done.
    Exit,
}

struct Term {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

// Kill the shell whenever a Term is dropped — so clearing the table (on reload /
// app exit) reaps the child process and its reader thread (which exits on EOF),
// instead of leaving zombie shells to pile up over a long session.
impl Drop for Term {
    fn drop(&mut self) {
        let _ = self.child.kill();
        // Reap it so the killed shell doesn't linger as a zombie (matches how the
        // LSP/DAP child processes are torn down).
        let _ = self.child.wait();
    }
}

#[derive(Default)]
pub struct TerminalState {
    // Arc 是为了让 PTY reader 线程也能拿到表：shell 自行退出时（用户敲 exit、服务被
    // Ctrl-C、进程崩了）由它把条目摘掉，从而触发 Term::drop 回收子进程和 PTY fd。
    // 在此之前只有 term_close / reset_all 会移除条目，也就是说**用户不手动关闭那个
    // 页签，僵尸进程和 fd 就一直挂着**。
    inner: Arc<Mutex<Inner>>,
}

impl TerminalState {
    /// Kill every shell and clear the table — reaps a previous page session on
    /// webview reload and on app exit.
    pub fn reset_all(&self) {
        // Drain + reap on a DETACHED thread — Term::drop does kill()+blocking wait() per
        // shell, which stalled the caller (boot cleanup_stale / app exit). Off-thread = instant.
        let drained: Vec<Term> = match self.inner.lock() {
            Ok(mut inner) => std::mem::take(&mut inner.terms).into_values().collect(),
            Err(_) => return,
        };
        if !drained.is_empty() {
            std::thread::spawn(move || drop(drained));
        }
    }
}

#[derive(Default)]
struct Inner {
    next_id: u32,
    terms: HashMap<u32, Term>,
}

fn default_shell() -> String {
    if cfg!(windows) {
        std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".into())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into())
    }
}

/// Spawn a new shell in a PTY and start streaming its output to `on_event`.
/// Returns an id used by the other `term_*` commands.
#[tauri::command(async)]
pub fn term_open(
    state: State<TerminalState>,
    cwd: Option<String>,
    cols: u16,
    rows: u16,
    on_event: Channel<TermEvent>,
) -> Result<u32, String> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: rows.max(1),
            cols: cols.max(1),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    let mut cmd = CommandBuilder::new(default_shell());
    // Windows: force the console to UTF-8 so Chinese / non-ASCII output isn't GBK(936)
    // mojibake (this terminal decodes bytes as UTF-8). cmd.exe → `/K chcp 65001`;
    // PowerShell → chcp + set [Console]::OutputEncoding (chcp alone doesn't fix PS output).
    #[cfg(windows)]
    {
        let sh = default_shell().to_lowercase();
        if sh.contains("powershell") || sh.contains("pwsh") {
            cmd.arg("-NoExit");
            cmd.arg("-Command");
            cmd.arg("chcp 65001 > $null; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $OutputEncoding = [System.Text.Encoding]::UTF8");
        } else {
            cmd.arg("/K");
            cmd.arg("chcp 65001>nul");
        }
    }
    if let Some(dir) = cwd.filter(|d| !d.is_empty()) {
        cmd.cwd(&dir);
        // Full toolchain PATH (a Finder-launched app inherits a minimal PATH) + auto-activate a
        // project venv, so `python`/`pip`/`pytest` in THIS terminal resolve INTO it. An AI-installed
        // env then persists across restarts instead of looking "lost" and getting reinstalled.
        cmd.env(
            "PATH",
            crate::process_util::augmented_path(Some(dir.as_str())),
        );
        for name in [".venv", "venv"] {
            let venv = std::path::Path::new(&dir).join(name);
            let has_venv = if cfg!(windows) {
                venv.join("Scripts/activate").exists() || venv.join("Scripts/activate.bat").exists()
            } else {
                venv.join("bin/activate").exists()
            };
            if has_venv {
                cmd.env("VIRTUAL_ENV", venv.to_string_lossy().to_string());
                cmd.env_remove("PYTHONHOME");
                break;
            }
        }
    }
    cmd.env(
        "TERM",
        if cfg!(windows) {
            "dumb"
        } else {
            "xterm-256color"
        },
    );
    // 以前这里无条件写死 en_US.UTF-8，把用户自己配的 zh_CN.UTF-8 / ja_JP.UTF-8 也一起
    // 顶掉了（排序、月份名、报错语言都会跟着变）。现在只在**一个 locale 都没有**时才补。
    //
    // 判定读的是本进程的环境，这是对的：CommandBuilder::new 的 envs 就是从
    // std::env::vars_os() 拷过来的（portable-pty 的 get_base_env），父进程有什么子进程
    // 就有什么——判定看到的和子进程实际拿到的是同一份。
    for (k, v) in crate::process_util::utf8_locale_env() {
        cmd.env(k, v);
    }
    #[cfg(not(windows))]
    {
        cmd.env("CLICOLOR", "1");
        cmd.env("CLICOLOR_FORCE", "1");
        cmd.env("LSCOLORS", "ExGxFxdaCxDaDahbadacec");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("PROMPT_EOL_MARK", "");
    }

    let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    // The slave handle is no longer needed once the child owns it; dropping it
    // lets the master observe EOF when the shell exits.
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    let terms_for_reader = state.inner.clone();
    let id = {
        let mut inner = state.inner.lock().map_err(|_| "terminal state poisoned")?;
        let id = inner.next_id;
        inner.next_id += 1;
        inner.terms.insert(
            id,
            Term {
                master: pair.master,
                writer,
                child,
            },
        );
        id
    };

    // Pump shell output to the frontend on a dedicated thread.
    //
    // Two concerns handled here:
    //  1) UTF-8 across read boundaries — a multibyte char (e.g. a 3-byte Chinese
    //     character) can be split between two PTY reads. Decoding each read on its
    //     own with `from_utf8_lossy` turns the split char into a `�` (the garbled-
    //     Chinese bug). So we keep a `carry` of the trailing incomplete bytes and
    //     only decode the complete UTF-8 prefix, carrying the tail to the next read.
    //  2) Flooding — a noisy command (build logs, `yes`, big cat) otherwise fires
    //     hundreds of events/sec, each a `term.write` on the main thread. We coalesce
    //     during a burst, but cap each batch modestly so a single huge `term.write`
    //     can't freeze xterm's parser (the old 256 KB cap caused visible stalls).
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        let mut carry: Vec<u8> = Vec::new();
        let mut pending = String::new();
        const MAX_BATCH: usize = 48 * 1024;
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    carry.extend_from_slice(&buf[..n]);
                    // Decode every complete UTF-8 char now; keep only an incomplete
                    // trailing char (if any) in `carry` for the next read.
                    loop {
                        match std::str::from_utf8(&carry) {
                            Ok(s) => {
                                pending.push_str(s);
                                carry.clear();
                                break;
                            }
                            Err(e) => {
                                let good = e.valid_up_to();
                                if good > 0 {
                                    // `good` bytes are valid UTF-8 → no replacement chars.
                                    pending.push_str(&String::from_utf8_lossy(&carry[..good]));
                                }
                                match e.error_len() {
                                    // Incomplete char at the end → wait for more bytes.
                                    None => {
                                        carry.drain(..good);
                                        break;
                                    }
                                    // Genuinely invalid bytes → emit one replacement, skip them.
                                    Some(bad) => {
                                        pending.push('\u{FFFD}');
                                        carry.drain(..good + bad);
                                    }
                                }
                            }
                        }
                    }
                    let bursting = n == buf.len();
                    if (!bursting || pending.len() >= MAX_BATCH)
                        && !pending.is_empty()
                        && on_event
                            .send(TermEvent::Data {
                                data: std::mem::take(&mut pending),
                            })
                            .is_err()
                    {
                        return;
                    }
                }
                Err(_) => break,
            }
        }
        if !pending.is_empty() {
            let _ = on_event.send(TermEvent::Data { data: pending });
        }
        if !carry.is_empty() {
            // Flush any leftover tail (a truncated char at EOF) lossily.
            let _ = on_event.send(TermEvent::Data {
                data: String::from_utf8_lossy(&carry).into_owned(),
            });
        }
        let _ = on_event.send(TermEvent::Exit);
        // shell 已经结束：摘掉条目，让 Term::drop 回收子进程（避免僵尸）和 PTY 主端 fd。
        // 前端收到 Exit 只是把页签标成"已退出"，Rust 侧的资源必须在这里自己释放。
        // Never run Term::drop while holding the table lock: Drop performs a
        // blocking child.wait(), and every UI terminal command needs this lock.
        // A slow shell shutdown used to make term_write/resize/close block the
        // Tauri event thread until the child finally exited.
        let finished = terms_for_reader
            .lock()
            .ok()
            .and_then(|mut inner| inner.terms.remove(&id));
        drop(finished);
    });

    Ok(id)
}

/// Forward keystrokes (or pasted text) to the shell.
#[tauri::command(async)]
pub fn term_write(state: State<TerminalState>, id: u32, data: String) -> Result<(), String> {
    let mut inner = state.inner.lock().map_err(|_| "terminal state poisoned")?;
    let term = inner.terms.get_mut(&id).ok_or("no such terminal")?;
    term.writer
        .write_all(data.as_bytes())
        .map_err(|e| e.to_string())?;
    term.writer.flush().map_err(|e| e.to_string())
}

/// Resize the PTY so the shell wraps output at the right width.
#[tauri::command(async)]
pub fn term_resize(
    state: State<TerminalState>,
    id: u32,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let inner = state.inner.lock().map_err(|_| "terminal state poisoned")?;
    let term = inner.terms.get(&id).ok_or("no such terminal")?;
    term.master
        .resize(PtySize {
            rows: rows.max(1),
            cols: cols.max(1),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())
}

/// Kill the shell and drop the session.
#[tauri::command(async)]
pub fn term_close(state: State<TerminalState>, id: u32) -> Result<(), String> {
    let removed = {
        let mut inner = state.inner.lock().map_err(|_| "terminal state poisoned")?;
        inner.terms.remove(&id)
    };
    if let Some(mut term) = removed {
        let _ = term.child.kill();
        // Term::drop waits for the child, but the table is already unlocked and
        // this command is dispatched away from the Tauri event thread.
        drop(term);
    }
    Ok(())
}

/// List every executable found on the user's `$PATH` (deduped, sorted).
/// Powers terminal autosuggestions so completion covers all installed tools,
/// not just a hand-written list. Uses augmented PATH to find tools installed
/// in ~/.local/bin, ~/.cargo/bin, etc. even from a Finder-launched app.
/// 哪些终端里**真的有命令在跑**（返回终端 id 列表）。
///
/// 标签页上那个 `▶` 只是"这个终端是智能体开的"，是**出身**不是**状态**：任务早就跑完了
/// 它还在，用户根本分不出哪个还在动。真状态得问 PTY。
///
/// Unix 上判据很直接：PTY 的前台进程组 != shell 自己的 pid，就说明 shell 让出了前台、
/// 有别的进程在跑。这个信号对谁敲的命令一视同仁——智能体派的任务和用户自己敲的 npm run
/// dev 都算，也不需要往用户命令里塞探针。dev server 这种"没有输出但确实在跑"的进程也能
/// 正确识别，靠输出活跃度猜是猜不出来的。
///
/// 一次返回全部，而不是一个终端一次调用：这是要按秒轮询的，N 次 IPC 没必要。
/// Windows 上 ConPTY 没有前台进程组这个概念，portable-pty 也就不提供
/// `process_group_leader`。这里返回空表，标签页退回到"出身"标记——比编不过强，
/// 也比在 Windows 上乱猜一个假状态强。
#[cfg(windows)]
#[tauri::command(async)]
pub fn term_running_ids(_state: State<TerminalState>) -> Vec<u32> {
    Vec::new()
}

#[cfg(not(windows))]
#[tauri::command(async)]
pub fn term_running_ids(state: State<TerminalState>) -> Vec<u32> {
    let Ok(inner) = state.inner.lock() else {
        return Vec::new();
    };
    inner
        .terms
        .iter()
        .filter_map(|(id, term)| {
            let shell = term.child.process_id()?;
            let fg = term.master.process_group_leader()?;
            // fg <= 0 表示拿不到（PTY 已经没有前台进程组），按"没在跑"处理。
            if fg > 0 && fg as u32 != shell {
                Some(*id)
            } else {
                None
            }
        })
        .collect()
}

/// 去掉 Windows 可执行扩展名之后的裸名（非 Windows 上恒为空）。
///
/// **不带 cfg**，两个平台都编译、都测得到：这段逻辑只在 Windows 上生效，而 Windows
/// 上跑不了这套测试——上一次同类改动就是因为整段被 cfg 挡住而没人验证过。
fn strip_windows_exec_ext(name: &str) -> Vec<String> {
    if !cfg!(windows) {
        return Vec::new();
    }
    windows_bare_names(name)
}

/// 纯逻辑：`npm.cmd` → `npm`。只认可执行扩展名，`README.md` 这种不动。
fn windows_bare_names(name: &str) -> Vec<String> {
    const EXEC_EXTS: &[&str] = &["exe", "cmd", "bat", "com", "ps1"];
    let Some((stem, ext)) = name.rsplit_once('.') else {
        return Vec::new();
    };
    if stem.is_empty() || !EXEC_EXTS.iter().any(|e| ext.eq_ignore_ascii_case(e)) {
        return Vec::new();
    }
    vec![stem.to_string()]
}

#[tauri::command(async)]
pub fn term_list_commands() -> Vec<String> {
    use std::collections::BTreeSet;
    let mut set: BTreeSet<String> = BTreeSet::new();
    let path_str = crate::process_util::augmented_path(None);
    let sep = if cfg!(windows) { ';' } else { ':' };
    for dir_str in path_str.split(sep) {
        let dir = std::path::PathBuf::from(dir_str);
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_dir() {
                        continue;
                    }
                }
                let name = match entry.file_name().into_string() {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                if name.starts_with('.') {
                    continue;
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(meta) = entry.metadata() {
                        if meta.permissions().mode() & 0o111 == 0 {
                            continue;
                        }
                    }
                }
                for stripped in strip_windows_exec_ext(&name) {
                    set.insert(stripped);
                }
                set.insert(name);
            }
        }
    }
    set.into_iter().collect()
}

/// Read the user's shell history (zsh or bash), returning recent commands
/// most-recent-first and deduped. Powers history-aware autosuggestions.
#[tauri::command(async)]
pub fn term_history() -> Vec<String> {
    let home = match std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }) {
        Some(h) => std::path::PathBuf::from(h),
        None => return Vec::new(),
    };
    let mut raw: Vec<String> = Vec::new();
    // Unix: zsh/bash history; Windows: PowerShell PSReadLine history
    #[cfg(windows)]
    let history_paths: Vec<std::path::PathBuf> = {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        if appdata.is_empty() {
            vec![]
        } else {
            vec![std::path::PathBuf::from(&appdata)
                .join("Microsoft/Windows/PowerShell/PSReadLine/ConsoleHost_history.txt")]
        }
    };
    #[cfg(not(windows))]
    let history_paths: Vec<std::path::PathBuf> = [".zsh_history", ".bash_history"]
        .iter()
        .map(|n| home.join(n))
        .collect();
    for path in &history_paths {
        if let Ok(bytes) = std::fs::read(path) {
            let text = String::from_utf8_lossy(&bytes);
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                // zsh extended history format: ": <ts>:<dur>;<cmd>"
                let cmd = if line.starts_with(':') {
                    line.find(';').map(|i| &line[i + 1..]).unwrap_or(line)
                } else {
                    line
                };
                let cmd = cmd.trim();
                if !cmd.is_empty() && cmd.len() <= 256 {
                    raw.push(cmd.to_string());
                }
            }
            if !raw.is_empty() {
                break;
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for cmd in raw.into_iter().rev() {
        if seen.insert(cmd.clone()) {
            out.push(cmd);
        }
        if out.len() >= 600 {
            break;
        }
    }
    out
}

#[cfg(test)]
mod windows_command_tests {
    /// Windows 上目录项带扩展名，而所有调用方查的都是裸名：`checkToolForLanguage`
    /// 用 `cmds.includes("rust-analyzer")` 判断装没装，于是不管装没装都弹
    /// 「缺少 X 语言服务器」；终端补全同理，敲 `np` 补不出 npm。
    /// 这段只在 Windows 生效但**不带 cfg**，所以 macOS 上照样测得到。
    #[test]
    fn windows_exec_names_also_appear_bare() {
        assert_eq!(super::windows_bare_names("rust-analyzer.exe"), vec!["rust-analyzer"]);
        assert_eq!(super::windows_bare_names("pyright-langserver.cmd"), vec!["pyright-langserver"]);
        assert_eq!(super::windows_bare_names("NPM.CMD"), vec!["NPM"]);
        // 非可执行扩展名不动，免得把 README.md 也塞成一条"命令"
        assert!(super::windows_bare_names("README.md").is_empty());
        assert!(super::windows_bare_names("gopls").is_empty());
    }
}
