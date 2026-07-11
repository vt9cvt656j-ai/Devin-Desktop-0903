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
/// Serialize IDE-originated workspace mutations. Conditional agent writes hold
/// this lock across their final read/compare/write sequence, so two concurrent
/// agent runs cannot both validate the same old version and silently overwrite
/// one another.
static FILE_MUTATION_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

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
        candidate == prefix || candidate.starts_with(&format!("{}/", prefix.trim_end_matches('/')))
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

/// Read any (binary) file as a `data:<mime>;base64,...` URL. Used to render images in the
/// chat (e.g. `design_board` wardrobe previews) WITHOUT the Tauri asset protocol, whose glob
/// scope rejects hidden directories like `.wardrobe/` → the `<img>` stays blank. Reusing the
/// genimage `data_url` is the fast path; this is the universal fallback (works for pre-existing
/// images, cross-session, any path inside the workspace).
#[tauri::command]
pub fn read_file_data_url(path: String) -> Result<String, String> {
    require_inside_workspace(&path)?;
    let meta = std::fs::metadata(&path).map_err(|e| format!("cannot stat '{}': {}", path, e))?;
    if meta.is_dir() {
        return Err(format!("'{}' is a directory, not a file.", path));
    }
    if meta.len() > 25 * 1024 * 1024 {
        return Err(format!(
            "file too large ({} bytes) for a data URL",
            meta.len()
        ));
    }
    let bytes = std::fs::read(&path).map_err(|e| format!("cannot read '{}': {}", path, e))?;
    let ext = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        _ => "application/octet-stream",
    };
    Ok(format!(
        "data:{};base64,{}",
        mime,
        crate::capture::b64(&bytes)
    ))
}

/// Extract readable TEXT from a document (PDF / Word / Excel / PowerPoint), so the agent
/// can read specs / papers / manuals that `read_text_file` would otherwise return as
/// binary garbage. Office formats are ZIP+XML (reuses the existing `zip` dep); PDF uses
/// `pdf-extract` (pure Rust). The agent's read_file routes here automatically by extension.
#[tauri::command]
pub fn read_document(path: String) -> Result<String, String> {
    require_inside_workspace(&path)?;
    let meta = std::fs::metadata(&path).map_err(|e| format!("cannot stat '{}': {}", path, e))?;
    if meta.is_dir() {
        return Err(format!("'{}' is a directory", path));
    }
    if meta.len() > 50 * 1024 * 1024 {
        return Err("文档过大（>50MB），无法解析".into());
    }
    let ext = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let text = match ext.as_str() {
        "pdf" => pdf_extract::extract_text(&path).map_err(|e| format!("PDF 解析失败: {e}"))?,
        "docx" | "odt" => extract_office(&path, |n| n == "word/document.xml" || n == "content.xml")?,
        "pptx" => extract_office(&path, |n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))?,
        "xlsx" => extract_office(&path, |n| n == "xl/sharedStrings.xml")?,
        other => {
            return Err(format!(
                "read_document 不支持 .{other}（支持 pdf/docx/pptx/xlsx/odt）；普通文本文件用 read_file 即可"
            ))
        }
    };
    let t = text.trim();
    if t.is_empty() {
        return Err(
            "没从文档里提取到文本（可能是扫描件/纯图片 PDF——需要 OCR，本工具只读文本层）".into(),
        );
    }
    Ok(t.to_string())
}

/// Pull the text out of the ZIP entries an Office doc keeps its content in.
fn extract_office(path: &str, want: impl Fn(&str) -> bool) -> Result<String, String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|e| format!("不是有效的 Office 文档(zip): {e}"))?;
    let names: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();
    let mut out: Vec<String> = Vec::new();
    for name in names {
        if want(&name) {
            if let Ok(mut f) = zip.by_name(&name) {
                let mut xml = String::new();
                if f.read_to_string(&mut xml).is_ok() {
                    out.push(strip_xml(&xml));
                }
            }
        }
    }
    Ok(out.join("\n"))
}

/// Turn an Office XML part into plain text: paragraph/row/break tags → newlines, strip the
/// rest of the tags, decode the common entities, collapse blank-line runs.
fn strip_xml(xml: &str) -> String {
    let mut s = xml.to_string();
    for tag in [
        "</w:p>",
        "</a:p>",
        "</text:p>",
        "</si>",
        "</row>",
        "<w:br/>",
        "<w:br></w:br>",
    ] {
        s = s.replace(tag, "\n");
    }
    s = s.replace("<w:tab/>", "\t");
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let out = out
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#10;", "\n");
    let mut res = String::new();
    let mut blank = 0;
    for l in out.lines().map(|l| l.trim_end()) {
        if l.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        res.push_str(l);
        res.push('\n');
    }
    res
}

/// Overwrite a file with new text content.
#[tauri::command]
pub fn write_text_file(path: String, content: String) -> Result<(), String> {
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
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

/// Write only if the file still has the exact version the caller read. `None`
/// means the caller observed no file and therefore requires an atomic create.
/// Every normal IDE text write shares FILE_MUTATION_LOCK, making this comparison
/// and write one critical section across editor saves and concurrent agent runs.
#[tauri::command]
pub fn write_text_file_if_unchanged(
    path: String,
    expected_content: Option<String>,
    content: String,
) -> Result<(), String> {
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
    let resolved = require_inside_workspace(&path)?;
    let exists = resolved.exists();
    match expected_content {
        Some(expected) => {
            if !exists {
                return Err("[CONFLICT] file was deleted after it was read".into());
            }
            let current = std::fs::read_to_string(&resolved).map_err(|e| e.to_string())?;
            if current != expected {
                return Err("[CONFLICT] file changed after it was read".into());
            }
        }
        None if exists => {
            return Err("[CONFLICT] file was created by another task".into());
        }
        None => {}
    }
    if let Some(parent) = resolved.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(resolved, content).map_err(|e| e.to_string())
}

/// Delete a text file only if it still contains the version the caller last
/// wrote. This is the deletion counterpart to `write_text_file_if_unchanged`
/// and keeps Undo/revert from removing a user's newer edit.
#[tauri::command]
pub fn delete_text_file_if_unchanged(path: String, expected_content: String) -> Result<(), String> {
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
    let resolved = require_inside_workspace(&path)?;
    let meta = std::fs::symlink_metadata(&resolved).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("[CONFLICT] path is no longer the expected text file".into());
    }
    let current = std::fs::read_to_string(&resolved).map_err(|e| e.to_string())?;
    if current != expected_content {
        return Err("[CONFLICT] file changed after it was written".into());
    }
    std::fs::remove_file(resolved).map_err(|e| e.to_string())
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
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
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
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
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
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
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

fn copy_dir_recursive(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let dest = to.join(entry.file_name());
        if ft.is_dir() {
            copy_dir_recursive(&entry.path(), &dest)?;
        } else if ft.is_file() {
            std::fs::copy(entry.path(), &dest)?;
        }
        // symlinks / special files are skipped for safety
    }
    Ok(())
}

/// Copy a file, or a directory and all of its contents, to `to`. Errors if the
/// destination already exists. Both endpoints must be inside the workspace.
#[tauri::command]
pub fn copy_path(from: String, to: String) -> Result<(), String> {
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
    require_inside_workspace(&from)?;
    let to_path = require_inside_workspace(&to)?;
    let from_p = Path::new(&from);
    if !from_p.exists() {
        return Err("source does not exist".into());
    }
    if to_path.exists() {
        return Err("a file or folder with that name already exists".into());
    }
    if let Some(parent) = to_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let meta = std::fs::symlink_metadata(from_p).map_err(|e| e.to_string())?;
    if meta.is_dir() {
        copy_dir_recursive(from_p, &to_path).map_err(|e| e.to_string())
    } else {
        std::fs::copy(from_p, &to_path)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

/// Delete a file, or a directory and all of its contents.
#[tauri::command]
pub fn delete_path(path: String) -> Result<(), String> {
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
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
            // Never FOLLOW symlinks/junctions: on Windows a junction can point at an
            // ancestor and is_dir()/metadata() (which follow the link) would loop forever
            // → hang. symlink_metadata() describes the entry itself; is_symlink() is true
            // for Windows reparse points (symlinks + junctions/mount points), so skip them.
            let md = match std::fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if md.file_type().is_symlink() {
                continue;
            }
            if md.is_dir() {
                if !IGNORED_DIRS.contains(&name.as_str()) {
                    stack.push(path);
                }
                continue;
            }
            if total >= SEARCH_MAX_RESULTS {
                break;
            }
            if md.len() > SEARCH_MAX_FILE {
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
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    fn temp_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "michael-ide-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn conditional_write_rejects_stale_content() {
        let path = temp_file("stale-write");
        std::fs::write(&path, "v1").unwrap();
        write_text_file_if_unchanged(
            path.to_string_lossy().into_owned(),
            Some("v1".into()),
            "v2".into(),
        )
        .unwrap();
        let error = write_text_file_if_unchanged(
            path.to_string_lossy().into_owned(),
            Some("v1".into()),
            "stale".into(),
        )
        .unwrap_err();
        assert!(error.contains("[CONFLICT]"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "v2");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn concurrent_conditional_writes_allow_only_one_winner() {
        let path = temp_file("concurrent-write");
        std::fs::write(&path, "v1").unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let mut threads = Vec::new();
        for value in ["from-a", "from-b"] {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            threads.push(std::thread::spawn(move || {
                barrier.wait();
                write_text_file_if_unchanged(
                    path.to_string_lossy().into_owned(),
                    Some("v1".into()),
                    value.into(),
                )
            }));
        }
        barrier.wait();
        let results = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
        let final_value = std::fs::read_to_string(&path).unwrap();
        assert!(final_value == "from-a" || final_value == "from-b");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn conditional_delete_preserves_a_newer_edit() {
        let path = temp_file("conditional-delete");
        std::fs::write(&path, "agent").unwrap();
        std::fs::write(&path, "user").unwrap();
        let error =
            delete_text_file_if_unchanged(path.to_string_lossy().into_owned(), "agent".into())
                .unwrap_err();
        assert!(error.contains("[CONFLICT]"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "user");

        delete_text_file_if_unchanged(path.to_string_lossy().into_owned(), "user".into()).unwrap();
        assert!(!path.exists());
    }
}
