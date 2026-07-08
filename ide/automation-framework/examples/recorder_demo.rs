use rust_automation_framework::prelude::*;
use std::thread;
use std::time::Duration;

/// 演示录制和回放功能
fn main() -> Result<()> {
    println!("🎬 自动化录制/回放演示\n");

    // 创建录制
    let mut recording = Recording::new("demo_recording");
    recording.metadata.description = Some("演示鼠标和键盘操作".to_string());

    println!("📝 开始录制操作序列...");

    // 录制一系列操作
    recording.add_command(AutomationCommand::Mouse(MouseAction::Move {
        x: 500,
        y: 300,
        mode: CoordinateMode::Absolute,
    }));

    recording.add_command(AutomationCommand::Mouse(MouseAction::Click {
        button: MouseButton::Left,
    }));

    recording.add_command(AutomationCommand::Keyboard(KeyboardAction::Text(
        "Hello from Recording!".to_string()
    )));

    recording.add_command(AutomationCommand::Keyboard(KeyboardAction::Press(
        Key::Return
    )));

    recording.add_command(AutomationCommand::Mouse(MouseAction::Move {
        x: 600,
        y: 400,
        mode: CoordinateMode::Absolute,
    }));

    recording.add_command(AutomationCommand::Mouse(MouseAction::DoubleClick {
        button: MouseButton::Left,
    }));

    println!("✅ 录制完成！共 {} 个操作", recording.len());

    // 保存录制到文件
    let save_path = "demo_recording.json";
    recording.save_to_file(save_path)?;
    println!("💾 已保存到: {}", save_path);

    println!("\n⏸️  等待 2 秒后开始回放...\n");
    thread::sleep(Duration::from_secs(2));

    // 从文件加载并回放
    println!("▶️  开始回放录制...");
    let mut replayer = Replayer::from_file(save_path)?;

    println!("📊 录制信息:");
    println!("  名称: {}", replayer.recording().name);
    println!("  操作系统: {}", replayer.recording().metadata.os);
    if let Some(desc) = &replayer.recording().metadata.description {
        println!("  描述: {}", desc);
    }
    let total_commands = replayer.recording().len();
    println!("  命令数: {}", total_commands);
    println!();

    // 创建系统自动化实例
    let mut system = SystemAutomation::new()?;

    // 逐步回放
    let mut current_index = 0;
    while let Some(command) = replayer.next_command() {
        current_index += 1;
        println!("[{}/{}] 执行: {:?}", current_index, total_commands, command);

        match command {
            AutomationCommand::Mouse(action) => match action {
                MouseAction::Move { x, y, mode } => {
                    match mode {
                        CoordinateMode::Absolute => system.move_mouse(*x, *y)?,
                        CoordinateMode::Relative => system.move_mouse_relative(*x, *y)?,
                    }
                }
                MouseAction::Click { button } => {
                    system.click(*button)?;
                }
                MouseAction::DoubleClick { button } => {
                    system.double_click(*button)?;
                }
                MouseAction::Drag { from_x, from_y, to_x, to_y } => {
                    system.drag(*from_x, *from_y, *to_x, *to_y)?;
                }
                MouseAction::Scroll { delta_x, delta_y } => {
                    system.scroll(*delta_x, *delta_y)?;
                }
                MouseAction::Down { .. } | MouseAction::Up { .. } => {
                    println!("  ⚠️  跳过 Down/Up 命令（demo 未实现）");
                }
            },
            AutomationCommand::Keyboard(action) => match action {
                KeyboardAction::Text(text) => {
                    system.type_text(text)?;
                }
                KeyboardAction::Press(key) => {
                    system.press_key(key.clone())?;
                }
                KeyboardAction::Combination(keys) => {
                    system.key_combination(keys.clone())?;
                }
                KeyboardAction::Down(_) | KeyboardAction::Up(_) => {
                    println!("  ⚠️  跳过 Down/Up 命令（demo 未实现）");
                }
            },
            AutomationCommand::Browser(_) => {
                println!("  ⚠️  跳过浏览器命令（需要浏览器实例）");
            }
            AutomationCommand::Wait(_) 
            | AutomationCommand::WaitUntil { .. } 
            | AutomationCommand::Batch(_) => {
                println!("  ⚠️  跳过高级命令（demo 未实现）");
            }
        }

        // 操作间隔
        thread::sleep(Duration::from_millis(300));
    }

    println!("\n✅ 回放完成！");
    
    // 演示重置和重新回放
    println!("\n🔄 重置回放器...");
    replayer.reset();
    let (current, total) = replayer.progress();
    println!("📊 重置后进度: {}/{}", current, total);
    
    println!("\n💡 提示: 录制文件已保存到 {}", save_path);
    println!("   可以编辑 JSON 文件来修改操作序列");

    Ok(())
}
