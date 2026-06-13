use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{header, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};

use crate::config::BridgeConfig;
use crate::error::BridgeError;
use crate::fs::ScopedFs;

/// Shared, cloneable application state.
#[derive(Clone)]
pub struct AppState {
    pub fs: ScopedFs,
    pub token: Arc<String>,
    pub allow_write: bool,
}

impl IntoResponse for BridgeError {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = Json(json!({ "error": { "code": self.code(), "message": self.to_string() } }));
        (status, body).into_response()
    }
}

/// Build the router for the bridge API.
pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    Router::new()
        .route("/api/health", get(health))
        .route("/api/list", get(list))
        .route("/api/read", get(read))
        .route("/api/search", get(search))
        .route("/api/write", post(write))
        .route("/api/mkdir", post(mkdir))
        .route("/api/delete", post(delete))
        .layer(middleware::from_fn_with_state(state.clone(), auth))
        .layer(cors)
        .with_state(state)
}

/// Bind and serve until the process is stopped. Returns the bound address via
/// the callback before entering the serve loop.
pub async fn serve(config: BridgeConfig, on_bound: impl FnOnce(SocketAddr)) -> anyhow::Result<()> {
    serve_with_shutdown(config, on_bound, std::future::pending::<()>()).await
}

/// Like [`serve`], but stops gracefully once `shutdown` resolves.
pub async fn serve_with_shutdown(
    config: BridgeConfig,
    on_bound: impl FnOnce(SocketAddr),
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    let fs = ScopedFs::new(&config.root)?;
    let state = AppState {
        fs,
        token: Arc::new(config.token.clone()),
        allow_write: config.allow_write,
    };
    let listener = tokio::net::TcpListener::bind((config.host, config.port)).await?;
    let addr = listener.local_addr()?;
    on_bound(addr);
    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

async fn auth(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let provided = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    match provided {
        Some(tok) if constant_time_eq(tok.as_bytes(), state.token.as_bytes()) => next.run(req).await,
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": { "code": "unauthorized", "message": "missing or invalid bearer token" } })),
        )
            .into_response(),
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn require_write(state: &AppState) -> Result<(), BridgeError> {
    if state.allow_write {
        Ok(())
    } else {
        Err(BridgeError::BadRequest(
            "bridge is running in read-only mode".into(),
        ))
    }
}

#[derive(Deserialize)]
struct PathQuery {
    #[serde(default)]
    path: String,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    content: bool,
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct WriteBody {
    path: String,
    /// Base64-encoded file contents.
    content_base64: String,
}

#[derive(Deserialize)]
struct PathBody {
    path: String,
}

#[derive(Serialize)]
struct ReadResponse {
    path: String,
    encoding: &'static str,
    size: usize,
    content_base64: String,
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "ok": true,
        "version": env!("CARGO_PKG_VERSION"),
        "root": state.fs.root().to_string_lossy(),
        "allow_write": state.allow_write,
    }))
}

async fn list(
    State(state): State<AppState>,
    Query(q): Query<PathQuery>,
) -> Result<Response, BridgeError> {
    let entries = state.fs.list_dir(&q.path)?;
    Ok(Json(json!({ "path": q.path, "entries": entries })).into_response())
}

async fn read(
    State(state): State<AppState>,
    Query(q): Query<PathQuery>,
) -> Result<Response, BridgeError> {
    let bytes = state.fs.read_file(&q.path)?;
    let resp = ReadResponse {
        path: q.path,
        encoding: "base64",
        size: bytes.len(),
        content_base64: base64::engine::general_purpose::STANDARD.encode(&bytes),
    };
    Ok(Json(resp).into_response())
}

async fn search(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Response, BridgeError> {
    let limit = q.limit.unwrap_or(200).min(2000);
    let hits = state.fs.search(&q.path, &q.q, q.content, limit)?;
    Ok(Json(json!({ "query": q.q, "hits": hits })).into_response())
}

async fn write(
    State(state): State<AppState>,
    Json(body): Json<WriteBody>,
) -> Result<Response, BridgeError> {
    require_write(&state)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(body.content_base64.as_bytes())
        .map_err(|e| BridgeError::BadRequest(format!("invalid base64: {e}")))?;
    state.fs.write_file(&body.path, &bytes)?;
    Ok(Json(json!({ "ok": true, "path": body.path, "size": bytes.len() })).into_response())
}

async fn mkdir(
    State(state): State<AppState>,
    Json(body): Json<PathBody>,
) -> Result<Response, BridgeError> {
    require_write(&state)?;
    state.fs.mkdir(&body.path)?;
    Ok(Json(json!({ "ok": true, "path": body.path })).into_response())
}

async fn delete(
    State(state): State<AppState>,
    Json(body): Json<PathBody>,
) -> Result<Response, BridgeError> {
    require_write(&state)?;
    state.fs.delete(&body.path)?;
    Ok(Json(json!({ "ok": true, "path": body.path })).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_works() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }
}
