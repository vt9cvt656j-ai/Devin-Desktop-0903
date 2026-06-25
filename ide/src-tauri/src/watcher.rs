use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEventKind};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

/// High-churn directories whose changes must NOT be forwarded to the UI: an
/// `npm install` / build / `cargo build` rewrites thousands of files in these,
/// which previously flooded the frontend (and its git refresh) and froze the app
/// while a command ran — even a backgrounded one, since the churn happens anyway.
const IGNORED_WATCH_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    "out",
    ".next",
    "coverage",
    ".cache",
    ".venv",
    "__pycache__",
    "vendor",
    ".gradle",
    ".idea",
    "Pods",
    "DerivedData",
];

fn is_ignored_path(path: &std::path::Path, roots: &[PathBuf]) -> bool {
    // Only consider components BELOW a watched root, so a workspace that itself
    // lives under a dir named e.g. "build"/"dist" isn't entirely ignored.
    let rel = roots
        .iter()
        .find_map(|r| path.strip_prefix(r).ok())
        .unwrap_or(path);
    rel.components().any(|c| match c {
        std::path::Component::Normal(name) => IGNORED_WATCH_DIRS
            .iter()
            .any(|d| name == std::ffi::OsStr::new(d)),
        _ => false,
    })
}

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
    let roots_filter = new_paths.clone();
    let mut debouncer = new_debouncer(
        Duration::from_millis(300),
        move |res: DebounceEventResult| {
            match res {
                Ok(events) => {
                    let mut changed: Vec<String> = events
                        .iter()
                        .filter(|e| {
                            e.kind == DebouncedEventKind::Any
                                && !is_ignored_path(&e.path, &roots_filter)
                        })
                        .map(|e| e.path.to_string_lossy().to_string())
                        .collect();
                    if !changed.is_empty() {
                        // Bound the batch so a huge legitimate change (e.g. a big
                        // `git checkout`) still can't hand the UI thousands of paths.
                        changed.truncate(500);
                        let _ = app_handle.emit("fs-change", FsChangeEvent { paths: changed });
                    }
                }
                Err(e) => {
                    tracing::warn!("[watcher] error: {e}");
                }
            }
        },
    )
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
