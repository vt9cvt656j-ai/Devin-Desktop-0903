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

/// 找一个能用的 Chromium 内核浏览器。
///
/// 以前这里写死的是 `/Applications/Google Chrome.app/...` 这个绝对路径，于是：
/// 没装 Chrome 的机器上（只有 Edge 或 Brave），`automation` 工具的整条 browser.*
/// 链路直接起不来——而同一台机器上 IDE 自己的 browser 工具明明跑得好好的，因为那边
/// 会探测。Linux / Windows 上这个路径更是永远不存在，等于整条链路不可用。
///
/// 顺序：`MICHAEL_BROWSER_PATH` 指定的可执行文件 → `MICHAEL_BROWSER` 指定的牌子 →
/// 装了的第一个。这个 crate 独立于 src-tauri，所以目录表在这里单独维护。
fn find_browser() -> Result<String> {
    if let Ok(p) = std::env::var("MICHAEL_BROWSER_PATH") {
        let p = p.trim().to_string();
        if !p.is_empty() && std::path::Path::new(&p).exists() {
            return Ok(p);
        }
    }
    let want = std::env::var("MICHAEL_BROWSER")
        .ok()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty());

    #[cfg(target_os = "macos")]
    let table: &[(&str, &[&str])] = &[
        ("chrome", &["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"]),
        ("edge", &["/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"]),
        ("brave", &["/Applications/Brave Browser.app/Contents/MacOS/Brave Browser"]),
        ("chromium", &["/Applications/Chromium.app/Contents/MacOS/Chromium"]),
    ];
    #[cfg(target_os = "windows")]
    let table: &[(&str, &[&str])] = &[
        ("chrome", &[
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ]),
        ("edge", &[
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        ]),
        ("brave", &[r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe"]),
    ];
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let table: &[(&str, &[&str])] = &[
        ("chrome", &["/usr/bin/google-chrome", "/usr/bin/google-chrome-stable"]),
        ("edge", &["/usr/bin/microsoft-edge", "/usr/bin/microsoft-edge-stable"]),
        ("brave", &["/usr/bin/brave-browser", "/usr/bin/brave"]),
        ("chromium", &["/usr/bin/chromium", "/usr/bin/chromium-browser"]),
    ];

    let first_installed = |id: &str| -> Option<String> {
        table
            .iter()
            .find(|(k, _)| *k == id)
            .and_then(|(_, paths)| {
                paths
                    .iter()
                    .find(|p| std::path::Path::new(p).exists())
                    .map(|p| p.to_string())
            })
    };
    if let Some(ref id) = want {
        if let Some(p) = first_installed(id) {
            return Ok(p);
        }
    }
    for (id, _) in table {
        if let Some(p) = first_installed(id) {
            return Ok(p);
        }
    }
    Err(Error::Browser(
        "未找到可用的 Chromium 内核浏览器（Chrome / Edge / Brave / Chromium）。\
装上其一，或用 MICHAEL_BROWSER_PATH 指定可执行文件路径。"
            .into(),
    ))
}

/// 浏览器要用哪套身份。
///
/// 这是自动化里最容易出错的一个选择：抓公开页面、测自己的 dev server 该用干净实例；
/// 而"去我的后台看一眼""把这条发出去"这类任务，用空白浏览器只会撞上登录墙——用户
/// 已经在自己的浏览器里登录了一堆网站，再开一个全新的等于把那些登录态全扔了。
///
/// 注意**不能**直接用用户 Chrome 的 profile：Chrome 运行时会独占那个目录，而复制一份
/// 4GB 的 profile 既慢又是隐私灾难。可行的是一个**专用的持久 profile**：用户在有头窗口
/// 里登录一次，此后每次自动化都带着这些会话，并且和用户自己的 Chrome 井水不犯河水
/// （实测两者可同时运行）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserProfile {
    /// 一次性目录，每次全新。默认，也是抓取/测试该用的。
    Isolated,
    /// 专用持久目录，保留登录态。需要用户身份的任务用这个。
    Session,
}

/// 一次性实例关掉扩展（干净、可复现）；持久实例**不关**。
///
/// 以前两种都无条件加 `--disable-extensions`，于是「在有头窗口里登录一次、此后一直
/// 带着这些会话」这条设计是残的：用户在那个持久窗口里装的密码管理器、拦截器，每次
/// 自动化启动都被关掉，等于装了个寂寞。
fn with_extension_policy(
    b: chromiumoxide::browser::BrowserConfigBuilder,
    profile: BrowserProfile,
) -> chromiumoxide::browser::BrowserConfigBuilder {
    match profile {
        BrowserProfile::Isolated => b.arg("--disable-extensions"),
        BrowserProfile::Session => b,
    }
}

fn profile_dir(profile: BrowserProfile) -> Result<std::path::PathBuf> {
    let dir = match profile {
        BrowserProfile::Isolated => std::env::temp_dir()
            .join(format!("rust_automation_browser_{}", std::process::id())),
        // 固定路径：跨进程、跨重启都是同一个，登录一次长期有效。
        BrowserProfile::Session => {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            std::path::PathBuf::from(home).join(".mrday-browser-session")
        }
    };
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

impl BrowserAutomation {
    /// 创建新的浏览器实例（默认无头模式）
    pub async fn new() -> Result<Self> {
        Self::new_with_profile(true, BrowserProfile::Isolated).await
    }

    /// 按「有头/无头 × 身份」创建实例。
    pub async fn new_with_profile(headless: bool, profile: BrowserProfile) -> Result<Self> {
        if headless {
            Self::launch_headless(profile).await
        } else {
            Self::launch_headed(profile).await
        }
    }

    async fn launch_headless(profile: BrowserProfile) -> Result<Self> {
        let temp_dir = profile_dir(profile)?;
        
        let config = with_extension_policy(BrowserConfig::builder().chrome_executable(find_browser()?), profile)
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-infobars")
            .arg("--disable-dev-shm-usage")
            .arg("--disable-software-rasterizer")
            .arg("--disable-background-networking")
            .arg("--metrics-recording-only")
            .arg("--disable-default-apps")
            .arg("--mute-audio")
            .arg("--no-first-run")
            .arg("--disable-gpu")
            .arg("--disable-features=site-per-process")
            .user_data_dir(&temp_dir)
            .build()?;
        Self::with_config(config).await
    }

    /// 创建有头（可见）浏览器实例
    pub async fn new_headed() -> Result<Self> {
        Self::launch_headed(BrowserProfile::Isolated).await
    }

    async fn launch_headed(profile: BrowserProfile) -> Result<Self> {
        let temp_dir = profile_dir(profile)?;
        
        let config = with_extension_policy(
            BrowserConfig::builder().chrome_executable(find_browser()?).with_head(),
            profile,
        )
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-infobars")
            .arg("--disable-dev-shm-usage")
            .arg("--disable-background-networking")
            .arg("--metrics-recording-only")
            .arg("--disable-default-apps")
            .arg("--mute-audio")
            .arg("--disable-features=site-per-process")
            .user_data_dir(&temp_dir)
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

    /// 优雅关闭（不消费 self）。
    ///
    /// 之前 browser_close 只是把 Arc drop 掉——chromiumoxide 的 Drop 会直接杀进程，
    /// Chrome 来不及把 cookie 刷盘。隔离 profile 下无所谓（本来就要丢），持久 profile 下
    /// 就是致命的：用户登录一次，关掉，登录态没了。所以必须发 CDP 的 Browser.close
    /// 再等进程自己退出。
    pub async fn close_gracefully(&mut self) -> Result<()> {
        info!("优雅关闭浏览器（等待落盘）");
        let _ = self.browser.close().await;
        // 等它自己退出，给 cookie/localStorage 刷盘的时间；超时就算了，Drop 会兜底。
        let _ = tokio::time::timeout(Duration::from_secs(8), self.browser.wait()).await;
        Ok(())
    }
}

#[cfg(test)]
mod profile_tests {
    use super::{profile_dir, BrowserProfile};

    /// 两种身份必须落到不同目录，而且持久那个不能在临时目录里。
    ///
    /// 这条守的是「登录一次长期有效」。持久 profile 一旦被指到 /tmp 或带上进程号，
    /// 用户登录完关掉就没了——而失败方式是安静的：下次照样能起浏览器，只是又变回未登录，
    /// 模型会以为"这网站需要登录"，其实是身份被丢了。
    #[test]
    fn 两种身份落在不同目录且持久那个真的持久() {
        let isolated = profile_dir(BrowserProfile::Isolated).expect("isolated");
        let session = profile_dir(BrowserProfile::Session).expect("session");
        assert_ne!(isolated, session, "两种身份不能共用目录");

        let tmp = std::env::temp_dir();
        assert!(isolated.starts_with(&tmp), "隔离身份就该在临时目录里：{isolated:?}");
        assert!(
            !session.starts_with(&tmp),
            "持久身份不能放临时目录，否则登录态随时会被清掉：{session:?}"
        );
        let s = session.to_string_lossy();
        assert!(
            !s.contains(&std::process::id().to_string()),
            "持久身份的路径里不能有进程号，否则每次启动都是新身份：{s}"
        );
        // 同一种身份重复取必须是同一个目录
        assert_eq!(session, profile_dir(BrowserProfile::Session).expect("session again"));
    }
}
