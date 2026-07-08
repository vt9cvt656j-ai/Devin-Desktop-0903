# Rust 自动化框架

🚀 **跨平台自动化框架** - 支持 Windows 和 macOS，提供浏览器和系统级自动化能力。

## ✨ 特性

- 🌐 **浏览器自动化**：基于 Chrome DevTools Protocol (CDP)
  - 页面导航、元素操作、JavaScript 执行
  - 自动化表单填写、点击、输入
Hello from Rust Automation Framework! 
支持中文输入！支持中文输入！  
- 🖱️ **系统级自动化**：跨平台鼠标键盘控制
  - 鼠标移动、点击、双击、拖拽、滚动
  - 键盘输入、按键、组合键（如 Ctrl+C）
  - 支持绝对和相对坐标
  
- 🪟 **平台原生集成**
  - **Windows**：窗口枚举、激活、最小化/最大化
  - **macOS**：应用切换、屏幕信息获取
  
- 🔄 **混合自动化**：浏览器 + 系统控制组合使用

## 📦 快速开始

### 前置要求

- Rust 1.70+
- Chrome/Chromium 浏览器（浏览器自动化需要）

### 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
rust-automation-framework = { path = ".", features = ["full"] }
tokio = { version = "1", features = ["full"] }
```

### 功能特性选择

```toml
# 只使用浏览器自动化
rust-automation-framework = { path = ".", features = ["browser"] }

# 只使用系统自动化
rust-automation-framework = { path = ".", features = ["system"] }

# 全功能
rust-automation-framework = { path = ".", features = ["full"] }
```

## 🎯 使用示例

### 浏览器自动化

```rust
use rust_automation_framework::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // 启动浏览器（可见窗口）
    let mut browser = BrowserAutomation::new_headed().await?;
    
    // 导航到网页
    browser.navigate("https://example.com").await?;
    
    // 等待元素出现
    browser.wait_for_element("input[name='q']", 5000).await?;
    
    // 输入文本
    browser.type_text("input[name='q']", "Rust programming").await?;
    
    // 点击按钮
    browser.click_element("button[type='submit']").await?;
    
    // 截图
    browser.screenshot(Some("result.png")).await?;
    
    // 执行 JavaScript
    let title = browser.execute_script("document.title").await?;
    println!("页面标题: {}", title);
    
    browser.close().await?;
    Ok(())
}
```

### 系统自动化

```rust
use rust_automation_framework::prelude::*;
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    let mut system = SystemAutomation::new()?;
    
    // 移动鼠标到指定位置
    system.move_mouse(500, 300)?;
    thread::sleep(Duration::from_millis(500));
    
    // 点击
    system.click(MouseButton::Left)?;
    
    // 输入文本
    system.type_text("Hello, Rust!")?;
    
    // 按下回车键
    system.press_key(Key::Return)?;
    
    // 组合键（Ctrl+A 全选）
    system.key_combination(vec![Key::Control, Key::Character('a')])?;
    
    // 拖拽操作
    system.drag(100, 100, 300, 300)?;
    
    // 滚动
    system.scroll(0, 5)?;
    
    Ok(())
}
```

### 混合自动化

```rust
use rust_automation_framework::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // 同时使用浏览器和系统控制
    let mut browser = BrowserAutomation::new_headed().await?;
    let mut system = SystemAutomation::new()?;
    
    // 浏览器打开网页
    browser.navigate("https://example.com").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // 使用系统控制移动鼠标（精确控制）
    system.move_mouse(960, 300)?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    // 系统级点击
    system.click(MouseButton::Left)?;
    
    // 浏览器截图
    browser.screenshot(Some("combined.png")).await?;
    
    browser.close().await?;
    Ok(())
}
```

### 平台特定功能

```rust
#[cfg(any(target_os = "windows", target_os = "macos"))]
use rust_automation_framework::platform::*;

fn main() -> Result<()> {
    let window_ctrl = get_window_controller();
    
    // 获取屏幕信息
    let screen = window_ctrl.get_screen_info()?;
    println!("屏幕尺寸: {}x{}", screen.width, screen.height);
    println!("缩放因子: {:.2}", screen.scale_factor);
    
    // 枚举窗口
    let windows = window_ctrl.enumerate_windows()?;
    for win in windows.iter().take(5) {
        println!("窗口: {}", win.title);
    }
    
    // 激活指定窗口
    window_ctrl.activate_window("Chrome")?;
    
    Ok(())
}
```

## 🎬 录制与回放

框架支持录制用户操作并序列化为 JSON，方便回放、编辑和分享自动化脚本。

### 录制操作

```rust
use rust_automation_framework::prelude::*;

fn main() -> Result<()> {
    // 创建录制
    let mut recording = Recording::new("my_workflow");
    recording.metadata.description = Some("日常工作流程".to_string());
    
    // 添加操作命令
    recording.add_command(AutomationCommand::Mouse(MouseAction::Move {
        x: 500,
        y: 300,
        mode: CoordinateMode::Absolute,
    }));
    
    recording.add_command(AutomationCommand::Mouse(MouseAction::Click {
        button: MouseButton::Left,
    }));
    
    recording.add_command(AutomationCommand::Keyboard(KeyboardAction::Text(
        "Hello, World!".to_string()
    )));
    
    // 保存到文件
    recording.save_to_file("workflow.json")?;
    println!("录制已保存，共 {} 个操作", recording.len());
    
    Ok(())
}
```

### 回放录制

```rust
use rust_automation_framework::prelude::*;

fn main() -> Result<()> {
    // 加载录制
    let mut replayer = Replayer::from_file("workflow.json")?;
    
    // 查看录制信息
    println!("录制名称: {}", replayer.recording().name);
    println!("命令数: {}", replayer.recording().len());
    
    // 创建自动化实例
    let mut system = SystemAutomation::new()?;
    
    // 逐步回放
    while let Some(command) = replayer.next_command() {
        match command {
            AutomationCommand::Mouse(action) => {
                // 执行鼠标操作
            }
            AutomationCommand::Keyboard(action) => {
                // 执行键盘操作
            }
            _ => {}
        }
        
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    
    println!("回放完成！");
    Ok(())
}
```

### 编辑录制文件

录制文件是标准 JSON 格式，可以手动编辑：

```json
{
  "name": "my_workflow",
  "commands": [
    {
      "Mouse": {
        "Move": {
          "x": 500,
          "y": 300,
          "mode": "Absolute"
        }
      }
    },
    {
      "Mouse": {
        "Click": {
          "button": "Left"
        }
      }
    }
  ],
  "timestamp": 1704067200,
  "metadata": {
    "os": "macos",
    "screen_resolution": null,
    "description": "日常工作流程"
  }
}
```

## 📚 API 文档

### BrowserAutomation

| 方法 | 说明 |
|------|------|
| `new()` | 创建无头浏览器实例 |
| `new_headed()` | 创建可见浏览器实例 |
| `navigate(url)` | 导航到指定 URL |
| `click_element(selector)` | 点击元素（CSS 选择器）|
| `type_text(selector, text)` | 在元素中输入文本 |
| `wait_for_element(selector, timeout)` | 等待元素出现 |
| `execute_script(script)` | 执行 JavaScript |
| `screenshot(path)` | 页面截图 |
| `get_content()` | 获取页面 HTML |
| `scroll(x, y)` | 滚动页面 |

### SystemAutomation

| 方法 | 说明 |
|------|------|
| `move_mouse(x, y)` | 移动鼠标到绝对坐标 |
| `move_mouse_relative(dx, dy)` | 相对移动鼠标 |
| `click(button)` | 鼠标点击 |
| `double_click(button)` | 鼠标双击 |
| `drag(from_x, from_y, to_x, to_y)` | 拖拽操作 |
| `scroll(delta_x, delta_y)` | 滚动 |
| `type_text(text)` | 输入文本 |
| `press_key(key)` | 按键 |
| `key_combination(keys)` | 组合键 |

## 🏗️ 架构设计

```
rust-automation-framework/
├── src/
│   ├── lib.rs              # 库入口
│   ├── error.rs            # 错误类型定义
│   ├── types.rs            # 公共类型
│   ├── browser.rs          # 浏览器自动化（CDP）
│   ├── system.rs           # 系统自动化（enigo）
│   └── platform/           # 平台特定实现
│       ├── mod.rs
│       ├── windows.rs      # Windows 原生 API
│       └── macos.rs        # macOS Cocoa/objc
├── examples/               # 示例代码
│   ├── browser_demo.rs
│   ├── system_demo.rs
│   └── hybrid_demo.rs
└── Cargo.toml
```

### 技术栈

- **浏览器自动化**：[chromiumoxide](https://github.com/mattsse/chromiumoxide) - Chrome DevTools Protocol
- **系统输入模拟**：[enigo](https://github.com/enigo-rs/enigo) - 跨平台鼠标键盘控制
- **Windows 平台**：[windows-rs](https://github.com/microsoft/windows-rs) - Win32 API 绑定
- **macOS 平台**：cocoa + objc - Cocoa 框架绑定
- **异步运行时**：[tokio](https://tokio.rs/)
- **错误处理**：[thiserror](https://github.com/dtolnay/thiserror) + [anyhow](https://github.com/dtolnay/anyhow)

## 🚀 运行示例

```bash
# 浏览器自动化演示
cargo run --example browser_demo --features browser

# 系统自动化演示
cargo run --example system_demo --features system

# 混合自动化演示
cargo run --example hybrid_demo --features full

# 录制/回放演示
cargo run --example recorder_demo --features system
```

## 🔧 平台依赖

### macOS

无需额外依赖，已包含 Cocoa 框架绑定。

### Windows

需要 Windows SDK（通过 windows-rs 自动处理）。

### Chrome/Chromium

浏览器自动化需要系统安装 Chrome 或 Chromium：

- **macOS**: `brew install --cask google-chrome`
- **Windows**: 从 [chrome.google.com](https://www.google.com/chrome/) 下载安装

## 📝 许可证

MIT

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## ⚠️ 注意事项

1. **浏览器自动化**需要系统安装 Chrome/Chromium
2. **系统自动化**需要相应的权限：
   - macOS: 辅助功能权限（Accessibility）
   - Windows: 某些操作可能需要管理员权限
3. **跨平台兼容性**：平台特定功能仅在对应平台可用
