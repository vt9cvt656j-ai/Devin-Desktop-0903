use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

const MANIFEST_CACHE_TTL: Duration = Duration::from_secs(300);
const MAX_MANIFEST_BYTES: usize = 1024 * 1024;

#[derive(Clone)]
struct CachedManifest {
    result: CachedUpdate,
    fetched_at: Instant,
}

#[derive(Clone)]
enum CachedUpdate {
    Manifest(Vec<u8>),
    NoUpdate,
}

static MANIFEST_CACHE: LazyLock<RwLock<Option<CachedManifest>>> =
    LazyLock::new(|| RwLock::new(None));
static MANIFEST_FETCH_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

// tag → (fetched_at, 资产名 → GitHub asset id)。私有仓库的安装包必须由网关带 token
// 代下载，这里缓存 release 的资产清单，公开下载路由不用每个请求都打 GitHub API。
const ASSET_MAP_TTL: Duration = Duration::from_secs(600);
static ASSET_MAP_CACHE: LazyLock<RwLock<HashMap<String, (Instant, HashMap<String, u64>)>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 公开下载路由的路径段白名单：GitHub tag/资产名只含字母数字与 . _ -（空格上传时已被
/// GitHub 转成点）。拒绝一切越界字符，杜绝路径拼接歧义。
fn safe_release_path_segment(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && !value.starts_with('.')
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// 私有仓库的 GitHub 资产 URL 匿名不可下载：把清单里的安装包地址重写到网关代理路由。
/// 签名内容内嵌在清单 signature 字段里，无需重写。
fn rewrite_manifest_urls(state: &AppState, manifest: &mut Value, tag: &str) {
    let base = state.cfg.ide_update_public_base.trim_end_matches('/').to_owned();
    let Some(platforms) = manifest.get_mut("platforms").and_then(Value::as_object_mut) else {
        return;
    };
    for entry in platforms.values_mut() {
        let Some(item) = entry.as_object_mut() else { continue };
        let Some(url) = item.get("url").and_then(Value::as_str) else { continue };
        let Some(file) = url.rsplit('/').next() else { continue };
        if !safe_release_path_segment(file, 160) {
            continue;
        }
        let proxied = format!("{base}/api/ide/update/download/{tag}/{file}");
        item.insert("url".to_owned(), Value::String(proxied));
    }
}

fn release_asset_map(release: &Value) -> HashMap<String, u64> {
    let mut map = HashMap::new();
    for asset in release
        .get("assets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let (Some(name), Some(id)) = (
            asset.get("name").and_then(Value::as_str),
            asset.get("id").and_then(Value::as_u64),
        ) {
            map.insert(name.to_owned(), id);
        }
    }
    map
}

fn manifest_response(body: Vec<u8>, stale: bool) -> Response {
    let cache_control = if stale {
        "public, max-age=30, stale-if-error=86400"
    } else {
        "public, max-age=300, stale-if-error=86400"
    };
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            (header::CACHE_CONTROL, cache_control),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        body,
    )
        .into_response()
}

fn no_update_response(stale: bool) -> Response {
    let cache_control = if stale {
        "public, max-age=30, stale-if-error=3600"
    } else {
        "public, max-age=300"
    };
    (
        StatusCode::NO_CONTENT,
        [
            (header::CACHE_CONTROL, cache_control),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
    )
        .into_response()
}

fn cached_response(cached: CachedManifest, stale: bool) -> Response {
    match cached.result {
        CachedUpdate::Manifest(body) => manifest_response(body, stale),
        CachedUpdate::NoUpdate => no_update_response(stale),
    }
}

fn validate_manifest(value: &Value) -> Result<(), &'static str> {
    let object = value.as_object().ok_or("manifest must be a JSON object")?;
    let version = object
        .get("version")
        .or_else(|| object.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim_start_matches('v');
    let mut parts = version.split('.');
    if version.is_empty()
        || parts.clone().count() < 3
        || !parts.all(|part| {
            !part.is_empty()
                && part
                    .split(['-', '+'])
                    .next()
                    .unwrap_or("")
                    .chars()
                    .all(|c| c.is_ascii_digit())
        })
    {
        return Err("manifest version must be semantic");
    }

    let platforms = object
        .get("platforms")
        .and_then(Value::as_object)
        .ok_or("manifest platforms must be an object")?;
    if platforms.is_empty() {
        return Err("manifest has no platforms");
    }
    for platform in platforms.values() {
        let item = platform
            .as_object()
            .ok_or("platform entry must be an object")?;
        let url = item.get("url").and_then(Value::as_str).unwrap_or("");
        let signature = item.get("signature").and_then(Value::as_str).unwrap_or("");
        if !url.starts_with("https://") || signature.trim().is_empty() {
            return Err("platform entry requires an HTTPS URL and signature");
        }
    }
    Ok(())
}

fn manifest_version(value: &Value) -> &str {
    value
        .get("version")
        .or_else(|| value.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim_start_matches('v')
}

fn validate_release_tag(tag: &str) -> Result<&str, &'static str> {
    let version = tag.strip_prefix('v').ok_or("tag 必须使用 vX.Y.Z 格式")?;
    let mut parts = version.split('.');
    if parts.clone().count() != 3
        || !parts.all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
    {
        return Err("tag 必须使用 vX.Y.Z 格式");
    }
    Ok(version)
}

fn valid_github_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    let valid_part = |part: &str| {
        !part.is_empty()
            && part.len() <= 100
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    };
    matches!((parts.next(), parts.next(), parts.next()), (Some(owner), Some(name), None) if valid_part(owner) && valid_part(name))
}

fn valid_workflow_name(workflow: &str) -> bool {
    !workflow.is_empty()
        && workflow.len() <= 128
        && workflow
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn validate_publish_manifest(value: &Value, tag: &str) -> Result<(), &'static str> {
    validate_manifest(value)?;
    let expected = validate_release_tag(tag)?;
    if manifest_version(value) != expected {
        return Err("latest.json 版本与待发布 tag 不一致");
    }
    let platforms = value
        .get("platforms")
        .and_then(Value::as_object)
        .ok_or("latest.json 缺少 platforms")?;
    for required in ["darwin-aarch64", "windows-x86_64"] {
        if !platforms.contains_key(required) {
            return Err("latest.json 尚未包含 macOS 与 Windows 两个平台");
        }
    }
    Ok(())
}

fn admin_only(claims: &Claims) -> ApiResult<()> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    Ok(())
}

fn github_api_base(state: &AppState) -> ApiResult<String> {
    let repo = state.cfg.ide_release_github_repo.trim();
    let workflow = state.cfg.ide_release_github_workflow.trim();
    if !valid_github_repo(repo) {
        return Err(AppError::internal("IDE_RELEASE_GITHUB_REPO 配置无效"));
    }
    if !valid_workflow_name(workflow) {
        return Err(AppError::internal("IDE_RELEASE_GITHUB_WORKFLOW 配置无效"));
    }
    Ok(format!("https://api.github.com/repos/{repo}"))
}

fn github_request(state: &AppState, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    request
        .bearer_auth(state.cfg.ide_release_github_token.trim())
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
}

/// 取 Release 资产的**字节**，而不是它的元数据。
///
/// 必须和 `github_request` 分开：reqwest 的 `.header()` 是追加不是覆盖，所以
/// `github_request(...).header("Accept", "application/octet-stream")` 发出去的是
/// `Accept: application/vnd.github+json, application/octet-stream`，GitHub 认前者，
/// 回的是资产的元数据 JSON。于是 latest.json 下载到的是一个没有 version 字段的对象，
/// 校验报「manifest version must be semantic」，更新接口 502；安装包代理同理会把一段
/// JSON 当成安装包发给客户端。这是自动更新一直不通的原因之一，而且它只在真的有一版
/// 带 latest.json 的 Release 时才会暴露——在那之前这条路径从没被走到过。
fn github_asset_request(state: &AppState, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    request
        .bearer_auth(state.cfg.ide_release_github_token.trim())
        .header("Accept", "application/octet-stream")
        .header("X-GitHub-Api-Version", "2022-11-28")
}

fn github_transport_error(action: &str, error: reqwest::Error) -> AppError {
    AppError {
        status: StatusCode::BAD_GATEWAY,
        msg: format!("GitHub {action}请求失败：{error}"),
    }
}

async fn github_error(action: &str, response: reqwest::Response) -> AppError {
    let status = response.status();
    let body = response.bytes().await.unwrap_or_default();
    let message = serde_json::from_slice::<Value>(&body)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    AppError {
        status: StatusCode::BAD_GATEWAY,
        msg: format!(
            "GitHub {action}失败：{}",
            message.chars().take(240).collect::<String>()
        ),
    }
}

async fn github_json(action: &str, request: reqwest::RequestBuilder) -> ApiResult<Value> {
    let response = request
        .send()
        .await
        .map_err(|error| github_transport_error(action, error))?;
    if !response.status().is_success() {
        return Err(github_error(action, response).await);
    }
    response
        .json::<Value>()
        .await
        .map_err(|error| github_transport_error(action, error))
}

async fn github_empty(action: &str, request: reqwest::RequestBuilder) -> ApiResult<()> {
    let response = request
        .send()
        .await
        .map_err(|error| github_transport_error(action, error))?;
    if !response.status().is_success() {
        return Err(github_error(action, response).await);
    }
    Ok(())
}

fn summarize_runs(value: &Value) -> Vec<Value> {
    value
        .get("workflow_runs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(12)
        .map(|run| {
            serde_json::json!({
                "id": run.get("id"),
                "name": run.get("name"),
                "display_title": run.get("display_title"),
                "event": run.get("event"),
                "head_branch": run.get("head_branch"),
                "head_sha": run.get("head_sha"),
                "status": run.get("status"),
                "conclusion": run.get("conclusion"),
                "html_url": run.get("html_url"),
                "created_at": run.get("created_at"),
                "updated_at": run.get("updated_at"),
                "run_attempt": run.get("run_attempt"),
            })
        })
        .collect()
}

fn summarize_releases(value: &Value) -> Vec<Value> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .take(20)
        .map(|release| {
            let assets = release
                .get("assets")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|asset| {
                    serde_json::json!({
                        "id": asset.get("id"),
                        "name": asset.get("name"),
                        "size": asset.get("size"),
                        "state": asset.get("state"),
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "id": release.get("id"),
                "tag_name": release.get("tag_name"),
                "name": release.get("name"),
                "draft": release.get("draft"),
                "prerelease": release.get("prerelease"),
                "html_url": release.get("html_url"),
                "created_at": release.get("created_at"),
                "published_at": release.get("published_at"),
                "assets": assets,
            })
        })
        .collect()
}

/// Admin release control status. The GitHub token never leaves the server.
pub async fn admin_release_status(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<Value>> {
    admin_only(&claims)?;
    let configured = !state.cfg.ide_release_github_token.trim().is_empty();
    if !configured {
        return Ok(Json(serde_json::json!({
            "configured": false,
            "connected": false,
            "repo": state.cfg.ide_release_github_repo,
            "workflow": state.cfg.ide_release_github_workflow,
            "manifest_url": state.cfg.ide_update_manifest_url,
            "runs": [],
            "releases": [],
        })));
    }

    let base = github_api_base(&state)?;
    let workflow = state.cfg.ide_release_github_workflow.trim();
    let runs_request = github_request(
        &state,
        state.update_http.get(format!(
            "{base}/actions/workflows/{workflow}/runs?per_page=12"
        )),
    );
    let releases_request = github_request(
        &state,
        state
            .update_http
            .get(format!("{base}/releases?per_page=20")),
    );
    let (runs, releases) = tokio::try_join!(
        github_json("读取工作流", runs_request),
        github_json("读取 Release", releases_request),
    )?;

    Ok(Json(serde_json::json!({
        "configured": true,
        "connected": true,
        "repo": state.cfg.ide_release_github_repo,
        "workflow": state.cfg.ide_release_github_workflow,
        "manifest_url": state.cfg.ide_update_manifest_url,
        "actions_url": format!("https://github.com/{}/actions/workflows/{}", state.cfg.ide_release_github_repo, state.cfg.ide_release_github_workflow),
        "releases_url": format!("https://github.com/{}/releases", state.cfg.ide_release_github_repo),
        "runs": summarize_runs(&runs),
        "releases": summarize_releases(&releases),
    })))
}

#[derive(Deserialize)]
pub struct ReleaseTagRequest {
    tag: String,
}

fn clean_release_tag(tag: &str) -> ApiResult<String> {
    let tag = tag.trim();
    validate_release_tag(tag).map_err(AppError::bad)?;
    Ok(tag.to_owned())
}

pub async fn admin_dispatch_release(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<ReleaseTagRequest>,
) -> ApiResult<Json<Value>> {
    admin_only(&claims)?;
    if state.cfg.ide_release_github_token.trim().is_empty() {
        return Err(AppError::bad("服务器尚未配置 IDE_RELEASE_GITHUB_TOKEN"));
    }
    let tag = clean_release_tag(&request.tag)?;
    let base = github_api_base(&state)?;
    let workflow = state.cfg.ide_release_github_workflow.trim();

    // workflow_dispatch accepts any existing ref. Requiring the exact release tag
    // prevents an accidental branch build from producing an unpublishable draft.
    github_json(
        "确认 release tag",
        github_request(
            &state,
            state.update_http.get(format!("{base}/git/ref/tags/{tag}")),
        ),
    )
    .await?;
    github_empty(
        "触发工作流",
        github_request(
            &state,
            state
                .update_http
                .post(format!("{base}/actions/workflows/{workflow}/dispatches"))
                .json(&serde_json::json!({ "ref": tag })),
        ),
    )
    .await?;

    crate::realtime::record_event(
        &state,
        claims.sub.parse().ok(),
        "ide_release_dispatched",
        serde_json::json!({ "by": claims.email, "tag": tag }),
    )
    .await;
    Ok(Json(serde_json::json!({
        "accepted": true,
        "tag": tag,
        "actions_url": format!("https://github.com/{}/actions/workflows/{workflow}", state.cfg.ide_release_github_repo),
    })))
}

async fn release_for_tag(state: &AppState, base: &str, tag: &str) -> ApiResult<Value> {
    let releases = github_json(
        "查找 Draft Release",
        github_request(
            state,
            state
                .update_http
                .get(format!("{base}/releases?per_page=100")),
        ),
    )
    .await?;
    releases
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|release| release.get("tag_name").and_then(Value::as_str) == Some(tag))
        })
        .cloned()
        .ok_or_else(|| AppError::bad(format!("没有找到 {tag} 对应的 Draft Release")))
}

async fn download_release_manifest(
    state: &AppState,
    base: &str,
    release: &Value,
) -> ApiResult<Value> {
    let manifest_asset = release
        .get("assets")
        .and_then(Value::as_array)
        .and_then(|assets| {
            assets
                .iter()
                .find(|asset| asset.get("name").and_then(Value::as_str) == Some("latest.json"))
        })
        .ok_or_else(|| AppError::bad("Draft Release 尚未生成 latest.json"))?;
    let asset_id = manifest_asset
        .get("id")
        .and_then(Value::as_u64)
        .ok_or_else(|| AppError::bad("latest.json asset id 无效"))?;
    let response = github_asset_request(
        state,
        state
            .update_http
            .get(format!("{base}/releases/assets/{asset_id}")),
    )
    .send()
    .await
    .map_err(|error| github_transport_error("读取 latest.json", error))?;
    if !response.status().is_success() {
        return Err(github_error("读取 latest.json", response).await);
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_MANIFEST_BYTES as u64)
    {
        return Err(AppError::bad("latest.json 超过 1 MB"));
    }
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| github_transport_error("读取 latest.json", error))?;
        if body.len().saturating_add(chunk.len()) > MAX_MANIFEST_BYTES {
            return Err(AppError::bad("latest.json 超过 1 MB"));
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|_| AppError::bad("latest.json 不是有效 JSON"))
}

pub async fn admin_publish_release(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<ReleaseTagRequest>,
) -> ApiResult<Json<Value>> {
    admin_only(&claims)?;
    if state.cfg.ide_release_github_token.trim().is_empty() {
        return Err(AppError::bad("服务器尚未配置 IDE_RELEASE_GITHUB_TOKEN"));
    }
    let tag = clean_release_tag(&request.tag)?;
    let base = github_api_base(&state)?;
    let release = release_for_tag(&state, &base, &tag).await?;
    let release_id = release
        .get("id")
        .and_then(Value::as_u64)
        .ok_or_else(|| AppError::bad("GitHub Release id 无效"))?;
    if release.get("draft").and_then(Value::as_bool) != Some(true) {
        return Ok(Json(serde_json::json!({
            "published": false,
            "already_published": true,
            "tag": tag,
            "html_url": release.get("html_url"),
        })));
    }

    let manifest = download_release_manifest(&state, &base, &release).await?;
    validate_publish_manifest(&manifest, &tag).map_err(AppError::bad)?;
    let published = github_json(
        "发布 Release",
        github_request(
            &state,
            state
                .update_http
                .patch(format!("{base}/releases/{release_id}"))
                .json(&serde_json::json!({ "draft": false })),
        ),
    )
    .await?;
    *MANIFEST_CACHE.write().await = None;

    crate::realtime::record_event(
        &state,
        claims.sub.parse().ok(),
        "ide_release_published",
        serde_json::json!({ "by": claims.email, "tag": tag, "release_id": release_id }),
    )
    .await;
    Ok(Json(serde_json::json!({
        "published": true,
        "tag": tag,
        "html_url": published.get("html_url"),
    })))
}

pub async fn admin_cancel_release_run(
    State(state): State<AppState>,
    claims: Claims,
    Path(run_id): Path<u64>,
) -> ApiResult<Json<Value>> {
    admin_only(&claims)?;
    if state.cfg.ide_release_github_token.trim().is_empty() {
        return Err(AppError::bad("服务器尚未配置 IDE_RELEASE_GITHUB_TOKEN"));
    }
    let base = github_api_base(&state)?;
    github_empty(
        "取消工作流",
        github_request(
            &state,
            state
                .update_http
                .post(format!("{base}/actions/runs/{run_id}/cancel")),
        ),
    )
    .await?;
    crate::realtime::record_event(
        &state,
        claims.sub.parse().ok(),
        "ide_release_cancelled",
        serde_json::json!({ "by": claims.email, "run_id": run_id }),
    )
    .await;
    Ok(Json(
        serde_json::json!({ "cancelled": true, "run_id": run_id }),
    ))
}

async fn cached_manifest(fresh_only: bool) -> Option<CachedManifest> {
    let cached = MANIFEST_CACHE.read().await.clone()?;
    if fresh_only && cached.fetched_at.elapsed() >= MANIFEST_CACHE_TTL {
        None
    } else {
        Some(cached)
    }
}

/// Public Tauri updater manifest endpoint. It mirrors the signed GitHub release
/// manifest so desktop clients have a stable Michael-owned endpoint, while the
/// installer itself still has to pass the updater public-key verification.
pub async fn latest(State(state): State<AppState>) -> Response {
    if let Some(cached) = cached_manifest(true).await {
        return cached_response(cached, false);
    }

    // The cache is process-wide, so serialize only cache misses. Without this,
    // the first check from a new client cohort can fan out into many identical
    // GitHub requests before the first response has populated the cache.
    let _fetch_guard = MANIFEST_FETCH_LOCK.lock().await;
    if let Some(cached) = cached_manifest(true).await {
        return cached_response(cached, false);
    }

    // 配了 GitHub token 就走 API 取清单（私有仓库匿名 404 → 之前永远 204"无更新"）。
    // 未配 token 的公开仓库保持原匿名直拉路径不变。
    if !state.cfg.ide_release_github_token.trim().is_empty() {
        return latest_via_github_api(&state).await;
    }

    let response = match state
        .update_http
        .get(&state.cfg.ide_update_manifest_url)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(%error, "IDE update manifest fetch failed");
            return cached_manifest(false)
                .await
                .map(|cached| cached_response(cached, true))
                .unwrap_or_else(|| StatusCode::BAD_GATEWAY.into_response());
        }
    };

    if response.status() == reqwest::StatusCode::NOT_FOUND
        || response.status() == reqwest::StatusCode::NO_CONTENT
    {
        *MANIFEST_CACHE.write().await = Some(CachedManifest {
            result: CachedUpdate::NoUpdate,
            fetched_at: Instant::now(),
        });
        return no_update_response(false);
    }
    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), "IDE update manifest upstream rejected request");
        return cached_manifest(false)
            .await
            .map(|cached| cached_response(cached, true))
            .unwrap_or_else(|| StatusCode::BAD_GATEWAY.into_response());
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_MANIFEST_BYTES as u64)
    {
        return StatusCode::BAD_GATEWAY.into_response();
    }

    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(error) => {
                tracing::warn!(%error, "IDE update manifest stream failed");
                return cached_manifest(false)
                    .await
                    .map(|cached| cached_response(cached, true))
                    .unwrap_or_else(|| StatusCode::BAD_GATEWAY.into_response());
            }
        };
        if body.len().saturating_add(chunk.len()) > MAX_MANIFEST_BYTES {
            return StatusCode::BAD_GATEWAY.into_response();
        }
        body.extend_from_slice(&chunk);
    }

    let value: Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(%error, "IDE update manifest was not valid JSON");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };
    if let Err(error) = validate_manifest(&value) {
        tracing::warn!(%error, "IDE update manifest failed validation");
        return StatusCode::BAD_GATEWAY.into_response();
    }
    let body = match serde_json::to_vec(&value) {
        Ok(body) => body,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    *MANIFEST_CACHE.write().await = Some(CachedManifest {
        result: CachedUpdate::Manifest(body.clone()),
        fetched_at: Instant::now(),
    });
    manifest_response(body, false)
}

/// 私有仓库路径：带 token 从 GitHub API 取最新已发布 Release 的 latest.json，
/// 登记资产清单供代理下载，并把安装包 URL 重写到网关。缓存/兜底语义与匿名路径一致。
async fn latest_via_github_api(state: &AppState) -> Response {
    let base = match github_api_base(state) {
        Ok(base) => base,
        Err(_) => return StatusCode::BAD_GATEWAY.into_response(),
    };
    let response = match github_request(
        state,
        state.update_http.get(format!("{base}/releases/latest")),
    )
    .send()
    .await
    {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(%error, "IDE update latest release fetch failed");
            return cached_manifest(false)
                .await
                .map(|cached| cached_response(cached, true))
                .unwrap_or_else(|| StatusCode::BAD_GATEWAY.into_response());
        }
    };
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        *MANIFEST_CACHE.write().await = Some(CachedManifest {
            result: CachedUpdate::NoUpdate,
            fetched_at: Instant::now(),
        });
        return no_update_response(false);
    }
    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), "IDE update latest release rejected");
        return cached_manifest(false)
            .await
            .map(|cached| cached_response(cached, true))
            .unwrap_or_else(|| StatusCode::BAD_GATEWAY.into_response());
    }
    let release: Value = match response.json().await {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(%error, "IDE update latest release was not valid JSON");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };
    let tag = release
        .get("tag_name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if !safe_release_path_segment(&tag, 64) {
        tracing::warn!(%tag, "IDE update release tag unsafe for proxy path");
        return StatusCode::BAD_GATEWAY.into_response();
    }
    // 旧流水线的 Release 没有 latest.json：这不是故障，是「没有可推送的更新」。
    // 缓存 NoUpdate → 客户端看到"已是最新"，而不是每次手动检查都报 502。
    let has_manifest_asset = release_asset_map(&release).contains_key("latest.json");
    if !has_manifest_asset {
        *MANIFEST_CACHE.write().await = Some(CachedManifest {
            result: CachedUpdate::NoUpdate,
            fetched_at: Instant::now(),
        });
        return no_update_response(false);
    }
    let mut manifest = match download_release_manifest(state, &base, &release).await {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(error = %error.msg, "IDE update manifest download failed");
            return cached_manifest(false)
                .await
                .map(|cached| cached_response(cached, true))
                .unwrap_or_else(|| StatusCode::BAD_GATEWAY.into_response());
        }
    };
    if let Err(error) = validate_manifest(&manifest) {
        tracing::warn!(%error, "IDE update manifest failed validation");
        return StatusCode::BAD_GATEWAY.into_response();
    }
    ASSET_MAP_CACHE
        .write()
        .await
        .insert(tag.clone(), (Instant::now(), release_asset_map(&release)));
    rewrite_manifest_urls(state, &mut manifest, &tag);
    let body = match serde_json::to_vec(&manifest) {
        Ok(body) => body,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    *MANIFEST_CACHE.write().await = Some(CachedManifest {
        result: CachedUpdate::Manifest(body.clone()),
        fetched_at: Instant::now(),
    });
    manifest_response(body, false)
}

/// `GET /api/ide/releases` — the published changelog.
///
/// Separate from `admin_release_status`, which exists to drive the build console and
/// therefore includes drafts, run state and asset ids. This one is for people reading
/// "what changed": published releases only, newest first, with their notes.
///
/// Notes are returned as raw text and rendered as text by the console — never as HTML.
/// They are markdown written in GitHub's editor, and putting anyone's markdown through an
/// HTML renderer on a page that carries a session is not worth the formatting.
pub async fn releases(State(state): State<AppState>, _claims: Claims) -> Response {
    if state.cfg.ide_release_github_token.trim().is_empty() {
        return axum::Json(serde_json::json!({ "releases": [] })).into_response();
    }
    let Ok(base) = github_api_base(&state) else {
        return axum::Json(serde_json::json!({ "releases": [] })).into_response();
    };

    let response = match github_request(
        &state,
        state.update_http.get(format!("{base}/releases?per_page=30")),
    )
    .send()
    .await
    {
        Ok(r) if r.status().is_success() => r,
        _ => return axum::Json(serde_json::json!({ "releases": [] })).into_response(),
    };
    let Ok(list) = response.json::<Value>().await else {
        return axum::Json(serde_json::json!({ "releases": [] })).into_response();
    };

    let releases: Vec<Value> = list
        .as_array()
        .into_iter()
        .flatten()
        // Drafts are unpublished by definition — showing them would announce a version
        // nobody can install, and drafts are exactly where unfinished notes live.
        .filter(|r| !r.get("draft").and_then(Value::as_bool).unwrap_or(false))
        .map(|r| {
            let installers = r
                .get("assets")
                .and_then(Value::as_array)
                .map(|assets| {
                    assets
                        .iter()
                        .filter_map(|a| a.get("name").and_then(Value::as_str))
                        .filter(|n| {
                            let n = n.to_ascii_lowercase();
                            n.ends_with(".dmg") || n.ends_with(".exe") || n.ends_with(".msi")
                        })
                        .count()
                })
                .unwrap_or(0);

            serde_json::json!({
                "tag": r.get("tag_name").and_then(Value::as_str).unwrap_or_default(),
                "name": r.get("name").and_then(Value::as_str).unwrap_or_default(),
                "published_at": r.get("published_at"),
                "prerelease": r.get("prerelease").and_then(Value::as_bool).unwrap_or(false),
                // Bounded: a release note is prose, and an unbounded field from an
                // external service should not be relayed at whatever size it arrives.
                "notes": r
                    .get("body")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .chars()
                    .take(20_000)
                    .collect::<String>(),
                "installers": installers,
            })
        })
        .collect();

    (
        [(header::CACHE_CONTROL, "private, max-age=120")],
        axum::Json(serde_json::json!({ "releases": releases })),
    )
        .into_response()
}

/// `GET /api/ide/downloads` — the installers a person can download right now.
///
/// Separate from `/api/ide/update`, and deliberately so. That endpoint answers "is there
/// something newer than what you are running", which requires `latest.json` — the signed
/// manifest the auto-updater needs. A release built before that file was generated has no
/// manifest, so the update feed correctly answers "nothing", and the website concluded
/// there was nothing to download and offered a sign-up link instead.
///
/// But the installers are right there in the release. Auto-updating and installing are
/// different questions, and only the first one needs a manifest. This answers the second
/// by reading the release's own asset list, so the download buttons work with whatever is
/// actually published rather than waiting on a build pipeline.
///
/// Public: the URLs it returns are the proxy below, which is already public, and it
/// reveals nothing beyond the filenames of a release meant to be installed.
///
/// **Which release counts.** This used to ask GitHub for `releases/latest`, which sounds
/// like "the newest one" and is not: GitHub defines it as the newest release that is
/// neither a draft **nor a prerelease**. On a 0.x product where prereleases are the normal
/// way to ship, that means publishing a new build can leave both the download buttons and
/// the version line on the site frozen at an old number — with nothing anywhere saying
/// why. The site is not printing a hardcoded version; it prints this one, and this one had
/// a way of quietly refusing to move.
///
/// So the newest **published** release wins, prerelease or not. Drafts stay excluded, and
/// that is the deliberate signal for "not ready yet" — an unfinished build is a draft, not
/// a published release nobody is meant to find.
///
/// A release with no installer in it is skipped rather than treated as the answer.
/// Publishing a tag with the binaries still uploading used to take the downloads offline
/// entirely; now the previous working release stays up until the new one has something to
/// hand over.
pub async fn downloads(State(state): State<AppState>) -> Response {
    if state.cfg.ide_release_github_token.trim().is_empty() {
        return no_downloads();
    }
    let Ok(base) = github_api_base(&state) else {
        return no_downloads();
    };

    // 30 is far more than the number of releases anyone scrolls back through, and it is
    // one request. GitHub returns them newest-first.
    let response = match github_request(
        &state,
        state.update_http.get(format!("{base}/releases?per_page=30")),
    )
    .send()
    .await
    {
        Ok(r) if r.status().is_success() => r,
        _ => return no_downloads(),
    };
    let Ok(releases) = response.json::<Value>().await else {
        return no_downloads();
    };
    let Some(releases) = releases.as_array() else {
        return no_downloads();
    };

    let public = state.cfg.ide_update_public_base.trim_end_matches('/');

    for release in releases {
        // Drafts are invisible on purpose: they are the "still working on it" state, and a
        // draft's assets are not reachable by the public proxy below either.
        if release.get("draft").and_then(Value::as_bool).unwrap_or(false) {
            continue;
        }
        let tag = release
            .get("tag_name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if !safe_release_path_segment(&tag, 64) {
            continue;
        }

        let names: Vec<&str> = release
            .get("assets")
            .and_then(Value::as_array)
            .map(|assets| {
                assets
                    .iter()
                    .filter_map(|a| a.get("name").and_then(Value::as_str))
                    .collect()
            })
            .unwrap_or_default();

        // A universal .dmg is preferred where one exists — it covers both Intel and Apple
        // Silicon, which a per-architecture build does not. `.app.tar.gz` is the updater's
        // payload, not something a person should be handed, so it is never offered here.
        let pick = |wanted: &[&str]| -> Option<String> {
            for suffix in wanted {
                if let Some(name) = names
                    .iter()
                    .find(|n| n.to_ascii_lowercase().ends_with(suffix) && n.contains("universal"))
                {
                    return Some((*name).to_owned());
                }
                if let Some(name) = names.iter().find(|n| n.to_ascii_lowercase().ends_with(suffix))
                {
                    return Some((*name).to_owned());
                }
            }
            None
        };

        let mac = pick(&[".dmg"]);
        // .exe first: the NSIS installer is the one to hand a person, with the .msi kept as
        // a fallback for machines where policy blocks it.
        let windows = pick(&[".exe", ".msi"]);
        if mac.is_none() && windows.is_none() {
            // Nothing installable in this one — keep looking rather than reporting that
            // the product has no downloads.
            continue;
        }

        let url = |file: &str| format!("{public}/api/ide/update/download/{tag}/{file}");
        let body = serde_json::json!({
            "tag": tag,
            "version": tag.trim_start_matches(|c: char| !c.is_ascii_digit()),
            "prerelease": release.get("prerelease").and_then(Value::as_bool).unwrap_or(false),
            "published_at": release.get("published_at"),
            "mac": mac.as_deref().map(&url),
            "windows": windows.as_deref().map(&url),
        });

        return (
            // Five minutes. Long enough that this is not a GitHub API call per visitor,
            // short enough that a release published now is on the site within one coffee.
            [(header::CACHE_CONTROL, "public, max-age=300")],
            axum::Json(body),
        )
            .into_response();
    }

    no_downloads()
}

/// Shape-compatible with a successful answer, so the caller has one thing to parse.
fn no_downloads() -> Response {
    (
        [(header::CACHE_CONTROL, "public, max-age=60")],
        axum::Json(serde_json::json!({ "mac": null, "windows": null })),
    )
        .into_response()
}

/// 公开的更新包代理：私有仓库的 Release 资产匿名拿不到，网关带 token 流式转发。
/// 只暴露「已发布（非草稿）Release 的真实资产」，路径段严格白名单，token 绝不外泄
///（reqwest 跨域名重定向到 S3 时自动剥掉 Authorization）。
pub async fn download_asset(
    State(state): State<AppState>,
    Path((tag, file)): Path<(String, String)>,
) -> Response {
    if state.cfg.ide_release_github_token.trim().is_empty() {
        return StatusCode::NOT_FOUND.into_response();
    }
    if !safe_release_path_segment(&tag, 64) || !safe_release_path_segment(&file, 160) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let base = match github_api_base(&state) {
        Ok(base) => base,
        Err(_) => return StatusCode::BAD_GATEWAY.into_response(),
    };
    let mut asset_id = None;
    if let Some((fetched_at, map)) = ASSET_MAP_CACHE.read().await.get(&tag) {
        if fetched_at.elapsed() < ASSET_MAP_TTL {
            asset_id = map.get(&file).copied();
        }
    }
    if asset_id.is_none() {
        let release = match github_json(
            "读取 Release",
            github_request(
                &state,
                state.update_http.get(format!("{base}/releases/tags/{tag}")),
            ),
        )
        .await
        {
            Ok(value) => value,
            Err(_) => return StatusCode::NOT_FOUND.into_response(),
        };
        // 草稿 Release 不通过公开代理暴露（发布流转正之前不可下载）。
        if release.get("draft").and_then(Value::as_bool) == Some(true) {
            return StatusCode::NOT_FOUND.into_response();
        }
        let map = release_asset_map(&release);
        asset_id = map.get(&file).copied();
        ASSET_MAP_CACHE
            .write()
            .await
            .insert(tag.clone(), (Instant::now(), map));
    }
    let Some(asset_id) = asset_id else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let response = match github_asset_request(
        &state,
        state
            .update_http
            .get(format!("{base}/releases/assets/{asset_id}")),
    )
    .send()
    .await
    {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(%error, "IDE update asset fetch failed");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };
    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), "IDE update asset upstream rejected");
        return StatusCode::BAD_GATEWAY.into_response();
    }
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CACHE_CONTROL, "public, max-age=86400, immutable")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{file}\""),
        );
    if let Some(length) = response.content_length() {
        builder = builder.header(header::CONTENT_LENGTH, length);
    }
    builder
        .body(axum::body::Body::from_stream(response.bytes_stream()))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

#[cfg(test)]
mod tests {
    use super::{
        safe_release_path_segment, valid_github_repo, valid_workflow_name, validate_manifest,
        validate_publish_manifest, validate_release_tag,
    };
    use serde_json::json;

    #[test]
    fn accepts_signed_static_tauri_manifest() {
        let manifest = json!({
            "version": "0.3.16",
            "notes": "Bug fixes",
            "pub_date": "2026-07-26T12:00:00Z",
            "platforms": {
                "darwin-aarch64": {
                    "url": "https://example.test/Michael.IDE.app.tar.gz",
                    "signature": "signed"
                }
            }
        });
        assert_eq!(validate_manifest(&manifest), Ok(()));
    }

    #[test]
    fn rejects_unsigned_or_insecure_release_entries() {
        let unsigned = json!({
            "version": "0.3.16",
            "platforms": { "darwin-aarch64": { "url": "https://example.test/update", "signature": "" } }
        });
        assert!(validate_manifest(&unsigned).is_err());

        let insecure = json!({
            "version": "0.3.16",
            "platforms": { "windows-x86_64": { "url": "http://example.test/update", "signature": "signed" } }
        });
        assert!(validate_manifest(&insecure).is_err());
    }

    #[test]
    fn release_control_accepts_only_exact_stable_tags_and_safe_repo_paths() {
        assert_eq!(validate_release_tag("v0.3.16"), Ok("0.3.16"));
        for invalid in ["0.3.16", "v0.3", "v0.3.16-beta", "main", "v0.3.16/extra"] {
            assert!(validate_release_tag(invalid).is_err(), "accepted {invalid}");
        }
        assert!(valid_github_repo("fendoushaonian/Devin-Desktop"));
        assert!(!valid_github_repo("https://github.com/owner/repo"));
        assert!(!valid_github_repo("owner/repo/extra"));
        assert!(valid_workflow_name("ide-package.yml"));
        assert!(!valid_workflow_name("../ide-package.yml"));
    }

    #[test]
    fn publish_gate_requires_matching_signed_mac_and_windows_manifest() {
        let complete = json!({
            "version": "0.3.16",
            "platforms": {
                "darwin-aarch64": {
                    "url": "https://example.test/Michael.IDE.app.tar.gz",
                    "signature": "mac-signature"
                },
                "windows-x86_64": {
                    "url": "https://example.test/Michael.IDE.exe",
                    "signature": "windows-signature"
                }
            }
        });
        assert_eq!(validate_publish_manifest(&complete, "v0.3.16"), Ok(()));
        assert!(validate_publish_manifest(&complete, "v0.3.17").is_err());

        let missing_windows = json!({
            "version": "0.3.16",
            "platforms": {
                "darwin-aarch64": {
                    "url": "https://example.test/Michael.IDE.app.tar.gz",
                    "signature": "mac-signature"
                }
            }
        });
        assert!(validate_publish_manifest(&missing_windows, "v0.3.16").is_err());
    }

    /// 站点上那行版本号来自这里，所以"哪个 Release 算数"就是站点会显示哪个版本。
    ///
    /// 这一组断言盯的是选取规则本身：草稿排除、预发布**不**排除、没有安装包的那一版跳过。
    /// 之前用的是 GitHub 的 `releases/latest` —— 名字像"最新一个"，实际含义是"最新的
    /// 非草稿**且非预发布**"。0.x 阶段发预发布是常态，于是发了新版站点却纹丝不动，
    /// 看起来就像版本号是写死的。
    #[test]
    fn the_release_picker_rules_are_the_ones_documented() {
        let src = include_str!("update.rs");
        let f = src
            .split("pub async fn downloads(")
            .nth(1)
            .expect("downloads() 还在吗");
        let body = &f[..f.find("\n/// Shape-compatible").unwrap_or(f.len())];

        assert!(
            body.contains("releases?per_page=30"),
            "必须读整个列表；releases/latest 会把预发布一起漏掉",
        );
        assert!(
            !body.contains("releases/latest"),
            "回到 releases/latest 就等于把预发布重新拒之门外",
        );
        assert!(
            body.contains(r#"release.get("draft")"#) && body.contains("continue"),
            "草稿必须跳过：草稿是'还没做完'的信号，它的资产公开代理也拿不到",
        );
        assert!(
            body.contains("mac.is_none() && windows.is_none()"),
            "没有安装包的那一版要跳过，否则发一个还在传资产的 tag 就把下载整个打没了",
        );
        assert!(
            body.contains("safe_release_path_segment(&tag, 64)"),
            "tag 会拼进公开下载路径，必须先过白名单",
        );
    }

    /// 资产名里的空格到不了这里——GitHub 上传时把它换成点。实测：
    /// `Mr. Day One.app.tar.gz` 上传后叫 `Mr.Day.One.app.tar.gz`。
    ///
    /// 记下来是因为产品名带空格（Mr. Day One，旧名 Michael IDE 也带），很容易据此
    /// 推断"白名单会把资产名拒掉、导致地址重写被跳过、更新静默失效"，然后去放宽这条
    /// 校验。那是为一个不存在的问题削弱安全边界——这条测试就是拦这个的。
    #[test]
    fn release_asset_names_arrive_without_spaces_so_the_whitelist_stays_tight() {
        // GitHub 规范化之后的真实形状
        assert!(safe_release_path_segment("Mr.Day.One.app.tar.gz", 160));
        assert!(safe_release_path_segment("Mr.Day.One_0.3.89_aarch64.dmg", 160));
        assert!(safe_release_path_segment("v0.3.89", 64));

        // 带空格的原始名不该被接受，也不需要被接受
        assert!(!safe_release_path_segment("Mr. Day One.app.tar.gz", 160));

        // 真正危险的形状
        assert!(!safe_release_path_segment("../etc/passwd", 160));
        assert!(!safe_release_path_segment("a/b.tar.gz", 160));
        assert!(!safe_release_path_segment("a\\b.tar.gz", 160));
        assert!(!safe_release_path_segment("..", 160));
        assert!(!safe_release_path_segment(".hidden", 160));
        assert!(!safe_release_path_segment("", 160));
        assert!(!safe_release_path_segment(&"a".repeat(161), 160));
    }
}
