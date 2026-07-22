use rust_automation_framework::prelude::*;
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    println!("🚀 Rust 自动化框架综合演示");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    // ============ 阶段 1: 系统自动化 - 文本编辑演示 ============
    println!("📝 阶段 1: 系统自动化 - 自动化文本编辑");
    let mut system = SystemAutomation::new()?;
    
    // 打开 TextEdit
    println!("  ↳ 启动 TextEdit");
    std::process::Command::new("open")
        .arg("-a")
        .arg("TextEdit")
        .spawn()?;
    thread::sleep(Duration::from_secs(2));
    
    // 新建文档
    println!("  ↳ 新建文档 (⌘N)");
    system.key_combination(vec![Key::Meta, Key::Character('n')])?;
    thread::sleep(Duration::from_millis(800));
    
    // 输入标题
    println!("  ↳ 输入演示内容");
    system.type_text("🦀 Rust 自动化框架演示\n")?;
    thread::sleep(Duration::from_millis(300));
    system.type_text("━━━━━━━━━━━━━━━━━━━━━━\n\n")?;
    thread::sleep(Duration::from_millis(300));
    
    // 输入时间戳
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    system.type_text(&format!("📅 演示时间: {}\n\n", now))?;
    thread::sleep(Duration::from_millis(300));
    
    // 展示不同功能
    system.type_text("✅ 系统自动化能力演示:\n\n")?;
    thread::sleep(Duration::from_millis(300));
    
    let features = vec![
        "1. 鼠标控制 - 移动/点击/拖拽",
        "2. 键盘输入 - 文本/按键/组合键",
        "3. 跨平台支持 - Windows/macOS",
        "4. 录制回放 - JSON 格式保存",
        "5. 浏览器自动化 - CDP 协议",
    ];
    
    for feature in features {
        system.type_text(feature)?;
        system.press_key(Key::Return)?;
        thread::sleep(Duration::from_millis(200));
    }
    
    thread::sleep(Duration::from_millis(500));
    system.type_text("\n\n💡 这段文字由 Rust 自动化框架自动输入！\n")?;
    
    println!("  ✅ 文本输入完成");
    thread::sleep(Duration::from_secs(1));
    
    // ============ 阶段 2: 鼠标演示 ============
    println!("\n🖱️  阶段 2: 鼠标精确控制演示");
    
    // 画一个正方形轨迹
    println!("  ↳ 绘制鼠标轨迹（正方形）");
    let start_x = 400;
    let start_y = 400;
    let size = 200;
    
    // 移动到起点
    system.move_mouse(start_x, start_y)?;
    thread::sleep(Duration::from_millis(500));
    
    // 画正方形（四条边）
    let corners = [
        (start_x + size, start_y),       // 右上
        (start_x + size, start_y + size), // 右下
        (start_x, start_y + size),        // 左下
        (start_x, start_y),               // 回到起点
    ];
    
    for (x, y) in corners.iter() {
        system.move_mouse(*x, *y)?;
        thread::sleep(Duration::from_millis(300));
    }
    
    println!("  ✅ 鼠标轨迹绘制完成");
    
    // ============ 阶段 3: 录制演示 ============
    println!("\n📼 阶段 3: 录制功能演示");
    let mut recording = Recording::new("综合演示录制");
    
    // 录制一系列操作
    println!("  ↳ 录制操作序列");
    recording.add_command(AutomationCommand::Mouse(MouseAction::Move {
        x: 960,
        y: 540,
        mode: CoordinateMode::Absolute,
    }));
    recording.add_command(AutomationCommand::Mouse(MouseAction::Click {
        button: MouseButton::Left,
    }));
    recording.add_command(AutomationCommand::Keyboard(KeyboardAction::Text(
        "这是录制的内容".to_string()
    )));
    recording.add_command(AutomationCommand::Keyboard(KeyboardAction::Press(Key::Return)));
    recording.add_command(AutomationCommand::Keyboard(KeyboardAction::Combination(vec![
        Key::Meta,
        Key::Character('a'),
    ])));
    
    // 保存录制
    let record_path = "showcase_recording.json";
    recording.save_to_file(record_path)?;
    println!("  ↳ 录制已保存: {}", record_path);
    
    // 读取并显示录制信息
    let loaded = Recording::load_from_file(record_path)?;
    println!("  ↳ 录制包含 {} 条命令", loaded.commands.len());
    println!("  ↳ 录制名称: {}", loaded.name);
    println!("  ↳ 操作系统: {}", loaded.metadata.os);
    
    // 显示录制内容摘要
    println!("\n  📋 录制内容摘要:");
    for (i, cmd) in loaded.commands.iter().enumerate() {
        let desc = match cmd {
            AutomationCommand::Mouse(action) => match action {
                MouseAction::Move { .. } => "鼠标移动",
                MouseAction::Click { .. } => "鼠标点击",
                MouseAction::DoubleClick { .. } => "鼠标双击",
                MouseAction::Down { .. } => "按下鼠标",
                MouseAction::Up { .. } => "释放鼠标",
                MouseAction::Drag { .. } => "鼠标拖拽",
                MouseAction::Scroll { .. } => "鼠标滚动",
            },
            AutomationCommand::Keyboard(action) => match action {
                KeyboardAction::Text(_) => "键盘输入",
                KeyboardAction::Press(_) => "按键",
                KeyboardAction::Combination(_) => "组合键",
                KeyboardAction::Down(_) => "按下按键",
                KeyboardAction::Up(_) => "释放按键",
            },
            AutomationCommand::Wait(_) => "等待",
            AutomationCommand::WaitUntil { .. } => "条件等待",
            AutomationCommand::Batch(_) => "批量操作",
            #[cfg(feature = "browser")]
            AutomationCommand::Browser(_) => "浏览器操作",
        };
        println!("     {}. {}", i + 1, desc);
    }
    
    // ============ 阶段 4: 回放演示 ============
    println!("\n▶️  阶段 4: 回放演示（演示前 3 个操作）");
    let mut replayer = Replayer::new(loaded);
    
    for step in 0..3 {
        if let Some(cmd) = replayer.next_command() {
            println!("  ↳ 执行第 {} 步: {:?}", step + 1, cmd);
            thread::sleep(Duration::from_millis(500));
        }
    }
    
    let progress = replayer.progress();
    println!("  ✅ 回放进度: {}/{}", progress.0, progress.1);
    
    // ============ 阶段 5: 性能测试 ============
    println!("\n⚡ 阶段 5: 性能测试");
    
    let iterations = 100;
    let start = std::time::Instant::now();
    
    for _ in 0..iterations {
        system.move_mouse(500, 500)?;
    }
    
    let duration = start.elapsed();
    let ops_per_sec = iterations as f64 / duration.as_secs_f64();
    
    println!("  ↳ {} 次鼠标移动操作", iterations);
    println!("  ↳ 总耗时: {:.2}ms", duration.as_millis());
    println!("  ↳ 平均延迟: {:.2}ms/次", duration.as_millis() as f64 / iterations as f64);
    println!("  ↳ 吞吐量: {:.0} 操作/秒", ops_per_sec);
    
    // ============ 总结 ============
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎉 演示完成！\n");
    println!("📊 演示总结:");
    println!("  ✅ 系统自动化 - 文本输入、快捷键");
    println!("  ✅ 鼠标控制 - 精确移动、轨迹绘制");
    println!("  ✅ 录制功能 - 保存为 JSON 格式");
    println!("  ✅ 回放功能 - 逐步执行录制内容");
    println!("  ✅ 性能测试 - 高吞吐量操作");
    println!("\n💾 生成文件:");
    println!("  • showcase_recording.json - 操作录制文件");
    println!("  • TextEdit 中的自动生成文档");
    
    Ok(())
}
