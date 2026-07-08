//! 浏览器自动化模块
//!
//! 基于 Chrome DevTools Protocol 实现跨平台浏览器自动化

use crate::error::{Error, Result};
use crate::types::{BrowserAction, ExecutionResult};
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::Page;
use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// 浏览器自动化控制器
pub struct BrowserAutomation {
    browser: Browser,
    pub current_page: Option<Arc<Page>>,
}

impl BrowserAutomation {
    /// 创建新的浏览器实例（默认无头模式）
    pub async fn new() -> Result<Self> {
        let temp_dir = std::env::temp_dir().join(format!("rust_automation_browser_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir)?;
        
        let config = BrowserConfig::builder()
            .chrome_executable("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-infobars")
            .arg("--disable-extensions")
            .arg("--disable-dev-shm-usage")
            .arg("--disable-software-rasterizer")
            .arg("--disable-background-networking")
            .arg("--disable-sync")
            .arg("--metrics-recording-only")
            .arg("--disable-default-apps")
            .arg("--mute-audio")
            .arg("--no-first-run")
            .arg("--disable-gpu")
            .arg("--disable-features=site-per-process")
            .arg(format!("--user-data-dir={}", temp_dir.display()))
            .build()?;
        Self::with_config(config).await
    }

    /// 创建有头（可见）浏览器实例
    pub async fn new_headed() -> Result<Self> {
        let temp_dir = std::env::temp_dir().join(format!("rust_automation_browser_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir)?;
        
        let config = BrowserConfig::builder()
            .chrome_executable("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
            .with_head()
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-infobars")
            .arg("--disable-extensions")
            .arg("--disable-dev-shm-usage")
            .arg("--disable-background-networking")
            .arg("--disable-sync")
            .arg("--metrics-recording-only")
            .arg("--disable-default-apps")
            .arg("--mute-audio")
            .arg("--disable-features=site-per-process")
            .arg(format!("--user-data-dir={}", temp_dir.display()))
            .build()?;
        Self::with_config(config).await
    }

    /// 使用自定义配置创建
    pub async fn with_config(config: BrowserConfig) -> Result<Self> {
        info!("启动浏览器实例...");
        let (browser, mut handler) = Browser::launch(config).await?;

        // 在后台任务中持续处理浏览器事件，保持连接
        let _handle = tokio::task::spawn(async move {
            loop {
                if let Some(event) = handler.next().await {
                    if let Err(e) = event {
                        warn!("浏览器事件错误: {:?}", e);
                        // 不要立即 break，继续处理
                    }
                } else {
                    debug!("浏览器事件流结束");
                    break;
                }
            }
        });

        // 等待浏览器完全启动
        tokio::time::sleep(Duration::from_millis(500)).await;

        Ok(Self {
            browser,
            current_page: None,
        })
    }

    /// 检查浏览器连接是否活跃
    pub async fn is_connected(&self) -> bool {
        if let Some(page) = &self.current_page {
            page.evaluate("1+1".to_string()).await.is_ok()
        } else {
            true // 没有页面时认为浏览器本身是连接的
        }
    }

    /// 导航到指定 URL（带重试机制）
    pub async fn navigate(&mut self, url: &str) -> Result<()> {
        info!("导航到: {}", url);
        
        let max_retries = 3;
        let mut last_error = None;
        
        for attempt in 0..max_retries {
            if attempt > 0 {
                warn!("导航重试 {}/{}", attempt, max_retries);
                tokio::time::sleep(Duration::from_millis(500)).await;
                
                if !self.is_connected().await {
                    return Err(Error::Other(anyhow::anyhow!("浏览器连接已断开")));
                }
            }
            
            if let Some(page) = &self.current_page {
                match page.goto(url).await {
                    Ok(_) => {
                        tokio::time::sleep(Duration::from_millis(1000)).await;
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("导航失败 (尝试 {}): {:?}", attempt + 1, e);
                        last_error = Some(e);
                    }
                }
            } else {
                match self.browser.new_page(url).await {
                    Ok(page) => {
                        self.current_page = Some(Arc::new(page));
                        tokio::time::sleep(Duration::from_millis(1000)).await;
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("创建页面失败 (尝试 {}): {:?}", attempt + 1, e);
                        last_error = Some(e);
                    }
                }
            }
        }
        
        Err(Error::Chromium(last_error.unwrap()))
    }

    /// 获取当前页面（如果没有则创建新页面）
    async fn get_or_create_page(&mut self) -> Result<Arc<Page>> {
        if let Some(page) = &self.current_page {
            return Ok(Arc::clone(page));
        }

        info!("创建新页面");
        let page = self.browser.new_page("about:blank").await?;
        let page = Arc::new(page);
        self.current_page = Some(Arc::clone(&page));
        Ok(page)
    }

    /// 点击元素（CSS 选择器，带重试）
    pub async fn click_element(&mut self, selector: &str) -> Result<()> {
        debug!("点击元素: {}", selector);
        
        let max_retries = 3;
        let mut last_error = None;
        
        for attempt in 0..max_retries {
            if attempt > 0 {
                warn!("点击重试 {}/{}", attempt, max_retries);
                tokio::time::sleep(Duration::from_millis(300)).await;
                
                if !self.is_connected().await {
                    return Err(Error::Other(anyhow::anyhow!("浏览器连接已断开")));
                }
            }
            
            let page = match self.get_or_create_page().await {
                Ok(p) => p,
                Err(e) => {
                    last_error = Some(Error::Other(anyhow::anyhow!("获取页面失败: {}", e)));
                    continue;
                }
            };
            
            match page.find_element(selector).await {
                Ok(element) => {
                    match element.click().await {
                        Ok(_) => return Ok(()),
                        Err(e) => {
                            warn!("点击失败 (尝试 {}): {:?}", attempt + 1, e);
                            last_error = Some(Error::Chromium(e));
                        }
                    }
                }
                Err(e) => {
                    warn!("查找元素失败 (尝试 {}): {:?}", attempt + 1, e);
                    last_error = Some(Error::ElementNotFound(format!("{}: {}", selector, e)));
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| Error::ElementNotFound(format!("元素 {} 未找到", selector))))
    }

    /// 在元素中输入文本（带重试）
    pub async fn type_text(&mut self, selector: &str, text: &str) -> Result<()> {
        debug!("在 {} 中输入文本", selector);
        
        let max_retries = 3;
        let mut last_error = None;
        
        for attempt in 0..max_retries {
            if attempt > 0 {
                warn!("输入文本重试 {}/{}", attempt, max_retries);
                tokio::time::sleep(Duration::from_millis(300)).await;
                
                if !self.is_connected().await {
                    return Err(Error::Other(anyhow::anyhow!("浏览器连接已断开")));
                }
            }
            
            let page = match self.get_or_create_page().await {
                Ok(p) => p,
                Err(e) => {
                    last_error = Some(Error::Other(anyhow::anyhow!("获取页面失败: {}", e)));
                    continue;
                }
            };
            
            match page.find_element(selector).await {
                Ok(element) => {
                    if let Err(e) = element.click().await {
                        warn!("点击前置失败 (尝试 {}): {:?}", attempt + 1, e);
                        last_error = Some(Error::Chromium(e));
                        continue;
                    }
                    
                    match element.type_str(text).await {
                        Ok(_) => return Ok(()),
                        Err(e) => {
                            warn!("输入文本失败 (尝试 {}): {:?}", attempt + 1, e);
                            last_error = Some(Error::Chromium(e));
                        }
                    }
                }
                Err(e) => {
                    warn!("查找元素失败 (尝试 {}): {:?}", attempt + 1, e);
                    last_error = Some(Error::ElementNotFound(format!("{}: {}", selector, e)));
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| Error::ElementNotFound(format!("元素 {} 未找到", selector))))
    }

    /// 等待元素出现
    pub async fn wait_for_element(&mut self, selector: &str, timeout_ms: u64) -> Result<()> {
        debug!("等待元素: {} (超时: {}ms)", selector, timeout_ms);
        let page = self.get_or_create_page().await?;
        
        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(timeout_ms);
        
        while start.elapsed() < timeout {
            if page.find_element(selector).await.is_ok() {
                return Ok(());
            }
            sleep(Duration::from_millis(100)).await;
        }
        
        Err(Error::Timeout(format!("等待元素 {} 超时", selector)))
    }

    /// 执行 JavaScript 代码
    pub async fn execute_script(&mut self, script: &str) -> Result<serde_json::Value> {
        debug!("执行脚本");
        let page = self.get_or_create_page().await?;
        
        let result = page.evaluate(script.to_string()).await?;
        Ok(result.into_value()?)
    }

    /// 截图并保存
    pub async fn screenshot(&mut self, path: Option<&str>) -> Result<Vec<u8>> {
        info!("截图: {:?}", path);
        let page = self.get_or_create_page().await?;
        
        let screenshot = page
            .screenshot(
                chromiumoxide::page::ScreenshotParams::builder()
                    .format(CaptureScreenshotFormat::Png)
                    .full_page(true)
                    .build(),
            )
            .await?;
        
        if let Some(path) = path {
            std::fs::write(path, &screenshot)?;
            info!("截图已保存到: {}", path);
        }
        
        Ok(screenshot)
    }

    /// 获取页面 HTML 内容
    pub async fn get_content(&mut self) -> Result<String> {
        debug!("获取页面内容");
        let page = self.get_or_create_page().await?;
        let content = page.content().await?;
        Ok(content)
    }

    /// 页面滚动
    pub async fn scroll(&mut self, x: i32, y: i32) -> Result<()> {
        debug!("滚动页面: x={}, y={}", x, y);
        let page = self.get_or_create_page().await?;
        
        let script = format!("window.scrollBy({}, {})", x, y);
        page.evaluate(script).await?;
        
        Ok(())
    }

    /// 执行浏览器操作
    pub async fn execute_action(&mut self, action: BrowserAction) -> Result<ExecutionResult> {
        match action {
            BrowserAction::Navigate(url) => {
                self.navigate(&url).await?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("已导航到 {}", url)),
                    data: None,
                })
            }
            BrowserAction::Click(selector) => {
                self.click_element(&selector).await?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("已点击 {}", selector)),
                    data: None,
                })
            }
            BrowserAction::Type { selector, text } => {
                self.type_text(&selector, &text).await?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("已在 {} 中输入文本", selector)),
                    data: None,
                })
            }
            BrowserAction::WaitForElement { selector, timeout_ms } => {
                self.wait_for_element(&selector, timeout_ms).await?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("元素 {} 已出现", selector)),
                    data: None,
                })
            }
            BrowserAction::ExecuteScript(script) => {
                let result = self.execute_script(&script).await?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some("脚本已执行".to_string()),
                    data: Some(result),
                })
            }
            BrowserAction::Screenshot { path } => {
                let data = self.screenshot(path.as_deref()).await?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("截图完成，大小: {} 字节", data.len())),
                    data: Some(serde_json::json!({ "size": data.len() })),
                })
            }
            BrowserAction::GetContent => {
                let content = self.get_content().await?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("已获取内容，长度: {}", content.len())),
                    data: Some(serde_json::json!({ "content": content })),
                })
            }
            BrowserAction::Scroll { x, y } => {
                self.scroll(x, y).await?;
                Ok(ExecutionResult {
                    success: true,
                    message: Some(format!("已滚动 x={}, y={}", x, y)),
                    data: None,
                })
            }
        }
    }

    /// 关闭浏览器
    pub async fn close(mut self) -> Result<()> {
        info!("关闭浏览器");
        self.browser.close().await?;
        Ok(())
    }
}
