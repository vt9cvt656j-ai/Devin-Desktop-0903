use serde::Serialize;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

/// Maximum size of a file we will load into the editor (5 MiB).
const MAX_FILE: u64 = 5 * 1024 * 1024;

/// Files larger than this are skipped during project search (2 MiB).
const SEARCH_MAX_FILE: u64 = 2 * 1024 * 1024;
/// Hard cap on the total number of matches returned by a single search.
const SEARCH_MAX_RESULTS: usize = 2000;
/// Cap on matches reported per file so one file can't drown the results.
const SEARCH_MAX_PER_FILE: usize = 50;
/// Display lines are truncated to this many characters in search results.
const SEARCH_MAX_LINE_CHARS: usize = 500;

/// Directories that are never descended into during search (build output,
/// vendored deps, VCS metadata). Dot-directories are skipped separately.
const IGNORED_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    "vendor",
    "__pycache__",
    "coverage",
];

/// Records the currently open workspace folder. Every file operation is
/// constrained to stay inside it, mirroring the ScopedFs guarantee the sibling
/// `bridge-core` crate enforces. Until a folder is opened, file commands are
/// refused outright.
#[derive(Default)]
pub struct Workspace(Mutex<Option<Roots>>);

struct Roots {
    /// The root as the frontend names it (lexically normalized, symlinks NOT
    /// resolved) — used for the cheap textual containment check.
    lexical: PathBuf,
    /// The fully canonicalized root — used to detect symlink escapes.
    canonical: PathBuf,
}

/// Lexically normalize a path (resolve `.` and `..`) without touching the disk.
fn lexical_abs(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Walk up from `norm` to the longest existing ancestor, canonicalize it, and
/// require it to remain within `canonical_root`. This defeats escapes through a
/// symlinked component (e.g. a link inside the workspace pointing to `/etc`).
fn ensure_canonical_within(canonical_root: &Path, norm: &Path) -> Result<(), String> {
    let mut probe: &Path = norm;
    loop {
        match std::fs::canonicalize(probe) {
            Ok(real) => {
                if real.starts_with(canonical_root) {
                    return Ok(());
                }
                return Err("path escapes the workspace folder".into());
            }
            Err(_) => match probe.parent() {
                Some(parent) => probe = parent,
                None => return Err("path escapes the workspace folder".into()),
            },
        }
    }
}

/// Resolve a frontend-supplied path against the open workspace, rejecting
/// anything that would read or write outside it. Returns the normalized
/// absolute path to operate on.
fn scoped(ws: &tauri::State<Workspace>, path: &str) -> Result<PathBuf, String> {
    let guard = ws.0.lock().map_err(|_| "workspace state poisoned")?;
    let roots = guard.as_ref().ok_or("no workspace folder is open")?;
    let p = Path::new(path);
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        roots.lexical.join(p)
    };
    let norm = lexical_abs(&abs);
    if norm != roots.lexical && !norm.starts_with(&roots.lexical) {
        return Err("path escapes the workspace folder".into());
    }
    ensure_canonical_within(&roots.canonical, &norm)?;
    Ok(norm)
}

/// Record the workspace folder the frontend just opened. Canonicalizes it so
/// later containment checks have a stable, symlink-resolved root.
#[tauri::command]
pub fn set_workspace_root(path: String, ws: tauri::State<Workspace>) -> Result<(), String> {
    let canonical = std::fs::canonicalize(&path).map_err(|e| e.to_string())?;
    if !canonical.is_dir() {
        return Err("workspace root is not a directory".into());
    }
    let lexical = lexical_abs(Path::new(&path));
    *ws.0.lock().map_err(|_| "workspace state poisoned")? = Some(Roots { lexical, canonical });
    Ok(())
}

#[derive(Serialize)]
pub struct DirEntry {
    name: String,
    path: String,
    is_dir: bool,
}

/// List the immediate children of a directory, directories first, then by name.
/// Dotfiles are hidden by default to keep the tree tidy.
#[tauri::command]
pub fn read_dir(path: String, ws: tauri::State<Workspace>) -> Result<Vec<DirEntry>, String> {
    let dir = scoped(&ws, &path)?;
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let p = entry.path();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        entries.push(DirEntry {
            name,
            path: p.to_string_lossy().to_string(),
            is_dir,
        });
    }
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    Ok(entries)
}

/// Read a UTF-8 text file. Rejects oversized and binary files so the editor
/// never tries to render garbage.
#[tauri::command]
pub fn read_text_file(path: String, ws: tauri::State<Workspace>) -> Result<String, String> {
    let path = scoped(&ws, &path)?;
    let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    if meta.len() > MAX_FILE {
        return Err("file is too large to open in the editor (> 5 MB)".into());
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    if bytes.iter().take(8000).any(|&b| b == 0) {
        return Err("cannot open a binary file in the editor".into());
    }
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Overwrite a file with new text content.
#[tauri::command]
pub fn write_text_file(
    path: String,
    content: String,
    ws: tauri::State<Workspace>,
) -> Result<(), String> {
    let path = scoped(&ws, &path)?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

/// The current user's home directory, used as the default tree root.
#[tauri::command]
pub fn home_dir() -> Option<String> {
    std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok())
}

/// Create a new empty file. Errors if anything already exists at `path`.
#[tauri::command]
pub fn create_file(path: String, ws: tauri::State<Workspace>) -> Result<(), String> {
    let p = scoped(&ws, &path)?;
    if p.exists() {
        return Err("a file or folder with that name already exists".into());
    }
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&p, b"").map_err(|e| e.to_string())
}

/// Create a new directory (including any missing parents).
#[tauri::command]
pub fn create_dir(path: String, ws: tauri::State<Workspace>) -> Result<(), String> {
    let p = scoped(&ws, &path)?;
    if p.exists() {
        return Err("a file or folder with that name already exists".into());
    }
    std::fs::create_dir_all(&p).map_err(|e| e.to_string())
}

/// Rename or move a file/folder. Errors if the destination already exists.
#[tauri::command]
pub fn rename_path(from: String, to: String, ws: tauri::State<Workspace>) -> Result<(), String> {
    let from_p = scoped(&ws, &from)?;
    let to_p = scoped(&ws, &to)?;
    if to_p.exists() {
        return Err("a file or folder with that name already exists".into());
    }
    if let Some(parent) = to_p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&from_p, &to_p).map_err(|e| e.to_string())
}

/// Delete a file, or a directory and all of its contents.
#[tauri::command]
pub fn delete_path(path: String, ws: tauri::State<Workspace>) -> Result<(), String> {
    let p = scoped(&ws, &path)?;
    let p = p.as_path();
    let meta = std::fs::symlink_metadata(p).map_err(|e| e.to_string())?;
    if meta.is_dir() {
        std::fs::remove_dir_all(p).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(p).map_err(|e| e.to_string())
    }
}

/// A single matching location within a file.
#[derive(Serialize)]
pub struct SearchMatch {
    /// 1-based line number.
    line: usize,
    /// 1-based column (in characters) of the match start.
    column: usize,
    /// The (possibly truncated) text of the matching line.
    text: String,
    /// Character offset of the match start within `text`.
    start: usize,
    /// Character offset of the match end within `text`.
    end: usize,
}

/// All matches found within a single file.
#[derive(Serialize)]
pub struct FileMatches {
    path: String,
    name: String,
    /// Path relative to the search root, for compact display.
    rel: String,
    matches: Vec<SearchMatch>,
}

fn find_matches_in_line(
    line: &str,
    needle: &str,
    case_sensitive: bool,
) -> Vec<(usize, usize, usize)> {
    let hay = if case_sensitive {
        line.to_string()
    } else {
        line.to_lowercase()
    };
    let needle_chars = needle.chars().count();
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(rel) = hay[from..].find(needle) {
        let byte_start = from + rel;
        let col0 = hay[..byte_start].chars().count();
        out.push((col0, col0, col0 + needle_chars));
        from = byte_start + needle.len();
        if from > hay.len() {
            break;
        }
    }
    out
}

/// Recursively search the project tree under `root` for `query`.
///
/// Skips dot-entries, common build/dependency directories, oversized files,
/// and binary files. Results are capped to keep responses bounded.
#[tauri::command]
pub fn search_in_project(
    root: String,
    query: String,
    case_sensitive: bool,
    ws: tauri::State<Workspace>,
) -> Result<Vec<FileMatches>, String> {
    let needle = if case_sensitive {
        query.clone()
    } else {
        query.to_lowercase()
    };
    if needle.is_empty() {
        return Ok(Vec::new());
    }

    let root_path = scoped(&ws, &root)?;
    let mut results: Vec<FileMatches> = Vec::new();
    let mut total = 0usize;
    let mut stack: Vec<PathBuf> = vec![root_path.clone()];

    while let Some(dir) = stack.pop() {
        if total >= SEARCH_MAX_RESULTS {
            break;
        }
        let rd = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let mut children: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        children.sort();
        for path in children {
            let name = match path.file_name() {
                Some(n) => n.to_string_lossy().to_string(),
                None => continue,
            };
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                if !IGNORED_DIRS.contains(&name.as_str()) {
                    stack.push(path);
                }
                continue;
            }
            if total >= SEARCH_MAX_RESULTS {
                break;
            }
            let meta = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.len() > SEARCH_MAX_FILE {
                continue;
            }
            let bytes = match std::fs::read(&path) {
                Ok(b) => b,
                Err(_) => continue,
            };
            if bytes.iter().take(8000).any(|&b| b == 0) {
                continue;
            }
            let content = String::from_utf8_lossy(&bytes);
            let mut file_matches: Vec<SearchMatch> = Vec::new();
            for (i, line) in content.lines().enumerate() {
                if file_matches.len() >= SEARCH_MAX_PER_FILE || total >= SEARCH_MAX_RESULTS {
                    break;
                }
                let hits = find_matches_in_line(line, &needle, case_sensitive);
                if hits.is_empty() {
                    continue;
                }
                let mut chars = line.chars();
                let display: String = if line.chars().count() > SEARCH_MAX_LINE_CHARS {
                    let mut s: String = chars.by_ref().take(SEARCH_MAX_LINE_CHARS).collect();
                    s.push('\u{2026}');
                    s
                } else {
                    line.to_string()
                };
                for (col0, start, end) in hits {
                    if file_matches.len() >= SEARCH_MAX_PER_FILE || total >= SEARCH_MAX_RESULTS {
                        break;
                    }
                    file_matches.push(SearchMatch {
                        line: i + 1,
                        column: col0 + 1,
                        text: display.clone(),
                        start,
                        end,
                    });
                    total += 1;
                }
            }
            if !file_matches.is_empty() {
                let rel = path
                    .strip_prefix(&root_path)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                results.push(FileMatches {
                    path: path.to_string_lossy().to_string(),
                    name,
                    rel,
                    matches: file_matches,
                });
            }
        }
    }

    results.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexical_abs_resolves_dot_and_dotdot() {
        assert_eq!(lexical_abs(Path::new("/a/b/../c")), PathBuf::from("/a/c"));
        assert_eq!(lexical_abs(Path::new("/a/./b")), PathBuf::from("/a/b"));
        assert_eq!(
            lexical_abs(Path::new("/a/b/../../../etc/passwd")),
            PathBuf::from("/etc/passwd")
        );
    }

    #[test]
    fn lexical_containment_blocks_traversal_and_prefix_tricks() {
        let root = PathBuf::from("/home/u/proj");
        // A path inside the workspace is accepted.
        assert!(lexical_abs(Path::new("/home/u/proj/src/main.rs")).starts_with(&root));
        // `..` traversal that escapes the root is rejected.
        assert!(!lexical_abs(Path::new("/home/u/proj/../../../etc/passwd")).starts_with(&root));
        // A sibling sharing a string prefix ("proj-evil") is not "inside" the root.
        assert!(!lexical_abs(Path::new("/home/u/proj-evil/secret")).starts_with(&root));
    }

    #[test]
    fn canonical_within_accepts_inside_and_rejects_outside() {
        let base = std::env::temp_dir().join(format!("mide_scope_test_{}", std::process::id()));
        let root = base.join("root");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        let canonical_root = std::fs::canonicalize(&root).unwrap();

        let inside = root.join("sub/file.txt");
        std::fs::write(&inside, b"hi").unwrap();
        assert!(ensure_canonical_within(&canonical_root, &lexical_abs(&inside)).is_ok());

        // A path resolving outside the root (a real system file) is rejected.
        assert!(ensure_canonical_within(&canonical_root, Path::new("/etc/passwd")).is_err());

        std::fs::remove_dir_all(&base).ok();
    }
}
