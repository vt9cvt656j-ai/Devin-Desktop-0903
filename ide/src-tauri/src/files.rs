use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use serde::Serialize;

/// Maximum size of a file we will load into the editor (5 MiB).
const MAX_FILE: u64 = 5 * 1024 * 1024;

/// Files larger than this are skipped during project search (2 MiB).
const SEARCH_MAX_FILE: u64 = 2 * 1024 * 1024;
/// Hard cap on the total number of matches returned by a single search.
const SEARCH_MAX_RESULTS: usize = 2000;
/// Cap on matches reported per file so one file can't drown the results.
const SEARCH_MAX_PER_FILE: usize = 50;
/// Bound directory walks even when a query has no matches.
const SEARCH_MAX_SCANNED_FILES: usize = 20_000;
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

/// Stage complete, durable contents beside `path`. Callers publish the staged
/// inode with either replace or no-clobber semantics, then remove its temp name.
fn stage_text_file(
    path: &Path,
    content: &str,
    permissions: Option<std::fs::Permissions>,
) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| format!("cannot determine parent directory for '{}'", path.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("cannot create parent directory '{}': {e}", parent.display()))?;

    let file_name = path
        .file_name()
        .ok_or_else(|| format!("invalid file path '{}'", path.display()))?
        .to_string_lossy();

    let mut staged = None;
    let mut last_error = None;
    for _ in 0..8 {
        let candidate = parent.join(format!(
            ".{file_name}.michael-write-{}.tmp",
            uuid::Uuid::new_v4()
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                staged = Some((candidate, file));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                last_error = Some(error);
            }
            Err(error) => {
                return Err(format!(
                    "cannot stage write for '{}': {error}",
                    path.display()
                ));
            }
        }
    }

    let (temporary_path, mut temporary_file) = staged.ok_or_else(|| {
        format!(
            "cannot allocate a temporary file for '{}': {}",
            path.display(),
            last_error
                .map(|error| error.to_string())
                .unwrap_or_else(|| "temporary-name collision".into())
        )
    })?;

    let stage_result = (|| -> Result<(), String> {
        temporary_file
            .write_all(content.as_bytes())
            .map_err(|e| format!("cannot write staged contents for '{}': {e}", path.display()))?;
        temporary_file
            .flush()
            .map_err(|e| format!("cannot flush staged write for '{}': {e}", path.display()))?;
        if let Some(permissions) = permissions {
            temporary_file.set_permissions(permissions).map_err(|e| {
                format!("cannot preserve permissions for '{}': {e}", path.display())
            })?;
        }
        temporary_file
            .sync_all()
            .map_err(|e| format!("cannot sync staged write for '{}': {e}", path.display()))
    })();
    drop(temporary_file);

    if let Err(error) = stage_result {
        let _ = std::fs::remove_file(&temporary_path);
        return Err(error);
    }
    Ok(temporary_path)
}

fn sync_parent_directory(path: &Path) {
    if let Some(parent) = path.parent() {
        if let Ok(directory) = std::fs::File::open(parent) {
            let _ = directory.sync_all();
        }
    }
}

/// Replace `path` only after the complete new contents have been staged and
/// synced in the same directory. Keeping the temporary file beside the target
/// makes the final rename atomic on the target filesystem, so an interrupted or
/// failed write cannot leave the original file truncated or partially written.
fn atomic_write_text(path: &Path, content: &str) -> Result<(), String> {
    let original_permissions = std::fs::metadata(path)
        .ok()
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.permissions());
    let temporary_path = stage_text_file(path, content, original_permissions)?;
    if let Err(error) = atomic_replace_file(&temporary_path, path) {
        let _ = std::fs::remove_file(&temporary_path);
        return Err(format!(
            "cannot atomically replace '{}': {error}",
            path.display()
        ));
    }

    // The staged file itself is already durable. Syncing the directory also
    // persists the rename on filesystems that support directory fsync; failure
    // here is deliberately best-effort because the replacement has committed.
    sync_parent_directory(path);
    Ok(())
}

/// Publish a fully staged file only if `path` still does not exist. A hard link
/// is an atomic no-clobber operation on the same filesystem: either the complete
/// staged inode appears at `path`, or an independently-created target wins and
/// remains untouched.
fn atomic_create_text(path: &Path, content: &str) -> Result<(), String> {
    let temporary_path = stage_text_file(path, content, None)?;
    if let Err(error) = std::fs::hard_link(&temporary_path, path) {
        let _ = std::fs::remove_file(&temporary_path);
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            return Err("[CONFLICT] file was created by another task".into());
        }
        return Err(format!(
            "cannot atomically create '{}': {error}",
            path.display()
        ));
    }

    let cleanup = std::fs::remove_file(&temporary_path).map_err(|error| {
        format!(
            "file was created at '{}' but its staged link '{}' could not be removed: {error}",
            path.display(),
            temporary_path.display()
        )
    });
    sync_parent_directory(path);
    cleanup
}

#[cfg(not(windows))]
fn atomic_replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::rename(from, to)
}

#[cfg(windows)]
fn atomic_replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

    #[link(name = "Kernel32")]
    extern "system" {
        fn MoveFileExW(
            existing_file_name: *const u16,
            new_file_name: *const u16,
            flags: u32,
        ) -> i32;
    }

    let from_wide = from
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let to_wide = to
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let moved = unsafe {
        MoveFileExW(
            from_wide.as_ptr(),
            to_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

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

fn has_safe_prefix(path: &Path) -> bool {
    SAFE_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(Path::new(prefix)))
}

fn is_within_allowed_root(path: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|root| path.starts_with(root))
}

/// Verify `target` is inside an allowed workspace root.  Resolves symlinks and
/// normalises components so that `../../etc/passwd` tricks are caught even when
/// intermediate directories exist.
fn require_inside_workspace(target: &str) -> Result<PathBuf, String> {
    let target_path = Path::new(target);
    let raw_target = target.to_string();

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
    if has_safe_prefix(&resolved) {
        return Ok(resolved);
    }

    let roots = ALLOWED_ROOTS.lock().map_err(|e| e.to_string())?;
    if roots.is_empty() {
        return Ok(resolved);
    }

    // Compare only canonicalized paths. Checking the raw user-supplied path here
    // would let a symlink inside a workspace authorize its target outside it.
    if is_within_allowed_root(&resolved, &roots) {
        return Ok(resolved);
    }

    // Always allow paths under HOME regardless of what roots are registered.
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        if let Ok(home_canonical) = std::fs::canonicalize(&home) {
            if resolved.starts_with(home_canonical) {
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
    let resolved = require_inside_workspace(&path)?;
    let meta =
        std::fs::metadata(&resolved).map_err(|e| format!("cannot stat '{}': {}", path, e))?;
    if meta.is_dir() {
        return Err(format!(
            "'{}' is a directory, not a file. Use read_dir to list its contents.",
            path
        ));
    }
    if meta.len() > MAX_FILE {
        return Err("file is too large to open in the editor (> 5 MB)".into());
    }
    let bytes = std::fs::read(&resolved).map_err(|e| format!("cannot read '{}': {}", path, e))?;
    if bytes.iter().take(8000).any(|&b| b == 0) {
        return Err("cannot open a binary file in the editor".into());
    }
    String::from_utf8(bytes)
        .map_err(|_| format!("cannot open '{}': file is not valid UTF-8 text", path))
}

/// Read any (binary) file as a `data:<mime>;base64,...` URL. Used to render images in the
/// chat (e.g. `design_board` wardrobe previews) WITHOUT the Tauri asset protocol, whose glob
/// scope rejects hidden directories like `.wardrobe/` → the `<img>` stays blank. Reusing the
/// genimage `data_url` is the fast path; this is the universal fallback (works for pre-existing
/// images, cross-session, any path inside the workspace).
fn data_url_mime(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ogv" | "ogg" => "video/ogg",
        "mov" => "video/quicktime",
        "m4v" => "video/x-m4v",
        _ => "application/octet-stream",
    }
}

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
    let mime = data_url_mime(std::path::Path::new(&path));
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
    let resolved = require_inside_workspace(&path)?;
    atomic_write_text(&resolved, &content)
}

/// Write only if the file still has the exact version the caller read. `None`
/// means the caller observed no file and therefore uses a filesystem-level
/// atomic no-clobber create. Existing-file compare-and-replace is serialized
/// across IDE writers by FILE_MUTATION_LOCK; external processes do not share
/// that lock and can still race in the small interval after the comparison.
#[tauri::command]
pub fn write_text_file_if_unchanged(
    path: String,
    expected_content: Option<String>,
    content: String,
) -> Result<(), String> {
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
    let resolved = require_inside_workspace(&path)?;
    match expected_content {
        Some(expected) => {
            if !resolved.exists() {
                return Err("[CONFLICT] file was deleted after it was read".into());
            }
            let current = std::fs::read_to_string(&resolved).map_err(|e| e.to_string())?;
            if current != expected {
                return Err("[CONFLICT] file changed after it was read".into());
            }
            atomic_write_text(&resolved, &content)
        }
        None => atomic_create_text(&resolved, &content),
    }
}

/// Delete a text file only if it still contains the version the caller last
/// wrote. This is the deletion counterpart to `write_text_file_if_unchanged`
/// and keeps Undo/revert from removing a newer IDE edit. External processes do
/// not share FILE_MUTATION_LOCK and can still race after the content comparison.
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
    let resolved = require_inside_workspace(&path)?;
    atomic_create_text(&resolved, "")
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

fn build_search_matcher(
    query: &str,
    case_sensitive: bool,
    mode: Option<&str>,
) -> Result<Regex, String> {
    if query.is_empty() {
        return Err("[INVALID_SEARCH_QUERY] search query cannot be empty".into());
    }

    let pattern = match mode
        .unwrap_or("literal")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "literal" => regex::escape(query),
        "regex" => query.to_string(),
        other => {
            return Err(format!(
                "[INVALID_SEARCH_MODE] unsupported search mode '{other}'; expected 'literal' or 'regex'"
            ));
        }
    };

    RegexBuilder::new(&pattern)
        .case_insensitive(!case_sensitive)
        .build()
        .map_err(|error| format!("[INVALID_SEARCH_PATTERN] invalid regex: {error}"))
}

fn find_matches_in_line<'a>(
    line: &'a str,
    matcher: &'a Regex,
) -> impl Iterator<Item = (usize, usize, usize)> + 'a {
    matcher.find_iter(line).map(|matched| {
        let start = line[..matched.start()].chars().count();
        let end = line[..matched.end()].chars().count();
        (start, start, end)
    })
}

struct ProjectSearch {
    files: Vec<FileMatches>,
    scanned_files: usize,
}

fn search_project_scope(
    root: &str,
    query: &str,
    case_sensitive: bool,
    mode: Option<&str>,
) -> Result<ProjectSearch, String> {
    let matcher = build_search_matcher(query, case_sensitive, mode)?;

    // Keep the search confined to a registered workspace root and validate the
    // resolved scope before traversal. `read_dir` on a file and `os.walk` on a
    // missing path previously looked exactly like a legitimate no-match result.
    let root_path = require_inside_workspace(root)?;
    let root_meta = std::fs::symlink_metadata(&root_path).map_err(|error| {
        format!(
            "[INVALID_SEARCH_SCOPE] cannot inspect search scope '{}': {error}",
            root_path.display()
        )
    })?;
    if !root_meta.is_dir() && !root_meta.is_file() {
        return Err(format!(
            "[INVALID_SEARCH_SCOPE] search scope '{}' is neither a file nor a directory",
            root_path.display()
        ));
    }

    let root_is_file = root_meta.is_file();
    let relative_base = if root_is_file {
        root_path.parent().unwrap_or(&root_path)
    } else {
        &root_path
    };
    let mut results: Vec<FileMatches> = Vec::new();
    let mut total = 0usize;
    let mut scanned_files = 0usize;
    let mut stack: Vec<PathBuf> = vec![root_path.clone()];

    while let Some(path) = stack.pop() {
        if total >= SEARCH_MAX_RESULTS || scanned_files >= SEARCH_MAX_SCANNED_FILES {
            break;
        }

        let md = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if md.file_type().is_symlink() {
            continue;
        }
        if md.is_dir() {
            if path != root_path
                && path
                    .file_name()
                    .is_some_and(|name| IGNORED_DIRS.contains(&name.to_string_lossy().as_ref()))
            {
                continue;
            }
            let rd = match std::fs::read_dir(&path) {
                Ok(entries) => entries,
                Err(_) => continue,
            };
            let mut children: Vec<PathBuf> = rd.flatten().map(|entry| entry.path()).collect();
            children.sort_by(|a, b| b.cmp(a));
            for child in children {
                let Some(name) = child.file_name().map(|name| name.to_string_lossy()) else {
                    continue;
                };
                if name.starts_with('.') {
                    continue;
                }
                stack.push(child);
            }
            continue;
        }
        if !md.is_file() || md.len() > SEARCH_MAX_FILE {
            continue;
        }

        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        if bytes.iter().take(8000).any(|&byte| byte == 0) {
            continue;
        }
        let content = match String::from_utf8(bytes) {
            Ok(content) => content,
            Err(_) => continue,
        };
        scanned_files += 1;

        let mut file_matches: Vec<SearchMatch> = Vec::new();
        for (line_index, line) in content.lines().enumerate() {
            if file_matches.len() >= SEARCH_MAX_PER_FILE || total >= SEARCH_MAX_RESULTS {
                break;
            }
            let mut hits = find_matches_in_line(line, &matcher).peekable();
            if hits.peek().is_none() {
                continue;
            }
            let display = if line.chars().count() > SEARCH_MAX_LINE_CHARS {
                let mut truncated: String = line.chars().take(SEARCH_MAX_LINE_CHARS).collect();
                truncated.push('\u{2026}');
                truncated
            } else {
                line.to_string()
            };
            for (column, start, end) in hits {
                if file_matches.len() >= SEARCH_MAX_PER_FILE || total >= SEARCH_MAX_RESULTS {
                    break;
                }
                file_matches.push(SearchMatch {
                    line: line_index + 1,
                    column: column + 1,
                    text: display.clone(),
                    start,
                    end,
                });
                total += 1;
            }
        }

        if !file_matches.is_empty() {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            let rel = path
                .strip_prefix(relative_base)
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

    if scanned_files == 0 {
        return Err(format!(
            "[NO_SEARCHABLE_FILES] search scope '{}' contained no readable UTF-8 text files",
            root_path.display()
        ));
    }

    results.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(ProjectSearch {
        files: results,
        scanned_files,
    })
}

/// Search one file, or recursively search a directory, for `query`.
///
/// `mode` defaults to literal matching for compatibility and accepts `regex`
/// explicitly. Dot-entries, common build/dependency directories, oversized
/// files, and binary files are skipped. Results and traversal are bounded.
#[tauri::command]
pub fn search_in_project(
    root: String,
    query: String,
    case_sensitive: bool,
    mode: Option<String>,
) -> Result<Vec<FileMatches>, String> {
    let search = search_project_scope(&root, &query, case_sensitive, mode.as_deref())?;
    debug_assert!(search.scanned_files > 0);
    Ok(search.files)
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
    use std::sync::atomic::{AtomicBool, Ordering};
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
    fn data_url_mime_recognizes_common_video_formats() {
        assert_eq!(data_url_mime(Path::new("clip.mp4")), "video/mp4");
        assert_eq!(data_url_mime(Path::new("clip.WEBM")), "video/webm");
        assert_eq!(data_url_mime(Path::new("clip.ogv")), "video/ogg");
        assert_eq!(data_url_mime(Path::new("clip.ogg")), "video/ogg");
        assert_eq!(data_url_mime(Path::new("clip.mov")), "video/quicktime");
        assert_eq!(data_url_mime(Path::new("clip.m4v")), "video/x-m4v");
    }

    #[test]
    fn data_url_mime_keeps_known_images_and_rejects_unknown_types() {
        assert_eq!(data_url_mime(Path::new("frame.png")), "image/png");
        assert_eq!(data_url_mime(Path::new("frame.jpeg")), "image/jpeg");
        assert_eq!(
            data_url_mime(Path::new("payload.html")),
            "application/octet-stream"
        );
    }

    fn staged_files_for(path: &Path) -> Vec<PathBuf> {
        let Some(parent) = path.parent() else {
            return Vec::new();
        };
        let prefix = format!(
            ".{}.michael-write-",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        std::fs::read_dir(parent)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|candidate| {
                candidate
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with(&prefix))
            })
            .collect()
    }

    #[test]
    fn safe_prefixes_require_a_path_component_boundary() {
        assert!(has_safe_prefix(Path::new("/tmp/project/file.txt")));
        assert!(has_safe_prefix(Path::new(
            "/private/var/folders/session/file.txt"
        )));
        assert!(!has_safe_prefix(Path::new("/tmp-evil/file.txt")));
        assert!(!has_safe_prefix(Path::new("/tmpfoo/file.txt")));
        assert!(!has_safe_prefix(Path::new(
            "/private/var/folders_evil/file.txt"
        )));
    }

    #[cfg(unix)]
    #[test]
    fn safe_prefix_check_happens_after_parent_components_are_resolved() {
        let escaped = std::fs::canonicalize("/tmp/../etc").unwrap();

        assert!(!has_safe_prefix(&escaped));
    }

    #[cfg(unix)]
    #[test]
    fn canonical_root_check_rejects_a_symlink_target_outside_the_workspace() {
        use std::os::unix::fs::symlink;

        let parent = temp_file("symlink-root-check");
        let root = parent.join("workspace");
        let outside = parent.join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("secret.txt"), "secret").unwrap();
        symlink(&outside, root.join("escape")).unwrap();

        let canonical_root = std::fs::canonicalize(&root).unwrap();
        let escaped = std::fs::canonicalize(root.join("escape/secret.txt")).unwrap();

        assert!(!is_within_allowed_root(&escaped, &[canonical_root]));
        let _ = std::fs::remove_dir_all(parent);
    }

    #[test]
    fn text_read_rejects_invalid_utf8_without_changing_the_file() {
        let path = temp_file("invalid-utf8");
        let original = [b'v', b'a', b'l', 0x80, b'e'];
        std::fs::write(&path, original).unwrap();

        let error = read_text_file(path.to_string_lossy().into_owned()).unwrap_err();

        assert!(error.contains("not valid UTF-8"));
        assert_eq!(std::fs::read(&path).unwrap(), original);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn atomic_create_paths_never_clobber_an_existing_target() {
        let path = temp_file("existing-create-target");
        std::fs::write(&path, "keep me").unwrap();

        let create_error = create_file(path.to_string_lossy().into_owned()).unwrap_err();
        assert!(create_error.contains("[CONFLICT]"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep me");
        assert!(staged_files_for(&path).is_empty());

        let conditional_error = write_text_file_if_unchanged(
            path.to_string_lossy().into_owned(),
            None,
            "replacement".into(),
        )
        .unwrap_err();
        assert!(conditional_error.contains("[CONFLICT]"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep me");
        assert!(staged_files_for(&path).is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn atomic_create_commands_publish_complete_new_files() {
        let empty_path = temp_file("create-empty-file");
        create_file(empty_path.to_string_lossy().into_owned()).unwrap();
        assert_eq!(std::fs::read(&empty_path).unwrap(), b"");
        assert!(staged_files_for(&empty_path).is_empty());

        let content_path = temp_file("conditional-create-file");
        write_text_file_if_unchanged(
            content_path.to_string_lossy().into_owned(),
            None,
            "complete contents".into(),
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(&content_path).unwrap(),
            "complete contents"
        );
        assert!(staged_files_for(&content_path).is_empty());

        let _ = std::fs::remove_file(empty_path);
        let _ = std::fs::remove_file(content_path);
    }

    #[test]
    fn concurrent_atomic_creates_publish_one_complete_winner() {
        let path = temp_file("concurrent-atomic-create");
        let first = "a".repeat(2 * 1024 * 1024);
        let second = "b".repeat(2 * 1024 * 1024);
        let barrier = Arc::new(Barrier::new(4));

        let mut writers = Vec::new();
        for content in [first.clone(), second.clone()] {
            let writer_path = path.clone();
            let writer_barrier = Arc::clone(&barrier);
            writers.push(std::thread::spawn(move || {
                writer_barrier.wait();
                atomic_create_text(&writer_path, &content)
            }));
        }

        let reader_path = path.clone();
        let reader_barrier = Arc::clone(&barrier);
        let expected_first = first.clone();
        let expected_second = second.clone();
        let reader = std::thread::spawn(move || {
            reader_barrier.wait();
            loop {
                match std::fs::read_to_string(&reader_path) {
                    Ok(observed) => {
                        assert!(observed == expected_first || observed == expected_second);
                        return observed;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        std::thread::yield_now();
                    }
                    Err(error) => panic!("cannot observe atomic create: {error}"),
                }
            }
        });

        barrier.wait();
        let results = writers
            .into_iter()
            .map(|writer| writer.join().unwrap())
            .collect::<Vec<_>>();
        let observed = reader.join().unwrap();

        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), observed);
        assert!(staged_files_for(&path).is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn unconditional_write_allows_intentional_empty_content() {
        let path = temp_file("empty-write");
        std::fs::write(&path, "previous contents").unwrap();

        write_text_file(path.to_string_lossy().into_owned(), String::new()).unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "");
        assert!(staged_files_for(&path).is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn atomic_write_never_exposes_truncated_or_partial_content() {
        let path = temp_file("atomic-visibility");
        let first = "a".repeat(2 * 1024 * 1024);
        let second = "b".repeat(2 * 1024 * 1024);
        std::fs::write(&path, &first).unwrap();

        let done = Arc::new(AtomicBool::new(false));
        let started = Arc::new(Barrier::new(2));
        let reader_done = Arc::clone(&done);
        let reader_started = Arc::clone(&started);
        let reader_path = path.clone();
        let reader_first = first.clone();
        let reader_second = second.clone();
        let reader = std::thread::spawn(move || {
            let initial = std::fs::read_to_string(&reader_path).unwrap();
            assert_eq!(initial, reader_first);
            let mut observations = 1;
            reader_started.wait();
            while !reader_done.load(Ordering::Acquire) {
                let observed = std::fs::read_to_string(&reader_path).unwrap();
                assert!(observed == reader_first || observed == reader_second);
                observations += 1;
            }
            observations
        });

        started.wait();
        for index in 0..6 {
            let content = if index % 2 == 0 { &second } else { &first };
            write_text_file(path.to_string_lossy().into_owned(), content.clone()).unwrap();
        }
        done.store(true, Ordering::Release);

        assert!(reader.join().unwrap() > 0);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), first);
        assert!(staged_files_for(&path).is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn failed_atomic_replace_keeps_target_and_cleans_staged_file() {
        let path = temp_file("atomic-replace-failure");
        std::fs::create_dir(&path).unwrap();

        let error =
            write_text_file(path.to_string_lossy().into_owned(), "content".into()).unwrap_err();

        assert!(error.contains("atomically replace"));
        assert!(path.is_dir());
        assert!(staged_files_for(&path).is_empty());
        let _ = std::fs::remove_dir(path);
    }

    #[cfg(unix)]
    #[test]
    fn unconditional_write_uses_the_resolved_symlink_target() {
        use std::os::unix::fs::symlink;

        let target = temp_file("resolved-target");
        let link = temp_file("resolved-link");
        std::fs::write(&target, "old").unwrap();
        symlink(&target, &link).unwrap();

        write_text_file(link.to_string_lossy().into_owned(), "new".into()).unwrap();

        assert!(std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
        assert_eq!(std::fs::read_to_string(&link).unwrap(), "new");
        assert!(staged_files_for(&target).is_empty());
        let _ = std::fs::remove_file(link);
        let _ = std::fs::remove_file(target);
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_preserves_existing_unix_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let path = temp_file("preserve-permissions");
        std::fs::write(&path, "old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o751)).unwrap();

        write_text_file(path.to_string_lossy().into_owned(), "new".into()).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o751);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        let _ = std::fs::remove_file(path);
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

    #[test]
    fn project_search_defaults_to_literal_and_supports_regex_for_file_scope() {
        let path = temp_file("search-single-file");
        std::fs::write(&path, "alpha[1]\nALPHA 42\nalpha111\n").unwrap();

        let literal =
            search_project_scope(&path.to_string_lossy(), "alpha[1]", false, None).unwrap();
        assert_eq!(literal.scanned_files, 1);
        assert_eq!(literal.files.len(), 1);
        assert_eq!(
            literal.files[0].rel,
            path.file_name().unwrap().to_string_lossy()
        );
        assert_eq!(literal.files[0].matches.len(), 1);
        assert_eq!(literal.files[0].matches[0].line, 1);

        let regex = search_project_scope(
            &path.to_string_lossy(),
            r"alpha(?:\[\d+\]|\s+\d+)",
            false,
            Some("regex"),
        )
        .unwrap();
        assert_eq!(regex.scanned_files, 1);
        assert_eq!(regex.files[0].matches.len(), 2);
        assert_eq!(regex.files[0].matches[0].line, 1);
        assert_eq!(regex.files[0].matches[1].line, 2);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn project_search_reports_invalid_mode_pattern_and_scope() {
        let path = temp_file("search-errors");
        std::fs::write(&path, "content\n").unwrap();

        let bad_mode = search_project_scope(&path.to_string_lossy(), "x", false, Some("glob"))
            .err()
            .expect("glob mode must be rejected");
        assert!(bad_mode.contains("[INVALID_SEARCH_MODE]"));

        let bad_pattern = search_project_scope(&path.to_string_lossy(), "[", false, Some("regex"))
            .err()
            .expect("invalid regex must be rejected");
        assert!(bad_pattern.contains("[INVALID_SEARCH_PATTERN]"));

        let missing = temp_file("missing-search-scope");
        let bad_scope = search_project_scope(&missing.to_string_lossy(), "x", false, None)
            .err()
            .expect("missing scope must be rejected");
        assert!(bad_scope.contains("[INVALID_SEARCH_SCOPE]"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn project_search_distinguishes_no_matches_from_zero_scanned_files() {
        let root = temp_file("search-directory");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("one.txt"), "first\n").unwrap();
        std::fs::write(root.join("two.txt"), "second\n").unwrap();

        let no_match =
            search_project_scope(&root.to_string_lossy(), "absent", false, None).unwrap();
        assert_eq!(no_match.scanned_files, 2);
        assert!(no_match.files.is_empty());

        let empty = temp_file("empty-search-directory");
        std::fs::create_dir(&empty).unwrap();
        let no_files = search_project_scope(&empty.to_string_lossy(), "anything", false, None)
            .err()
            .expect("empty scope must be rejected");
        assert!(no_files.contains("[NO_SEARCHABLE_FILES]"));

        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(empty);
    }
}
