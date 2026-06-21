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

/// Maximum number of concurrent LSP or DAP processes allowed.
pub const MAX_CHILD_PROCESSES: usize = 16;
