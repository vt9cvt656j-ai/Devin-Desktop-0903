//! 单元测试模块

#[cfg(test)]
mod error_tests;

#[cfg(test)]
mod types_tests;

#[cfg(all(test, feature = "system"))]
mod system_tests;

#[cfg(all(test, feature = "browser"))]
mod browser_tests;
