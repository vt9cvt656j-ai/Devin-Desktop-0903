//! Local extension storage and a tiny bundled "registry".
//!
//! Extensions are unpacked under the app-data directory (one folder per id) and
//! are loaded into a sandboxed Web Worker by the frontend. Nothing in this
//! module executes extension code — the backend only stores, validates, and
//! serves files. All filesystem access is confined to the extensions directory
//! and guarded against path traversal / zip-slip.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

/// Largest text asset the frontend may read back from an extension (2 MiB).
const MAX_ASSET: u64 = 2 * 1024 * 1024;

/// Per-file cap applied when unpacking an extension archive (16 MiB).
const MAX_UNPACKED_FILE: u64 = 16 * 1024 * 1024;
/// Total cap on the unpacked size of an extension archive (64 MiB).
const MAX_UNPACKED_TOTAL: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandContribution {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Contributes {
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default = "default_main")]
    pub main: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub contributes: Contributes,
}

fn default_main() -> String {
    "index.js".to_string()
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledExtension {
    pub manifest: ExtensionManifest,
    pub enabled: bool,
}

/// Extensions bundled with the app, used as the default "registry".
const BUILTINS: &[(&str, &str)] = &[
    (
        include_str!("../extensions/word-count/extension.json"),
        include_str!("../extensions/word-count/index.js"),
    ),
    (
        include_str!("../extensions/insert-date/extension.json"),
        include_str!("../extensions/insert-date/index.js"),
    ),
    (
        include_str!("../extensions/ai-assistant/extension.json"),
        include_str!("../extensions/ai-assistant/index.js"),
    ),
    (
        include_str!("../extensions/code-formatter/extension.json"),
        include_str!("../extensions/code-formatter/index.js"),
    ),
    (
        include_str!("../extensions/bracket-colorizer/extension.json"),
        include_str!("../extensions/bracket-colorizer/index.js"),
    ),
    (
        include_str!("../extensions/todo-highlight/extension.json"),
        include_str!("../extensions/todo-highlight/index.js"),
    ),
    (
        include_str!("../extensions/color-picker/extension.json"),
        include_str!("../extensions/color-picker/index.js"),
    ),
    (
        include_str!("../extensions/chinese-language-pack/extension.json"),
        include_str!("../extensions/chinese-language-pack/index.js"),
    ),
    (
        include_str!("../extensions/tailwind-intellisense/extension.json"),
        include_str!("../extensions/tailwind-intellisense/index.js"),
    ),
    (
        include_str!("../extensions/hanzi-counter/extension.json"),
        include_str!("../extensions/hanzi-counter/index.js"),
    ),
    (
        include_str!("../extensions/translate-helper/extension.json"),
        include_str!("../extensions/translate-helper/index.js"),
    ),
    (
        include_str!("../extensions/docker-tools/extension.json"),
        include_str!("../extensions/docker-tools/index.js"),
    ),
    (
        include_str!("../extensions/polacode-screenshot/extension.json"),
        include_str!("../extensions/polacode-screenshot/index.js"),
    ),
    (
        include_str!("../extensions/project-manager/extension.json"),
        include_str!("../extensions/project-manager/index.js"),
    ),
    (
        include_str!("../extensions/spell-checker/extension.json"),
        include_str!("../extensions/spell-checker/index.js"),
    ),
    (
        include_str!("../extensions/material-icons/extension.json"),
        include_str!("../extensions/material-icons/index.js"),
    ),
    (
        include_str!("../extensions/svelte-language/extension.json"),
        include_str!("../extensions/svelte-language/index.js"),
    ),
];

/// An extension id is used as a directory name, so keep it to a safe charset.
fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 100 {
        return Err("invalid extension id".into());
    }
    let ok = id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if !ok || id.starts_with('.') {
        return Err("extension id may only contain letters, digits, '.', '_' or '-'".into());
    }
    Ok(())
}

/// Join a relative path onto `base`, rejecting anything that would escape it
/// (absolute paths, `..`, drive prefixes, etc.).
fn safe_join(base: &Path, rel: &str) -> Result<PathBuf, String> {
    let rel_path = Path::new(rel);
    let mut out = base.to_path_buf();
    for comp in rel_path.components() {
        match comp {
            Component::Normal(c) => out.push(c),
            Component::CurDir => {}
            _ => return Err("path escapes the extension directory".into()),
        }
    }
    Ok(out)
}

fn extensions_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("extensions");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn state_path(dir: &Path) -> PathBuf {
    dir.join("state.json")
}

/// Map of extension id -> enabled. Missing ids default to enabled.
fn load_state(dir: &Path) -> BTreeMap<String, bool> {
    fs::read_to_string(state_path(dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_state(dir: &Path, state: &BTreeMap<String, bool>) -> Result<(), String> {
    let s = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    fs::write(state_path(dir), s).map_err(|e| e.to_string())
}

fn read_manifest(ext_dir: &Path) -> Result<ExtensionManifest, String> {
    let raw = fs::read_to_string(ext_dir.join("extension.json"))
        .map_err(|e| format!("missing extension.json: {e}"))?;
    let manifest: ExtensionManifest =
        serde_json::from_str(&raw).map_err(|e| format!("invalid extension.json: {e}"))?;
    validate_id(&manifest.id)?;
    // The entry point must be a contained, relative path.
    safe_join(ext_dir, &manifest.main)?;
    Ok(manifest)
}

fn builtin_manifests() -> Result<Vec<(ExtensionManifest, &'static str)>, String> {
    let mut out = Vec::new();
    for (manifest_src, main_src) in BUILTINS {
        let manifest: ExtensionManifest =
            serde_json::from_str(manifest_src).map_err(|e| e.to_string())?;
        validate_id(&manifest.id)?;
        out.push((manifest, *main_src));
    }
    Ok(out)
}

/// List every installed extension along with its enabled state.
#[tauri::command]
pub fn ext_list_installed(app: AppHandle) -> Result<Vec<InstalledExtension>, String> {
    let dir = extensions_dir(&app)?;
    let state = load_state(&dir);
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        // Skip folders that are not valid extensions instead of failing.
        if let Ok(manifest) = read_manifest(&entry.path()) {
            let enabled = *state.get(&manifest.id).unwrap_or(&true);
            out.push(InstalledExtension { manifest, enabled });
        }
    }
    out.sort_by(|a, b| {
        a.manifest
            .name
            .to_lowercase()
            .cmp(&b.manifest.name.to_lowercase())
    });
    Ok(out)
}

/// Verify an extension is installed, enabled, and (optionally) holds a required
/// permission before allowing the caller to proceed.
fn require_active(
    dir: &Path,
    id: &str,
    required_perm: Option<&str>,
) -> Result<ExtensionManifest, String> {
    let ext_dir = dir.join(id);
    if !ext_dir.is_dir() {
        return Err(format!("extension '{id}' is not installed"));
    }
    let state = load_state(dir);
    if !*state.get(id).unwrap_or(&true) {
        return Err(format!("extension '{id}' is disabled"));
    }
    let manifest = read_manifest(&ext_dir)?;
    if let Some(perm) = required_perm {
        if !manifest.permissions.iter().any(|p| p == perm) {
            return Err(format!(
                "extension '{id}' does not declare the '{perm}' permission"
            ));
        }
    }
    Ok(manifest)
}

/// Read a UTF-8 asset (e.g. the entry script) from an installed extension.
/// Rejects disabled extensions and enforces the declared permission model.
#[tauri::command]
pub fn ext_read_asset(app: AppHandle, id: String, rel: String) -> Result<String, String> {
    validate_id(&id)?;
    let dir = extensions_dir(&app)?;
    require_active(&dir, &id, None)?;
    let ext_dir = dir.join(&id);
    let target = safe_join(&ext_dir, &rel)?;
    let meta = fs::metadata(&target).map_err(|e| e.to_string())?;
    if meta.len() > MAX_ASSET {
        return Err("extension asset is too large".into());
    }
    fs::read_to_string(&target).map_err(|e| e.to_string())
}

/// Enable or disable an installed extension.
#[tauri::command]
pub fn ext_set_enabled(app: AppHandle, id: String, enabled: bool) -> Result<(), String> {
    validate_id(&id)?;
    let dir = extensions_dir(&app)?;
    if !dir.join(&id).is_dir() {
        return Err("extension is not installed".into());
    }
    let mut state = load_state(&dir);
    state.insert(id, enabled);
    save_state(&dir, &state)
}

/// Remove an installed extension and forget its enabled state.
#[tauri::command]
pub fn ext_uninstall(app: AppHandle, id: String) -> Result<(), String> {
    validate_id(&id)?;
    let dir = extensions_dir(&app)?;
    let ext_dir = dir.join(&id);
    if ext_dir.is_dir() {
        fs::remove_dir_all(&ext_dir).map_err(|e| e.to_string())?;
    }
    let mut state = load_state(&dir);
    state.remove(&id);
    save_state(&dir, &state)
}

/// Manifests of the extensions bundled with the app (the default registry).
#[tauri::command]
pub fn ext_available_builtin() -> Result<Vec<ExtensionManifest>, String> {
    Ok(builtin_manifests()?.into_iter().map(|(m, _)| m).collect())
}

/// Install one of the bundled extensions into the app-data directory.
#[tauri::command]
pub fn ext_install_builtin(app: AppHandle, id: String) -> Result<InstalledExtension, String> {
    validate_id(&id)?;
    let (manifest, main_src) = builtin_manifests()?
        .into_iter()
        .find(|(m, _)| m.id == id)
        .ok_or_else(|| format!("unknown built-in extension: {id}"))?;

    let dir = extensions_dir(&app)?;
    let ext_dir = dir.join(&manifest.id);
    fs::create_dir_all(&ext_dir).map_err(|e| e.to_string())?;

    let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(ext_dir.join("extension.json"), manifest_json).map_err(|e| e.to_string())?;
    let main_target = safe_join(&ext_dir, &manifest.main)?;
    if let Some(parent) = main_target.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&main_target, main_src).map_err(|e| e.to_string())?;

    let mut state = load_state(&dir);
    state.insert(manifest.id.clone(), true);
    save_state(&dir, &state)?;
    Ok(InstalledExtension {
        manifest,
        enabled: true,
    })
}

/// Locate `extension.json` in the archive, at the root or one folder deep.
/// Returns the path prefix to strip when extracting ("" or "folder/").
fn find_manifest_prefix(archive: &mut zip::ZipArchive<fs::File>) -> Result<String, String> {
    let mut nested: Option<String> = None;
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let name = name.to_string_lossy().replace('\\', "/");
        if name == "extension.json" {
            return Ok(String::new());
        }
        if let Some(rest) = name.strip_suffix("/extension.json") {
            if !rest.contains('/') {
                nested = Some(format!("{rest}/"));
            }
        }
    }
    nested.ok_or_else(|| "archive has no extension.json".into())
}

fn read_zip_manifest(
    archive: &mut zip::ZipArchive<fs::File>,
    prefix: &str,
) -> Result<ExtensionManifest, String> {
    let mut f = archive
        .by_name(&format!("{prefix}extension.json"))
        .map_err(|e| e.to_string())?;
    let mut s = String::new();
    f.read_to_string(&mut s).map_err(|e| e.to_string())?;
    serde_json::from_str(&s).map_err(|e| format!("invalid extension.json: {e}"))
}

/// Install an extension from a `.zip` archive chosen by the user.
#[tauri::command]
pub fn ext_install_from_path(
    app: AppHandle,
    archive_path: String,
) -> Result<InstalledExtension, String> {
    let dir = extensions_dir(&app)?;
    let file = fs::File::open(&archive_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("not a valid .zip: {e}"))?;

    let prefix = find_manifest_prefix(&mut archive)?;
    let manifest = read_zip_manifest(&mut archive, &prefix)?;
    validate_id(&manifest.id)?;
    let ext_dir = dir.join(&manifest.id);

    // Validate every entry up front, BEFORE touching the existing install:
    // reject unsafe paths (zip-slip) and enforce an extraction budget
    // (zip-bomb). Doing this first guarantees a malformed archive can never
    // delete an already-installed extension partway through extraction.
    let mut declared_total: u64 = 0;
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| e.to_string())?;
        // `enclosed_name` returns None for unsafe paths (zip-slip protection).
        let Some(name) = entry.enclosed_name() else {
            return Err("archive contains an unsafe path".into());
        };
        let name = name.to_string_lossy().replace('\\', "/");
        let rel = name.strip_prefix(&prefix).unwrap_or(&name);
        if rel.is_empty() {
            continue;
        }
        // Ensure the destination stays inside the extension directory.
        safe_join(&ext_dir, rel)?;
        if !entry.is_dir() {
            if entry.size() > MAX_UNPACKED_FILE {
                return Err("extension file is too large".into());
            }
            declared_total = declared_total.saturating_add(entry.size());
            if declared_total > MAX_UNPACKED_TOTAL {
                return Err("extension archive is too large".into());
            }
        }
    }

    if ext_dir.exists() {
        fs::remove_dir_all(&ext_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&ext_dir).map_err(|e| e.to_string())?;

    let mut written_total: u64 = 0;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let Some(name) = entry.enclosed_name() else {
            return Err("archive contains an unsafe path".into());
        };
        let name = name.to_string_lossy().replace('\\', "/");
        let rel = name.strip_prefix(&prefix).unwrap_or(&name);
        if rel.is_empty() {
            continue;
        }
        let target = safe_join(&ext_dir, rel)?;
        if entry.is_dir() {
            fs::create_dir_all(&target).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out = fs::File::create(&target).map_err(|e| e.to_string())?;
            // Defense-in-depth: cap the bytes actually written in case a zip
            // header understated the real entry size.
            let mut limited = entry.by_ref().take(MAX_UNPACKED_FILE + 1);
            let n = io::copy(&mut limited, &mut out).map_err(|e| e.to_string())?;
            if n > MAX_UNPACKED_FILE {
                return Err("extension file is too large".into());
            }
            written_total = written_total.saturating_add(n);
            if written_total > MAX_UNPACKED_TOTAL {
                return Err("extension archive is too large".into());
            }
        }
    }

    // Validate the installed result by re-reading from disk.
    let manifest = read_manifest(&ext_dir)?;
    let mut state = load_state(&dir);
    state.insert(manifest.id.clone(), true);
    save_state(&dir, &state)?;
    Ok(InstalledExtension {
        manifest,
        enabled: true,
    })
}
