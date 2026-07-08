use crate::error::*;

#[test]
fn test_error_from_string() {
    let error: Error = "测试错误".to_string().into();
    assert!(matches!(error, Error::Browser(_)));
    assert_eq!(error.to_string(), "浏览器错误: 测试错误");
}

#[test]
fn test_error_display() {
    let errors = vec![
        (Error::Browser("连接失败".into()), "浏览器错误: 连接失败"),
        (Error::System("权限不足".into()), "系统自动化错误: 权限不足"),
        (Error::ElementNotFound("#btn".into()), "元素未找到: #btn"),
        (Error::Timeout("5秒".into()), "超时: 5秒"),
        (Error::UnsupportedPlatform("Linux".into()), "不支持的平台: Linux"),
    ];

    for (error, expected) in errors {
        assert_eq!(error.to_string(), expected);
    }
}

#[test]
fn test_result_type() {
    fn returns_ok() -> Result<i32> {
        Ok(42)
    }

    fn returns_err() -> Result<i32> {
        Err(Error::System("测试".into()))
    }

    assert!(returns_ok().is_ok());
    assert_eq!(returns_ok().unwrap(), 42);
    assert!(returns_err().is_err());
}

#[test]
fn test_io_error_conversion() {
    use std::io;
    let io_err = io::Error::new(io::ErrorKind::NotFound, "文件未找到");
    let error: Error = io_err.into();
    assert!(matches!(error, Error::Io(_)));
}

#[test]
fn test_json_error_conversion() {
    let json_err = serde_json::from_str::<serde_json::Value>("{invalid").unwrap_err();
    let error: Error = json_err.into();
    assert!(matches!(error, Error::Serialization(_)));
}
