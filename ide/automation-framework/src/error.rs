use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("浏览器错误: {0}")]
    Browser(String),

    #[error("系统自动化错误: {0}")]
    System(String),

    #[error("元素未找到: {0}")]
    ElementNotFound(String),

    #[error("超时: {0}")]
    Timeout(String),

    #[error("不支持的平台: {0}")]
    UnsupportedPlatform(String),

    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[cfg(feature = "browser")]
    #[error("Chromium 错误: {0}")]
    Chromium(#[from] chromiumoxide::error::CdpError),

    #[error("其他错误: {0}")]
    Other(#[from] anyhow::Error),
}

// 添加 From<String> 实现以支持 BrowserConfig 错误
impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Browser(s)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
