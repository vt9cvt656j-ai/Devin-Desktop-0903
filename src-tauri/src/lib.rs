//! Tauri shell for **Devin Desktop**.
//!
//! Owns the lifecycle of the [`bridge_core`] HTTP server and exposes a small
//! set of commands to the macOS-style control-panel UI.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Mutex;

use bridge_core::{generate_token, BridgeConfig};
use serde::Serialize;
use tauri::async_runtime;
use tokio::sync::oneshot;

/// A currently-running bridge instance.
struct RunningBridge {
    addr: SocketAddr,
    root: PathBuf,
    token: String,
    allow_write: bool,
    shutdown: oneshot::Sender<()>,
}

/// Tauri-managed state holding the (optional) running bridge.
#[derive(Default)]
pub struct BridgeManager {
    inner: Mutex<Option<RunningBridge>>,
}

/// Status reported to the UI.
#[derive(Debug, Clone, Serialize)]
pub struct BridgeStatus {
    pub running: bool,
    pub addr: Option<String>,
    pub url: Option<String>,
    pub token: Option<String>,
    pub root: Option<String>,
    pub allow_write: bool,
}

impl BridgeStatus {
    fn stopped() -> Self {
        Self {
            running: false,
            addr: None,
            url: None,
            token: None,
            root: None,
            allow_write: false,
        }
    }

    fn from(b: &RunningBridge) -> Self {
        Self {
            running: true,
            addr: Some(b.addr.to_string()),
            url: Some(format!("http://{}", b.addr)),
            token: Some(b.token.clone()),
            root: Some(b.root.to_string_lossy().to_string()),
            allow_write: b.allow_write,
        }
    }
}

#[tauri::command]
async fn get_status(state: tauri::State<'_, BridgeManager>) -> Result<BridgeStatus, String> {
    let guard = state.inner.lock().map_err(|e| e.to_string())?;
    Ok(guard
        .as_ref()
        .map(BridgeStatus::from)
        .unwrap_or_else(BridgeStatus::stopped))
}

#[tauri::command]
async fn start_bridge(
    state: tauri::State<'_, BridgeManager>,
    root: String,
    allow_write: bool,
) -> Result<BridgeStatus, String> {
    {
        let guard = state.inner.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Err("bridge is already running; stop it first".into());
        }
    }

    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("not a directory: {root}"));
    }

    let token = generate_token(40);
    let mut config = BridgeConfig::new(root_path.clone()).with_token(token.clone());
    if !allow_write {
        config = config.read_only();
    }

    let (addr_tx, addr_rx) = oneshot::channel::<SocketAddr>();
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let serve_config = config.clone();
    async_runtime::spawn(async move {
        let on_bound = move |addr: SocketAddr| {
            let _ = addr_tx.send(addr);
        };
        let shutdown = async move {
            let _ = shutdown_rx.await;
        };
        if let Err(e) = bridge_core::serve_with_shutdown(serve_config, on_bound, shutdown).await {
            tracing::error!("bridge server error: {e}");
        }
    });

    let addr = addr_rx
        .await
        .map_err(|_| "server failed to start".to_string())?;

    let running = RunningBridge {
        addr,
        root: root_path,
        token,
        allow_write,
        shutdown: shutdown_tx,
    };
    let status = BridgeStatus::from(&running);
    let mut guard = state.inner.lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        // Another concurrent start_bridge won the race; tear down the server we
        // just spawned so we don't leave an orphan, and report the conflict.
        let _ = running.shutdown.send(());
        return Err("bridge is already running; stop it first".into());
    }
    *guard = Some(running);
    Ok(status)
}

#[tauri::command]
async fn stop_bridge(state: tauri::State<'_, BridgeManager>) -> Result<BridgeStatus, String> {
    let running = {
        let mut guard = state.inner.lock().map_err(|e| e.to_string())?;
        guard.take()
    };
    if let Some(b) = running {
        let _ = b.shutdown.send(());
    }
    Ok(BridgeStatus::stopped())
}

/// Entry point shared by the binary and (potentially) mobile targets.
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(BridgeManager::default())
        .invoke_handler(tauri::generate_handler![
            get_status,
            start_bridge,
            stop_bridge
        ])
        .run(tauri::generate_context!())
        .expect("error while running Devin Desktop");
}
