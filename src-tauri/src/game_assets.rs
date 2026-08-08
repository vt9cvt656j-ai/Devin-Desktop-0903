use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_GATEWAY_JSON_BYTES: usize = 8 * 1024 * 1024;
const MAX_GATEWAY_ERROR_BYTES: usize = 64 * 1024;

fn asset_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())
}

fn safe_dest(root: &str, sub: &str, name: &str, ext: &str) -> Result<PathBuf, String> {
    let base = Path::new(root);
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let fname = if sanitized.is_empty() {
        "asset".to_string()
    } else {
        sanitized
    };
    let target = base.join("assets").join(sub).join(format!("{fname}.{ext}"));
    if target
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("路径不能包含 ..".into());
    }
    if let Some(p) = target.parent() {
        std::fs::create_dir_all(p).map_err(|e| format!("建目录失败: {e}"))?;
    }
    Ok(target)
}

async fn response_bytes_limited(
    mut response: reqwest::Response,
    limit: usize,
    label: &str,
) -> Result<Vec<u8>, String> {
    if let Some(length) = response.content_length() {
        if length > limit as u64 {
            return Err(format!(
                "{label}响应过大（{length} 字节，上限 {limit} 字节）"
            ));
        }
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(format!("{label}响应超过 {limit} 字节"));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

async fn stream_asset_to_path(
    mut response: reqwest::Response,
    target: &Path,
) -> Result<u64, String> {
    use tokio::io::AsyncWriteExt;

    let extension = target
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("asset");
    let temporary = target.with_extension(format!(
        "{extension}.michael-download-{}.part",
        uuid::Uuid::new_v4()
    ));
    let download = async {
        let mut file = tokio::fs::File::create(&temporary)
            .await
            .map_err(|e| format!("创建临时文件失败: {e}"))?;
        let mut written = 0_u64;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| format!("下载读取失败: {e}"))?
        {
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("写入临时文件失败: {e}"))?;
            written = written.saturating_add(chunk.len() as u64);
        }
        file.flush()
            .await
            .map_err(|e| format!("刷新临时文件失败: {e}"))?;
        drop(file);
        if written == 0 {
            return Err("下载结果为空".to_string());
        }

        match tokio::fs::rename(&temporary, target).await {
            Ok(()) => Ok(written),
            Err(first_error) if target.exists() => {
                tokio::fs::remove_file(target)
                    .await
                    .map_err(|e| format!("替换旧资产失败: {e}"))?;
                tokio::fs::rename(&temporary, target)
                    .await
                    .map_err(|e| format!("保存资产失败（{first_error}; {e}）"))?;
                Ok(written)
            }
            Err(error) => Err(format!("保存资产失败: {error}")),
        }
    }
    .await;

    if download.is_err() {
        let _ = tokio::fs::remove_file(&temporary).await;
    }
    download
}

async fn gateway_post(
    base_url: &str,
    api_key: &str,
    action: &str,
    body: &serde_json::Value,
) -> Result<reqwest::Response, String> {
    let url = format!("{}/v1/game/{action}", base_url.trim_end_matches('/'));
    let resp = asset_client()?
        .post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .json(body)
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        let bytes = response_bytes_limited(resp, MAX_GATEWAY_ERROR_BYTES, "网关错误").await?;
        let text = String::from_utf8_lossy(&bytes);
        return Err(format!("网关返回 {}: {text}", status.as_u16()));
    }
    Ok(resp)
}

// Mirrors the gateway route plus the destination tuple; keeping these fields
// explicit makes every Tauri command call site auditable.
#[allow(clippy::too_many_arguments)]
async fn gateway_download(
    base_url: &str,
    api_key: &str,
    action: &str,
    body: &serde_json::Value,
    root: &str,
    sub_dir: &str,
    name: &str,
    ext: &str,
) -> Result<serde_json::Value, String> {
    let resp = gateway_post(base_url, api_key, action, body).await?;
    let task_id = resp
        .headers()
        .get("x-michael-task-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_owned);

    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if ct.contains("application/json") {
        let bytes = response_bytes_limited(resp, MAX_GATEWAY_JSON_BYTES, "网关 JSON").await?;
        if bytes.is_empty() {
            return Err("生成结果为空".into());
        }
        let mut j: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| format!("JSON 解析失败: {e}"))?;
        if let Some(err) = j.get("error").and_then(|v| v.as_str()) {
            return Err(err.to_string());
        }
        if let Some(task_id) = task_id.as_ref() {
            if let Some(object) = j.as_object_mut() {
                object
                    .entry("task_id")
                    .or_insert_with(|| serde_json::Value::String(task_id.clone()));
            }
        }
        if let Some(url) = j
            .get("download_url")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
        {
            let response = asset_client()?
                .get(url)
                .timeout(Duration::from_secs(120))
                .send()
                .await
                .map_err(|e| format!("下载文件失败: {e}"))?;
            let status = response.status();
            if !status.is_success() {
                let bytes =
                    response_bytes_limited(response, MAX_GATEWAY_ERROR_BYTES, "资产下载错误")
                        .await?;
                return Err(format!(
                    "下载文件 HTTP {}: {}",
                    status.as_u16(),
                    String::from_utf8_lossy(&bytes)
                ));
            }
            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("")
                .to_string();
            if !downloadable_asset_content_type(ext, &content_type) {
                return Err(format!("下载文件失败: 不接受的响应类型 {content_type}"));
            }
            let target = safe_dest(root, sub_dir, name, ext)?;
            let written = stream_asset_to_path(response, &target).await?;
            let rel = target.strip_prefix(root).unwrap_or(&target);
            let mut output = serde_json::json!({
                "path": rel.to_string_lossy(),
                "bytes": written,
            });
            if let Some(task_id) = task_id {
                output["task_id"] = serde_json::Value::String(task_id);
            }
            return Ok(output);
        }
        return Ok(j);
    }

    if !downloadable_asset_content_type(ext, &ct) {
        return Err(format!("生成失败: 不接受的响应类型 {ct}"));
    }
    let target = safe_dest(root, sub_dir, name, ext)?;
    let written = stream_asset_to_path(resp, &target).await?;
    let rel = target.strip_prefix(root).unwrap_or(&target);
    let mut output = serde_json::json!({
        "path": rel.to_string_lossy(),
        "bytes": written,
    });
    if let Some(task_id) = task_id {
        output["task_id"] = serde_json::Value::String(task_id);
    }
    Ok(output)
}

fn downloadable_asset_content_type(ext: &str, content_type: &str) -> bool {
    let media_type = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if media_type.is_empty() {
        return true;
    }
    if media_type == "text/html" || media_type == "application/xhtml+xml" {
        return false;
    }
    if media_type == "application/json" {
        return ext == "gltf";
    }
    media_type.starts_with("image/")
        || media_type.starts_with("audio/")
        || media_type.starts_with("model/")
        || media_type == "application/octet-stream"
        || media_type == "application/gltf-buffer"
        || media_type == "application/gltf+json"
        || media_type == "application/zip"
        || media_type.starts_with("application/x-")
}

// ── 1. 3D Model Generation ─────────────────────────────────────────

#[tauri::command]
pub async fn generate_3d(
    base_url: String,
    api_key: String,
    workspace: String,
    prompt: String,
    name: String,
    style: Option<String>,
    model: Option<String>,
) -> Result<serde_json::Value, String> {
    if prompt.trim().is_empty() {
        return Err("缺少 3D 模型描述".into());
    }
    let body = serde_json::json!({
        "prompt": prompt.trim(),
        "style": style.unwrap_or_else(|| "realistic".into()),
        "model": model.unwrap_or_default(),
    });
    gateway_download(
        &base_url,
        &api_key,
        "generate-3d",
        &body,
        &workspace,
        "models",
        &name,
        "glb",
    )
    .await
}

// ── 2. Sound Effect Generation ──────────────────────────────────────

#[tauri::command]
pub async fn generate_sound(
    base_url: String,
    api_key: String,
    workspace: String,
    prompt: String,
    name: String,
    duration: Option<f32>,
) -> Result<serde_json::Value, String> {
    if prompt.trim().is_empty() {
        return Err("缺少音效描述".into());
    }
    let body = serde_json::json!({
        "prompt": prompt.trim(),
        "duration": duration.unwrap_or(5.0),
    });
    gateway_download(
        &base_url,
        &api_key,
        "generate-sound",
        &body,
        &workspace,
        "audio",
        &name,
        "mp3",
    )
    .await
}

// ── 3. Music Generation ────────────────────────────────────────────

#[tauri::command]
pub async fn generate_music(
    base_url: String,
    api_key: String,
    workspace: String,
    prompt: String,
    name: String,
    duration: Option<f32>,
) -> Result<serde_json::Value, String> {
    if prompt.trim().is_empty() {
        return Err("缺少音乐描述".into());
    }
    let body = serde_json::json!({
        "prompt": prompt.trim(),
        "duration": duration.unwrap_or(30.0),
    });
    gateway_download(
        &base_url,
        &api_key,
        "generate-music",
        &body,
        &workspace,
        "music",
        &name,
        "mp3",
    )
    .await
}

// ── 4. Voice / TTS ──────────────────────────────────────────────────

#[tauri::command]
pub async fn generate_voice(
    base_url: String,
    api_key: String,
    workspace: String,
    text: String,
    name: String,
    voice: Option<String>,
) -> Result<serde_json::Value, String> {
    if text.trim().is_empty() {
        return Err("缺少语音文本".into());
    }
    let body = serde_json::json!({
        "text": text.trim(),
        "voice": voice.unwrap_or_else(|| "default".into()),
    });
    gateway_download(
        &base_url,
        &api_key,
        "generate-voice",
        &body,
        &workspace,
        "voice",
        &name,
        "mp3",
    )
    .await
}

// ── 5. Auto-Rig ────────────────────────────────────────────────────

#[tauri::command]
pub async fn auto_rig(
    base_url: String,
    api_key: String,
    workspace: String,
    task_id: String,
    name: String,
) -> Result<serde_json::Value, String> {
    if task_id.trim().is_empty() {
        return Err("auto_rig 需要 generate_3d 返回的 task_id".into());
    }
    let body = serde_json::json!({ "task_id": task_id.trim() });
    gateway_download(
        &base_url, &api_key, "auto-rig", &body, &workspace, "models", &name, "glb",
    )
    .await
}

// ── 6. Motion / Animation Generation ───────────────────────────────

#[tauri::command]
pub async fn generate_motion(
    base_url: String,
    api_key: String,
    workspace: String,
    prompt: String,
    name: String,
    task_id: String,
) -> Result<serde_json::Value, String> {
    if prompt.trim().is_empty() {
        return Err("缺少动作描述".into());
    }
    if task_id.trim().is_empty() {
        return Err("generate_motion 需要 auto_rig 返回的 task_id".into());
    }
    let body = serde_json::json!({ "prompt": prompt.trim(), "task_id": task_id.trim() });
    gateway_download(
        &base_url,
        &api_key,
        "generate-motion",
        &body,
        &workspace,
        "animations",
        &name,
        "glb",
    )
    .await
}

// ── 7. Texture / Material Generation ───────────────────────────────

#[tauri::command]
pub async fn generate_texture(
    base_url: String,
    api_key: String,
    workspace: String,
    prompt: String,
    name: String,
    resolution: Option<u32>,
) -> Result<serde_json::Value, String> {
    if prompt.trim().is_empty() {
        return Err("缺少纹理描述".into());
    }
    let body = serde_json::json!({
        "prompt": prompt.trim(),
        "resolution": resolution.unwrap_or(1024),
    });
    gateway_download(
        &base_url,
        &api_key,
        "generate-texture",
        &body,
        &workspace,
        "textures",
        &name,
        "png",
    )
    .await
}

// ── 8. Search Game Assets ──────────────────────────────────────────

#[tauri::command]
pub async fn search_game_assets(
    base_url: String,
    api_key: String,
    query: String,
    asset_type: Option<String>,
) -> Result<serde_json::Value, String> {
    if query.trim().is_empty() {
        return Err("缺少搜索关键词".into());
    }
    let body = serde_json::json!({
        "query": query.trim(),
        "type": asset_type.unwrap_or_else(|| "all".into()),
    });
    let resp = gateway_post(&base_url, &api_key, "search-assets", &body).await?;
    let bytes = response_bytes_limited(resp, MAX_GATEWAY_JSON_BYTES, "资产搜索").await?;
    let j: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    Ok(j)
}

// ── 9. Download Asset ──────────────────────────────────────────────

#[tauri::command]
pub async fn download_asset(
    workspace: String,
    url: String,
    name: String,
    asset_type: Option<String>,
) -> Result<serde_json::Value, String> {
    if url.trim().is_empty() {
        return Err("缺少下载 URL".into());
    }
    let atype = asset_type.unwrap_or_else(|| "models".into());
    let sub = match atype.as_str() {
        "sound" | "audio" | "sfx" => "audio",
        "music" => "music",
        "voice" => "voice",
        "texture" | "material" => "textures",
        "hdri" | "environment" => "environments",
        "animation" | "motion" => "animations",
        _ => "models",
    };
    let ext_guess = url
        .rsplit('/')
        .next()
        .and_then(|s| s.rsplit('.').next())
        .unwrap_or("glb");
    let ext = match ext_guess {
        "glb" | "gltf" | "fbx" | "obj" | "ply" | "usdz" => ext_guess,
        "mp3" | "wav" | "ogg" | "flac" => ext_guess,
        "png" | "jpg" | "jpeg" | "exr" | "hdr" => ext_guess,
        "bvh" => ext_guess,
        _ => "bin",
    };
    let response = asset_client()?
        .get(url.trim())
        .send()
        .await
        .map_err(|e| format!("下载失败: {e}"))?;
    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if !status.is_success() {
        let bytes = response_bytes_limited(response, MAX_GATEWAY_ERROR_BYTES, "下载错误").await?;
        let body = String::from_utf8_lossy(&bytes);
        return Err(format!(
            "下载失败: HTTP {}: {}",
            status.as_u16(),
            body.chars().take(300).collect::<String>()
        ));
    }
    if !downloadable_asset_content_type(ext, &content_type) {
        return Err(format!("下载失败: 不接受的响应类型 {content_type}"));
    }
    let target = safe_dest(&workspace, sub, &name, ext)?;
    let written = stream_asset_to_path(response, &target).await?;
    let rel = target.strip_prefix(&workspace).unwrap_or(&target);
    Ok(serde_json::json!({
        "path": rel.to_string_lossy(),
        "bytes": written,
    }))
}

#[cfg(test)]
mod tests {
    use super::{downloadable_asset_content_type, gateway_post};
    use std::io::{Read, Write};

    #[test]
    fn rejects_html_error_pages_as_assets() {
        assert!(!downloadable_asset_content_type(
            "glb",
            "text/html; charset=utf-8"
        ));
        assert!(!downloadable_asset_content_type("png", "application/json"));
    }

    #[test]
    fn accepts_expected_binary_asset_content_types() {
        assert!(downloadable_asset_content_type("glb", "model/gltf-binary"));
        assert!(downloadable_asset_content_type("png", "image/png"));
        assert!(downloadable_asset_content_type("gltf", "application/json"));
    }

    #[tokio::test]
    async fn gateway_post_accepts_any_success_status() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let _ = socket.read(&mut [0_u8; 4096]);
            socket
                .write_all(
                    b"HTTP/1.1 202 Accepted\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                )
                .unwrap();
        });

        let response = gateway_post(
            &format!("http://{address}"),
            "test-key",
            "generate-3d",
            &serde_json::json!({}),
        )
        .await
        .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::ACCEPTED);
        server.join().unwrap();
    }
}
