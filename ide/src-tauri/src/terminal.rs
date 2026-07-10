//! Integrated terminal backed by a real PTY.
//!
//! Each terminal owns a pseudo-terminal running the user's login shell. Bytes
//! the shell writes are streamed to the frontend over a Tauri [`Channel`]; the
//! frontend (xterm.js) sends keystrokes back via [`term_write`] and window
//! resizes via [`term_resize`].

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Mutex;

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
    inner: Mutex<Inner>,
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
#[tauri::command]
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
    cmd.env("LANG", "en_US.UTF-8");
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
    });

    Ok(id)
}

/// Forward keystrokes (or pasted text) to the shell.
#[tauri::command]
pub fn term_write(state: State<TerminalState>, id: u32, data: String) -> Result<(), String> {
    let mut inner = state.inner.lock().map_err(|_| "terminal state poisoned")?;
    let term = inner.terms.get_mut(&id).ok_or("no such terminal")?;
    term.writer
        .write_all(data.as_bytes())
        .map_err(|e| e.to_string())?;
    term.writer.flush().map_err(|e| e.to_string())
}

/// Resize the PTY so the shell wraps output at the right width.
#[tauri::command]
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
#[tauri::command]
pub fn term_close(state: State<TerminalState>, id: u32) -> Result<(), String> {
    let mut inner = state.inner.lock().map_err(|_| "terminal state poisoned")?;
    if let Some(mut term) = inner.terms.remove(&id) {
        let _ = term.child.kill();
    }
    Ok(())
}

/// List every executable found on the user's `$PATH` (deduped, sorted).
/// Powers terminal autosuggestions so completion covers all installed tools,
/// not just a hand-written list. Uses augmented PATH to find tools installed
/// in ~/.local/bin, ~/.cargo/bin, etc. even from a Finder-launched app.
#[tauri::command]
pub fn term_list_commands() -> Vec<String> {
    use std::collections::BTreeSet;
    let mut set: BTreeSet<String> = BTreeSet::new();
    #[cfg(not(windows))]
    let path_str = crate::process_util::augmented_path(None);
    #[cfg(windows)]
    let path_str = std::env::var("PATH").unwrap_or_default();
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
                set.insert(name);
            }
        }
    }
    set.into_iter().collect()
}

/// Read the user's shell history (zsh or bash), returning recent commands
/// most-recent-first and deduped. Powers history-aware autosuggestions.
#[tauri::command]
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
