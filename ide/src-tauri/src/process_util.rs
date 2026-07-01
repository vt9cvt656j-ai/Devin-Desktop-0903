//! Shared utilities for LSP and DAP child process management.

/// Build a PATH that includes the workspace's `node_modules/.bin` plus common
/// toolchain directories so project-local and user-installed tools resolve
/// even when the app is launched from a GUI with a minimal PATH.
#[cfg(not(windows))]
pub fn augmented_path(workspace: Option<&str>) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut parts: Vec<String> = Vec::new();
    if let Some(ws) = workspace.filter(|w| !w.is_empty()) {
        parts.push(format!("{ws}/node_modules/.bin"));
    }
    parts.push(format!("{home}/.cargo/bin"));
    parts.push("/opt/homebrew/bin".into());
    parts.push("/usr/local/bin".into());
    parts.push(format!("{home}/go/bin"));
    parts.push(format!("{home}/.local/bin"));
    parts.push("/usr/bin".into());
    parts.push("/bin".into());
    let extra = parts.join(":");
    match std::env::var("PATH") {
        Ok(p) if !p.is_empty() => format!("{extra}:{p}"),
        _ => extra,
    }
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

/// Windows: prepend the workspace's `node_modules\.bin` to the existing PATH.
#[cfg(windows)]
pub fn augmented_path(workspace: Option<&str>) -> String {
    let cur = std::env::var("PATH").unwrap_or_default();
    match workspace.filter(|w| !w.is_empty()) {
        Some(ws) if cur.is_empty() => format!("{ws}\\node_modules\\.bin"),
        Some(ws) => format!("{ws}\\node_modules\\.bin;{cur}"),
        None => cur,
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
