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
        (self.status, Json(json!({ "error": self.msg }))).into_response()
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
