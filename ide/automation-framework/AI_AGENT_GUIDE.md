# AI Agent 集成指南

本框架专为 **AI Agent** 设计，提供简洁的统一接口，让 AI 能轻松调用浏览器和系统自动化能力。

## 🎯 设计理念

传统自动化框架需要手动管理 async/await、tokio runtime、错误处理等复杂细节。本框架的 `Agent` API 封装了所有底层复杂性：

- ✅ **同步接口**：所有方法都是同步的，自动处理异步逻辑
- ✅ **统一入口**：一个 `Agent` 实例控制所有功能
- ✅ **自动初始化**：无需手动创建 runtime 或管理生命周期
- ✅ **错误友好**：统一的 `Result<T>` 返回类型

## 📦 快速开始

### 1. 添加依赖

```toml
[dependencies]
rust-automation-framework = { path = ".", features = ["full"] }
anyhow = "1.0"
```

### 2. 基础用法

```rust
use rust_automation_framework::Agent;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 Agent 实例
    let mut agent = Agent::new()?;
    
    // 初始化系统控制
    agent.system_init()?;
    
    // 鼠标操作
    agent.mouse_move(500, 300)?;
    agent.mouse_click(Some("left"))?;
    
    // 键盘操作
    agent.keyboard_type("Hello from AI Agent!")?;
    agent.keyboard_press("enter")?;
    
    Ok(())
}
```

## 🌐 浏览器自动化

### 启动浏览器

```rust
// 有头模式（可见窗口）
agent.browser_start(false)?;

// 无头模式（后台运行）
agent.browser_start(true)?;
```

### 页面导航

```rust
agent.browser_goto("https://example.com")?;
```

### 元素操作

```rust
// 点击元素（CSS 选择器）
agent.browser_click("button#submit")?;

// 输入文本
agent.browser_type("input[name='q']", "search query")?;

// 等待元素出现（超时毫秒）
agent.browser_wait("div.result", 5000)?;
```

### 执行 JavaScript

```rust
let title = agent.browser_eval("document.title")?;
println!("页面标题: {}", title);

// 复杂操作
let result = agent.browser_eval(r#"
    document.querySelectorAll('a').length
"#)?;
```

### 页面截图

```rust
// 保存到指定路径
agent.browser_screenshot(Some("screenshot.png"))?;

// 默认路径
agent.browser_screenshot(None)?;
```

### 获取页面内容

```rust
let html = agent.browser_content()?;
println!("页面长度: {} 字符", html.len());
```

### 关闭浏览器

```rust
agent.browser_close()?;
```

## 🖱️ 系统自动化

### 鼠标控制

```rust
// 初始化系统控制（首次调用）
agent.system_init()?;

// 移动鼠标到绝对坐标
agent.mouse_move(800, 600)?;

// 点击（left/right/middle）
agent.mouse_click(Some("left"))?;
agent.mouse_click(Some("right"))?;

// 双击
agent.mouse_double_click(Some("left"))?;

// 拖拽
agent.mouse_drag(100, 100, 500, 500)?;

// 滚动（delta_x, delta_y）
agent.mouse_scroll(0, 5)?;   // 向下滚动
agent.mouse_scroll(0, -3)?;  // 向上滚动
```

### 键盘控制

```rust
// 输入文本
agent.keyboard_type("Hello, World!")?;

// 按单个键
agent.keyboard_press("enter")?;
agent.keyboard_press("tab")?;
agent.keyboard_press("escape")?;

// 组合键
agent.keyboard_combo(vec!["ctrl", "c"])?;     // 复制
agent.keyboard_combo(vec!["ctrl", "v"])?;     // 粘贴
agent.keyboard_combo(vec!["cmd", "a"])?;      // macOS 全选
agent.keyboard_combo(vec!["ctrl", "alt", "delete"])?;
```

### 支持的按键

常用键：`enter`, `tab`, `space`, `escape`, `backspace`, `delete`

方向键：`up`, `down`, `left`, `right`

功能键：`home`, `end`, `pageup`, `pagedown`

修饰键：`ctrl`, `alt`, `shift`, `cmd`/`meta`

单字符：直接传字符串，如 `"a"`, `"1"`, `"!"`

### 延时

```rust
// 延时（毫秒）
agent.sleep(1000);  // 等待 1 秒
```

## 🔄 混合使用示例

```rust
fn automate_workflow(agent: &mut Agent) -> Result<(), Box<dyn std::error::Error>> {
    // 1. 初始化
    agent.system_init()?;
    agent.browser_start(false)?;
    
    // 2. 浏览器操作
    agent.browser_goto("https://example.com/login")?;
    agent.browser_wait("input#username", 5000)?;
    
    // 3. 系统级精确控制
    agent.mouse_move(500, 300)?;
    agent.sleep(200);
    agent.mouse_click(Some("left"))?;
    
    // 4. 键盘输入
    agent.keyboard_type("admin")?;
    agent.keyboard_press("tab")?;
    agent.keyboard_type("password123")?;
    agent.keyboard_press("enter")?;
    
    // 5. 等待页面加载
    agent.sleep(2000);
    agent.browser_wait("div.dashboard", 10000)?;
    
    // 6. 截图验证
    agent.browser_screenshot(Some("dashboard.png"))?;
    
    // 7. 清理
    agent.browser_close()?;
    
    Ok(())
}
```

## 📡 JSON-RPC 接口（跨语言调用）

框架提供 JSON-RPC 服务层，让其他语言的 AI agent 也能调用：

### 启动 RPC 服务器

```rust
use rust_automation_framework::RpcServer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = RpcServer::new(8080)?;
    
    // 注意：HTTP 服务器需要自行实现（推荐 axum/actix-web）
    // 或使用 handle_request 方法处理单个请求
    
    Ok(())
}
```

### RPC 请求格式

```json
{
  "jsonrpc": "2.0",
  "method": "browser.goto",
  "params": {
    "url": "https://example.com"
  },
  "id": 1
}
```

### 可用方法

**浏览器**：
- `browser.start` - 启动浏览器，params: `{headless: bool}`
- `browser.goto` - 导航，params: `{url: string}`
- `browser.click` - 点击元素，params: `{selector: string}`
- `browser.type` - 输入文本，params: `{selector: string, text: string}`
- `browser.wait` - 等待元素，params: `{selector: string, timeout: number}`
- `browser.eval` - 执行 JS，params: `{script: string}`
- `browser.screenshot` - 截图，params: `{path?: string}`
- `browser.content` - 获取 HTML，params: `{}`
- `browser.close` - 关闭浏览器，params: `{}`

**系统**：
- `system.init` - 初始化，params: `{}`
- `mouse.move` - 移动鼠标，params: `{x: number, y: number}`
- `mouse.click` - 点击，params: `{button?: string}`
- `mouse.double_click` - 双击，params: `{button?: string}`
- `mouse.drag` - 拖拽，params: `{from_x, from_y, to_x, to_y: number}`
- `mouse.scroll` - 滚动，params: `{delta_x?, delta_y: number}`
- `keyboard.type` - 输入文本，params: `{text: string}`
- `keyboard.press` - 按键，params: `{key: string}`
- `keyboard.combo` - 组合键，params: `{keys: string[]}`
- `sleep` - 延时，params: `{ms: number}`

### Python 调用示例

```python
import requests

def call_agent(method, params):
    response = requests.post('http://localhost:8080/', json={
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1
    })
    return response.json()

# 使用
call_agent("system.init", {})
call_agent("mouse.move", {"x": 500, "y": 300})
call_agent("mouse.click", {"button": "left"})
call_agent("keyboard.type", {"text": "Hello from Python!"})
```

## 🛡️ 错误处理

所有方法返回 `Result<T>`，建议使用 `?` 操作符传播错误：

```rust
fn my_automation() -> Result<(), Box<dyn std::error::Error>> {
    let mut agent = Agent::new()?;
    
    agent.system_init()?;
    agent.mouse_move(500, 300)?;
    
    // 如果任何步骤失败，错误会自动传播
    
    Ok(())
}
```

## ⚠️ 注意事项

### 浏览器

- 需要系统安装 Chrome/Chromium
- macOS: `brew install --cask google-chrome`
- Windows: 从官网下载安装

### 系统权限

- **macOS**：需要在「系统设置 → 隐私与安全性 → 辅助功能」中授予权限
- **Windows**：某些操作可能需要管理员权限

### 线程安全

- `Agent` 实例不是 `Send`/`Sync`，不要跨线程共享
- 每个线程创建独立的 `Agent` 实例

### 资源清理

- `Agent` 在 `Drop` 时自动清理资源
- 建议显式调用 `browser_close()` 确保浏览器正常关闭

## 📚 完整示例

参见 `examples/` 目录：
- `browser_demo.rs` - 浏览器自动化
- `system_demo.rs` - 系统自动化
- `hybrid_demo.rs` - 混合使用
- `recorder_demo.rs` - 录制回放
- `showcase_demo.rs` - 综合演示
- `http_server_demo.rs` - HTTP 服务器

---

# 🌐 HTTP API 服务器

**为 AI agent 提供本地 HTTP 接口**，无需编写 Rust 代码，通过 HTTP 请求调用所有自动化能力。

## 快速启动

```bash
# 启动 HTTP 服务器（默认端口 3030）
cargo run --example http_server_demo --features full

# 或者编译后台运行
cargo build --example http_server_demo --features full --release
./target/release/examples/http_server_demo &
```

服务器启动后会显示可用的 API 端点。

## API 端点

### 健康检查

```bash
GET /health
```

返回服务状态和所有可用端点列表。

### 系统控制 API

#### 鼠标移动

```bash
POST /api/mouse/move
Content-Type: application/json

{
  "x": 500,
  "y": 300
}
```

#### 鼠标点击

```bash
POST /api/mouse/click
Content-Type: application/json

{
  "button": "left"  # "left" | "right" | "middle"
}
```

#### 键盘输入

```bash
POST /api/keyboard/type
Content-Type: application/json

{
  "text": "Hello from AI!"
}
```

### 任务型 API（推荐）

#### 网页搜索

```bash
POST /api/task/web_search
Content-Type: application/json

{
  "query": "Rust programming",
  "engine": "duckduckgo"  # "google" | "bing" | "duckduckgo"
}
```

**响应示例**：
```json
{
  "success": true,
  "data": {
    "success": true,
    "message": "搜索完成",
    "data": {
      "query": "Rust programming",
      "engine": "duckduckgo",
      "content_length": 168354
    },
    "screenshot_path": "search_result_1234567890.png"
  }
}
```

#### 提取网页内容

```bash
POST /api/task/extract_content
Content-Type: application/json

{
  "url": "https://example.com",
  "selectors": ["h1", "p.intro", ".price"]
}
```

#### 打开应用并输入

```bash
POST /api/task/open_and_type
Content-Type: application/json

{
  "app_name": "TextEdit",
  "content": "Meeting notes:\n- Item 1\n- Item 2"
}
```

## Python 调用示例

```python
import requests

# 服务器地址
BASE_URL = "http://localhost:3030"

# 1. 键盘输入
response = requests.post(
    f"{BASE_URL}/api/keyboard/type",
    json={"text": "Hello from Python!"}
)
print(response.json())
# {"success": true, "data": "键盘输入成功"}

# 2. 鼠标移动并点击
requests.post(f"{BASE_URL}/api/mouse/move", json={"x": 500, "y": 300})
requests.post(f"{BASE_URL}/api/mouse/click", json={"button": "left"})

# 3. 网页搜索（推荐使用任务型 API）
result = requests.post(
    f"{BASE_URL}/api/task/web_search",
    json={
        "query": "AI automation",
        "engine": "duckduckgo"
    }
)
data = result.json()
if data["success"]:
    task_result = data["data"]
    print(f"搜索完成，内容长度: {task_result['data']['content_length']}")
    print(f"截图保存在: {task_result['screenshot_path']}")

# 4. 提取网页内容
result = requests.post(
    f"{BASE_URL}/api/task/extract_content",
    json={
        "url": "https://news.ycombinator.com",
        "selectors": [".titleline > a", ".score"]
    }
)
print(result.json())
```

## Node.js 调用示例

```javascript
const axios = require('axios');

const BASE_URL = 'http://localhost:3030';

// 键盘输入
async function typeText(text) {
  const response = await axios.post(`${BASE_URL}/api/keyboard/type`, {
    text: text
  });
  return response.data;
}

// 网页搜索
async function webSearch(query, engine = 'duckduckgo') {
  const response = await axios.post(`${BASE_URL}/api/task/web_search`, {
    query: query,
    engine: engine
  });
  return response.data;
}

// 使用
(async () => {
  await typeText('Hello from Node.js!');
  
  const result = await webSearch('Rust automation');
  if (result.success) {
    console.log('搜索完成:', result.data.message);
    console.log('截图:', result.data.screenshot_path);
  }
})();
```

## 响应格式

所有 API 返回统一的 JSON 格式：

**成功响应**：
```json
{
  "success": true,
  "data": <具体数据>
}
```

**错误响应**：
```json
{
  "success": false,
  "error": "错误信息"
}
```

## 技术细节

### 并发处理

- 系统控制 API 使用 `tokio::task::spawn_blocking` 在独立线程执行，避免阻塞异步运行时
- 每个请求创建独立的自动化实例，无状态设计，天然支持并发

### 浏览器限制

由于浏览器状态无法跨 HTTP 请求保持，不提供低层浏览器 API（如 `browser.start`、`browser.goto`）。

**推荐方式**：使用任务型 API（`/api/task/*`），在单个请求内完成完整的浏览器任务。

### 性能优化

- 系统控制（鼠标/键盘）响应时间：**< 50ms**
- 任务型 API（浏览器）响应时间：**2-10s**（取决于网络和页面复杂度）

## 💡 最佳实践

1. **优先使用任务型 API**：封装了完整的工作流，更稳定可靠
2. **错误处理**：检查 `response.success` 字段，处理 `error` 信息
3. **超时设置**：浏览器任务可能耗时较长，建议设置 30-60s 超时
4. **并发控制**：虽然支持并发，但建议限制同时请求数（如 5-10 个）避免资源耗尽

---

## 🚀 性能参考

基于 macOS M1，测试数据：
- 鼠标移动：**0.56ms/次**，吞吐量 **1761 ops/s**
- 浏览器导航：**~1-3s**（取决于网络和页面复杂度）
- JavaScript 执行：**~10-50ms**
- HTTP API 响应（系统控制）：**< 50ms**
- HTTP API 响应（任务型）：**2-10s**

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

MIT
