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
/// 按魔数认文件类型。
///
/// 保存用的扩展名是**每个命令写死的**（auto_rig/generate_motion 一律 glb、
/// generate_sound 一律 mp3、generate_texture 一律 png），而
/// `downloadable_asset_content_type` 明摆着放行 application/zip、
/// application/octet-stream、image/*——上游给一个打包好的 FBX，落到盘上就叫
/// `rigged.glb`。之后 three.js 的 GLTFLoader 抛一句看不懂的解析错误，而智能体
/// 手上的回执写着「已生成骨骼绑定并保存到 assets/models/rigged.glb」，
/// 它只会去改加载代码——错的地方根本不在那儿。
///
/// 只在**确凿**时返回 Some：认不出来就 None，保持声明的扩展名，绝不瞎猜。
fn sniff_asset_ext(head: &[u8]) -> Option<&'static str> {
    if head.starts_with(b"glTF") {
        return Some("glb");
    }
    if head.starts_with(b"PK\x03\x04") {
        return Some("zip");
    }
    if head.starts_with(b"Kaydara FBX Binary") {
        return Some("fbx");
    }
    if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("png");
    }
    if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("jpg");
    }
    if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
        return Some("gif");
    }
    if head.starts_with(b"OggS") {
        return Some("ogg");
    }
    if head.starts_with(b"fLaC") {
        return Some("flac");
    }
    if head.starts_with(b"ID3") || head.starts_with(&[0xFF, 0xFB]) || head.starts_with(&[0xFF, 0xF3])
    {
        return Some("mp3");
    }
    // RIFF 是个容器，光看头四个字节分不出 wav / webp / avi。
    if head.starts_with(b"RIFF") && head.len() >= 12 {
        return match &head[8..12] {
            b"WAVE" => Some("wav"),
            b"WEBP" => Some("webp"),
            _ => None,
        };
    }
    // ftyp 盒：mp4 / m4a 一族，第 4..8 字节是 "ftyp"。
    if head.len() >= 12 && &head[4..8] == b"ftyp" {
        return match &head[8..11] {
            b"M4A" => Some("m4a"),
            _ => Some("mp4"),
        };
    }
    None
}

/// 落盘之后按魔数复核扩展名；对不上就改名，并把这件事**说出去**。
///
/// 认不出来（None）时保持原样——不知道是什么就不要乱改，那会把一个能用的文件
/// 改成一个没人认识的名字。
fn correct_extension_after_write(target: &Path, declared: &str) -> (PathBuf, Option<String>) {
    let mut head = [0u8; 16];
    let read = match std::fs::File::open(target) {
        Ok(mut f) => {
            use std::io::Read;
            f.read(&mut head).unwrap_or(0)
        }
        Err(_) => 0,
    };
    let Some(actual) = sniff_asset_ext(&head[..read]) else {
        return (target.to_path_buf(), None);
    };
    if actual.eq_ignore_ascii_case(declared) {
        return (target.to_path_buf(), None);
    }
    let renamed = target.with_extension(actual);
    match std::fs::rename(target, &renamed) {
        Ok(()) => (
            renamed,
            Some(format!(
                "上游返回的其实是 {actual}，不是 {declared}；文件已按真实格式改名保存。                 按 {actual} 处理它，别当成 {declared}。"
            )),
        ),
        // 改名失败也要说：内容和扩展名对不上这件事比改名本身重要。
        Err(e) => (
            target.to_path_buf(),
            Some(format!(
                "警告：文件内容其实是 {actual}，扩展名却是 {declared}，自动改名失败（{e}）。                 按 {actual} 处理它。"
            )),
        ),
    }
}

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
            let (target, ext_note) = correct_extension_after_write(&target, ext);
            let rel = target.strip_prefix(root).unwrap_or(&target);
            let mut output = serde_json::json!({
                "path": rel.to_string_lossy(),
                "bytes": written,
            });
            if let Some(note) = ext_note {
                output["ext_note"] = serde_json::Value::String(note);
            }
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
    let (target, ext_note) = correct_extension_after_write(&target, ext);
    let rel = target.strip_prefix(root).unwrap_or(&target);
    let mut output = serde_json::json!({
        "path": rel.to_string_lossy(),
        "bytes": written,
    });
    if let Some(note) = ext_note {
        output["ext_note"] = serde_json::Value::String(note);
    }
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

#[cfg(test)]
mod asset_extension_tests {
    use super::{correct_extension_after_write, sniff_asset_ext};

    /// 函数写对了但没接上，等于没写。gateway_download 有**两个**落盘点
    /// （JSON 里带 download_url 的那条、直接流式返回的那条），两条都要复核，
    /// 而且 ext_note 都要塞进返回的 JSON——不然后端认出来了、模型收不到。
    #[test]
    fn both_write_paths_are_actually_wired() {
        let src = include_str!("game_assets.rs");
        let body = src
            .split("async fn gateway_download(")
            .nth(1)
            .and_then(|s| s.split("\nfn downloadable_asset_content_type").next())
            .expect("gateway_download 的函数体不见了");
        assert_eq!(
            body.matches("stream_asset_to_path").count(),
            2,
            "落盘点数量变了，这条断言的前提得重算"
        );
        assert_eq!(
            body.matches("correct_extension_after_write").count(),
            2,
            "有落盘点没做扩展名复核"
        );
        assert_eq!(
            body.matches("output[\"ext_note\"]").count(),
            2,
            "有落盘点没把 ext_note 塞回 JSON —— 后端认出来了，模型收不到"
        );
    }

    #[test]
    fn known_magics_are_recognised() {
        assert_eq!(sniff_asset_ext(b"glTF\x02\x00\x00\x00"), Some("glb"));
        assert_eq!(sniff_asset_ext(b"PK\x03\x04\x14\x00"), Some("zip"));
        assert_eq!(sniff_asset_ext(b"Kaydara FBX Binary  \x00"), Some("fbx"));
        assert_eq!(sniff_asset_ext(b"\x89PNG\r\n\x1a\n"), Some("png"));
        assert_eq!(sniff_asset_ext(b"ID3\x03\x00"), Some("mp3"));
        assert_eq!(sniff_asset_ext(b"RIFF\x24\x00\x00\x00WAVEfmt "), Some("wav"));
        assert_eq!(sniff_asset_ext(b"RIFF\x24\x00\x00\x00WEBPVP8 "), Some("webp"));
    }

    /// 认不出来就必须是 None。瞎猜会把一个能用的文件改成没人认识的名字。
    #[test]
    fn unknown_bytes_are_left_alone() {
        assert_eq!(sniff_asset_ext(b"\x00\x01\x02\x03"), None);
        assert_eq!(sniff_asset_ext(b""), None);
        assert_eq!(sniff_asset_ext(b"RIFF\x00\x00\x00\x00AVI "), None);
    }

    /// 这就是审计里那条：auto_rig 一律按 glb 存，上游给的其实是打包好的 FBX。
    #[test]
    fn a_zip_saved_as_glb_gets_renamed_and_reported() {
        let dir = std::env::temp_dir().join(format!(
            "mrday-asset-ext-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("rigged.glb");
        std::fs::write(&target, b"PK\x03\x04rest of a zipped fbx").unwrap();

        let (path, note) = correct_extension_after_write(&target, "glb");
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("zip"));
        assert!(path.exists(), "改名后的文件不在盘上");
        assert!(!target.exists(), "旧的 .glb 还留着，会有两份");
        let note = note.expect("扩展名对不上却一个字都不说");
        assert!(note.contains("zip") && note.contains("glb"), "{note}");

        // 对得上时不动、也不多话。
        let ok = dir.join("model.glb");
        std::fs::write(&ok, b"glTF\x02\x00\x00\x00").unwrap();
        let (p2, n2) = correct_extension_after_write(&ok, "glb");
        assert_eq!(p2, ok);
        assert!(n2.is_none());

        // 认不出来的字节：保持原样，不改名也不报警。
        let unknown = dir.join("thing.glb");
        std::fs::write(&unknown, b"\x00\x01\x02\x03").unwrap();
        let (p3, n3) = correct_extension_after_write(&unknown, "glb");
        assert_eq!(p3, unknown);
        assert!(n3.is_none());

        std::fs::remove_dir_all(&dir).ok();
    }
}
