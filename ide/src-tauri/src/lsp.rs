use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::State;

use crate::process_util;

#[derive(Serialize, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[allow(dead_code)]
pub enum LspEvent {
    Message { data: String },
    Started { lang: String },
    Error { message: String },
    Stopped { lang: String },
}

struct LspProcess {
    child: Child,
    stdin_tx: std::sync::mpsc::Sender<String>,
}

#[derive(Default)]
pub struct LspManager {
    inner: Mutex<HashMap<String, LspProcess>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LspServerConfig {
    pub lang: String,
    pub command: String,
    pub args: Vec<String>,
    pub root_uri: String,
}

const KNOWN_SERVERS: &[(&str, &str, &[&str])] = &[
    ("typescript", "typescript-language-server", &["--stdio"]),
    ("javascript", "typescript-language-server", &["--stdio"]),
    ("rust", "rust-analyzer", &[]),
    ("python", "pyright-langserver", &["--stdio"]),
    ("go", "gopls", &["serve"]),
    ("c", "clangd", &[]),
    ("cpp", "clangd", &[]),
    ("html", "vscode-html-language-server", &["--stdio"]),
    ("css", "vscode-css-language-server", &["--stdio"]),
    ("json", "vscode-json-language-server", &["--stdio"]),
];

fn find_server(lang: &str) -> Option<(&'static str, &'static [&'static str])> {
    KNOWN_SERVERS
        .iter()
        .find(|(l, _, _)| *l == lang)
        .map(|(_, cmd, args)| (*cmd, *args))
}

fn prune_stopped(inner: &mut HashMap<String, LspProcess>) {
    inner.retain(|_, proc| match proc.child.try_wait() {
        Ok(Some(_)) => false,
        Ok(None) => true,
        Err(_) => false,
    });
}

fn encode_lsp_message(content: &str) -> String {
    format!(
        "Content-Length: {}\r\n\r\n{}",
        content.len(),
        content
    )
}


/// Strip a `file://` prefix from a root URI to get a filesystem path.
fn workspace_dir_from_uri(uri: &str) -> Option<String> {
    let trimmed = uri.strip_prefix("file://").unwrap_or(uri);
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[tauri::command]
pub fn lsp_start(
    state: State<LspManager>,
    config: LspServerConfig,
    on_event: Channel<LspEvent>,
) -> Result<(), String> {
    let mut inner = state.inner.lock().map_err(|e| e.to_string())?;
    prune_stopped(&mut inner);

    if inner.contains_key(&config.lang) {
        return Err(format!("LSP for '{}' is already running", config.lang));
    }
    if inner.len() >= process_util::MAX_CHILD_PROCESSES {
        return Err("too many language servers running; stop one first".into());
    }

    let command = if config.command.is_empty() {
        let (cmd, _) = find_server(&config.lang).ok_or_else(|| {
            format!("no known LSP server for '{}'; provide a custom command", config.lang)
        })?;
        cmd.to_string()
    } else {
        config.command.clone()
    };
    let args: Vec<String> = if config.command.is_empty() {
        let (_, default_args) = find_server(&config.lang).ok_or_else(|| {
            format!("no known LSP server for '{}'; provide a custom command", config.lang)
        })?;
        default_args.iter().map(|arg| (*arg).to_string()).collect()
    } else {
        config.args.clone()
    };

    let mut builder = Command::new(&command);
    builder
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(ws) = workspace_dir_from_uri(&config.root_uri) {
        builder.current_dir(&ws);
        #[cfg(not(windows))]
        builder.env("PATH", process_util::augmented_path(Some(&ws)));
    } else {
        #[cfg(not(windows))]
        builder.env("PATH", process_util::augmented_path(None));
    }
    let mut child = builder
        .spawn()
        .map_err(|e| format!("failed to start '{}': {}", command, e))?;

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stderr = child.stderr.take().ok_or("no stderr")?;

    let (stdin_tx, stdin_rx) = std::sync::mpsc::channel::<String>();

    let mut stdin_handle = child.stdin.take().ok_or("no stdin")?;
    std::thread::spawn(move || {
        while let Ok(msg) = stdin_rx.recv() {
            let encoded = encode_lsp_message(&msg);
            if stdin_handle.write_all(encoded.as_bytes()).is_err() {
                break;
            }
            if stdin_handle.flush().is_err() {
                break;
            }
        }
    });

    let lang = config.lang.clone();
    let evt = on_event.clone();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut header_buf = String::new();
        loop {
            header_buf.clear();
            match reader.read_line(&mut header_buf) {
                Ok(0) => break,
                Err(_) => break,
                _ => {}
            }
            let trimmed = header_buf.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(len_str) = trimmed.strip_prefix("Content-Length:") {
                let content_len: usize = match len_str.trim().parse() {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                let mut sep = String::new();
                let _ = reader.read_line(&mut sep);

                let mut body = vec![0u8; content_len];
                if std::io::Read::read_exact(&mut reader, &mut body).is_err() {
                    break;
                }
                let data = String::from_utf8_lossy(&body).to_string();
                if evt.send(LspEvent::Message { data }).is_err() {
                    break;
                }
            }
        }
        let _ = evt.send(LspEvent::Stopped { lang });
    });

    let lang2 = config.lang.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(l) => tracing::debug!("[lsp-{}] {}", lang2, l),
                Err(_) => break,
            }
        }
    });

    let _ = on_event.send(LspEvent::Started {
        lang: config.lang.clone(),
    });

    inner.insert(config.lang, LspProcess { child, stdin_tx });
    Ok(())
}

#[tauri::command]
pub fn lsp_send(
    state: State<LspManager>,
    lang: String,
    message: String,
) -> Result<(), String> {
    let inner = state.inner.lock().map_err(|e| e.to_string())?;
    let proc = inner.get(&lang).ok_or_else(|| format!("no LSP for '{lang}'"))?;
    proc.stdin_tx
        .send(message)
        .map_err(|e| format!("failed to send to LSP: {e}"))
}

#[tauri::command]
pub fn lsp_stop(
    state: State<LspManager>,
    lang: String,
) -> Result<(), String> {
    let mut inner = state.inner.lock().map_err(|e| e.to_string())?;
    if let Some(mut proc) = inner.remove(&lang) {
        let _ = proc.child.kill();
    }
    Ok(())
}

#[derive(Serialize)]
pub struct LspInfo {
    lang: String,
    running: bool,
}

#[tauri::command]
pub fn lsp_list(state: State<LspManager>) -> Result<Vec<LspInfo>, String> {
    let mut inner = state.inner.lock().map_err(|e| e.to_string())?;
    prune_stopped(&mut inner);
    let known: Vec<&str> = KNOWN_SERVERS.iter().map(|(l, _, _)| *l).collect();
    let mut out: Vec<LspInfo> = known
        .into_iter()
        .map(|l| LspInfo {
            lang: l.to_string(),
            running: inner.contains_key(l),
        })
        .collect();
    for key in inner.keys() {
        if !out.iter().any(|i| i.lang == *key) {
            out.push(LspInfo {
                lang: key.clone(),
                running: true,
            });
        }
    }
    Ok(out)
}
