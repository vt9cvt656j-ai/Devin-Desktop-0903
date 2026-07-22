//! Shared utilities for LSP and DAP child process management.

/// The user's REAL login-shell PATH, captured ONCE (cached). A GUI-launched app inherits a
/// minimal PATH that misses everything a version manager sets up in `.zshrc`/`.zprofile` —
/// nvm's `~/.nvm/versions/node/<v>/bin`, volta, pyenv, asdf shims, a custom `npm -g` prefix,
/// pipx, etc. So a language server the user (or the AI) installed is invisible → false "缺少
/// 语言服务器" prompts and installs that "succeed" but still can't be found. Running the login
/// shell and reading its `$PATH` makes the IDE resolve exactly what the user's terminal does.
#[cfg(not(windows))]
fn login_shell_path() -> &'static str {
    static CACHE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
        // Run in a thread with a timeout so a slow/pathological rc file can never wedge the app.
        // Wrap $PATH in markers so any `.zshrc` echo/MOTD noise is trivially stripped. `-l -i -c`
        // sources login + interactive rc (where version managers live). `command printf` dodges
        // aliases/functions named printf.
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let out = std::process::Command::new(&shell)
                .args(["-lic", "command printf '__WP__%s__WP__' \"$PATH\""])
                .output();
            let _ = tx.send(
                out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                    .unwrap_or_default(),
            );
        });
        let raw = rx
            .recv_timeout(std::time::Duration::from_millis(4000))
            .unwrap_or_default();
        match (raw.find("__WP__"), raw.rfind("__WP__")) {
            (Some(a), Some(b)) if b > a + 6 => {
                let p = &raw[a + 6..b];
                if p.contains('/') {
                    p.to_string()
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        }
    })
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
