use rust_automation_framework::prelude::*;
use std::thread;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    rust_automation_framework::init_logging();
    
    println!("🚀 混合自动化演示：浏览器 + 系统控制\n");
    println!("场景：自动化网页截图并使用系统级操作处理\n");
    
    // 初始化两个控制器
    let mut browser = BrowserAutomation::new_headed().await?;
    let mut system = SystemAutomation::new()?;
    
    // === 第一部分：浏览器自动化 ===
    println!("📌 第一阶段：浏览器操作");
    
    println!("1. 打开网页...");
    browser.navigate("https://www.rust-lang.org").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    println!("2. 截图保存...");
    browser.screenshot(Some("rust_homepage.png")).await?;
    
    println!("3. 执行页面滚动...");
    browser.scroll(0, 500).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    browser.screenshot(Some("rust_homepage_scrolled.png")).await?;
    
    // === 第二部分：系统级操作 ===
    println!("\n📌 第二阶段：系统级操作");
    
    println!("4. 使用快捷键截图当前窗口（macOS: Cmd+Shift+4, Windows: Win+Shift+S）...");
    thread::sleep(Duration::from_secs(1));
    
    #[cfg(target_os = "macos")]
    system.key_combination(vec![Key::Meta, Key::Shift, Key::Character('4')])?;
    
    #[cfg(target_os = "windows")]
    system.key_combination(vec![Key::Meta, Key::Shift, Key::Character('s')])?;
    
    thread::sleep(Duration::from_secs(2));
    
    // === 第三部分：组合操作 ===
    println!("\n📌 第三阶段：组合操作");
    
    println!("5. 在浏览器中搜索...");
    browser.navigate("https://github.com/search?q=rust+automation").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    println!("6. 使用系统控制移动鼠标到页面特定位置...");
    system.move_mouse(960, 300)?;
    thread::sleep(Duration::from_millis(500));
    
    println!("7. 模拟鼠标点击（与浏览器交互）...");
    system.click(MouseButton::Left)?;
    thread::sleep(Duration::from_millis(500));
    
    println!("8. 使用键盘导航（Tab 键切换焦点）...");
    for _ in 0..3 {
        system.press_key(Key::Tab)?;
        thread::sleep(Duration::from_millis(200));
    }
    
    println!("9. 最终截图...");
    browser.screenshot(Some("github_search.png")).await?;
    
    // === 第四部分：窗口控制（平台特定） ===
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        use rust_automation_framework::platform::*;
        
        println!("\n📌 第四阶段：窗口控制（平台特定）");
        
        let window_ctrl = get_window_controller();
        
        println!("10. 获取屏幕信息...");
        let screen_info = window_ctrl.get_screen_info()?;
        println!("   屏幕尺寸: {}x{}", screen_info.width, screen_info.height);
        println!("   缩放因子: {:.2}", screen_info.scale_factor);
        
        println!("11. 枚举当前打开的窗口...");
        let windows = window_ctrl.enumerate_windows()?;
        println!("   找到 {} 个窗口", windows.len());
        
        for (i, win) in windows.iter().take(5).enumerate() {
            println!("   [{}] {}", i + 1, win.title);
        }
    }
    
    println!("\n✅ 混合自动化演示完成！");
    println!("\n生成的文件：");
    println!("  - rust_homepage.png");
    println!("  - rust_homepage_scrolled.png");
    println!("  - github_search.png");
    
    // 清理
    browser.close().await?;
    
    Ok(())
}
