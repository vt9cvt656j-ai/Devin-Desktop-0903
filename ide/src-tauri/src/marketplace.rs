use std::net::IpAddr;
use std::path::PathBuf;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/fendoushaonian/Devin-Desktop/main/marketplace/registry.json";
const MAX_MARKETPLACE_ARCHIVE: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub download_url: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub downloads: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryResponse {
    extensions: Vec<MarketplaceEntry>,
}

fn extensions_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("extensions");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn validate_entry_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 100 || id.starts_with('.') {
        return Err("invalid marketplace extension id".into());
    }
    let ok = id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if ok {
        Ok(())
    } else {
        Err("marketplace extension id may only contain letters, digits, '.', '_' or '-'".into())
    }
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast()
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
        }
    }
}

fn validate_download_url(url: &str) -> Result<reqwest::Url, String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("invalid download URL: {e}"))?;
    if parsed.scheme() != "https" {
        return Err("marketplace downloads must use HTTPS".into());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("download URL must not contain credentials".into());
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "download URL must include a host".to_string())?;
    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost") {
        return Err("download URL must not target localhost".into());
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_ip(ip) {
            return Err("download URL must not target local or private addresses".into());
        }
    }
    Ok(parsed)
}

async fn download_archive_with_limit(resp: reqwest::Response) -> Result<Vec<u8>, String> {
    if let Some(len) = resp.content_length() {
        if len > MAX_MARKETPLACE_ARCHIVE as u64 {
            return Err("download is too large".into());
        }
    }

    let mut bytes = Vec::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("failed to read download: {e}"))?;
        if bytes.len().saturating_add(chunk.len()) > MAX_MARKETPLACE_ARCHIVE {
            return Err("download is too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[tauri::command]
pub async fn marketplace_list() -> Result<Vec<MarketplaceEntry>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(REGISTRY_URL)
        .send()
        .await
        .map_err(|e| format!("failed to fetch registry: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "registry returned status {}",
            resp.status()
        ));
    }

    let registry: RegistryResponse = resp
        .json()
        .await
        .map_err(|e| format!("invalid registry format: {e}"))?;

    Ok(registry.extensions)
}

#[tauri::command]
pub async fn marketplace_install(
    app: AppHandle,
    entry: MarketplaceEntry,
) -> Result<String, String> {
    validate_entry_id(&entry.id)?;
    let download_url = validate_download_url(&entry.download_url)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(download_url.clone())
        .send()
        .await
        .map_err(|e| format!("download failed: {e}"))?;

    // DNS rebinding guard: verify the resolved IP is not private/local.
    if let Some(addr) = resp.remote_addr() {
        if is_blocked_ip(addr.ip()) {
            return Err("download resolved to a private/local IP (DNS rebinding blocked)".into());
        }
    }

    if !resp.status().is_success() {
        return Err(format!("download returned status {}", resp.status()));
    }

    let bytes = download_archive_with_limit(resp).await?;

    let dir = extensions_dir(&app)?;
    let archive_path = dir.join(format!("{}.zip", entry.id));
    std::fs::write(&archive_path, &bytes).map_err(|e| e.to_string())?;

    let archive_str = archive_path.to_string_lossy().to_string();
    let result = crate::extensions::ext_install_from_path(app.clone(), archive_str)?;
    if result.manifest.id != entry.id {
        let _ = crate::extensions::ext_uninstall(app, result.manifest.id.clone());
        let _ = std::fs::remove_file(&archive_path);
        return Err("downloaded extension id does not match the marketplace entry".into());
    }

    let _ = std::fs::remove_file(&archive_path);

    Ok(format!("Installed {} v{}", result.manifest.name, result.manifest.version))
}

#[tauri::command]
pub async fn marketplace_search(query: String) -> Result<Vec<MarketplaceEntry>, String> {
    let all = marketplace_list().await?;
    if query.is_empty() {
        return Ok(all);
    }
    let q = query.to_lowercase();
    Ok(all
        .into_iter()
        .filter(|e| {
            e.name.to_lowercase().contains(&q)
                || e.description.to_lowercase().contains(&q)
                || e.tags.iter().any(|t| t.to_lowercase().contains(&q))
                || e.id.to_lowercase().contains(&q)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_like_marketplace_ids() {
        assert!(validate_entry_id("../evil").is_err());
        assert!(validate_entry_id("nested/evil").is_err());
        assert!(validate_entry_id(".hidden").is_err());
    }

    #[test]
    fn accepts_safe_marketplace_ids() {
        assert!(validate_entry_id("publisher.extension-1").is_ok());
        assert!(validate_entry_id("publisher_extension.2026").is_ok());
    }

    #[test]
    fn rejects_unsafe_download_urls() {
        assert!(validate_download_url("http://example.com/ext.zip").is_err());
        assert!(validate_download_url("https://localhost/ext.zip").is_err());
        assert!(validate_download_url("https://127.0.0.1/ext.zip").is_err());
        assert!(validate_download_url("file:///tmp/ext.zip").is_err());
    }

    #[test]
    fn accepts_public_https_download_url() {
        assert!(validate_download_url("https://example.com/ext.zip").is_ok());
    }
}
