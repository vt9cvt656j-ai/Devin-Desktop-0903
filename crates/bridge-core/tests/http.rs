use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::Engine;
use bridge_core::server::{router, AppState};
use bridge_core::ScopedFs;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

const TOKEN: &str = "test-token-1234567890";

fn app() -> (tempfile::TempDir, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("hello.txt"), b"hello world").unwrap();
    std::fs::create_dir_all(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("sub/notes.md"), b"# notes\nalpha beta").unwrap();
    let state = AppState {
        fs: ScopedFs::new(dir.path()).unwrap(),
        token: Arc::new(TOKEN.to_string()),
        allow_write: true,
        rate_limiter: None,
    };
    (dir, router(state))
}

fn readonly_app() -> (tempfile::TempDir, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("hello.txt"), b"hello world").unwrap();
    let state = AppState {
        fs: ScopedFs::new(dir.path()).unwrap(),
        token: Arc::new(TOKEN.to_string()),
        allow_write: false,
        rate_limiter: None,
    };
    (dir, router(state))
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn authed(method: &str, uri: &str, body: Option<Value>) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {TOKEN}"));
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    match body {
        Some(b) => builder.body(Body::from(b.to_string())).unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

#[tokio::test]
async fn rejects_requests_without_token() {
    let (_d, app) = app();
    let req = Request::builder()
        .uri("/api/health")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rejects_wrong_token() {
    let (_d, app) = app();
    let req = Request::builder()
        .uri("/api/health")
        .header("authorization", "Bearer nope")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn health_ok_with_token() {
    let (_d, app) = app();
    let resp = app
        .oneshot(authed("GET", "/api/health", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
}

#[tokio::test]
async fn lists_and_reads() {
    let (_d, app) = app();
    let resp = app
        .clone()
        .oneshot(authed("GET", "/api/list?path=", None))
        .await
        .unwrap();
    let json = body_json(resp).await;
    assert!(json["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["name"] == "hello.txt"));

    let resp = app
        .oneshot(authed("GET", "/api/read?path=hello.txt", None))
        .await
        .unwrap();
    let json = body_json(resp).await;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(json["content_base64"].as_str().unwrap())
        .unwrap();
    assert_eq!(decoded, b"hello world");
}

#[tokio::test]
async fn write_then_read_roundtrip() {
    let (_d, app) = app();
    let content = base64::engine::general_purpose::STANDARD.encode(b"new content");
    let resp = app
        .clone()
        .oneshot(authed(
            "POST",
            "/api/write",
            Some(serde_json::json!({ "path": "a/b.txt", "content_base64": content })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .oneshot(authed("GET", "/api/read?path=a/b.txt", None))
        .await
        .unwrap();
    let json = body_json(resp).await;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(json["content_base64"].as_str().unwrap())
        .unwrap();
    assert_eq!(decoded, b"new content");
}

#[tokio::test]
async fn read_outside_root_is_forbidden() {
    let (_d, app) = app();
    let resp = app
        .oneshot(authed("GET", "/api/read?path=../../etc/passwd", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn readonly_rejects_write() {
    let (_d, app) = readonly_app();
    let content = base64::engine::general_purpose::STANDARD.encode(b"test");
    let resp = app
        .oneshot(authed(
            "POST",
            "/api/write",
            Some(serde_json::json!({ "path": "new.txt", "content_base64": content })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn readonly_rejects_mkdir() {
    let (_d, app) = readonly_app();
    let resp = app
        .oneshot(authed(
            "POST",
            "/api/mkdir",
            Some(serde_json::json!({ "path": "newdir" })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn readonly_rejects_delete() {
    let (_d, app) = readonly_app();
    let resp = app
        .oneshot(authed(
            "POST",
            "/api/delete",
            Some(serde_json::json!({ "path": "hello.txt" })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert!(
        _d.path().join("hello.txt").exists(),
        "file must not be deleted in read-only mode"
    );
}

#[tokio::test]
async fn readonly_allows_read() {
    let (_d, app) = readonly_app();
    let resp = app
        .oneshot(authed("GET", "/api/read?path=hello.txt", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn mkdir_then_list() {
    let (_d, app) = app();
    let resp = app
        .clone()
        .oneshot(authed(
            "POST",
            "/api/mkdir",
            Some(serde_json::json!({ "path": "brand-new" })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .oneshot(authed("GET", "/api/list?path=", None))
        .await
        .unwrap();
    let json = body_json(resp).await;
    assert!(json["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["name"] == "brand-new" && e["is_dir"] == true));
}

#[tokio::test]
async fn delete_file() {
    let (_d, app) = app();
    let resp = app
        .clone()
        .oneshot(authed(
            "POST",
            "/api/delete",
            Some(serde_json::json!({ "path": "hello.txt" })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(!_d.path().join("hello.txt").exists());
}

#[tokio::test]
async fn search_by_name() {
    let (_d, app) = app();
    let resp = app
        .oneshot(authed("GET", "/api/search?q=notes&path=", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["hits"]
        .as_array()
        .unwrap()
        .iter()
        .any(|h| h["path"].as_str().unwrap().contains("notes")));
}

#[tokio::test]
async fn search_by_content() {
    let (_d, app) = app();
    let resp = app
        .oneshot(authed("GET", "/api/search?q=alpha&path=&content=true", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let hits = json["hits"].as_array().unwrap();
    assert!(!hits.is_empty());
    assert!(hits.iter().any(|h| h["line"].as_u64().is_some()));
}

#[tokio::test]
async fn read_nonexistent_file_returns_404() {
    let (_d, app) = app();
    let resp = app
        .oneshot(authed("GET", "/api/read?path=does-not-exist.txt", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn health_reports_mode() {
    let (_d, app) = readonly_app();
    let resp = app
        .oneshot(authed("GET", "/api/health", None))
        .await
        .unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["allow_write"], false);
}
