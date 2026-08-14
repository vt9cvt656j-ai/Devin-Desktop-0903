use std::io::{Read, Seek, SeekFrom, Write};
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
const INSPECT_FULL_PARSE_MAX: u64 = 64 * 1024 * 1024;
const INSPECT_HEX_BYTES: usize = 16 * 1024;
const INSPECT_STRING_SCAN_BYTES: usize = 1024 * 1024;

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
    // Additional universally-vendored / generated trees so search returns signal,
    // not noise (dot-dirs like .next/.gradle/.venv are already skipped separately).
    "bower_components",
    "Pods",
    "venv",
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

/// 新建一个项目目录并把它注册成工作区根。
///
/// # 为什么需要一个专用命令
///
/// `create_dir` 走 `require_inside_workspace`，也就是"必须先有工作区才能建目录"——而这里要解决的
/// 恰恰是**一个工作区都还没有**的情况。用户说「帮我写个 telegram 机器人」而左边没打开任何文件夹时，
/// 智能体以前只能把问题退回给用户：它自己开不了工。这是它唯一真正缺的能力，不是策略问题。
///
/// # 边界
///
/// 位置**不由调用方决定**。模型只能给一个名字，目录一律建在 `~/MrDayOne/<name>` 下：
/// - 名字过一遍白名单（字母数字、`.`、`_`、`-`），任何分隔符和 `..` 直接拒——所以拼不出逃逸路径；
/// - 建完仍然走 `register_workspace_root` 的全部守卫（规范化、深度 ≤2 拒绝、系统目录黑名单）；
/// - 已存在同名目录时**不覆盖**，直接复用并注册——重复调用是幂等的，不会悄悄清空谁的代码。
///
/// 换句话说：它能创造的最坏结果，是在用户自己的主目录下多出一个空文件夹。
#[tauri::command(async)]
pub fn create_project_dir(name: String) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("project name is empty".into());
    }
    // 单段名字，白名单字符。`/`、`\`、`..` 一律不在表内，所以拼不出逃逸路径。
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
        || trimmed.starts_with('.')
        || trimmed.contains("..")
    {
        return Err(format!(
            "invalid project name '{trimmed}': use letters, digits, '.', '_' or '-' only, and not starting with '.'"
        ));
    }
    if trimmed.len() > 64 {
        return Err("project name is too long (max 64)".into());
    }
    let home = home_dir().ok_or_else(|| "cannot resolve home directory".to_string())?;
    let base = PathBuf::from(home).join("MrDayOne");
    std::fs::create_dir_all(&base).map_err(|e| friendly_write_error("创建目录", &base.to_string_lossy(), &e))?;
    let target = base.join(trimmed);
    if !target.exists() {
        std::fs::create_dir_all(&target)
            .map_err(|e| friendly_write_error("创建目录", &target.to_string_lossy(), &e))?;
    } else if !target.is_dir() {
        return Err(format!("{} already exists and is not a directory", target.display()));
    }
    // 复用同一套注册守卫——这里不另开一条路径。
    let canonical = std::fs::canonicalize(&target).map_err(|e| e.to_string())?;
    register_workspace_root(canonical.to_string_lossy().into_owned())?;
    Ok(canonical.to_string_lossy().into_owned())
}

/// Register a workspace root that the user explicitly opened.
/// Called from the frontend after a successful folder-open dialog.
#[tauri::command(async)]
pub fn register_workspace_root(path: String) -> Result<(), String> {
    let raw_path = PathBuf::from(&path);
    let canonical = std::fs::canonicalize(&path).map_err(|e| e.to_string())?;
    // Block overly broad roots that would defeat the workspace boundary.
    let depth = canonical.components().count();
    if depth <= 2 {
        return Err(format!(
            "refusing to register '{}' as a workspace root: path is too broad (depth {depth}).",
            canonical.display()
        ));
    }
    // Depth alone is not a safety property. `/etc` and `/Users` are depth 2 and were
    // caught, but `/usr/local`, `/private/etc`, `/etc/ssl` and `/Library/LaunchAgents`
    // are all depth >= 3 and registered cleanly — each one granting write access to a
    // directory that can change what the machine executes on next login. Name the
    // system trees explicitly instead of inferring safety from a component count.
    if let Some(blocked) = blocked_root_prefix(&canonical) {
        return Err(format!(
            "refusing to register '{}' as a workspace root: '{}' is a system directory.",
            canonical.display(),
            blocked
        ));
    }
    if !canonical.is_dir() {
        return Err("workspace root must be a directory".into());
    }
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

/// System trees that must never become a workspace root. Registering one grants write
/// access to everything under it, and each of these can change what the machine runs:
/// launch agents/daemons, shell profiles, PAM/sudo config, CA trust stores, package
/// prefixes that sit on PATH. Compared component-wise via `Path::starts_with`, so
/// `/usr/locale` does not match `/usr/local`.
#[cfg(not(windows))]
const BLOCKED_ROOT_PREFIXES: &[&str] = &[
    "/bin", "/sbin", "/boot", "/dev", "/proc", "/sys",
    "/etc", "/private/etc",
    "/usr", "/opt",
    "/var", "/private/var",
    "/Library", "/System", "/Applications",
];
#[cfg(windows)]
const BLOCKED_ROOT_PREFIXES: &[&str] = &[
    "C:\\Windows",
    "C:\\Program Files",
    "C:\\Program Files (x86)",
    "C:\\ProgramData",
];

/// The blocked system prefix `path` sits under, if any.
///
/// Temp directories are carved back out: on macOS `std::env::temp_dir()` lives under
/// `/var/folders`, and on Linux under `/tmp` — both inside blocked trees, and both
/// legitimate scratch space that `SAFE_PREFIXES` already grants.
fn blocked_root_prefix(path: &Path) -> Option<&'static str> {
    if has_safe_prefix(path) {
        return None;
    }
    BLOCKED_ROOT_PREFIXES
        .iter()
        .find(|prefix| path.starts_with(Path::new(prefix)))
        .copied()
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
/// Require that a target path is inside one of the registered workspace roots, or (for
/// read-only operations) under the user's home directory. Write/delete/rename operations
/// must stay strictly within workspace to prevent writing ~/.ssh/id_rsa, etc.
pub fn require_inside_workspace(target: &str, is_write_op: bool) -> Result<PathBuf, String> {
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
    // Temp dirs are deliberate scratch space — staging, atomic writes, archive
    // extraction — and stay available whether or not a workspace is open. This is an
    // explicit, narrow allowance, not the general boundary.
    if has_safe_prefix(&resolved) {
        return Ok(resolved);
    }

    let roots = ALLOWED_ROOTS.lock().map_err(|e| e.to_string())?;
    // Fail CLOSED. This used to `return Ok(resolved)` on an empty root list, which
    // handed out unrestricted read AND write access to the entire filesystem the moment
    // the list happened to be empty. `bootstrap_home_root` normally fills it during
    // `setup()`, but it is best-effort: if HOME/USERPROFILE is unset or canonicalize
    // fails, the list stays empty — and the security boundary silently disappeared in
    // exactly the degraded environment where it matters most. A missing boundary must
    // deny, never allow.
    if roots.is_empty() {
        return Err(format!(
            "access denied: no workspace root is registered yet, so '{}' cannot be \
             {}. Open a folder first (temp directories remain available).",
            raw_target,
            if is_write_op { "written" } else { "read" }
        ));
    }

    // Compare only canonicalized paths. Checking the raw user-supplied path here
    // would let a symlink inside a workspace authorize its target outside it.
    if is_within_allowed_root(&resolved, &roots) {
        if is_write_op {
            // HOME is in ALLOWED_ROOTS for read convenience, but write/delete/rename
            // must only succeed when the path is under an explicitly opened workspace
            // root — not just under ~/. Otherwise ~/.ssh, ~/.bashrc etc. are writable.
            let home_raw = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .ok()
                .map(PathBuf::from);
            let home_canonical = home_raw
                .as_ref()
                .and_then(|h| std::fs::canonicalize(h).ok());
            #[cfg(unix)]
            let home_meta = home_canonical
                .as_ref()
                .and_then(|h| std::fs::metadata(h).ok());
            let is_home = |root: &PathBuf| -> bool {
                // String comparison catches the common case.
                if home_canonical.as_ref().map_or(false, |h| root == h)
                    || home_raw.as_ref().map_or(false, |h| root == h)
                {
                    return true;
                }
                // On macOS, firmlinks (e.g. /System/Volumes/Data/Users/X vs /Users/X)
                // produce different canonical paths for the same inode. Compare by
                // device + inode to defeat that aliasing.
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    if let (Some(hm), Ok(rm)) = (&home_meta, std::fs::metadata(root)) {
                        if hm.dev() == rm.dev() && hm.ino() == rm.ino() {
                            return true;
                        }
                    }
                }
                false
            };
            let under_workspace = roots
                .iter()
                .any(|root| resolved.starts_with(root) && !is_home(root));
            if !under_workspace {
                return Err(format!(
                    "write denied: '{}' is under HOME but not inside any opened workspace.",
                    raw_target
                ));
            }
        }
        return Ok(resolved);
    }

    // For read-only operations, allow paths under HOME as a fallback (useful for
    // reading logs, config files, etc. when HOME was evicted from ALLOWED_ROOTS).
    if !is_write_op {
        if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            if let Ok(home_canonical) = std::fs::canonicalize(&home) {
                if resolved.starts_with(home_canonical) {
                    return Ok(resolved);
                }
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
// These commands perform synchronous filesystem work. Tauri's async execution
// context keeps slow disks, network mounts, recursive walks, and large files
// away from the native window event thread without changing their IPC contract.
#[tauri::command(async)]
pub fn read_dir(path: String) -> Result<Vec<DirEntry>, String> {
    require_inside_workspace(&path, false)?;
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

/// Translate a raw IO error into a friendly, actionable message instead of
/// leaking `os error N` straight to the UI (see PATH_RESOLUTION_DIAGNOSIS.md).
fn friendly_read_error(path: &str, e: &std::io::Error) -> String {
    match e.kind() {
        // Human-facing only. This string is rendered as a UI toast, so it must not carry
        // agent coaching ("先用 list_dir/find_files 确认真实路径…"): the user is not the one
        // guessing paths, and the toast covered the terminal to say so. The model gets that
        // guidance on its own channel — the tool result — from `pathGuidance` in main.js.
        std::io::ErrorKind::NotFound => format!("文件不存在：{}", path),
        std::io::ErrorKind::PermissionDenied => format!("没有读取权限：{}", path),
        std::io::ErrorKind::IsADirectory => {
            format!("{} 是目录不是文件，请用 read_dir 查看其内容", path)
        }
        std::io::ErrorKind::NotADirectory => format!(
            "路径中间有一段不是目录：{}（可能把某个文件当成了目录）",
            path
        ),
        kind => format!("读取失败：{}（{}）", path, kind),
    }
}

fn friendly_write_error(op: &str, path: &str, e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::NotFound => format!(
            "{}失败：路径不存在：{}（请先用 list_dir 确认父目录存在）",
            op, path
        ),
        std::io::ErrorKind::PermissionDenied => format!("{}失败：没有写入权限：{}", op, path),
        std::io::ErrorKind::AlreadyExists => format!("{}失败：目标已存在：{}", op, path),
        std::io::ErrorKind::IsADirectory => format!("{}失败：{} 是目录", op, path),
        std::io::ErrorKind::NotADirectory => {
            format!("{}失败：路径中间有一段不是目录：{}", op, path)
        }
        std::io::ErrorKind::DirectoryNotEmpty => format!("{}失败：目录不为空：{}", op, path),
        kind => format!("{}失败：{}（{}）", op, path, kind),
    }
}

/// Read a UTF-8 text file. Rejects directories, oversized and binary files so
/// the editor never tries to render garbage.
#[tauri::command(async)]
pub fn read_text_file(path: String) -> Result<String, String> {
    let resolved = require_inside_workspace(&path, false)?;
    let meta = std::fs::metadata(&resolved).map_err(|e| friendly_read_error(&path, &e))?;
    if meta.is_dir() {
        return Err(format!(
            "'{}' is a directory, not a file. Use read_dir to list its contents.",
            path
        ));
    }
    if meta.len() > MAX_FILE {
        return Err("file is too large to open in the editor (> 5 MB)".into());
    }
    let bytes = std::fs::read(&resolved).map_err(|e| friendly_read_error(&path, &e))?;
    if bytes.iter().take(8000).any(|&b| b == 0) {
        return Err("cannot open a binary file in the editor".into());
    }
    String::from_utf8(bytes)
        .map_err(|_| format!("cannot open '{}': file is not valid UTF-8 text", path))
}

/// Read the tail of a log-like text file, including common logs under the user's
/// home directory (npm debug logs, app logs, temp logs). Unlike `read_text_file`,
/// this does not load the entire file, so a large debug log can still be used as
/// evidence by the agent without opening it in the editor.
#[tauri::command(async)]
pub fn read_log_tail(
    path: String,
    lines: Option<usize>,
    max_bytes: Option<usize>,
) -> Result<String, String> {
    let path = if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| "cannot expand ~/ because HOME is not set".to_string())?;
        Path::new(&home).join(rest).to_string_lossy().to_string()
    } else {
        path
    };
    let resolved = require_inside_workspace(&path, false)?;
    let meta = std::fs::metadata(&resolved).map_err(|e| friendly_read_error(&path, &e))?;
    if meta.is_dir() {
        return Err(format!("'{}' is a directory, not a log file.", path));
    }
    let name = resolved
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let ext = resolved
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let log_like = matches!(ext.as_str(), "log" | "out" | "err")
        || name.ends_with("-debug.log")
        || name.contains("debug") && name.ends_with(".txt");
    if !log_like {
        return Err("read_log_tail only reads log-like files (.log/.out/.err/debug*.txt)".into());
    }

    let line_limit = lines.unwrap_or(200).clamp(1, 2000);
    let byte_limit = max_bytes
        .unwrap_or(512 * 1024)
        .clamp(4 * 1024, 2 * 1024 * 1024) as u64;
    let file_len = meta.len();
    let start = file_len.saturating_sub(byte_limit);
    let mut file = std::fs::File::open(&resolved).map_err(|e| friendly_read_error(&path, &e))?;
    file.seek(SeekFrom::Start(start))
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::with_capacity((file_len - start).min(byte_limit) as usize);
    file.read_to_end(&mut bytes)
        .map_err(|e| friendly_read_error(&path, &e))?;
    if bytes.iter().take(8000).any(|&b| b == 0) {
        return Err("cannot read log tail: file appears to be binary".into());
    }
    let mut text = String::from_utf8_lossy(&bytes).to_string();
    if start > 0 {
        if let Some(idx) = text.find('\n') {
            text = text[idx + 1..].to_string();
        }
    }
    let mut tail_lines: Vec<&str> = text.lines().rev().take(line_limit).collect();
    tail_lines.reverse();
    let mut out = tail_lines.join("\n");
    if start > 0 {
        out = format!(
            "…（日志较大，仅读取最后约 {} KB / {} 行）\n{}",
            byte_limit / 1024,
            line_limit,
            out
        );
    }
    Ok(out)
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

#[tauri::command(async)]
pub fn read_file_data_url(path: String) -> Result<String, String> {
    require_inside_workspace(&path, false)?;
    let meta = std::fs::metadata(&path).map_err(|e| friendly_read_error(&path, &e))?;
    if meta.is_dir() {
        return Err(format!("'{}' is a directory, not a file.", path));
    }
    if meta.len() > 25 * 1024 * 1024 {
        return Err(format!(
            "file too large ({} bytes) for a data URL",
            meta.len()
        ));
    }
    let bytes = std::fs::read(&path).map_err(|e| friendly_read_error(&path, &e))?;
    let mime = data_url_mime(std::path::Path::new(&path));
    Ok(format!(
        "data:{};base64,{}",
        mime,
        crate::capture::b64(&bytes)
    ))
}

#[derive(Serialize)]
pub struct HexRow {
    offset: u64,
    hex: String,
    ascii: String,
}

#[derive(Serialize)]
pub struct SqlitePreview {
    page_size: u32,
    page_count: u32,
    schema_format: u32,
    text_encoding: String,
    tables: Vec<SqliteTablePreview>,
}

#[derive(Serialize)]
pub struct SqliteColumnPreview {
    name: String,
    decl_type: String,
    not_null: bool,
    primary_key: bool,
}

#[derive(Serialize)]
pub struct SqliteTablePreview {
    name: String,
    table_type: String,
    row_count: Option<i64>,
    columns: Vec<SqliteColumnPreview>,
    sample_rows: Vec<serde_json::Value>,
}

#[derive(Serialize)]
pub struct ImagePreviewInfo {
    width: u32,
    height: u32,
}

#[derive(Serialize)]
pub struct TrainedDataComponentPreview {
    type_index: usize,
    name: String,
    offset: u64,
    size: u64,
    hex_prefix: String,
    text_preview: Option<String>,
}

#[derive(Serialize)]
pub struct TrainedDataPreview {
    entry_count: usize,
    present_count: usize,
    version: Option<String>,
    components: Vec<TrainedDataComponentPreview>,
}

#[derive(Serialize)]
pub struct FileInspection {
    path: String,
    name: String,
    extension: String,
    kind: String,
    mime: String,
    size: u64,
    modified_ms: Option<u64>,
    readonly: bool,
    sampled: bool,
    signature: String,
    summary: Vec<String>,
    text_preview: Option<String>,
    hex_rows: Vec<HexRow>,
    strings: Vec<String>,
    traineddata: Option<TrainedDataPreview>,
    archive: Option<crate::archive::ArchiveListing>,
    sqlite: Option<SqlitePreview>,
    image: Option<ImagePreviewInfo>,
}

fn file_extension(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string()
}

fn read_prefix(path: &Path, len: usize) -> Result<Vec<u8>, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| friendly_read_error(&path.display().to_string(), &e))?;
    let mut bytes = vec![0u8; len];
    let n = file
        .read(&mut bytes)
        .map_err(|e| friendly_read_error(&path.display().to_string(), &e))?;
    bytes.truncate(n);
    Ok(bytes)
}

fn hex_prefix(bytes: &[u8], len: usize) -> String {
    bytes
        .iter()
        .take(len)
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_hex_rows(bytes: &[u8]) -> Vec<HexRow> {
    let take = bytes.len().min(INSPECT_HEX_BYTES);
    bytes[..take]
        .chunks(16)
        .enumerate()
        .map(|(idx, chunk)| HexRow {
            offset: (idx * 16) as u64,
            hex: chunk
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" "),
            ascii: chunk
                .iter()
                .map(|&b| {
                    if (0x20..=0x7e).contains(&b) {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect(),
        })
        .collect()
}

fn clean_preview_text(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for ch in text.replace('\0', "").chars() {
        if out.chars().count() >= max_chars {
            out.push('…');
            break;
        }
        if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out.trim().to_string()
}

fn text_preview(bytes: &[u8]) -> Option<String> {
    let take = bytes.len().min(64 * 1024);
    let sample = &bytes[..take];
    if sample.iter().take(8000).any(|&b| b == 0) {
        return None;
    }
    let text = std::str::from_utf8(sample).ok()?;
    let cleaned = clean_preview_text(text, 5000);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn printable_preview(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let take = bytes.len().min(4096);
    let sample = &bytes[..take];
    let text = std::str::from_utf8(sample).ok()?;
    let printable = text
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
        .count();
    let total = text.chars().count().max(1);
    if printable * 100 / total < 70 {
        return None;
    }
    let cleaned = clean_preview_text(text, 1200);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn extract_ascii_strings(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut run = Vec::new();
    let scan = &bytes[..bytes.len().min(INSPECT_STRING_SCAN_BYTES)];
    let flush = |run: &mut Vec<u8>, out: &mut Vec<String>| {
        if run.len() >= 4 {
            let text = String::from_utf8_lossy(run).trim().to_string();
            if !text.is_empty()
                && out.last().map(|prev| prev != &text).unwrap_or(true)
                && !out.contains(&text)
            {
                out.push(text.chars().take(160).collect());
            }
        }
        run.clear();
    };
    for &b in scan {
        if b == b'\t' || b == b' ' || (0x21..=0x7e).contains(&b) {
            run.push(b);
        } else {
            flush(&mut run, &mut out);
            if out.len() >= 120 {
                return out;
            }
        }
    }
    flush(&mut run, &mut out);
    out.truncate(120);
    out
}

fn read_be_u16(bytes: &[u8], start: usize) -> Option<u16> {
    let slice = bytes.get(start..start + 2)?;
    Some(u16::from_be_bytes([slice[0], slice[1]]))
}

fn read_be_u32(bytes: &[u8], start: usize) -> Option<u32> {
    let slice = bytes.get(start..start + 4)?;
    Some(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn quote_sqlite_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn sqlite_value_to_json(row: &sqlx::sqlite::SqliteRow, index: usize) -> serde_json::Value {
    use sqlx::Row;
    if let Ok(v) = row.try_get::<Option<i64>, _>(index) {
        return v
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<f64>, _>(index) {
        return v
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<String>, _>(index) {
        return v
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(index) {
        return v
            .map(|bytes| {
                serde_json::json!({
                    "blob_bytes": bytes.len(),
                    "preview": hex_prefix(&bytes, 16)
                })
            })
            .unwrap_or(serde_json::Value::Null);
    }
    serde_json::Value::Null
}

fn inspect_sqlite_tables(path: &Path) -> Vec<SqliteTablePreview> {
    let path = path.to_path_buf();
    tauri::async_runtime::block_on(async move {
        use sqlx::{Column, Connection, Row};
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&path)
            .read_only(true)
            .create_if_missing(false);
        let mut conn = match sqlx::SqliteConnection::connect_with(&options).await {
            Ok(conn) => conn,
            Err(_) => return Vec::new(),
        };
        let objects = match sqlx::query(
            "SELECT name, type FROM sqlite_master \
             WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' \
             ORDER BY type, name LIMIT 40",
        )
        .fetch_all(&mut conn)
        .await
        {
            Ok(rows) => rows,
            Err(_) => return Vec::new(),
        };

        let mut tables = Vec::new();
        for object in objects {
            let name = object.try_get::<String, _>("name").unwrap_or_default();
            if name.is_empty() {
                continue;
            }
            let table_type = object
                .try_get::<String, _>("type")
                .unwrap_or_else(|_| "table".to_string());
            let pragma_name = name.replace('\'', "''");
            let column_rows = sqlx::query(sqlx::AssertSqlSafe(format!(
                "PRAGMA table_info('{pragma_name}')"
            )))
            .fetch_all(&mut conn)
            .await
            .unwrap_or_default();
            let columns = column_rows
                .into_iter()
                .map(|row| SqliteColumnPreview {
                    name: row.try_get::<String, _>("name").unwrap_or_default(),
                    decl_type: row.try_get::<String, _>("type").unwrap_or_default(),
                    not_null: row.try_get::<i64, _>("notnull").unwrap_or(0) != 0,
                    primary_key: row.try_get::<i64, _>("pk").unwrap_or(0) != 0,
                })
                .collect::<Vec<_>>();
            let quoted = quote_sqlite_ident(&name);
            let row_count = sqlx::query(sqlx::AssertSqlSafe(format!(
                "SELECT COUNT(*) AS count FROM {quoted}"
            )))
            .fetch_one(&mut conn)
            .await
            .ok()
            .and_then(|row| row.try_get::<i64, _>("count").ok());
            let sample_rows = sqlx::query(sqlx::AssertSqlSafe(format!(
                "SELECT * FROM {quoted} LIMIT 20"
            )))
            .fetch_all(&mut conn)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|row| {
                let mut object = serde_json::Map::new();
                for (index, column) in row.columns().iter().enumerate().take(16) {
                    object.insert(column.name().to_string(), sqlite_value_to_json(&row, index));
                }
                serde_json::Value::Object(object)
            })
            .collect::<Vec<_>>();
            tables.push(SqliteTablePreview {
                name,
                table_type,
                row_count,
                columns,
                sample_rows,
            });
        }
        tables
    })
}

fn sqlite_preview(bytes: &[u8], path: &Path) -> Option<SqlitePreview> {
    if !bytes.starts_with(b"SQLite format 3\0") || bytes.len() < 100 {
        return None;
    }
    let raw_page_size = read_be_u16(bytes, 16)?;
    let page_size = if raw_page_size == 1 {
        65536
    } else {
        raw_page_size as u32
    };
    let page_count = read_be_u32(bytes, 28).unwrap_or(0);
    let schema_format = read_be_u32(bytes, 44).unwrap_or(0);
    let encoding = match read_be_u32(bytes, 56).unwrap_or(0) {
        1 => "UTF-8",
        2 => "UTF-16le",
        3 => "UTF-16be",
        _ => "unknown",
    }
    .to_string();
    Some(SqlitePreview {
        page_size,
        page_count,
        schema_format,
        text_encoding: encoding,
        tables: inspect_sqlite_tables(path),
    })
}

fn traineddata_component_name(index: usize) -> &'static str {
    match index {
        0 => "lang_config",
        1 => "unicharset",
        2 => "ambigs",
        3 => "inttemp",
        4 => "pffmtable",
        5 => "normproto",
        6 => "punc-dawg",
        7 => "system-dawg",
        8 => "number-dawg",
        9 => "freq-dawg",
        10 => "fixed-length-dawgs",
        11 => "cube-unicharset",
        12 => "cube-system-dawg",
        13 => "shape-table",
        14 => "bigram-dawg",
        15 => "unambig-dawg",
        16 => "params-model",
        17 => "lstm",
        18 => "lstm-punc-dawg",
        19 => "lstm-system-dawg",
        20 => "lstm-number-dawg",
        21 => "lstm-unicharset",
        22 => "lstm-recoder",
        23 => "version",
        _ => "unknown",
    }
}

fn read_le_i64(bytes: &[u8], start: usize) -> Option<i64> {
    let slice = bytes.get(start..start + 8)?;
    Some(i64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}

fn inspect_traineddata(bytes: &[u8], file_size: u64) -> Option<TrainedDataPreview> {
    let count_bytes = bytes.get(0..4)?;
    let entry_count = u32::from_le_bytes([
        count_bytes[0],
        count_bytes[1],
        count_bytes[2],
        count_bytes[3],
    ]) as usize;
    if entry_count == 0 || entry_count > 128 {
        return None;
    }
    let table_len = 4usize.checked_add(entry_count.checked_mul(8)?)?;
    if bytes.len() < table_len {
        return None;
    }
    let mut present = Vec::new();
    for i in 0..entry_count {
        let offset = read_le_i64(bytes, 4 + i * 8)?;
        if offset >= table_len as i64 && (offset as u64) < file_size {
            present.push((i, offset as u64));
        } else if offset != -1 {
            return None;
        }
    }
    if present.is_empty() {
        return None;
    }
    let mut by_offset = present.clone();
    by_offset.sort_by_key(|(_, offset)| *offset);
    if by_offset.windows(2).any(|w| w[0].1 >= w[1].1) {
        return None;
    }

    let mut components = Vec::new();
    for (pos, (type_index, offset)) in by_offset.iter().enumerate() {
        let next = by_offset
            .get(pos + 1)
            .map(|(_, next_offset)| *next_offset)
            .unwrap_or(file_size);
        if next <= *offset {
            continue;
        }
        let size = next - *offset;
        let local = if (*offset as usize) < bytes.len() {
            let end = ((*offset + size).min(bytes.len() as u64)) as usize;
            &bytes[*offset as usize..end]
        } else {
            &[]
        };
        components.push(TrainedDataComponentPreview {
            type_index: *type_index,
            name: traineddata_component_name(*type_index).to_string(),
            offset: *offset,
            size,
            hex_prefix: hex_prefix(local, 24),
            text_preview: printable_preview(local),
        });
    }
    if components.is_empty() {
        return None;
    }
    components.sort_by_key(|component| component.type_index);
    let version = components
        .iter()
        .find(|component| component.type_index == 23)
        .and_then(|component| component.text_preview.clone());
    Some(TrainedDataPreview {
        entry_count,
        present_count: components.len(),
        version,
        components,
    })
}

fn detect_kind_and_mime(ext: &str, bytes: &[u8], traineddata: bool) -> (String, String) {
    if traineddata || ext == "traineddata" {
        return (
            "tesseract_traineddata".into(),
            "application/x-tesseract-traineddata".into(),
        );
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return ("image".into(), "image/png".into());
    }
    if bytes.starts_with(b"\xFF\xD8\xFF") {
        return ("image".into(), "image/jpeg".into());
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return ("image".into(), "image/gif".into());
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return ("image".into(), "image/webp".into());
    }
    if bytes.starts_with(b"%PDF-") {
        return ("pdf".into(), "application/pdf".into());
    }
    if bytes.starts_with(b"PK\x03\x04")
        || bytes.starts_with(b"PK\x05\x06")
        || bytes.starts_with(b"PK\x07\x08")
    {
        return ("archive".into(), "application/zip".into());
    }
    if bytes.starts_with(b"SQLite format 3\0") {
        return ("sqlite_database".into(), "application/vnd.sqlite3".into());
    }
    if bytes.starts_with(b"\x7FELF") {
        return ("executable".into(), "application/x-elf".into());
    }
    if bytes.starts_with(b"MZ") {
        return (
            "executable".into(),
            "application/vnd.microsoft.portable-executable".into(),
        );
    }
    if bytes.starts_with(&[0x1f, 0x8b]) {
        return ("archive".into(), "application/gzip".into());
    }
    if bytes.starts_with(b"7z\xBC\xAF\x27\x1C") {
        return ("archive".into(), "application/x-7z-compressed".into());
    }
    if bytes.starts_with(b"Rar!\x1A\x07") {
        return ("archive".into(), "application/vnd.rar".into());
    }
    match ext {
        "mp3" => ("audio".into(), "audio/mpeg".into()),
        "wav" => ("audio".into(), "audio/wav".into()),
        "flac" => ("audio".into(), "audio/flac".into()),
        "mp4" | "m4v" | "mov" | "webm" | "mkv" | "avi" => ("video".into(), "video/*".into()),
        "onnx" | "pt" | "pth" | "bin" | "dat" => {
            ("model_or_binary".into(), "application/octet-stream".into())
        }
        "sqlite" | "sqlite3" | "db" | "db3" | "sdb" => {
            ("sqlite_database".into(), "application/vnd.sqlite3".into())
        }
        "duckdb" => ("database_file".into(), "application/x-duckdb".into()),
        "mdb" | "accdb" => ("database_file".into(), "application/x-msaccess".into()),
        "dbf" => ("database_file".into(), "application/x-dbf".into()),
        "ibd" | "frm" | "myd" | "myi" => {
            ("database_file".into(), "application/x-mysql-table".into())
        }
        "zip" | "jar" | "war" | "apk" | "docx" | "pptx" | "xlsx" | "odt" => {
            ("archive".into(), "application/zip".into())
        }
        _ => {
            if text_preview(bytes).is_some() {
                ("text".into(), "text/plain; charset=utf-8".into())
            } else {
                ("binary".into(), "application/octet-stream".into())
            }
        }
    }
}

#[tauri::command(async)]
pub fn inspect_file(path: String, max_bytes: Option<u64>) -> Result<FileInspection, String> {
    let resolved = require_inside_workspace(&path, false)?;
    let meta = std::fs::metadata(&resolved).map_err(|e| friendly_read_error(&path, &e))?;
    if meta.is_dir() {
        return Err(format!("'{}' is a directory, not a file.", path));
    }

    let sample_limit = max_bytes
        .unwrap_or(INSPECT_STRING_SCAN_BYTES as u64)
        .clamp(4096, INSPECT_STRING_SCAN_BYTES as u64) as usize;
    let bytes = read_prefix(&resolved, sample_limit)?;
    let sampled = meta.len() > bytes.len() as u64 || meta.len() > INSPECT_FULL_PARSE_MAX;
    let ext = file_extension(&resolved);
    let traineddata = inspect_traineddata(&bytes, meta.len());
    let (kind, mime) = detect_kind_and_mime(&ext, &bytes, traineddata.is_some());
    // Every container format goes through one reader, which sniffs the real bytes rather than
    // trusting the extension. It is given the path, not the sampled prefix: ZIP and 7z keep their
    // index in a footer, so reading only the head of a 2 GB archive can never find it.
    let archive = if matches!(kind.as_str(), "archive")
        || matches!(
            ext.as_str(),
            "zip" | "jar" | "war" | "apk" | "aar" | "ipa" | "whl" | "egg" | "nupkg"
                | "docx" | "pptx" | "xlsx" | "odt" | "ods" | "odp" | "epub" | "crx" | "vsix"
                | "xpi" | "tar" | "tgz" | "tbz" | "tbz2" | "txz" | "tzst" | "gz" | "bz2"
                | "xz" | "zst" | "zstd" | "7z" | "rar"
        )
        || crate::archive::looks_like_compressed_tar(&resolved)
    {
        let listing = crate::archive::read_listing(&resolved);
        // Nothing found and nothing to say means it was not a container after all.
        (!listing.entries.is_empty() || listing.note.is_some()).then_some(listing)
    } else {
        None
    };
    let sqlite = sqlite_preview(&bytes, &resolved);
    let image = if kind == "image" {
        image::image_dimensions(&resolved)
            .ok()
            .map(|(width, height)| ImagePreviewInfo { width, height })
    } else {
        None
    };

    let modified_ms = meta.modified().ok().and_then(|time| {
        time.duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
    });
    let mut summary = Vec::new();
    summary.push(format!("类型：{}", kind));
    summary.push(format!("大小：{} bytes", meta.len()));
    // The sample bound applies to the RAW-BYTES views — hex, strings, text preview. It says
    // nothing about an archive, whose index is read in full from the file itself however large it
    // is. Printing it beside "1652 entries" read as "and it gave up after 1 MB", which is exactly
    // backwards: those 1652 entries are the complete listing of a 2 GB archive.
    let archive_read_in_full = archive
        .as_ref()
        .is_some_and(|listing| !listing.count_is_partial && !listing.entries.is_empty());
    if sampled && !archive_read_in_full {
        summary.push(format!(
            "文件超过 {}MB，当前只解析前 {} bytes",
            INSPECT_FULL_PARSE_MAX / 1024 / 1024,
            bytes.len()
        ));
    } else if sampled {
        summary.push(format!(
            "压缩包目录已完整读取；下面的十六进制/字符串预览只取了前 {} bytes",
            bytes.len()
        ));
    }
    if let Some(info) = &traineddata {
        summary.push(format!(
            "Tesseract traineddata：{}/{} 个组件",
            info.present_count, info.entry_count
        ));
        if let Some(version) = &info.version {
            summary.push(format!("版本：{}", version));
        }
    }
    if let Some(listing) = &archive {
        // The count is the archive's, never the preview's. Saying "200 (max 200 shown)" for an
        // archive of fifty thousand files is the bug this whole path was rewritten for.
        let approx = if listing.count_is_partial { "至少 " } else { "" };
        summary.push(format!(
            "{} 压缩包：{}{} 个条目",
            listing.format, approx, listing.total
        ));
        if listing.truncated && !listing.entries.is_empty() {
            summary.push(format!("列表只显示前 {} 个", listing.entries.len()));
        }
        if listing.total_size > 0 {
            summary.push(format!("解压后共 {} bytes", listing.total_size));
        }
        if listing.metadata_entries > 0 {
            summary.push(format!(
                "另有 {} 个 macOS 扩展属性附属条目（._* / __MACOSX），未计入上面的数量",
                listing.metadata_entries
            ));
        }
        if listing.encrypted {
            summary.push("包含加密条目：文件名可读，内容需要密码".into());
        }
        if let Some(note) = &listing.note {
            summary.push(note.clone());
        }
    }
    if let Some(sqlite) = &sqlite {
        summary.push(format!(
            "SQLite：{} 页 × {} bytes，编码 {}",
            sqlite.page_count, sqlite.page_size, sqlite.text_encoding
        ));
    }
    if let Some(image) = &image {
        summary.push(format!("图片尺寸：{} × {}", image.width, image.height));
    }

    Ok(FileInspection {
        path,
        name: file_name(&resolved),
        extension: ext,
        kind,
        mime,
        size: meta.len(),
        modified_ms,
        readonly: meta.permissions().readonly(),
        sampled,
        signature: hex_prefix(&bytes, 32),
        summary,
        text_preview: text_preview(&bytes),
        hex_rows: build_hex_rows(&bytes),
        strings: extract_ascii_strings(&bytes),
        traineddata,
        archive,
        sqlite,
        image,
    })
}

/// Extract readable TEXT from a document (PDF / Word / Excel / PowerPoint), so the agent
/// can read specs / papers / manuals that `read_text_file` would otherwise return as
/// binary garbage. Office formats are ZIP+XML (reuses the existing `zip` dep); PDF uses
/// `pdf-extract` (pure Rust). The agent's read_file routes here automatically by extension.
#[tauri::command(async)]
pub fn read_document(path: String) -> Result<String, String> {
    require_inside_workspace(&path, false)?;
    let meta = std::fs::metadata(&path).map_err(|e| friendly_read_error(&path, &e))?;
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
        // 只抽 sharedStrings 是**去重后的字符串池**：没有行列、没有 sheet 名，而且数值单元格
        // 根本不在这个文件里（数字存在 worksheet XML 的 <v> 里）。纯数字表抽出来是空的。
        // 把 worksheet 一起抽出来，至少单元格引用和数值在场，模型能对上号；结构化读取仍然
        // 要靠 run_cmd + python，下面的抬头会说明这一点。
        "xlsx" => {
            let text = extract_office(&path, |n| {
                n == "xl/sharedStrings.xml"
                    || (n.starts_with("xl/worksheets/sheet") && n.ends_with(".xml"))
            })?;
            format!(
                "【注意：这是 Excel 的**文本层抽取**，不是结构化表格】共享字符串与单元格内容\
                 被拼在一起，行列关系、sheet 名、公式与格式都不在其中，数值与文本的对应关系\
                 需要自行对照单元格引用。要按行列读真实数据，用 run_cmd 跑 python\
                 （openpyxl / pandas.read_excel），不要凭下面的文本推断表结构。\n\n{text}"
            )
        }
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
#[tauri::command(async)]
pub fn write_text_file(path: String, content: String) -> Result<(), String> {
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
    let resolved = require_inside_workspace(&path, true)?;
    atomic_write_text(&resolved, &content)
}

/// Write only if the file still has the exact version the caller read. `None`
/// means the caller observed no file and therefore uses a filesystem-level
/// atomic no-clobber create. Existing-file compare-and-replace is serialized
/// across IDE writers by FILE_MUTATION_LOCK; external processes do not share
/// that lock and can still race in the small interval after the comparison.
#[tauri::command(async)]
pub fn write_text_file_if_unchanged(
    path: String,
    expected_content: Option<String>,
    content: String,
) -> Result<(), String> {
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
    let resolved = require_inside_workspace(&path, true)?;
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
#[tauri::command(async)]
pub fn delete_text_file_if_unchanged(path: String, expected_content: String) -> Result<(), String> {
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
    let resolved = require_inside_workspace(&path, true)?;
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
#[tauri::command(async)]
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
#[tauri::command(async)]
pub fn create_file(path: String) -> Result<(), String> {
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
    let resolved = require_inside_workspace(&path, true)?;
    atomic_create_text(&resolved, "")
}

/// Create a new directory (including any missing parents).
#[tauri::command(async)]
pub fn create_dir(path: String) -> Result<(), String> {
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
    require_inside_workspace(&path, true)?;
    let p = Path::new(&path);
    if p.exists() {
        return Err("a file or folder with that name already exists".into());
    }
    std::fs::create_dir_all(p).map_err(|e| friendly_write_error("创建目录", &path, &e))
}

/// Rename or move a file/folder. Errors if the destination already exists.
#[tauri::command(async)]
pub fn rename_path(from: String, to: String) -> Result<(), String> {
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
    require_inside_workspace(&from, true)?;
    require_inside_workspace(&to, true)?;
    let to_p = Path::new(&to);
    if to_p.exists() {
        return Err("a file or folder with that name already exists".into());
    }
    if let Some(parent) = to_p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| friendly_write_error("创建目录", &to, &e))?;
    }
    std::fs::rename(&from, &to).map_err(|e| friendly_write_error("重命名", &from, &e))
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
#[tauri::command(async)]
pub fn copy_path(from: String, to: String) -> Result<(), String> {
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
    require_inside_workspace(&from, false)?;
    let to_path = require_inside_workspace(&to, true)?;
    let from_p = Path::new(&from);
    if !from_p.exists() {
        return Err("source does not exist".into());
    }
    if to_path.exists() {
        return Err("a file or folder with that name already exists".into());
    }
    if let Some(parent) = to_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| friendly_write_error("创建目录", &to, &e))?;
    }
    let meta = std::fs::symlink_metadata(from_p)
        .map_err(|e| friendly_write_error("读取源文件", &from, &e))?;
    if meta.is_dir() {
        copy_dir_recursive(from_p, &to_path)
            .map_err(|e| friendly_write_error("复制目录", &from, &e))
    } else {
        std::fs::copy(from_p, &to_path)
            .map(|_| ())
            .map_err(|e| friendly_write_error("复制文件", &from, &e))
    }
}

/// Delete a file, or a directory and all of its contents.
#[tauri::command(async)]
pub fn delete_path(path: String) -> Result<(), String> {
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
    require_inside_workspace(&path, true)?;
    let p = Path::new(&path);
    let meta = std::fs::symlink_metadata(p).map_err(|e| friendly_write_error("删除", &path, &e))?;
    if meta.is_dir() {
        std::fs::remove_dir_all(p).map_err(|e| friendly_write_error("删除目录", &path, &e))
    } else {
        std::fs::remove_file(p).map_err(|e| friendly_write_error("删除文件", &path, &e))
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
    let root_path = require_inside_workspace(root, false)?;
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

    // Rank most-relevant files first: more matches = more likely what the caller
    // wants, alphabetical rel path as a stable tiebreaker. (Was pure alphabetical,
    // which buried a 30-hit file below an incidental 1-hit one.)
    results.sort_by(|a, b| {
        b.matches
            .len()
            .cmp(&a.matches.len())
            .then_with(|| a.rel.cmp(&b.rel))
    });
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
#[tauri::command(async)]
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
#[tauri::command(async)]
pub fn replace_in_file(
    file_path: String,
    query: String,
    replacement: String,
    case_sensitive: bool,
) -> Result<usize, String> {
    let _guard = FILE_MUTATION_LOCK.lock().map_err(|e| e.to_string())?;
    // Same workspace boundary every other mutating fs command enforces — without
    // it this is an arbitrary-file-write primitive.
    let resolved = require_inside_workspace(&file_path, true)?;
    let file_path = resolved.to_string_lossy().to_string();
    let metadata = std::fs::metadata(&file_path).map_err(|e| e.to_string())?;
    if metadata.len() > MAX_FILE {
        return Err("file is too large to replace (> 5 MB)".into());
    }
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
#[tauri::command(async)]
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
    let root_path = require_inside_workspace(&root, true)?;
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
mod archive_summary_tests {
    use super::*;
    use std::io::Write;

    /// The sample bound describes the hex/strings preview, not the archive. Shown next to the
    /// entry count it read as "it only got through 1 MB of your zip", which is the opposite of
    /// what happened — and it is why a working build looked like an old one.
    #[test]
    fn a_fully_read_archive_is_not_labelled_as_truncated() {
        let dir = std::env::temp_dir().join("michael-inspect-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("big.zip");

        // Comfortably past the sampling threshold, so `sampled` is true.
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let blob = vec![b'x'; 256 * 1024];
        for i in 0..8 {
            zip.start_file(format!("payload{i}.bin"), opts).unwrap();
            zip.write_all(&blob).unwrap();
        }
        zip.finish().unwrap();

        let inspection = inspect_file(path.to_string_lossy().into_owned(), Some(64 * 1024)).unwrap();
        let summary = inspection.summary.join("\n");
        assert!(
            !summary.contains("当前只解析前"),
            "a complete archive listing must not carry the raw-bytes truncation notice:\n{summary}"
        );
        assert!(
            summary.contains("8 个条目"),
            "and it must still report the real entry count:\n{summary}"
        );
        let _ = std::fs::remove_file(&path);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn creating_a_project_binds_it_as_a_workspace_root() {
        // 真跑一次：建目录 → 注册成工作区根 → 目录里能写文件。
        // 只断言"参数校验能拒"是不够的——这个工具存在的意义是让没有工作区的智能体开得了工。
        let name = format!("mrday-test-{}", std::process::id());
        let abs = create_project_dir(name.clone()).expect("应当建成");
        let p = std::path::Path::new(&abs);
        assert!(p.is_dir(), "目录要真的存在");
        assert!(abs.contains("MrDayOne"), "位置固定在 ~/MrDayOne 下，不由调用方决定");
        assert!(abs.ends_with(&name));
        // 注册成功之后，工作区边界应当放行这个目录里的写入——这正是"开得了工"的定义。
        let inside = p.join("bot.py").to_string_lossy().into_owned();
        assert!(require_inside_workspace(&inside, true).is_ok(), "建完就该能在里面写文件");
        // 重复调用是幂等的：复用同名目录，不覆盖。
        std::fs::write(p.join("keep.txt"), b"user code").unwrap();
        let again = create_project_dir(name).expect("重复调用应当复用");
        assert_eq!(again, abs);
        assert!(p.join("keep.txt").exists(), "绝不能清空已有目录");
        std::fs::remove_dir_all(p).ok();
    }

    #[test]
    fn project_name_cannot_escape_the_base_directory() {
        // 位置不由调用方决定：模型只能给一个名字，目录固定建在 ~/MrDayOne/<name> 下。
        // 名字过白名单是这条边界的**唯一**依赖，所以逃逸形状要逐个测到。
        for bad in [
            "../../etc",       // 相对路径上跳
            "a/b",             // 正斜杠
            "a\\b",            // 反斜杠
            "..",              // 纯上跳
            ".ssh",            // 隐藏目录（点开头一律拒）
            "",                // 空
            "   ",             // 全空白
            "a..b",            // 内嵌 ..
        ] {
            assert!(create_project_dir(bad.to_string()).is_err(), "应当拒绝: {bad:?}");
        }
        // 长度上限也要守住，免得拼出病态路径。
        assert!(create_project_dir("x".repeat(65)).is_err());
    }

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

    // `require_inside_workspace` used to `return Ok(resolved)` whenever ALLOWED_ROOTS was
    // empty — handing out unrestricted read AND write access to the whole filesystem in
    // exactly the degraded startup where the boundary matters most. It must deny instead.
    #[cfg(unix)]
    #[test]
    fn an_empty_root_list_denies_instead_of_allowing_everything() {
        let roots: Vec<PathBuf> = Vec::new();
        // The property under test is the one the old code got backwards: "not inside any
        // root" must never be satisfiable by an empty root list.
        assert!(!is_within_allowed_root(Path::new("/etc/ssh/sshd_config"), &roots));
        assert!(!is_within_allowed_root(Path::new("/Users/someone/.ssh/id_rsa"), &roots));
        // …while temp scratch space stays reachable through its own explicit allowance.
        assert!(has_safe_prefix(&std::fs::canonicalize(std::env::temp_dir()).unwrap()));
    }

    // Depth alone was the only check: `/etc` and `/Users` (depth 2) were rejected, but
    // `/usr/local`, `/private/etc` and `/Library/LaunchAgents` all sailed through at depth
    // >= 3 — each granting write access to a tree that decides what the machine executes.
    #[cfg(not(windows))]
    #[test]
    fn system_directories_are_rejected_as_workspace_roots_regardless_of_depth() {
        for path in [
            "/usr/local",
            "/usr/local/share/very/deep/path",
            "/private/etc",
            "/etc/ssl/certs",
            "/Library/LaunchAgents",
            "/var/db",
            "/System/Library",
            "/opt/homebrew",
        ] {
            assert!(
                blocked_root_prefix(Path::new(path)).is_some(),
                "{path} must be refused as a workspace root"
            );
        }
        // Ordinary project locations stay registrable.
        for path in ["/Users/tester/code/app", "/home/tester/src/app", "/data/projects/x"] {
            assert!(
                blocked_root_prefix(Path::new(path)).is_none(),
                "{path} must remain a valid workspace root"
            );
        }
        // Component-wise matching: a prefix must not match a longer sibling name.
        assert!(blocked_root_prefix(Path::new("/usrlocal/project")).is_none());
        assert!(blocked_root_prefix(Path::new("/etcetera/project")).is_none());
        // Temp dirs live under blocked trees (/var/folders, /tmp) but are legitimate
        // scratch space and are carved back out.
        assert!(blocked_root_prefix(Path::new("/tmp/scratch")).is_none());
        assert!(blocked_root_prefix(Path::new("/private/var/folders/ab/session")).is_none());
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

    #[test]
    fn project_search_ranks_files_by_match_count() {
        let root = temp_file("search-ranking");
        std::fs::create_dir(&root).unwrap();
        // `low.txt` sorts first alphabetically but has one hit; `zzz.txt` sorts
        // last yet has three. Ranking by match count must surface zzz.txt first.
        std::fs::write(root.join("low.txt"), "needle here\nnothing\n").unwrap();
        std::fs::write(root.join("zzz.txt"), "needle\nneedle\nneedle\n").unwrap();

        let ranked = search_project_scope(&root.to_string_lossy(), "needle", false, None).unwrap();
        assert_eq!(ranked.files.len(), 2);
        assert_eq!(
            ranked.files[0].rel, "zzz.txt",
            "the file with more matches must rank first, not the alphabetical one"
        );
        assert_eq!(ranked.files[0].matches.len(), 3);
        assert_eq!(ranked.files[1].rel, "low.txt");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn project_search_skips_expanded_vendored_dirs() {
        let root = temp_file("search-ignore-expanded");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("app.js"), "token\n").unwrap();
        for noise in ["bower_components", "Pods", "venv"] {
            let d = root.join(noise);
            std::fs::create_dir(&d).unwrap();
            std::fs::write(d.join("dep.js"), "token\n").unwrap();
        }
        let out = search_project_scope(&root.to_string_lossy(), "token", false, None).unwrap();
        assert_eq!(
            out.files.len(),
            1,
            "vendored dirs must be skipped so only app.js matches"
        );
        assert_eq!(out.files[0].rel, "app.js");

        let _ = std::fs::remove_dir_all(root);
    }
}
