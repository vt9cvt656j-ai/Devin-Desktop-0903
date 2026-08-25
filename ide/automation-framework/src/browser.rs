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
    /// 这个实例**实际**是怎么起来的。回执必须按它说话——见 BrowserIdentity。
    identity: BrowserIdentity,
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
    // 用户级安装（%LOCALAPPDATA%）必须在列。Chrome/Edge 的安装器在非管理员账户下
    // 默认就装那儿，而这张表以前只有 Program Files——于是装着 Chrome 的机器被告知
    // "没装浏览器"。chromium 那一项以前整个不在表里，找不到时的提示却叫人去装它。
    #[cfg(target_os = "windows")]
    let owned_table: Vec<(&str, Vec<String>)> = {
        let local = std::env::var("LOCALAPPDATA").ok();
        let l = |rel: &str| -> Option<String> { local.as_ref().map(|b| format!(r"{b}\{rel}")) };
        let with = |fixed: &[&str], user: Option<String>| -> Vec<String> {
            let mut v: Vec<String> = fixed.iter().map(|s| s.to_string()).collect();
            if let Some(u) = user {
                v.push(u);
            }
            v
        };
        vec![
            ("chrome", with(&[
                r"C:\Program Files\Google\Chrome\Application\chrome.exe",
                r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            ], l(r"Google\Chrome\Application\chrome.exe"))),
            ("edge", with(&[
                r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
                r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
            ], l(r"Microsoft\Edge\Application\msedge.exe"))),
            ("brave", with(&[
                r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe",
                r"C:\Program Files (x86)\BraveSoftware\Brave-Browser\Application\brave.exe",
            ], l(r"BraveSoftware\Brave-Browser\Application\brave.exe"))),
            ("chromium", with(&[
                r"C:\Program Files\Chromium\Application\chrome.exe",
            ], l(r"Chromium\Application\chrome.exe"))),
        ]
    };
    #[cfg(target_os = "windows")]
    let table_owned: Vec<(&str, Vec<&str>)> = owned_table
        .iter()
        .map(|(id, paths)| (*id, paths.iter().map(|s| s.as_str()).collect()))
        .collect();
    #[cfg(target_os = "windows")]
    let table: Vec<(&str, &[&str])> = table_owned
        .iter()
        .map(|(id, paths)| (*id, paths.as_slice()))
        .collect();
    #[cfg(target_os = "windows")]
    let table: &[(&str, &[&str])] = &table;
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

/// 一个跑着的实例的真实身份：**它是怎么起来的**，不是这次请求要它怎么起。
///
/// 这两样以前根本没存下来，于是 browser.start 命中已有实例时只能拿本次请求的参数
/// 去编回执：模型先用默认 isolated 起过浏览器，之后按工具描述带 profile="session"
/// 再 start，收到的是「持久 profile、登录态可用」——而真正在跑的还是那只无 cookie 的
/// 隔离实例。headless 同理：它拿着一句 ok，以为窗口已经弹出来给用户登录了。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrowserIdentity {
    pub headless: bool,
    pub profile: BrowserProfile,
}

impl BrowserIdentity {
    pub fn new(headless: bool, profile: BrowserProfile) -> Self {
        Self { headless, profile }
    }
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
            // Windows 上没有 HOME，只有 USERPROFILE。原来硬回落到 "/tmp"，
            // 在 Windows 上就成了当前盘根下的 C:\tmp——一个多半不存在、
            // 也不属于这个用户的目录。于是"登录一次长期有效"这个承诺在 Windows 上
            // 要么创建失败、要么每次落在别处，而回执照旧那么说。
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .map(std::path::PathBuf::from)
                // 两个都没有才退到系统临时目录——那是**平台正确**的临时目录，
                // 不是写死的 /tmp。这时候会话不再跨重启，但至少能建出来。
                .unwrap_or_else(|_| std::env::temp_dir());
            home.join(".mrday-browser-session")
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
        Self::with_config(config, BrowserIdentity::new(true, profile)).await
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
        Self::with_config(config, BrowserIdentity::new(false, profile)).await
    }

    /// 使用自定义配置创建。
    ///
    /// identity 必须如实描述这份 config 起出来的是什么实例：它是之后所有回执的唯一
    /// 事实来源，编错了就等于回执说谎。
    pub async fn with_config(config: BrowserConfig, identity: BrowserIdentity) -> Result<Self> {
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
            identity,
        })
    }

    /// 这个实例实际的（有头/无头 × 身份）。回执按它编，不按请求参数编。
    pub fn identity(&self) -> BrowserIdentity {
        self.identity
    }

    /// 检查**当前页面**是否还应答。重试循环用它区分「这一次点击失败」和「整条链路没了」。
    ///
    /// 注意它不是浏览器存活判据：current_page 为 None 时它恒返回 true（那时确实没有
    /// 页面可问），而 page.evaluate 失败也可能只是这一个标签页崩了。要问「浏览器进程
    /// 还在不在」用 is_alive。
    pub async fn is_connected(&self) -> bool {
        if let Some(page) = &self.current_page {
            page.evaluate("1+1".to_string()).await.is_ok()
        } else {
            true // 没有页面时认为浏览器本身是连接的
        }
    }

    /// 连接级探活：直接问浏览器自己（CDP Browser.getVersion），不经过任何页面。
    ///
    /// 这是「这只浏览器还活着吗」唯一靠得住的问法。is_connected 回答不了：没开过页面时
    /// 它恒 true，于是用户手关了有头窗口、或 Chrome 崩了之后，browser.start 命中缓存实例
    /// 照样回一句 ok，接着每条 browser.* 抛一串底层连接错误，而模型手里没有任何「重启」
    /// 的出路——它再调 browser.start 拿到的还是那句无动作的 ok，出不去。
    ///
    /// 超时是必须的：进程还在但 websocket 卡住时，execute 会一直等下去。
    pub async fn is_alive(&self) -> bool {
        matches!(
            tokio::time::timeout(Duration::from_secs(5), self.browser.version()).await,
            Ok(Ok(_))
        )
    }

    /// 新建标签页等页面 load 的上限。
    ///
    /// chromiumoxide 的 `Browser::new_page` 是**没有上限**的：它等的是这个 frame 收到过
    /// 名为 "load" 的 Page.lifecycleEvent，而兑现它的那个 oneshot 在 `set_initiator` 之后
    /// 就被搬出了 pending_commands，从此不再受 30 秒逐出管辖。也就是说只要页面永远不触发
    /// load，这个 await 就**永远不返回**——挂着的子资源、下载型 URL、重定向循环、
    /// basic-auth 弹窗都能做到。这是整条自动化链上唯一一处真正的无限期等待，
    /// 而它还握着 rpc 那把 agent 锁，一挂就把读屏之类完全无关的调用一起堵死。
    const NEW_PAGE_TIMEOUT: Duration = Duration::from_secs(20);

    /// 新建标签页，带上限。超时按普通错误返回，让上层的重试和回执照常工作。
    async fn new_page_bounded(&self, url: &str) -> Result<Page> {
        match tokio::time::timeout(Self::NEW_PAGE_TIMEOUT, self.browser.new_page(url)).await {
            Ok(r) => r.map_err(Error::Chromium),
            Err(_) => Err(Error::Other(anyhow::anyhow!(
                "打开 {url} 超过 {} 秒还没加载完（页面的 load 事件一直没来，常见于挂住的子资源、\
                 下载链接、重定向循环或弹出的登录框）。换个 URL，或者先 browser.start 再单独 goto。",
                Self::NEW_PAGE_TIMEOUT.as_secs()
            ))),
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
                        last_error = Some(Error::Chromium(e));
                    }
                }
            } else {
                match self.new_page_bounded(url).await {
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
        
        // 这里原来是 Err(Error::Chromium(last_error.unwrap()))，也就是把累积的错误
        // 硬当成 CdpError。加上超时之后失败不再只有一种来源（超时是本 crate 的 Error），
        // 再那么写会丢掉"为什么失败"——而"页面 load 一直没来"恰恰是最需要说清的一种。
        Err(last_error.unwrap_or_else(|| {
            Error::Other(anyhow::anyhow!("导航失败，且没有留下任何错误"))
        }))
    }

    /// 获取当前页面（如果没有则创建新页面）
    async fn get_or_create_page(&mut self) -> Result<Arc<Page>> {
        if let Some(page) = &self.current_page {
            return Ok(Arc::clone(page));
        }

        info!("创建新页面");
        // 同样要有上限：click / type / eval / screenshot / content 在没有当前页面时
        // 都从这里过，一处无限等就把它们全拖下水。
        let page = self.new_page_bounded("about:blank").await?;
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
    /// 返回**是否真的落了盘**：Browser.close 被接受，且进程在超时前自己退了。
    ///
    /// 两个结果原来都被 `let _ =` 吞掉，于是「超时没等到退出 → Drop 走 kill_on_drop
    /// 强杀 → cookie 没写进 profile」这条路在回执里完全不可见：browser.close 照样回
    /// {"status":"ok"}，模型据此认为登录态保住了。持久 profile 下这就是「登录一次长期
    /// 有效」这个承诺静默失守的那一刻，而它下一次运行才会以「怎么又要登录」的形式冒出来。
    pub async fn close_gracefully(&mut self) -> Result<bool> {
        info!("优雅关闭浏览器（等待落盘）");
        // close() 自己没有超时：进程还在、handler 还活着但 websocket 卡住时，它会一直等
        // 下去。browser.start 的重启分支也会走到这里，所以这一步必须有上限，否则「浏览器
        // 卡死」会从「一条 RPC 报错」升级成「整个 sidecar 挂住」。
        let close_sent = matches!(
            tokio::time::timeout(Duration::from_secs(8), self.browser.close()).await,
            Ok(Ok(_))
        );
        // 等它自己退出，给 cookie/localStorage 刷盘的时间；超时就算了，Drop 会兜底——
        // 但那是强杀，所以「没等到」必须如实回给调用方。
        let exited = matches!(
            tokio::time::timeout(Duration::from_secs(8), self.browser.wait()).await,
            Ok(Ok(_))
        );
        if !(close_sent && exited) {
            warn!("浏览器没能自己退出（close_sent={close_sent}, exited={exited}），将被强杀，落盘无保证");
        }
        Ok(close_sent && exited)
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

#[cfg(test)]
mod liveness_tests {
    /// is_alive 必须直接问浏览器（CDP Browser.getVersion），不许经过任何页面。
    ///
    /// 页面级探活答不了「浏览器进程还在不在」：没开过页面时恒 true，开了页面时
    /// 一个崩掉的标签页也会被当成整只浏览器死了。启动路径拿它当判据，就等于
    /// 对「用户手关了那个有头窗口」全盲。
    #[test]
    fn liveness_is_probed_at_the_connection_not_through_a_page() {
        let src = include_str!("browser.rs");
        let at = src.find("pub async fn is_alive(").expect("is_alive 不见了");
        let end = src[at..].find("\n    }").map(|e| at + e).unwrap_or(src.len());
        let body: String = src[at..end]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(body.contains("self.browser.version()"), "没走连接级探活: {body}");
        assert!(!body.contains("current_page"), "又从页面探活了: {body}");
        assert!(!body.contains("evaluate"), "又从页面探活了: {body}");
        // 进程还在但 websocket 卡住时 execute 会一直等，探活不能变成挂起。
        assert!(body.contains("timeout("), "探活没有超时兜底: {body}");
    }

    /// close_gracefully 必须回答「有没有真的落盘」，而且两个结果都要算进去。
    ///
    /// 只发出 Browser.close 不等于关成功；等超时了 Drop 会 kill_on_drop 强杀，
    /// 那时 cookie 根本没写进 profile。任何一步没成，flushed 就必须是 false。
    #[test]
    fn a_graceful_close_reports_whether_the_process_really_exited() {
        let src = include_str!("browser.rs");
        let at = src.find("pub async fn close_gracefully(").expect("close_gracefully 不见了");
        let signature = src[at..].lines().next().unwrap_or("");
        assert!(
            signature.contains("Result<bool>"),
            "close_gracefully 又不回落盘结果了: {signature}"
        );
        let end = src[at..].find("\n    }").map(|e| at + e).unwrap_or(src.len());
        let body: String = src[at..end]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!body.contains("let _ = self.browser.close()"), "又吞掉 close 的结果了: {body}");
        // 钉的是**返回值**：只写在 warn! 的判据里不算，那句话不改变任何回执。
        assert!(
            body.contains("Ok(close_sent && exited)"),
            "落盘结论不是「两步都成了」——只看其中一个不够，超时被强杀也是没落盘: {body}"
        );
    }

    /// 实例的身份跟着实例走，不跟着请求走——回执唯一的事实来源。
    #[test]
    fn an_instance_carries_the_identity_it_was_launched_with() {
        let src = include_str!("browser.rs");
        assert!(src.contains("pub fn identity(&self) -> BrowserIdentity"), "identity() 不见了");
        // 两条启动路径都必须如实标注自己是有头还是无头。
        assert!(src.contains("BrowserIdentity::new(true, profile)"), "无头路径没标注身份");
        assert!(src.contains("BrowserIdentity::new(false, profile)"), "有头路径没标注身份");
    }
}
