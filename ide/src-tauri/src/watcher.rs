use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEventKind};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FsChangeEvent {
    pub paths: Vec<String>,
}

pub struct WatcherState {
    inner: Mutex<Option<WatcherHandle>>,
}

struct WatcherHandle {
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
    watched: HashSet<PathBuf>,
}

impl Default for WatcherState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }
}

#[tauri::command]
pub fn fs_watch(
    app: AppHandle,
    state: State<WatcherState>,
    paths: Vec<String>,
) -> Result<(), String> {
    let mut guard = state.inner.lock().map_err(|e| e.to_string())?;

    let new_paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();

    if let Some(handle) = guard.as_mut() {
        for p in &new_paths {
            if handle.watched.contains(p) {
                continue;
            }
            handle
                ._debouncer
                .watcher()
                .watch(p, notify::RecursiveMode::Recursive)
                .map_err(|e| format!("watch error: {e}"))?;
            handle.watched.insert(p.clone());
        }
        return Ok(());
    }

    let app_handle = app.clone();
    let mut debouncer = new_debouncer(Duration::from_millis(300), move |res: DebounceEventResult| {
        match res {
            Ok(events) => {
                let changed: Vec<String> = events
                    .iter()
                    .filter(|e| e.kind == DebouncedEventKind::Any)
                    .map(|e| e.path.to_string_lossy().to_string())
                    .collect();
                if !changed.is_empty() {
                    let _ = app_handle.emit("fs-change", FsChangeEvent { paths: changed });
                }
            }
            Err(e) => {
                tracing::warn!("[watcher] error: {e}");
            }
        }
    })
    .map_err(|e| format!("failed to create watcher: {e}"))?;

    let mut watched = HashSet::new();
    for p in &new_paths {
        debouncer
            .watcher()
            .watch(p, notify::RecursiveMode::Recursive)
            .map_err(|e| format!("watch error for {}: {e}", p.display()))?;
        watched.insert(p.clone());
    }

    *guard = Some(WatcherHandle {
        _debouncer: debouncer,
        watched,
    });
    Ok(())
}

#[tauri::command]
pub fn fs_unwatch(state: State<WatcherState>, paths: Vec<String>) -> Result<(), String> {
    let mut guard = state.inner.lock().map_err(|e| e.to_string())?;
    if let Some(handle) = guard.as_mut() {
        for p in paths {
            let pb = PathBuf::from(&p);
            if handle.watched.remove(&pb) {
                let _ = handle._debouncer.watcher().unwatch(&pb);
            }
        }
        if handle.watched.is_empty() {
            *guard = None;
        }
    }
    Ok(())
}
