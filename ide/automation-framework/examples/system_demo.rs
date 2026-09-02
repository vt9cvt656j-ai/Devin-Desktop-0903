use rust_automation_framework::prelude::*;
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    rust_automation_framework::init_logging();
    
    println!("🖱️ 系统自动化演示\n");
    
    let mut system = SystemAutomation::new()?;
    
    // 1. 鼠标移动演示
    println!("1. 移动鼠标到屏幕中心...");
    system.move_mouse(960, 540)?;
    thread::sleep(Duration::from_millis(500));
    
    // 2. 绘制正方形路径
    println!("2. 鼠标绘制正方形路径...");
    let positions = [
        (800, 400),
        (1120, 400),
        (1120, 680),
        (800, 680),
        (800, 400),
    ];
    
    for (x, y) in positions.iter() {
        system.move_mouse(*x, *y)?;
        thread::sleep(Duration::from_millis(200));
    }
    
    // 3. 点击演示
    println!("3. 执行点击...");
    system.click(MouseButton::Left)?;
    thread::sleep(Duration::from_millis(300));
    
    // 4. 双击演示
    println!("4. 执行双击...");
    system.double_click(MouseButton::Left)?;
    thread::sleep(Duration::from_millis(500));
    
    // 5. 键盘输入演示
    println!("5. 输入文本（请确保有焦点的文本框）...");
    thread::sleep(Duration::from_secs(2)); // 给用户时间打开文本编辑器
    
    system.type_text("Hello from Rust Automation Framework! ")?;
    thread::sleep(Duration::from_millis(300));
    
    // 6. 特殊按键
    println!("6. 按下 Enter 键...");
    system.press_key(Key::Return)?;
    thread::sleep(Duration::from_millis(300));
    
    system.type_text("支持中文输入！")?;
    thread::sleep(Duration::from_millis(300));
    
    // 7. 组合键演示（Ctrl+A 全选）
    println!("7. 组合键 Ctrl+A（全选）...");
    thread::sleep(Duration::from_secs(1));
    system.key_combination(vec![Key::Control, Key::Character('a')])?;
    thread::sleep(Duration::from_millis(500));
    
    // 8. 滚动演示
    println!("8. 鼠标滚轮滚动...");
    system.scroll(0, 3)?;
    thread::sleep(Duration::from_millis(300));
    system.scroll(0, -3)?;
    
    // 9. 拖拽演示（在屏幕上画一条线）
    println!("9. 拖拽演示...");
    thread::sleep(Duration::from_secs(1));
    system.drag(700, 400, 1000, 500)?;
    
    println!("\n✅ 系统自动化演示完成！");
    
    Ok(())
}
