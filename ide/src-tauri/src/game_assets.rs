use std::path::{Path, PathBuf};
use std::time::Duration;

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
    let status = resp.status().as_u16();
    if status != 200 {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("网关返回 {status}: {text}"));
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

    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let bytes = resp.bytes().await.map_err(|e| format!("下载失败: {e}"))?;
    if bytes.is_empty() {
        return Err("生成结果为空".into());
    }

    if ct.contains("application/json") {
        let j: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| format!("JSON 解析失败: {e}"))?;
        if let Some(err) = j.get("error").and_then(|v| v.as_str()) {
            return Err(err.to_string());
        }
        if let Some(url) = j.get("download_url").and_then(|v| v.as_str()) {
            let file_bytes = asset_client()?
                .get(url)
                .timeout(Duration::from_secs(120))
                .send()
                .await
                .map_err(|e| format!("下载文件失败: {e}"))?
                .bytes()
                .await
                .map_err(|e| e.to_string())?;
            let target = safe_dest(root, sub_dir, name, ext)?;
            std::fs::write(&target, &file_bytes).map_err(|e| format!("写入失败: {e}"))?;
            let rel = target.strip_prefix(root).unwrap_or(&target);
            return Ok(serde_json::json!({
                "path": rel.to_string_lossy(),
                "bytes": file_bytes.len(),
            }));
        }
        return Ok(j);
    }

    let target = safe_dest(root, sub_dir, name, ext)?;
    std::fs::write(&target, &bytes).map_err(|e| format!("写入失败: {e}"))?;
    let rel = target.strip_prefix(root).unwrap_or(&target);
    Ok(serde_json::json!({
        "path": rel.to_string_lossy(),
        "bytes": bytes.len(),
    }))
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
    model_path: String,
    name: String,
) -> Result<serde_json::Value, String> {
    let abs = Path::new(&workspace).join(&model_path);
    if !abs.exists() {
        return Err(format!("模型文件不存在: {model_path}"));
    }
    let model_bytes = std::fs::read(&abs).map_err(|e| format!("读取模型失败: {e}"))?;
    let b64 = crate::capture::b64(&model_bytes);
    let ext = abs.extension().and_then(|e| e.to_str()).unwrap_or("glb");
    let body = serde_json::json!({
        "model_data": b64,
        "format": ext,
    });
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
    duration: Option<f32>,
) -> Result<serde_json::Value, String> {
    if prompt.trim().is_empty() {
        return Err("缺少动作描述".into());
    }
    let body = serde_json::json!({
        "prompt": prompt.trim(),
        "duration": duration.unwrap_or(3.0),
    });
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
    let j: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
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
    let bytes = asset_client()?
        .get(url.trim())
        .send()
        .await
        .map_err(|e| format!("下载失败: {e}"))?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    if bytes.is_empty() {
        return Err("下载结果为空".into());
    }
    let target = safe_dest(&workspace, sub, &name, ext)?;
    std::fs::write(&target, &bytes).map_err(|e| format!("写入失败: {e}"))?;
    let rel = target.strip_prefix(&workspace).unwrap_or(&target);
    Ok(serde_json::json!({
        "path": rel.to_string_lossy(),
        "bytes": bytes.len(),
    }))
}
