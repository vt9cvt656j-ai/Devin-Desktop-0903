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
    let response = github_request(
        state,
        state
            .update_http
            .get(format!("{base}/releases/assets/{asset_id}")),
    )
    .header("Accept", "application/octet-stream")
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

#[cfg(test)]
mod tests {
    use super::{
        valid_github_repo, valid_workflow_name, validate_manifest, validate_publish_manifest,
        validate_release_tag,
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
}
