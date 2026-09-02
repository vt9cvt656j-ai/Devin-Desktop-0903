use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// Uniform API error → JSON `{ "error": "..." }` with a status code.
pub struct AppError {
    pub status: StatusCode,
    pub msg: String,
}

impl AppError {
    pub fn bad(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            msg: msg.into(),
        }
    }
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            msg: msg.into(),
        }
    }
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            msg: msg.into(),
        }
    }
    /// 「没有这个东西」。
    ///
    /// 和 `forbidden` 的区别值得写下来：403 说的是「有，但你不能看」，404 说的是「没有」。
    /// 对**未发布**的内容要用这一个 —— 403 等于承认草稿存在，那本身就是信息。
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            msg: msg.into(),
        }
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            msg: msg.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if matches!(
            self.status,
            StatusCode::BAD_GATEWAY | StatusCode::SERVICE_UNAVAILABLE | StatusCode::GATEWAY_TIMEOUT
        ) {
            tracing::warn!(status = self.status.as_u16(), "{}", self.msg);
        } else if self.status.is_server_error() {
            tracing::error!("{}", self.msg);
        }
        // Only 500 is redacted. The `into_internal!` impls below fill 500 messages with
        // raw `sqlx`/`redis` error text — table names, column names, constraint names,
        // SQL fragments — which was being returned verbatim to whoever made the request,
        // including unauthenticated callers on /api/auth/*. That belongs in the log.
        //
        // Scoped to 500 deliberately, NOT to is_server_error(): 502/503/504 carry the
        // hand-written upstream messages ("换个模型或稍后再试") that are meant for the user,
        // and blanking those would be a real UX regression.
        let body = if self.status == StatusCode::INTERNAL_SERVER_ERROR {
            "服务器内部错误，请稍后重试".to_string()
        } else {
            self.msg
        };
        (self.status, Json(json!({ "error": body }))).into_response()
    }
}

pub type ApiResult<T> = Result<T, AppError>;

// Explicit `From` impls (no blanket impl, to avoid coherence headaches) so `?`
// works on the error types we actually touch — all map to a 500.
macro_rules! into_internal {
    ($t:ty) => {
        impl From<$t> for AppError {
            fn from(e: $t) -> Self {
                AppError::internal(e.to_string())
            }
        }
    };
}
into_internal!(sqlx::Error);
into_internal!(redis::RedisError);
into_internal!(bcrypt::BcryptError);
into_internal!(jsonwebtoken::errors::Error);
into_internal!(lettre::error::Error);
into_internal!(lettre::transport::smtp::Error);
into_internal!(lettre::address::AddressError);
into_internal!(anyhow::Error);
