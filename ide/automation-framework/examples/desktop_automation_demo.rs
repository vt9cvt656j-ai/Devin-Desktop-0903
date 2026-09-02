//! 桌面应用自动化演示
//! 
//! 展示如何使用系统级自动化控制桌面应用

use rust_automation_framework::Agent;
use std::error::Error;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn Error>> {
    println!("🖥️  桌面应用自动化演示");
    println!("{}", "=".repeat(60));
    
    let mut agent = Agent::new()?;
    agent.system_init()?;
    
    println!("\n📍 第1步：鼠标控制演示");
    println!("   移动到屏幕中心...");
    agent.mouse_move(960, 540)?;
    thread::sleep(Duration::from_millis(500));
    
    println!("   绘制正方形轨迹...");
    let base_x = 960;
    let base_y = 540;
    let size = 150;
    
    agent.mouse_move(base_x + size, base_y)?;
    thread::sleep(Duration::from_millis(200));
    agent.mouse_move(base_x + size, base_y + size)?;
    thread::sleep(Duration::from_millis(200));
    agent.mouse_move(base_x, base_y + size)?;
    thread::sleep(Duration::from_millis(200));
    agent.mouse_move(base_x, base_y)?;
    thread::sleep(Duration::from_millis(200));
    
    println!("\n⌨️  第2步：键盘输入演示");
    println!("   等待3秒，请切换到任意文本编辑器（TextEdit/记事本）...");
    thread::sleep(Duration::from_secs(3));
    
    agent.keyboard_type("🖥️  桌面自动化演示\n")?;
    thread::sleep(Duration::from_millis(300));
    agent.keyboard_type("时间: ")?;
    agent.keyboard_type(&chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())?;
    agent.keyboard_type("\n\n功能列表:\n")?;
    
    let features = vec![
        "• 鼠标精确移动和点击",
        "• 键盘文本输入和快捷键",
        "• 拖拽操作",
        "• 滚动控制",
        "• 剪贴板操作",
    ];
    
    for feature in features {
        agent.keyboard_type(feature)?;
        agent.keyboard_type("\n")?;
        thread::sleep(Duration::from_millis(100));
    }
    
    println!("\n📋 第3步：剪贴板操作");
    let test_text = "复制粘贴测试：Rust Automation Framework";
    agent.clipboard_set_text(test_text)?;
    println!("   已写入剪贴板: {}", test_text);
    
    let clipboard_content = agent.clipboard_get_text()?;
    println!("   读取剪贴板: {}", clipboard_content);
    
    println!("\n🔄 第4步：组合键演示");
    thread::sleep(Duration::from_secs(1));
    agent.keyboard_combo(vec!["cmd", "a"])?;  // 全选（macOS用cmd，Windows用ctrl）
    thread::sleep(Duration::from_millis(300));
    
    #[cfg(target_os = "windows")]
    agent.keyboard_combo(vec!["ctrl", "c"])?;  // 复制
    
    #[cfg(target_os = "macos")]
    agent.keyboard_combo(vec!["cmd", "c"])?;  // 复制
    
    println!("   已执行：全选 + 复制");
    
    println!("\n{}", "=".repeat(60));
    println!("✅ 演示完成！");
    println!("\n💡 核心能力：");
    println!("   • 精确的鼠标控制（绝对/相对坐标）");
    println!("   • 完整的键盘输入（文本/按键/组合键）");
    println!("   • 剪贴板读写操作");
    println!("   • 跨平台兼容（Windows/macOS）");
    
    Ok(())
}
