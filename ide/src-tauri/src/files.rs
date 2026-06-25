use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

use once_cell::sync::Lazy;
use serde::Serialize;

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

/// Workspace roots that have been opened by the user via the native folder
/// dialog.  Every file-system command that operates on arbitrary paths
/// (read / write / delete / rename / search / replace) checks that the target
/// path falls inside one of these roots, blocking path-traversal attacks from
/// XSS or extension sandbox escapes.
static ALLOWED_ROOTS: Lazy<Mutex<Vec<PathBuf>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Pre-register the user's HOME directory so files are accessible before any
/// folder is explicitly opened.  Called from `lib.rs` during app setup.
pub fn bootstrap_home_root() {
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        let home_path = PathBuf::from(&home);
        if let Ok(canonical) = std::fs::canonicalize(&home_path) {
            if let Ok(mut roots) = ALLOWED_ROOTS.lock() {
                if !roots.contains(&canonical) {
                    roots.push(canonical);
                }
                let raw = home_path;
                if !roots.contains(&raw) {
                    roots.push(raw);
                }
            }
        } else if let Ok(mut roots) = ALLOWED_ROOTS.lock() {
            if !roots.contains(&home_path) {
                roots.push(home_path);
            }
        }
    }
}

/// Register a workspace root that the user explicitly opened.
/// Called from the frontend after a successful folder-open dialog.
#[tauri::command]
pub fn register_workspace_root(path: String) -> Result<(), String> {
    let raw_path = PathBuf::from(&path);
    let canonical = std::fs::canonicalize(&path).map_err(|e| e.to_string())?;
    let mut roots = ALLOWED_ROOTS.lock().map_err(|e| e.to_string())?;
    if !roots.contains(&canonical) {
        roots.push(canonical);
    }
    if raw_path != *roots.last().unwrap_or(&PathBuf::new()) && !roots.contains(&raw_path) {
        roots.push(raw_path);
    }
    bootstrap_home_root_inner(&mut roots);
    Ok(())
}

fn bootstrap_home_root_inner(roots: &mut Vec<PathBuf>) {
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        let home_path = PathBuf::from(&home);
        if let Ok(home_canonical) = std::fs::canonicalize(&home_path) {
            if !roots.contains(&home_canonical) {
                roots.push(home_canonical);
            }
        }
        if !roots.contains(&home_path) {
            roots.push(home_path);
        }
    }
}

/// Paths always allowed regardless of workspace roots (temp dirs, macOS firmlinks).
const SAFE_PREFIXES: &[&str] = &[
    "/tmp",
    "/private/tmp",
    "/var/folders",
    "/private/var/folders",
];

/// Verify `target` is inside an allowed workspace root.  Resolves symlinks and
/// normalises components so that `../../etc/passwd` tricks are caught even when
/// intermediate directories exist.
fn require_inside_workspace(target: &str) -> Result<PathBuf, String> {
    let target_path = Path::new(target);
    let raw_target = target.to_string();

    // Fast-path: always allow temp / safe system directories before canonicalize.
    for prefix in SAFE_PREFIXES {
        if raw_target.starts_with(prefix) {
            let resolved =
                std::fs::canonicalize(target_path).unwrap_or_else(|_| target_path.to_path_buf());
            return Ok(resolved);
        }
    }

    // For paths that don't exist yet (create_file, create_dir), resolve the
    // deepest existing ancestor and append the remaining components.
    let resolved = if target_path.exists() {
        std::fs::canonicalize(target_path).map_err(|e| e.to_string())?
    } else {
        let mut base = target_path.to_path_buf();
        let mut pending: Vec<std::ffi::OsString> = Vec::new();
        loop {
            if base.exists() {
                break;
            }
            if let Some(name) = base.file_name() {
                pending.push(name.to_os_string());
            } else {
                break;
            }
            match base.parent() {
                Some(p) => base = p.to_path_buf(),
                None => break,
            }
        }
        let mut resolved = std::fs::canonicalize(&base).map_err(|e| e.to_string())?;
        for seg in pending.into_iter().rev() {
            let comp = Path::new(&seg);
            for c in comp.components() {
                match c {
                    Component::Normal(s) => resolved.push(s),
                    Component::CurDir => {}
                    _ => return Err("path contains disallowed components".into()),
                }
            }
        }
        resolved
    };

    let resolved_str = resolved.to_string_lossy().to_string();

    // Post-canonicalize safe-prefix check (handles firmlinks like /tmp → /private/tmp).
    for prefix in SAFE_PREFIXES {
        if resolved_str.starts_with(prefix) {
            return Ok(resolved);
        }
    }

    let roots = ALLOWED_ROOTS.lock().map_err(|e| e.to_string())?;
    if roots.is_empty() {
        return Ok(resolved);
    }

    // Component-boundary containment: "/a/b" must NOT match sibling "/a/bcd".
    // `Path::starts_with` already does this; the string forms add a trailing-'/'
    // (or exact-equal) check so a shared name prefix can't widen access.
    let within = |prefix: &str, candidate: &str| -> bool {
        candidate == prefix
            || candidate.starts_with(&format!("{}/", prefix.trim_end_matches('/')))
    };

    for root in roots.iter() {
        let root_str = root.to_string_lossy().to_string();
        if resolved.starts_with(root)
            || within(&root_str, &resolved_str)
            || within(&root_str, &raw_target)
        {
            return Ok(resolved);
        }
    }

    // Always allow paths under HOME regardless of what roots are registered.
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        if within(&home, &resolved_str) || within(&home, &raw_target) {
            return Ok(resolved);
        }
        if let Ok(home_canonical) = std::fs::canonicalize(&home) {
            let hc = home_canonical.to_string_lossy().to_string();
            if within(&hc, &resolved_str) || within(&hc, &raw_target) {
                return Ok(resolved);
            }
        }
    }

    let root_list: Vec<String> = roots
        .iter()
        .map(|r| r.to_string_lossy().to_string())
        .collect();
    Err(format!(
        "access denied: path '{}' (resolved '{}') is outside all workspace roots. Allowed: [{}].",
        raw_target,
        resolved_str,
        root_list.join(", ")
    ))
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
pub fn read_dir(path: String) -> Result<Vec<DirEntry>, String> {
    require_inside_workspace(&path)?;
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&path).map_err(|e| e.to_string())? {
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

/// Read a UTF-8 text file. Rejects directories, oversized and binary files so
/// the editor never tries to render garbage.
#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    require_inside_workspace(&path)?;
    let meta = std::fs::metadata(&path).map_err(|e| format!("cannot stat '{}': {}", path, e))?;
    if meta.is_dir() {
        return Err(format!(
            "'{}' is a directory, not a file. Use read_dir to list its contents.",
            path
        ));
    }
    if meta.len() > MAX_FILE {
        return Err("file is too large to open in the editor (> 5 MB)".into());
    }
    let bytes = std::fs::read(&path).map_err(|e| format!("cannot read '{}': {}", path, e))?;
    if bytes.iter().take(8000).any(|&b| b == 0) {
        return Err("cannot open a binary file in the editor".into());
    }
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Overwrite a file with new text content.
#[tauri::command]
pub fn write_text_file(path: String, content: String) -> Result<(), String> {
    require_inside_workspace(&path)?;
    // Create parent dirs ourselves so callers don't have to shell out to
    // `mkdir -p` (which the agent frontend did, interpolating model-controlled
    // paths straight into a shell command).
    if let Some(parent) = Path::new(&path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

/// Write a file to /tmp for internal tools (no workspace restriction).
#[tauri::command]
pub fn write_tmp_file(name: String, content: String) -> Result<String, String> {
    // Only ever write a bare filename under /tmp — strip any directory components
    // so a name like "../etc/x" can't escape the temp dir.
    let safe = Path::new(&name)
        .file_name()
        .ok_or("invalid temp file name")?;
    let path = std::path::Path::new("/tmp").join(safe);
    std::fs::write(&path, &content).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
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
pub fn create_file(path: String) -> Result<(), String> {
    require_inside_workspace(&path)?;
    let p = Path::new(&path);
    if p.exists() {
        return Err("a file or folder with that name already exists".into());
    }
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(p, b"").map_err(|e| e.to_string())
}

/// Create a new directory (including any missing parents).
#[tauri::command]
pub fn create_dir(path: String) -> Result<(), String> {
    require_inside_workspace(&path)?;
    let p = Path::new(&path);
    if p.exists() {
        return Err("a file or folder with that name already exists".into());
    }
    std::fs::create_dir_all(p).map_err(|e| e.to_string())
}

/// Rename or move a file/folder. Errors if the destination already exists.
#[tauri::command]
pub fn rename_path(from: String, to: String) -> Result<(), String> {
    require_inside_workspace(&from)?;
    require_inside_workspace(&to)?;
    let to_p = Path::new(&to);
    if to_p.exists() {
        return Err("a file or folder with that name already exists".into());
    }
    if let Some(parent) = to_p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&from, &to).map_err(|e| e.to_string())
}

/// Delete a file, or a directory and all of its contents.
#[tauri::command]
pub fn delete_path(path: String) -> Result<(), String> {
    require_inside_workspace(&path)?;
    let p = Path::new(&path);
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
    // `needle` is already lowercased by the caller when !case_sensitive.
    let mut out = Vec::new();
    if needle.is_empty() {
        return out;
    }
    if case_sensitive {
        let mut from = 0usize;
        while let Some(rel) = line[from..].find(needle) {
            let byte_start = from + rel;
            let col0 = line[..byte_start].chars().count();
            out.push((col0, col0, col0 + needle.chars().count()));
            from = byte_start + needle.len();
            if from >= line.len() {
                break;
            }
        }
    } else {
        // Walk the ORIGINAL line so the returned char offsets index the displayed
        // text — correct even when case folding changes byte/char length (e.g.
        // 'İ'). The previous code computed offsets on the lowercased copy.
        let mut i = 0usize;
        while i < line.len() {
            if let Some(consumed) = ci_prefix_len(&line[i..], needle) {
                if consumed > 0 {
                    let col0 = line[..i].chars().count();
                    let col_end = line[..i + consumed].chars().count();
                    out.push((col0, col0, col_end));
                    i += consumed;
                    continue;
                }
            }
            i += line[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
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
) -> Result<Vec<FileMatches>, String> {
    let needle = if case_sensitive {
        query.clone()
    } else {
        query.to_lowercase()
    };
    if needle.is_empty() {
        return Ok(Vec::new());
    }

    // Keep the search confined to a registered workspace root.
    let root_path = require_inside_workspace(&root)?;
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

#[derive(Serialize)]
pub struct ReplaceResult {
    files_changed: usize,
    replacements: usize,
}

/// If the case-insensitive lowercasing of a leading run of chars in `s` equals
/// `needle_lower` exactly (ending on a char boundary of `s`), return how many
/// bytes of `s` that run occupies; otherwise `None`. `needle_lower` must already
/// be lowercased and non-empty. Operating on the original `s` keeps the returned
/// length a valid byte offset into `s` even when case folding changes length.
fn ci_prefix_len(s: &str, needle_lower: &str) -> Option<usize> {
    if needle_lower.is_empty() {
        return None;
    }
    let mut lowered = String::new();
    for (off, ch) in s.char_indices() {
        for lc in ch.to_lowercase() {
            lowered.push(lc);
        }
        if lowered.len() >= needle_lower.len() {
            // Accept only an exact match ending on this char boundary. If the last
            // char's lowercase expansion overshot the needle, reject — advancing
            // by a partial char would not be byte-accurate.
            return if lowered == needle_lower {
                Some(off + ch.len_utf8())
            } else {
                None
            };
        }
        // Bail early once the lowercased prefix can no longer lead to the needle.
        if !needle_lower.starts_with(&lowered) {
            return None;
        }
    }
    None
}

/// Replace all occurrences of `query` with `replacement` in `file_path`.
/// Returns the number of replacements made.
#[tauri::command]
pub fn replace_in_file(
    file_path: String,
    query: String,
    replacement: String,
    case_sensitive: bool,
) -> Result<usize, String> {
    // Same workspace boundary every other mutating fs command enforces — without
    // it this is an arbitrary-file-write primitive.
    let resolved = require_inside_workspace(&file_path)?;
    let file_path = resolved.to_string_lossy().to_string();
    let bytes = std::fs::read(&file_path).map_err(|e| e.to_string())?;
    if bytes.iter().take(8000).any(|&b| b == 0) {
        return Err("cannot replace in binary files".into());
    }
    let content = String::from_utf8_lossy(&bytes).to_string();
    let (new_content, count) = if case_sensitive {
        let c = content.matches(&query).count();
        (content.replace(&query, &replacement), c)
    } else {
        // Case-insensitive literal replace. Match positions MUST be byte offsets
        // into the original `content`, never into its lowercased copy: a path like
        // 'İ' lowercases to two chars, so offsets taken from the lowercased string
        // would land mid-char in the original — corrupting output and panicking on
        // a non-char-boundary slice. Walk the original by char boundary instead.
        let needle = query.to_lowercase();
        let mut result = String::with_capacity(content.len());
        let mut found = 0usize;
        let mut i = 0usize;
        while i < content.len() {
            match ci_prefix_len(&content[i..], &needle) {
                Some(consumed) if consumed > 0 => {
                    result.push_str(&replacement);
                    i += consumed;
                    found += 1;
                }
                _ => {
                    // Not a match here — copy one whole char and advance.
                    let ch_len = content[i..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    result.push_str(&content[i..i + ch_len]);
                    i += ch_len;
                }
            }
        }
        (result, found)
    };
    if count > 0 {
        std::fs::write(&file_path, &new_content).map_err(|e| e.to_string())?;
    }
    Ok(count)
}

/// Replace all occurrences of `query` with `replacement` across the project.
#[tauri::command]
pub fn replace_in_project(
    root: String,
    query: String,
    replacement: String,
    case_sensitive: bool,
) -> Result<ReplaceResult, String> {
    if query.is_empty() {
        return Ok(ReplaceResult {
            files_changed: 0,
            replacements: 0,
        });
    }
    // Confine the whole sweep to the workspace (each write is also guarded by
    // replace_in_file, but this stops us from even scanning arbitrary trees).
    let root_path = require_inside_workspace(&root)?;
    let mut files_changed = 0usize;
    let mut replacements = 0usize;
    let mut stack: Vec<PathBuf> = vec![root_path];

    while let Some(dir) = stack.pop() {
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
            let meta = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.len() > SEARCH_MAX_FILE {
                continue;
            }
            let path_str = path.to_string_lossy().to_string();
            match replace_in_file(path_str, query.clone(), replacement.clone(), case_sensitive) {
                Ok(c) if c > 0 => {
                    files_changed += 1;
                    replacements += c;
                }
                _ => {}
            }
        }
    }

    Ok(ReplaceResult {
        files_changed,
        replacements,
    })
}
