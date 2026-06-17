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
    cmd.env("LSCOLORS", "ExGxFxDxCxegedabagaced");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("PROMPT_EOL_MARK", "");

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
