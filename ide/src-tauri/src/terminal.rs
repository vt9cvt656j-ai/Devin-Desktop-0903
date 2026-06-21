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
    /// A chunk of output from the shell (UTF-8, lossily decoded).
    Data { data: String },
    /// The shell process exited; the terminal is done.
    Exit,
}

struct Term {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

#[derive(Default)]
pub struct TerminalState {
    inner: Mutex<Inner>,
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
    if let Some(dir) = cwd.filter(|d| !d.is_empty()) {
        cmd.cwd(dir);
    }
    cmd.env("TERM", "xterm-256color");
    cmd.env("CLICOLOR", "1");
    cmd.env("CLICOLOR_FORCE", "1");
    cmd.env("LSCOLORS", "ExGxFxdaCxDaDahbadacec");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("PROMPT_EOL_MARK", "");
    cmd.env("LANG", "en_US.UTF-8");

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
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    if on_event.send(TermEvent::Data { data }).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
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
    for dir_str in path_str.split(':') {
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
    let home = match std::env::var_os("HOME") {
        Some(h) => std::path::PathBuf::from(h),
        None => return Vec::new(),
    };
    let mut raw: Vec<String> = Vec::new();
    for name in [".zsh_history", ".bash_history"] {
        let path = home.join(name);
        if let Ok(bytes) = std::fs::read(&path) {
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
