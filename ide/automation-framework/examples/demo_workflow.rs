use rust_automation_framework::prelude::*;
use std::thread;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 启动综合自动化演示：搜索技术文档 → 提取内容 → 写入笔记");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // ============ 阶段 1: 浏览器自动化 ============
    println!("\n📱 阶段 1: 浏览器自动化");
    let mut browser = BrowserAutomation::new_headed().await?;
    
    // 搜索 Rust async/await 教程
    println!("  ↳ 导航到 DuckDuckGo");
    browser.navigate("https://duckduckgo.com").await?;
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    println!("  ↳ 输入搜索关键词：Rust async await tutorial");
    browser.type_text("input[name='q']", "Rust async await tutorial").await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // 执行搜索
    println!("  ↳ 提交搜索");
    let script = r#"
        const form = document.querySelector('form');
        if (form) form.submit();
    "#;
    browser.execute_script(script).await?;
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // 截图保存
    println!("  ↳ 保存搜索结果截图");
    browser.screenshot(Some("search_results.png")).await?;
    
    // 提取搜索结果标题
    println!("  ↳ 提取搜索结果");
    let extract_script = r#"
        const results = Array.from(document.querySelectorAll('article h2, .result__title'))
            .slice(0, 5)
            .map(el => el.textContent.trim())
            .filter(t => t.length > 0);
        JSON.stringify(results);
    "#;
    let results = browser.execute_script(extract_script).await?;
    
    // 解析结果
    let titles: Vec<String> = serde_json::from_value(results)
        .unwrap_or_else(|_| vec!["搜索结果加载中...".to_string()]);
    
    println!("  ✅ 提取到 {} 条结果", titles.len());
    for (i, title) in titles.iter().enumerate() {
        println!("     {}. {}", i + 1, title);
    }
    
    browser.close().await?;
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // ============ 阶段 2: 系统自动化 - 写入笔记 ============
    println!("\n⌨️  阶段 2: 系统自动化 - 写入笔记");
    let mut system = SystemAutomation::new()?;
    
    // 打开 TextEdit (macOS)
    println!("  ↳ 启动文本编辑器");
    std::process::Command::new("open")
        .arg("-a")
        .arg("TextEdit")
        .spawn()?;
    thread::sleep(Duration::from_secs(2));
    
    // 新建文档
    println!("  ↳ 创建新文档 (Cmd+N)");
    system.key_combination(vec![Key::Meta, Key::Character('n')])?;
    thread::sleep(Duration::from_millis(500));
    
    // 输入标题
    println!("  ↳ 输入笔记内容");
    system.type_text("🦀 Rust Async/Await 学习笔记\n")?;
    thread::sleep(Duration::from_millis(300));
    system.type_text("━━━━━━━━━━━━━━━━━━━━━━\n\n")?;
    thread::sleep(Duration::from_millis(300));
    
    // 输入时间戳
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    system.type_text(&format!("📅 创建时间: {}\n\n", now))?;
    thread::sleep(Duration::from_millis(300));
    
    // 输入搜索结果
    system.type_text("🔍 搜索结果:\n\n")?;
    thread::sleep(Duration::from_millis(300));
    
    for (i, title) in titles.iter().enumerate() {
        let line = format!("{}. {}\n", i + 1, title);
        system.type_text(&line)?;
        thread::sleep(Duration::from_millis(200));
    }
    
    thread::sleep(Duration::from_millis(500));
    system.type_text("\n\n✅ 自动化任务完成！\n")?;
    
    println!("  ✅ 笔记内容已输入");
    
    // ============ 阶段 3: 录制演示 ============
    println!("\n📼 阶段 3: 录制演示");
    let mut recording = Recording::new("综合演示流程".to_string());
    
    // 录制一段鼠标操作序列
    recording.add_command(AutomationCommand::Mouse(MouseAction::Move {
        x: 500,
        y: 300,
        mode: CoordinateMode::Absolute,
    }));
    recording.add_command(AutomationCommand::Mouse(MouseAction::Click {
        button: MouseButton::Left,
    }));
    recording.add_command(AutomationCommand::Keyboard(KeyboardAction::Text(
        "这是录制的操作".to_string()
    )));
    
    // 保存录制
    let record_path = "demo_recording.json";
    recording.save_to_file(record_path)?;
    println!("  ↳ 录制文件已保存: {}", record_path);
    
    // 显示录制内容
    let loaded = Recording::load_from_file(record_path)?;
    println!("  ↳ 录制包含 {} 条命令", loaded.commands.len());
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎉 综合演示完成！");
    println!("\n📊 演示总结:");
    println!("  ✅ 浏览器自动化: 搜索 + 截图 + 内容提取");
    println!("  ✅ 系统自动化: 打开应用 + 键盘输入 + 组合键");
    println!("  ✅ 录制功能: 保存操作序列到 JSON");
    println!("\n💾 生成文件:");
    println!("  • search_results.png - 搜索结果截图");
    println!("  • demo_recording.json - 操作录制文件");
    println!("  • TextEdit 中的自动生成笔记");
    
    Ok(())
}
