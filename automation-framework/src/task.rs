//! 任务型 API - 高层自动化接口
//!
//! 让 AI agent 通过任务描述而非底层 API 完成自动化

use crate::agent::Agent;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// 任务执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub message: String,
    pub data: Option<HashMap<String, serde_json::Value>>,
    pub screenshot_path: Option<String>,
}

impl TaskResult {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
            screenshot_path: None,
        }
    }

    pub fn with_data(mut self, data: HashMap<String, serde_json::Value>) -> Self {
        self.data = Some(data);
        self
    }

    pub fn with_screenshot(mut self, path: impl Into<String>) -> Self {
        self.screenshot_path = Some(path.into());
        self
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
            screenshot_path: None,
        }
    }
}

/// 任务执行器 - 高层自动化 API
pub struct TaskExecutor {
    agent: Agent,
}

impl TaskExecutor {
    /// 创建新的任务执行器
    pub fn new() -> Result<Self> {
        Ok(Self {
            agent: Agent::new()?,
        })
    }

    /// 初始化（必须先调用）
    pub fn init(&mut self) -> Result<()> {
        self.agent.system_init()
    }

    /// 搜索网页内容
    /// 
    /// # 参数
    /// - `query`: 搜索关键词
    /// - `engine`: 搜索引擎 ("google", "bing", "duckduckgo")
    /// 
    /// # 返回
    /// 搜索结果页面的截图和提取的文本
    pub fn web_search(&mut self, query: &str, engine: &str) -> Result<TaskResult> {
        let url = match engine {
            "google" => format!("https://www.google.com/search?q={}", urlencoding::encode(query)),
            "bing" => format!("https://www.bing.com/search?q={}", urlencoding::encode(query)),
            "duckduckgo" => format!("https://duckduckgo.com/?q={}", urlencoding::encode(query)),
            _ => return Ok(TaskResult::error(format!("不支持的搜索引擎: {}", engine))),
        };

        self.agent.browser_start(false)?;
        self.agent.browser_goto(&url)?;
        std::thread::sleep(Duration::from_millis(2000));

        let screenshot_path = format!("search_result_{}.png", chrono::Local::now().timestamp());
        self.agent.browser_screenshot(Some(&screenshot_path))?;

        let content = self.agent.browser_content()?;
        
        let mut data = HashMap::new();
        data.insert("query".to_string(), serde_json::json!(query));
        data.insert("engine".to_string(), serde_json::json!(engine));
        data.insert("content_length".to_string(), serde_json::json!(content.len()));

        self.agent.browser_close()?;

        Ok(TaskResult::success("搜索完成")
            .with_data(data)
            .with_screenshot(screenshot_path))
    }

    /// 访问网页并提取内容
    /// 
    /// # 参数
    /// - `url`: 目标网址
    /// - `selectors`: 要提取的 CSS 选择器列表
    /// 
    /// # 返回
    /// 提取的内容和截图
    pub fn extract_web_content(&mut self, url: &str, selectors: Vec<&str>) -> Result<TaskResult> {
        self.agent.browser_start(false)?;
        self.agent.browser_goto(url)?;
        std::thread::sleep(Duration::from_millis(2000));

        let mut data = HashMap::new();
        data.insert("url".to_string(), serde_json::json!(url));

        // 提取每个选择器的内容（选择器经 JSON 编码后嵌入，含引号/反斜杠的选择器不会撞坏脚本）
        for selector in selectors {
            let script = format!(
                "Array.from(document.querySelectorAll({})).map(el => el.textContent.trim()).join(' | ')",
                serde_json::json!(selector)
            );
            if let Ok(result) = self.agent.browser_eval(&script) {
                data.insert(selector.to_string(), result);
            }
        }

        let screenshot_path = format!("extract_{}.png", chrono::Local::now().timestamp());
        self.agent.browser_screenshot(Some(&screenshot_path))?;

        self.agent.browser_close()?;

        Ok(TaskResult::success("内容提取完成")
            .with_data(data)
            .with_screenshot(screenshot_path))
    }

    /// 填写表单并提交
    /// 
    /// # 参数
    /// - `url`: 表单页面 URL
    /// - `fields`: 字段映射 (选择器 -> 值)
    /// - `submit_selector`: 提交按钮选择器
    /// 
    /// # 返回
    /// 提交后的页面截图
    pub fn fill_form(&mut self, url: &str, fields: HashMap<String, String>, submit_selector: &str) -> Result<TaskResult> {
        self.agent.browser_start(false)?;
        self.agent.browser_goto(url)?;
        std::thread::sleep(Duration::from_millis(2000));

        // 填写字段
        for (selector, value) in fields.iter() {
            if let Err(e) = self.agent.browser_type(selector, value) {
                self.agent.browser_close()?;
                return Ok(TaskResult::error(format!("填写字段 {} 失败: {}", selector, e)));
            }
            std::thread::sleep(Duration::from_millis(300));
        }

        // 提交表单
        if let Err(e) = self.agent.browser_click(submit_selector) {
            self.agent.browser_close()?;
            return Ok(TaskResult::error(format!("点击提交按钮失败: {}", e)));
        }

        std::thread::sleep(Duration::from_millis(3000));

        let screenshot_path = format!("form_result_{}.png", chrono::Local::now().timestamp());
        self.agent.browser_screenshot(Some(&screenshot_path))?;

        self.agent.browser_close()?;

        Ok(TaskResult::success("表单提交完成")
            .with_screenshot(screenshot_path))
    }

    /// 打开应用并输入文本
    /// 
    /// # 参数
    /// - `app_name`: 应用名称 (如 "TextEdit", "Notes")
    /// - `content`: 要输入的内容
    /// 
    /// # 返回
    /// 执行结果
    pub fn open_and_type(&mut self, app_name: &str, content: &str) -> Result<TaskResult> {
        // 直接传参给 open，不经过 shell：app_name 含引号/分号时不会被当成命令执行
        let status = std::process::Command::new("open")
            .arg("-a")
            .arg(app_name)
            .status()?;
        if !status.success() {
            return Ok(TaskResult::error(format!("打开应用 {} 失败", app_name)));
        }

        std::thread::sleep(Duration::from_millis(2000));

        // 输入内容
        self.agent.keyboard_type(content)?;

        Ok(TaskResult::success(format!("已在 {} 中输入内容", app_name)))
    }

    /// 截图指定区域
    /// 
    /// # 参数
    /// - `x`, `y`: 起始坐标
    /// - `width`, `height`: 区域大小
    /// 
    /// # 返回
    /// 截图路径
    pub fn screenshot_region(&mut self, x: i32, y: i32, width: i32, height: i32) -> Result<TaskResult> {
        // 移动到区域起点
        self.agent.mouse_move(x, y)?;
        std::thread::sleep(Duration::from_millis(100));

        let screenshot_path = format!("region_{}_{}_{}_{}.png", x, y, width, height);

        // 直接调 screencapture，参数各自独立传递，不经过 shell 拼接
        let status = std::process::Command::new("screencapture")
            .arg("-R")
            .arg(format!("{},{},{},{}", x, y, width, height))
            .arg(&screenshot_path)
            .status()?;
        if !status.success() {
            return Ok(TaskResult::error("区域截图失败（screencapture 退出码非 0）"));
        }

        Ok(TaskResult::success("区域截图完成")
            .with_screenshot(screenshot_path))
    }

    /// 监控网页变化
    /// 
    /// # 参数
    /// - `url`: 目标网址
    /// - `selector`: 要监控的元素选择器
    /// - `interval_secs`: 检查间隔（秒）
    /// - `duration_secs`: 总监控时长（秒）
    /// 
    /// # 返回
    /// 变化记录
    pub fn monitor_web_changes(&mut self, url: &str, selector: &str, interval_secs: u64, duration_secs: u64) -> Result<TaskResult> {
        // interval 为 0 会除零 panic，直接拒绝
        if interval_secs == 0 {
            return Ok(TaskResult::error("interval_secs 必须大于 0"));
        }
        self.agent.browser_start(false)?;
        self.agent.browser_goto(url)?;

        let mut changes = Vec::new();
        let mut last_content = String::new();
        let iterations = duration_secs / interval_secs;

        for i in 0..iterations {
            std::thread::sleep(Duration::from_secs(interval_secs));

            let script = format!(
                "document.querySelector({})?.textContent?.trim() || ''",
                serde_json::json!(selector)
            );
            
            if let Ok(content) = self.agent.browser_eval(&script) {
                let content_str = content.as_str().unwrap_or("").to_string();
                if content_str != last_content {
                    changes.push(format!("[{}] 变化: {} -> {}", i, last_content, content_str));
                    last_content = content_str;
                }
            }
        }

        self.agent.browser_close()?;

        let mut data = HashMap::new();
        data.insert("changes".to_string(), serde_json::json!(changes));
        data.insert("change_count".to_string(), serde_json::json!(changes.len()));

        Ok(TaskResult::success(format!("监控完成，发现 {} 次变化", changes.len()))
            .with_data(data))
    }

    /// 批量访问网页并收集数据
    /// 
    /// # 参数
    /// - `urls`: 网址列表
    /// - `data_selector`: 数据提取选择器
    /// 
    /// # 返回
    /// 收集的数据
    pub fn batch_collect(&mut self, urls: Vec<&str>, data_selector: &str) -> Result<TaskResult> {
        self.agent.browser_start(false)?;

        let mut collected = Vec::new();

        for (i, url) in urls.iter().enumerate() {
            self.agent.browser_goto(url)?;
            self.agent.system_init()?;
            std::thread::sleep(Duration::from_millis(2000));

            let script = format!(
                "Array.from(document.querySelectorAll({})).map(el => el.textContent.trim()).join(' | ')",
                serde_json::json!(data_selector)
            );

            if let Ok(data) = self.agent.browser_eval(&script) {
                collected.push(serde_json::json!({
                    "url": url,
                    "index": i,
                    "data": data
                }));
            }
        }

        self.agent.browser_close()?;

        let mut result_data = HashMap::new();
        result_data.insert("collected".to_string(), serde_json::json!(collected));
        result_data.insert("total".to_string(), serde_json::json!(urls.len()));

        Ok(TaskResult::success(format!("收集完成，共 {} 个网页", urls.len()))
            .with_data(result_data))
    }
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new().expect("创建 TaskExecutor 失败")
    }
}
