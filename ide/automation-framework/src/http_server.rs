//! HTTP 服务器 - 为 AI agent 提供 RESTful API
//!
//! 启动本地 HTTP 服务，AI 通过标准 HTTP 请求调用自动化能力

use crate::task::{TaskExecutor, TaskResult};
use crate::error::Result;
use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tower_http::cors::CorsLayer;

#[cfg(feature = "system")]
#[cfg(feature = "system")]
use crate::system::SystemAutomation;
use crate::types::{MouseButton, CoordinateMode};

/// 统一响应格式
#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

// ========== 请求类型定义 ==========

#[derive(Debug, Deserialize)]
struct MouseMoveRequest {
    x: i32,
    y: i32,
}

#[derive(Debug, Deserialize)]
struct MouseClickRequest {
    button: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KeyboardTypeRequest {
    text: String,
}

#[derive(Debug, Deserialize)]
struct WebSearchRequest {
    query: String,
    engine: String,
}

#[derive(Debug, Deserialize)]
struct ExtractWebContentRequest {
    url: String,
    selectors: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAndTypeRequest {
    app_name: String,
    content: String,
}

/// HTTP 服务器
pub struct HttpServer {
    port: u16,
}

impl HttpServer {
    /// 创建新的 HTTP 服务器
    pub fn new(port: u16) -> Result<Self> {
        Ok(Self { port })
    }

    /// 启动服务器
    pub async fn start(self) -> Result<()> {
        let app = Router::new()
            .route("/health", get(health_check))
            .route("/api/mouse/move", post(mouse_move))
            .route("/api/mouse/click", post(mouse_click))
            .route("/api/keyboard/type", post(keyboard_type))
            .route("/api/task/web_search", post(task_web_search))
            .route("/api/task/extract_content", post(task_extract_content))
            .route("/api/task/open_and_type", post(task_open_and_type))
            .layer(CorsLayer::permissive());

        let addr = format!("127.0.0.1:{}", self.port);
        println!("🚀 HTTP 服务器启动: http://{}", addr);
        println!("📖 API 文档: http://{}/health", addr);
        println!();

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

// ========== 路由处理器 ==========

async fn health_check() -> impl IntoResponse {
    Json(ApiResponse::success(serde_json::json!({
        "status": "ok",
        "message": "Rust Automation Framework HTTP Server",
        "version": "0.1.0",
        "endpoints": {
            "mouse": ["/api/mouse/move", "/api/mouse/click"],
            "keyboard": ["/api/keyboard/type"],
            "task": ["/api/task/web_search", "/api/task/extract_content", "/api/task/open_and_type"]
        }
    })))
}

#[cfg(feature = "system")]
async fn mouse_move(Json(req): Json<MouseMoveRequest>) -> Response {
    tokio::task::spawn_blocking(move || {
        SystemAutomation::new()
            .and_then(|mut system| system.move_mouse(req.x, req.y))
    })
    .await
    .map(|result| match result {
        Ok(_) => Json(ApiResponse::success("鼠标移动成功")).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    })
    .unwrap_or_else(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("任务执行失败: {}", e))),
        )
            .into_response()
    })
}

#[cfg(not(feature = "system"))]
async fn mouse_move(Json(_req): Json<MouseMoveRequest>) -> Response {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::<()>::error("system feature 未启用")),
    )
        .into_response()
}

#[cfg(feature = "system")]
async fn mouse_click(Json(req): Json<MouseClickRequest>) -> Response {
    tokio::task::spawn_blocking(move || {
        let button = match req.button.as_deref() {
            Some("left") | None => MouseButton::Left,
            Some("right") => MouseButton::Right,
            Some("middle") => MouseButton::Middle,
            _ => MouseButton::Left,
        };
        SystemAutomation::new()
            .and_then(|mut system| system.click(button))
    })
    .await
    .map(|result| match result {
        Ok(_) => Json(ApiResponse::success("鼠标点击成功")).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    })
    .unwrap_or_else(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("任务执行失败: {}", e))),
        )
            .into_response()
    })
}

#[cfg(not(feature = "system"))]
async fn mouse_click(Json(_req): Json<MouseClickRequest>) -> Response {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::<()>::error("system feature 未启用")),
    )
        .into_response()
}

#[cfg(feature = "system")]
async fn keyboard_type(Json(req): Json<KeyboardTypeRequest>) -> Response {
    let text = req.text.clone();
    tokio::task::spawn_blocking(move || {
        SystemAutomation::new()
            .and_then(|mut system| system.type_text(&text))
    })
    .await
    .map(|result| match result {
        Ok(_) => Json(ApiResponse::success("键盘输入成功")).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    })
    .unwrap_or_else(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("任务执行失败: {}", e))),
        )
            .into_response()
    })
}

#[cfg(not(feature = "system"))]
async fn keyboard_type(Json(_req): Json<KeyboardTypeRequest>) -> Response {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::<()>::error("system feature 未启用")),
    )
        .into_response()
}

async fn task_web_search(Json(req): Json<WebSearchRequest>) -> Response {
    let query = req.query.clone();
    let engine = req.engine.clone();
    
    tokio::task::spawn_blocking(move || {
        TaskExecutor::new()
            .and_then(|mut executor| {
                executor.init()?;
                executor.web_search(&query, &engine)
            })
    })
    .await
    .map(|result| match result {
        Ok(task_result) => Json(ApiResponse::success(task_result)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    })
    .unwrap_or_else(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("任务执行失败: {}", e))),
        )
            .into_response()
    })
}

async fn task_extract_content(Json(req): Json<ExtractWebContentRequest>) -> Response {
    let url = req.url.clone();
    let selectors = req.selectors.clone();
    
    tokio::task::spawn_blocking(move || {
        let selectors_ref: Vec<&str> = selectors.iter().map(|s| s.as_str()).collect();
        TaskExecutor::new()
            .and_then(|mut executor| {
                executor.init()?;
                executor.extract_web_content(&url, selectors_ref)
            })
    })
    .await
    .map(|result| match result {
        Ok(task_result) => Json(ApiResponse::success(task_result)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    })
    .unwrap_or_else(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("任务执行失败: {}", e))),
        )
            .into_response()
    })
}

async fn task_open_and_type(Json(req): Json<OpenAndTypeRequest>) -> Response {
    let app_name = req.app_name.clone();
    let content = req.content.clone();
    
    tokio::task::spawn_blocking(move || {
        TaskExecutor::new()
            .and_then(|mut executor| {
                executor.init()?;
                executor.open_and_type(&app_name, &content)
            })
    })
    .await
    .map(|result| match result {
        Ok(task_result) => Json(ApiResponse::success(task_result)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    })
    .unwrap_or_else(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("任务执行失败: {}", e))),
        )
            .into_response()
    })
}
