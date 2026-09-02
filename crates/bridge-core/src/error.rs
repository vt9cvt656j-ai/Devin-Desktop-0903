use std::path::PathBuf;

/// Errors produced by the bridge core.
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("path escapes the allowed root: {0}")]
    PathEscapesRoot(String),

    #[error("path not found: {0}")]
    NotFound(String),

    #[error("not a directory: {0}")]
    NotADirectory(String),

    #[error("not a file: {0}")]
    NotAFile(String),

    #[error("the allowed root does not exist or is not a directory: {0}")]
    InvalidRoot(PathBuf),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid request: {0}")]
    BadRequest(String),
}

impl BridgeError {
    /// HTTP status code that best represents this error.
    pub fn status_code(&self) -> u16 {
        match self {
            BridgeError::PathEscapesRoot(_) => 403,
            BridgeError::NotFound(_) => 404,
            BridgeError::NotADirectory(_)
            | BridgeError::NotAFile(_)
            | BridgeError::BadRequest(_) => 400,
            BridgeError::InvalidRoot(_) | BridgeError::Io(_) => 500,
        }
    }

    /// Short machine-readable error code.
    pub fn code(&self) -> &'static str {
        match self {
            BridgeError::PathEscapesRoot(_) => "path_escapes_root",
            BridgeError::NotFound(_) => "not_found",
            BridgeError::NotADirectory(_) => "not_a_directory",
            BridgeError::NotAFile(_) => "not_a_file",
            BridgeError::InvalidRoot(_) => "invalid_root",
            BridgeError::Io(_) => "io_error",
            BridgeError::BadRequest(_) => "bad_request",
        }
    }
}

pub type Result<T> = std::result::Result<T, BridgeError>;
