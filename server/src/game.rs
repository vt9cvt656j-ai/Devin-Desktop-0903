use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::Json;
use reqwest::StatusCode;
use serde_json::json;

use crate::error::AppError;
use crate::AppState;

/// 重型本地生成（MusicGen / TRELLIS）的**全局**并发上限。
///
/// 原来唯一的闸是「每用户每小时 60 次」——它数频率，不数并发。这两条路会在网关
/// 容器里直接起 python 子进程，单个就能吃满 CPU 和几个 G 内存；一个账号并发打满
/// 就能把同一个容器里的聊天网关一起压垮，而计数器一次都没触发（60 次远没用完）。
///
/// 所以要的是全局并发上限，不是更严的频率。占不到名额就当场拒，不排队：排队等于
/// 把请求挂在那里继续占连接和内存，压垮的方式换了个样子而已。
static HEAVY_GEN_SLOTS: std::sync::LazyLock<tokio::sync::Semaphore> =
    std::sync::LazyLock::new(|| {
        let n = std::env::var("MICHAEL_HEAVY_GEN_CONCURRENCY")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(2);
        tokio::sync::Semaphore::new(n)
    });

fn heavy_gen_slot() -> Result<tokio::sync::SemaphorePermit<'static>, AppError> {
    HEAVY_GEN_SLOTS.try_acquire().map_err(|_| AppError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        msg: "本地生成正在满负荷运行，请稍后再试".into(),
    })
}

static HTTP: std::sync::LazyLock<reqwest::Client> = std::sync::LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36")
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(300))
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});

fn hf_key() -> String {
    std::env::var("HF_API_KEY").unwrap_or_default()
}
fn elevenlabs_key() -> String {
    std::env::var("ELEVENLABS_API_KEY").unwrap_or_default()
}
fn freesound_key() -> String {
    std::env::var("FREESOUND_API_KEY").unwrap_or_default()
}
fn replicate_key() -> String {
    std::env::var("REPLICATE_API_TOKEN").unwrap_or_default()
}
fn tripo_key() -> String {
    std::env::var("TRIPO_API_KEY").unwrap_or_default()
}

async fn hf_inference(
    model: &str,
    body: &serde_json::Value,
) -> Result<reqwest::Response, AppError> {
    let key = hf_key();
    if key.is_empty() {
        return Err(AppError::bad(
            "需要 HF_API_KEY（免费: huggingface.co → Settings → Access Tokens）",
        ));
    }
    let url = format!("https://router.huggingface.co/hf-inference/models/{model}");
    let resp = HTTP
        .post(&url)
        .header("Authorization", format!("Bearer {key}"))
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("HF 请求失败: {e}")))?;
    if !resp.status().is_success() {
        let st = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError {
            status: StatusCode::from_u16(st).unwrap_or(StatusCode::BAD_GATEWAY),
            msg: format!("HF {st}: {}", text.chars().take(300).collect::<String>()),
        });
    }
    Ok(resp)
}

fn relay(status: u16, ct: &str, body: axum::body::Bytes) -> Response {
    axum::http::Response::builder()
        .status(StatusCode::from_u16(status).unwrap_or(StatusCode::OK))
        .header(axum::http::header::CONTENT_TYPE, ct)
        .body(axum::body::Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn relay_with_task_id(status: u16, ct: &str, body: axum::body::Bytes, task_id: &str) -> Response {
    axum::http::Response::builder()
        .status(StatusCode::from_u16(status).unwrap_or(StatusCode::OK))
        .header(axum::http::header::CONTENT_TYPE, ct)
        .header("x-michael-task-id", task_id)
        .body(axum::body::Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

// ── Replicate helper (poll until complete) ────────────────────────────
async fn replicate_run(
    model: &str,
    input: serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    let key = replicate_key();
    if key.is_empty() {
        return Err(AppError::bad(
            "需要 REPLICATE_API_TOKEN（replicate.com 预付费制，无免费额度）",
        ));
    }
    let resp = HTTP
        .post("https://api.replicate.com/v1/predictions")
        .header("Authorization", format!("Bearer {key}"))
        .header("Prefer", "wait")
        .json(&json!({ "model": model, "input": input }))
        .send()
        .await
        .map_err(|e| AppError::internal(format!("Replicate 请求失败: {e}")))?;
    let st = resp.status().as_u16();
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    if st >= 400 {
        let detail = body
            .get("detail")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(AppError {
            status: StatusCode::from_u16(st).unwrap_or(StatusCode::BAD_GATEWAY),
            msg: format!("Replicate {st}: {detail}"),
        });
    }
    let status_str = body.get("status").and_then(|v| v.as_str()).unwrap_or("");
    if status_str == "failed" || status_str == "canceled" {
        let err = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("生成失败");
        return Err(AppError::bad(format!("Replicate: {err}")));
    }
    if status_str == "succeeded" {
        return Ok(body);
    }
    // poll if not using Prefer: wait
    let poll_url = body
        .get("urls")
        .and_then(|u| u.get("get"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if poll_url.is_empty() {
        return Ok(body);
    }
    for _ in 0..120 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let pr = HTTP
            .get(poll_url)
            .header("Authorization", format!("Bearer {key}"))
            .send()
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        let pb: serde_json::Value = pr
            .json()
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        let ps = pb.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if ps == "succeeded" {
            return Ok(pb);
        }
        if ps == "failed" || ps == "canceled" {
            let err = pb
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("生成失败");
            return Err(AppError::bad(format!("Replicate: {err}")));
        }
    }
    Err(AppError::internal("Replicate 超时（240s）"))
}

// ── Tripo3D helper (create task + poll) ───────────────────────────────
async fn tripo_task(task_body: serde_json::Value) -> Result<serde_json::Value, AppError> {
    let key = tripo_key();
    if key.is_empty() {
        return Err(AppError::bad(
            "需要 TRIPO_API_KEY。到 tripo3d.ai 注册送 300 额度（2 周有效），注册不需要信用卡",
        ));
    }
    let resp = HTTP
        .post("https://api.tripo3d.ai/v2/openapi/task")
        .header("Authorization", format!("Bearer {key}"))
        .json(&task_body)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("Tripo 请求失败: {e}")))?;
    let st = resp.status().as_u16();
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    if st >= 400 || body.get("code").and_then(|v| v.as_i64()).unwrap_or(0) != 0 {
        let msg = body
            .get("message")
            .and_then(|v| v.as_str())
            .or_else(|| body.get("error").and_then(|v| v.as_str()))
            .unwrap_or("未知错误");
        return Err(AppError::bad(format!("Tripo {st}: {msg}")));
    }
    let task_id = body
        .get("data")
        .and_then(|d| d.get("task_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if task_id.is_empty() {
        return Err(AppError::internal("Tripo 未返回 task_id"));
    }
    let poll_url = format!("https://api.tripo3d.ai/v2/openapi/task/{task_id}");
    for _ in 0..150 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let pr = HTTP
            .get(&poll_url)
            .header("Authorization", format!("Bearer {key}"))
            .send()
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        let pb: serde_json::Value = pr
            .json()
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        let status = pb
            .get("data")
            .and_then(|d| d.get("status"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        match status {
            "success" => return Ok(pb),
            "failed" | "cancelled" | "unknown" => {
                let msg = pb
                    .get("data")
                    .and_then(|d| d.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("任务失败");
                return Err(AppError::bad(format!("Tripo: {msg}")));
            }
            _ => {} // running/queued → keep polling
        }
    }
    Err(AppError::internal("Tripo 超时（300s）"))
}

// ── Internal LLM call for scene generation ───────────────────────────
async fn llm_scene(prompt: &str, model: &str, auth: &str) -> Result<serde_json::Value, AppError> {
    let model = if model.trim().is_empty() {
        "deepseek-v4-flash"
    } else {
        model.trim()
    };
    let resp = HTTP
        .post("http://127.0.0.1:8080/v1/chat/completions")
        .header("Authorization", auth)
        .json(&json!({
            "model": model,
            "messages": [
                {"role": "system", "content": crate::procedural_3d::SCENE_SYSTEM_PROMPT},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.7,
            "max_tokens": 4000
        }))
        .send()
        .await
        .map_err(|e| AppError::internal(format!("LLM 场景生成请求失败: {e}")))?;
    let st = resp.status().as_u16();
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::internal(format!("LLM 响应解析失败: {e}")))?;
    if st >= 400 {
        let msg = body
            .get("error")
            .and_then(|e| {
                e.get("message")
                    .and_then(|m| m.as_str())
                    .or_else(|| e.as_str())
            })
            .unwrap_or("LLM 请求失败");
        return Err(AppError::internal(format!("LLM {st}: {msg}")));
    }
    let content = body
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("");
    let trimmed = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    serde_json::from_str(trimmed).map_err(|e| {
        AppError::internal(format!(
            "LLM 返回的 JSON 无效: {e}\n内容: {}",
            &trimmed[..trimmed.len().min(200)]
        ))
    })
}

// ── Internal text→image (feeds the neural image-to-3D pipeline) ──────
// Returns an image reference usable by trellis_gen.py's handle_file():
// either a local file path (/tmp/n3d_in_*.jpg) or a public URL.
// Source order: HF FLUX (fast ~11s, reliable) → gpt-image-2 (retries).
async fn text_to_image(prompt: &str, auth: &str) -> Result<String, AppError> {
    // Image-to-3D needs ONE clean, centred, fully-visible object on a plain
    // background — enrich the prompt so the neural mesh has a clear silhouette.
    let enriched = format!(
        "{prompt}, a single 3D game asset, one object only, centered, isolated on a plain light gray background, \
         entire object fully visible in frame, three-quarter view, soft even studio lighting, no harsh shadows, no text, high detail"
    );

    // 1. HF FLUX — fast and reliable; returns raw image bytes → temp file.
    if !hf_key().is_empty() {
        match flux_image(&enriched).await {
            Ok(path) => return Ok(path),
            Err(e) => tracing::warn!(
                "[generate_3d] FLUX image failed, trying gpt-image-2: {}",
                e.msg
            ),
        }
    }

    // 2. gpt-image-2 (upstream image service) with retries on transient 5xx.
    gpt_image(&enriched, auth).await
}

// FLUX.1-schnell via the HF Inference router → raw image bytes → temp file path.
async fn flux_image(prompt: &str) -> Result<String, AppError> {
    let model = std::env::var("FLUX_MODEL")
        .unwrap_or_else(|_| "black-forest-labs/FLUX.1-schnell".to_string());
    let resp = hf_inference(&model, &json!({ "inputs": prompt })).await?;
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| AppError::internal(format!("FLUX 读取失败: {e}")))?;
    if bytes.len() < 500 || ct.contains("application/json") {
        return Err(AppError::internal("FLUX 未返回图像"));
    }
    let path = format!("/tmp/n3d_in_{}.jpg", uuid::Uuid::new_v4());
    tokio::fs::write(&path, &bytes)
        .await
        .map_err(|e| AppError::internal(format!("写入图像失败: {e}")))?;
    Ok(path)
}

// gpt-image-2 through the internal images endpoint, retrying transient 5xx.
async fn gpt_image(prompt: &str, auth: &str) -> Result<String, AppError> {
    let model = std::env::var("IMAGE_GEN_MODEL").unwrap_or_else(|_| "gpt-image-2".to_string());
    let payload = json!({ "model": model, "prompt": prompt, "size": "1024x1024", "n": 1 });
    let mut last = String::from("图像生成失败");
    for attempt in 0..3u32 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(3 * attempt as u64)).await;
        }
        let send = HTTP
            .post("http://127.0.0.1:8080/v1/images/generations")
            .header("Authorization", auth)
            .json(&payload)
            .send();
        let resp = match tokio::time::timeout(std::time::Duration::from_secs(75), send).await {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                last = format!("请求错误: {e}");
                continue;
            }
            Err(_) => {
                last = "图像生成超时(75s)".into();
                continue;
            }
        };
        let st = resp.status().as_u16();
        let body: serde_json::Value = match resp.json().await {
            Ok(b) => b,
            Err(e) => {
                last = format!("响应解析失败: {e}");
                continue;
            }
        };
        if st >= 500 {
            // upstream transient — retry
            last = format!("图像生成 {st}");
            tracing::warn!(
                "[generate_3d] gpt-image attempt {} got {st}, retrying",
                attempt + 1
            );
            continue;
        }
        if st >= 400 {
            // client error — don't retry
            let msg = body
                .get("error")
                .and_then(|e| {
                    e.get("message")
                        .and_then(|m| m.as_str())
                        .or_else(|| e.as_str())
                })
                .unwrap_or("图像生成失败");
            return Err(AppError::internal(format!("图像生成 {st}: {msg}")));
        }
        if let Some(url) = body.pointer("/data/0/url").and_then(|v| v.as_str()) {
            if !url.is_empty() {
                return Ok(url.to_string());
            }
        }
        if let Some(b64) = body.pointer("/data/0/b64_json").and_then(|v| v.as_str()) {
            if !b64.is_empty() {
                return Ok(format!("data:image/png;base64,{b64}"));
            }
        }
        last = "未返回 url/b64".into();
    }
    Err(AppError::internal(format!("图像生成失败(重试3次): {last}")))
}

// Remove a FLUX-produced temp input image once it's been consumed.
async fn cleanup_input(image_ref: &str) {
    if image_ref.starts_with("/tmp/n3d_in_") {
        let _ = tokio::fs::remove_file(image_ref).await;
    }
}

// ── Neural image→3D via free HF ZeroGPU Space (TRELLIS) ──────────────
// Shells out to trellis_gen.py (gradio_client) — it handles the Space's
// session handshake / upload / SSE that raw HTTP can't do reliably.
async fn neural_3d(image_url: &str) -> Result<Vec<u8>, AppError> {
    if hf_key().is_empty() {
        return Err(AppError::bad("需要 HF_API_KEY 才能用神经 3D"));
    }
    let out = format!("/tmp/n3d_{}.glb", uuid::Uuid::new_v4());
    tracing::info!(
        "[generate_3d] neural: spawning trellis_gen.py, img_url_len={}",
        image_url.len()
    );
    let _slot = heavy_gen_slot()?;
    let child = tokio::process::Command::new("python3")
        .arg("/app/trellis_gen.py")
        .arg(image_url)
        .arg(&out)
        .env("HOME", "/root")
        .env("HF_HOME", "/root/.cache/huggingface")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true) // ensure the python proc dies if we time out
        .spawn()
        .map_err(|e| AppError::internal(format!("神经 3D 启动失败: {e}")))?;
    let output = match tokio::time::timeout(
        std::time::Duration::from_secs(260),
        child.wait_with_output(),
    )
    .await
    {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => {
            cleanup_input(image_url).await;
            return Err(AppError::internal(format!("神经 3D 进程错误: {e}")));
        }
        Err(_) => {
            cleanup_input(image_url).await;
            let _ = tokio::fs::remove_file(&out).await;
            return Err(AppError::internal("神经 3D 超时（260s）"));
        }
    };
    // Input temp image (if FLUX-produced) is no longer needed once python ran.
    cleanup_input(image_url).await;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let tail = err.lines().rev().take(3).collect::<Vec<_>>();
        let tail: String = tail.into_iter().rev().collect::<Vec<_>>().join(" | ");
        let _ = tokio::fs::remove_file(&out).await;
        return Err(AppError::internal(format!("神经 3D 生成失败: {tail}")));
    }
    let bytes = tokio::fs::read(&out)
        .await
        .map_err(|e| AppError::internal(format!("读取神经 3D 结果失败: {e}")))?;
    let _ = tokio::fs::remove_file(&out).await;
    if bytes.len() < 100 {
        return Err(AppError::internal("神经 3D 结果为空"));
    }
    Ok(bytes)
}

// Procedural fallback: LLM scene graph → primitive-composed GLB. Always
// available, instant, no external dependency. The guaranteed floor.
async fn procedural_glb(prompt: &str, model: &str, auth: &str) -> Result<Response, AppError> {
    let scene = llm_scene(prompt, model, auth).await?;
    let nodes = crate::procedural_3d::parse_scene(&scene);
    if nodes.is_empty() {
        return Err(AppError::internal("LLM 生成的场景为空"));
    }
    let glb = crate::procedural_3d::build_glb(&nodes);
    Ok(relay(
        200,
        "model/gltf-binary",
        axum::body::Bytes::from(glb),
    ))
}

// ── POST /v1/game/generate-3d ────────────────────────────────────────
// Tiered cascade, best-available quality that never hard-fails:
//   0. explicit `scene` JSON  → procedural directly (agent-authored, instant)
//   1. `fast:true`            → procedural placeholder (skip slow neural)
//   2. TRIPO_API_KEY set      → Tripo3D (pro quality, uses user credits)
//   3. HF_API_KEY set         → neural TRELLIS (free Tripo-class, text→image→mesh)
//   4. always                 → procedural fallback (guaranteed floor)
pub async fn generate_3d(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    crate::models::require_paid_access(&state, &headers).await?;
    let prompt = body.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    if prompt.is_empty() {
        return Err(AppError::bad("缺少 prompt"));
    }
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let model = body.get("model").and_then(|v| v.as_str()).unwrap_or("");
    let fast = body.get("fast").and_then(|v| v.as_bool()).unwrap_or(false)
        || body.get("style").and_then(|v| v.as_str()) == Some("procedural");

    // 0. Explicit scene JSON → procedural directly (agent-authored, instant).
    if let Some(scene) = body.get("scene") {
        let nodes = crate::procedural_3d::parse_scene(scene);
        if nodes.is_empty() {
            return Err(AppError::bad("scene 中没有有效节点"));
        }
        let glb = crate::procedural_3d::build_glb(&nodes);
        return Ok(relay(
            200,
            "model/gltf-binary",
            axum::body::Bytes::from(glb),
        ));
    }

    // 1. Fast mode → procedural placeholder, skip slow neural gen.
    if fast {
        return procedural_glb(prompt, model, auth).await;
    }

    // 2. Tripo key → pro quality. On any failure, fall through to free paths.
    if !tripo_key().is_empty() {
        if let Ok(result) = tripo_task(json!({"type":"text_to_model","prompt":prompt})).await {
            let task_id = result
                .get("data")
                .and_then(|d| d.get("task_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let model_url = result
                .get("data")
                .and_then(|d| d.get("output"))
                .and_then(|o| o.get("model"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if !model_url.is_empty() {
                if let Ok(dl) = HTTP.get(&model_url).send().await {
                    let ct = dl
                        .headers()
                        .get("content-type")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("model/gltf-binary")
                        .to_string();
                    if let Ok(bytes) = dl.bytes().await {
                        return Ok(relay_with_task_id(200, &ct, bytes, &task_id));
                    }
                }
            }
        }
        tracing::warn!("[generate_3d] Tripo path failed, falling through to neural/procedural");
    }

    // 3. Neural TRELLIS — free Tripo-class: text→image→mesh. Falls back to
    //    procedural on any failure (quota exhausted, Space asleep, timeout).
    if !hf_key().is_empty() {
        match text_to_image(prompt, auth).await {
            Ok(img_url) => match neural_3d(&img_url).await {
                Ok(glb) => {
                    tracing::info!("[generate_3d] neural mesh OK, {} bytes", glb.len());
                    return Ok(relay(
                        200,
                        "model/gltf-binary",
                        axum::body::Bytes::from(glb),
                    ));
                }
                Err(e) => {
                    tracing::warn!("[generate_3d] neural mesh failed → procedural: {}", e.msg)
                }
            },
            Err(e) => tracing::warn!("[generate_3d] text→image failed → procedural: {}", e.msg),
        }
    }

    // 4. Procedural fallback — always works.
    procedural_glb(prompt, model, auth).await
}

// ── Local CPU MusicGen (free, unlimited, no quota, no external dep) ──
// Shells out to music_gen.py (transformers). ~3s load + ~4s per audio-sec
// on the container CPU. The default engine for music & (fallback) SFX.
async fn local_musicgen(prompt: &str, duration: f64) -> Result<Vec<u8>, AppError> {
    let dur = duration.clamp(2.0, 30.0);
    let out = format!("/tmp/mus_{}.mp3", uuid::Uuid::new_v4());
    tracing::info!(
        "[generate_music] local MusicGen: ~{dur:.0}s, prompt_len={}",
        prompt.len()
    );
    let _slot = heavy_gen_slot()?;
    let child = tokio::process::Command::new("python3")
        .arg("/app/music_gen.py")
        .arg(prompt)
        .arg(&out)
        .arg(format!("{dur}"))
        .env("HOME", "/root")
        .env("HF_HOME", "/root/.cache/huggingface")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| AppError::internal(format!("音乐生成启动失败: {e}")))?;
    let output = match tokio::time::timeout(
        std::time::Duration::from_secs(220),
        child.wait_with_output(),
    )
    .await
    {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => return Err(AppError::internal(format!("音乐进程错误: {e}"))),
        Err(_) => {
            let _ = tokio::fs::remove_file(&out).await;
            return Err(AppError::internal("音乐生成超时（220s）"));
        }
    };
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let tail: String = err
            .lines()
            .rev()
            .take(3)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" | ");
        let _ = tokio::fs::remove_file(&out).await;
        return Err(AppError::internal(format!("音乐生成失败: {tail}")));
    }
    let bytes = tokio::fs::read(&out)
        .await
        .map_err(|e| AppError::internal(format!("读取音乐结果失败: {e}")))?;
    let _ = tokio::fs::remove_file(&out).await;
    if bytes.len() < 100 {
        return Err(AppError::internal("音乐结果为空"));
    }
    Ok(bytes)
}

// ── POST /v1/game/generate-sound ─ ElevenLabs (key) / local MusicGen ─
pub async fn generate_sound(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    crate::models::require_paid_access(&state, &headers).await?;
    let prompt = body.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    if prompt.is_empty() {
        return Err(AppError::bad("缺少 prompt"));
    }
    let duration = body.get("duration").and_then(|v| v.as_f64()).unwrap_or(5.0);

    // Preferred: ElevenLabs SFX if a key is configured (crisp, purpose-built).
    let el_key = elevenlabs_key();
    if !el_key.is_empty() {
        let resp = HTTP
            .post("https://api.elevenlabs.io/v1/sound-generation")
            .header("xi-api-key", &el_key)
            .json(&json!({
                "text": prompt,
                "duration_seconds": duration,
                "prompt_influence": 0.3
            }))
            .send()
            .await
            .map_err(|e| AppError::internal(format!("ElevenLabs SFX 请求失败: {e}")))?;
        let st = resp.status().as_u16();
        if st >= 400 {
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError {
                status: StatusCode::from_u16(st).unwrap_or(StatusCode::BAD_GATEWAY),
                msg: format!(
                    "ElevenLabs SFX {st}: {}",
                    text.chars().take(300).collect::<String>()
                ),
            });
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        return Ok(relay(200, "audio/mpeg", bytes));
    }

    // Free fallback (no key): local MusicGen as a short sound cue. Not as crisp
    // as a dedicated SFX model, but $0, unlimited, and needs no signup.
    let sfx_prompt = format!("short game sound effect: {prompt}");
    let bytes = local_musicgen(&sfx_prompt, duration.min(4.0)).await?;
    Ok(relay(200, "audio/mpeg", axum::body::Bytes::from(bytes)))
}

// ── POST /v1/game/generate-music ─ ElevenLabs / Replicate ────────────
pub async fn generate_music(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    crate::models::require_paid_access(&state, &headers).await?;
    let prompt = body.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    if prompt.is_empty() {
        return Err(AppError::bad("缺少 prompt"));
    }
    let duration = body
        .get("duration")
        .and_then(|v| v.as_f64())
        .unwrap_or(30.0)
        .min(30.0);

    // Local MusicGen (free, unlimited, no quota) — the default engine.
    match local_musicgen(prompt, duration).await {
        Ok(bytes) => return Ok(relay(200, "audio/mpeg", axum::body::Bytes::from(bytes))),
        Err(e) => tracing::warn!("[generate_music] 本地 MusicGen 失败: {}", e.msg),
    }

    // Optional paid fallback: Replicate MusicGen (only if a token is configured).
    if !replicate_key().is_empty() {
        let result = replicate_run(
            "meta/musicgen",
            json!({ "prompt": prompt, "duration": duration as i32 }),
        )
        .await?;
        let output_url = result.get("output").and_then(|v| v.as_str()).unwrap_or("");
        if !output_url.is_empty() {
            let dl = HTTP
                .get(output_url)
                .send()
                .await
                .map_err(|e| AppError::internal(e.to_string()))?;
            let bytes = dl
                .bytes()
                .await
                .map_err(|e| AppError::internal(e.to_string()))?;
            return Ok(relay(200, "audio/wav", bytes));
        }
    }
    Err(AppError::internal("音乐生成失败（本地 MusicGen 未产出）"))
}

// ── POST /v1/game/generate-voice ─ Edge TTS (free) / ElevenLabs ──────
pub async fn generate_voice(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    crate::models::require_paid_access(&state, &headers).await?;
    let text = body
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if text.is_empty() {
        return Err(AppError::bad("缺少 text"));
    }
    let voice = body
        .get("voice")
        .and_then(|v| v.as_str())
        .unwrap_or("default");

    // If ElevenLabs key is set, use it (higher quality)
    let el_key = elevenlabs_key();
    if !el_key.is_empty() {
        let voice_id = match voice {
            "default" | "" => "21m00Tcm4TlvDq8ikWAM",
            other => other,
        };
        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{voice_id}");
        let resp = HTTP
            .post(&url)
            .header("xi-api-key", &el_key)
            .json(&json!({"text": text, "model_id": "eleven_multilingual_v2"}))
            .send()
            .await
            .map_err(|e| AppError::internal(format!("ElevenLabs: {e}")))?;
        let ct = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("audio/mpeg")
            .to_string();
        let st = resp.status().as_u16();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        return Ok(relay(st, &ct, bytes));
    }

    // Fallback: Edge TTS (free, no key)
    let edge_voice = match voice {
        "default" | "" | "male" => "en-US-GuyNeural",
        "female" => "en-US-AriaNeural",
        "zh" | "chinese" => "zh-CN-YunxiNeural",
        "zh-female" => "zh-CN-XiaoxiaoNeural",
        "ja" | "japanese" => "ja-JP-KeitaNeural",
        "ko" | "korean" => "ko-KR-InJoonNeural",
        other => other,
    };
    let tmp = format!(
        "/tmp/edge_tts_{}.mp3",
        std::process::id() as u64
            ^ (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64)
    );
    let out = tokio::process::Command::new("python3")
        .args([
            "-m",
            "edge_tts",
            "--text",
            text,
            "--voice",
            edge_voice,
            "--write-media",
            &tmp,
        ])
        .output()
        .await
        .map_err(|e| AppError::internal(format!("Edge TTS 启动失败: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(AppError::internal(format!(
            "Edge TTS 失败: {}",
            stderr.chars().take(300).collect::<String>()
        )));
    }
    let bytes = tokio::fs::read(&tmp)
        .await
        .map_err(|e| AppError::internal(format!("读取音频失败: {e}")))?;
    let _ = tokio::fs::remove_file(&tmp).await;
    if bytes.is_empty() {
        return Err(AppError::internal("Edge TTS 生成了空音频"));
    }
    Ok(relay(200, "audio/mpeg", axum::body::Bytes::from(bytes)))
}

// ── POST /v1/game/auto-rig ─ Tripo3D ─────────────────────────────────
pub async fn auto_rig(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    crate::models::require_paid_access(&state, &headers).await?;
    // Tripo rigging needs an existing task_id from a previous 3D generation
    let original_task_id = body.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
    if original_task_id.is_empty() {
        return Err(AppError::bad(
            "auto_rig 需要 task_id（先用 generate_3d 生成模型，再用返回的 task_id 绑骨）。\
             或者访问 mixamo.com 免费在线绑骨。",
        ));
    }
    let result = tripo_task(json!({
        "type": "animate_rig",
        "original_model_task_id": original_task_id
    }))
    .await?;
    let task_id = result
        .get("data")
        .and_then(|d| d.get("task_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::internal("Tripo 未返回绑定 task_id"))?
        .to_string();
    let model_url = result
        .get("data")
        .and_then(|d| d.get("output"))
        .and_then(|o| o.get("model"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if model_url.is_empty() {
        return Err(AppError::internal("Tripo 未返回绑骨模型链接"));
    }
    let dl = HTTP
        .get(model_url)
        .send()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    let bytes = dl
        .bytes()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    Ok(relay_with_task_id(200, "model/gltf-binary", bytes, &task_id))
}

// ── POST /v1/game/generate-motion ─ Tripo3D animation ─────────────────
pub async fn generate_motion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    crate::models::require_paid_access(&state, &headers).await?;
    let original_task_id = body.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
    if original_task_id.is_empty() {
        return Err(AppError::bad(
            "generate_motion 需要 task_id（先用 generate_3d + auto_rig 生成已绑骨模型）。\
             或者访问 mixamo.com 免费下载动画。",
        ));
    }
    let prompt = body
        .get("prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("walk");
    let result = tripo_task(json!({
        "type": "animate_retarget",
        "original_model_task_id": original_task_id,
        "animation": prompt
    }))
    .await?;
    let task_id = result
        .get("data")
        .and_then(|d| d.get("task_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::internal("Tripo 未返回动画 task_id"))?
        .to_string();
    let model_url = result
        .get("data")
        .and_then(|d| d.get("output"))
        .and_then(|o| o.get("model"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if model_url.is_empty() {
        return Err(AppError::internal("Tripo 未返回动画模型链接"));
    }
    let dl = HTTP
        .get(model_url)
        .send()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    let bytes = dl
        .bytes()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    Ok(relay_with_task_id(200, "model/gltf-binary", bytes, &task_id))
}

// ── POST /v1/game/generate-texture ─ HF FLUX ─────────────────────────
pub async fn generate_texture(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    crate::models::require_paid_access(&state, &headers).await?;
    let prompt = body.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    if prompt.is_empty() {
        return Err(AppError::bad("缺少 prompt"));
    }
    let full_prompt = format!("seamless tileable texture, {prompt}, game asset, PBR material");
    let resp = hf_inference(
        "black-forest-labs/FLUX.1-schnell",
        &json!({"inputs": full_prompt}),
    )
    .await?;
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/png")
        .to_string();
    let st = resp.status().as_u16();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    Ok(relay(st, &ct, bytes))
}

// ── POST /v1/game/search-assets ───────────────────────────────────────
pub async fn search_assets(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    crate::models::auth_any_user(&state, &headers).await?;
    let query = body.get("query").and_then(|v| v.as_str()).unwrap_or("");
    if query.is_empty() {
        return Err(AppError::bad("缺少 query"));
    }
    let asset_type = body.get("type").and_then(|v| v.as_str()).unwrap_or("all");
    let mut results = Vec::new();

    // 3D models: Sketchfab search link
    if matches!(asset_type, "all" | "3d" | "model") {
        let encoded: String = query
            .bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    (b as char).to_string()
                }
                b' ' => "+".to_string(),
                _ => format!("%{b:02X}"),
            })
            .collect();
        results.push(json!({
            "name": format!("在 Sketchfab 搜索「{query}」"),
            "source": "sketchfab", "type": "3d",
            "url": format!("https://sketchfab.com/search?q={encoded}&type=models"),
            "thumbnail": "", "license": "varies",
            "note": "Sketchfab API 需要认证，请通过链接在浏览器中搜索"
        }));
    }

    // Textures/HDRI: PolyHaven
    if matches!(asset_type, "all" | "texture" | "hdri") {
        let ph_type = if asset_type == "hdri" {
            "hdris"
        } else {
            "textures"
        };
        if let Ok(resp) = HTTP
            .get("https://api.polyhaven.com/assets")
            .query(&[("t", ph_type)])
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
        {
            if let Ok(j) = resp.json::<serde_json::Value>().await {
                if let Some(map) = j.as_object() {
                    let q_lower = query.to_lowercase();
                    for (name, meta) in map.iter() {
                        let name_lower = name.to_lowercase();
                        let tags = meta
                            .get("tags")
                            .and_then(|t| t.as_array())
                            .map(|a| {
                                a.iter()
                                    .filter_map(|v| v.as_str())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                                    .to_lowercase()
                            })
                            .unwrap_or_default();
                        if name_lower.contains(&q_lower) || tags.contains(&q_lower) {
                            results.push(json!({
                                "name": name, "source": "polyhaven", "type": ph_type.trim_end_matches('s'),
                                "url": format!("https://polyhaven.com/a/{name}"),
                                "download_url": format!("https://dl.polyhaven.org/file/ph-assets/Textures/jpg/2k/{name}/{name}_diff_2k.jpg"),
                                "thumbnail": format!("https://cdn.polyhaven.com/asset_img/thumbs/{name}.png?width=256"),
                                "license": "CC0",
                            }));
                            if results.len() >= 20 {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // Audio: Freesound
    if matches!(asset_type, "all" | "sound" | "audio" | "music") {
        let fs_key = freesound_key();
        if !fs_key.is_empty() {
            if let Ok(resp) = HTTP
                .get("https://freesound.org/apiv2/search/text/")
                .query(&[
                    ("query", query),
                    ("token", &fs_key),
                    ("fields", "id,name,previews,license,duration"),
                ])
                .timeout(std::time::Duration::from_secs(15))
                .send()
                .await
            {
                if let Ok(j) = resp.json::<serde_json::Value>().await {
                    if let Some(arr) = j.get("results").and_then(|v| v.as_array()) {
                        for item in arr.iter().take(10) {
                            results.push(json!({
                                "name": item.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                                "source": "freesound", "type": "audio",
                                "url": format!("https://freesound.org/people/unknown/sounds/{}/", item.get("id").and_then(|v| v.as_u64()).unwrap_or(0)),
                                "download_url": item.get("previews").and_then(|p| p.get("preview-hq-mp3")).and_then(|v| v.as_str()).unwrap_or(""),
                                "license": item.get("license").and_then(|v| v.as_str()).unwrap_or(""),
                            }));
                        }
                    }
                }
            }
        }
    }

    Ok(Json(json!({ "results": results })))
}
