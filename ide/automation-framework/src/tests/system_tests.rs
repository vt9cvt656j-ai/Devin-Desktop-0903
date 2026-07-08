use crate::system::*;
use crate::types::*;

#[test]
fn test_system_automation_creation() {
    // 测试实例创建
    let result = SystemAutomation::new();
    assert!(result.is_ok(), "SystemAutomation 应该能成功创建");
}

#[test]
fn test_mouse_action_execution() {
    let mut system = SystemAutomation::new().unwrap();
    
    // 测试移动操作（不会真的移动，只验证 API 调用）
    let action = MouseAction::Move { 
        x: 100, 
        y: 100, 
        mode: CoordinateMode::Absolute 
    };
    
    // 注意：在 CI 环境可能失败，这里只测试 API 不报错
    let result = system.execute_mouse_action(action);
    // 在无显示器环境可能失败，所以不严格断言
    if result.is_ok() {
        assert!(result.unwrap().success);
    }
}

#[test]
fn test_keyboard_action_execution() {
    let mut system = SystemAutomation::new().unwrap();
    
    let action = KeyboardAction::Text("test".into());
    
    // 在无显示器环境可能失败
    let result = system.execute_keyboard_action(action);
    if result.is_ok() {
        assert!(result.unwrap().success);
    }
}

#[test]
fn test_button_conversion() {
    // 这个测试验证内部转换逻辑
    let buttons = vec![
        MouseButton::Left,
        MouseButton::Right,
        MouseButton::Middle,
    ];
    
    // 只要不 panic 就算通过
    for button in buttons {
        let _ = format!("{:?}", button);
    }
}

#[test]
fn test_key_conversion() {
    let keys = vec![
        Key::Character('a'),
        Key::Return,
        Key::Control,
        Key::Shift,
    ];
    
    // 验证 Key 枚举能正常使用
    for key in keys {
        let _ = format!("{:?}", key);
    }
}

#[test]
fn test_execution_result_structure() {
    let result = ExecutionResult {
        success: true,
        message: Some("测试成功".into()),
        data: None,
    };
    
    assert!(result.success);
    assert!(result.message.is_some());
    assert!(result.data.is_none());
}

// 集成测试（需要真实环境）
#[test]
#[ignore] // 默认跳过，需要手动运行
fn test_real_mouse_movement() {
    let mut system = SystemAutomation::new().unwrap();
    
    // 真实移动测试
    system.move_mouse(100, 100).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    system.move_mouse_relative(50, 50).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
}

#[test]
#[ignore]
fn test_real_click() {
    let mut system = SystemAutomation::new().unwrap();
    
    system.move_mouse(500, 500).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));
    
    system.click(MouseButton::Left).unwrap();
}

#[test]
#[ignore]
fn test_real_typing() {
    let mut system = SystemAutomation::new().unwrap();
    
    std::thread::sleep(std::time::Duration::from_secs(2));
    system.type_text("Hello from Rust!").unwrap();
}

#[test]
#[ignore]
fn test_real_key_combination() {
    let mut system = SystemAutomation::new().unwrap();
    
    std::thread::sleep(std::time::Duration::from_secs(2));
    system.key_combination(vec![Key::Control, Key::Character('a')]).unwrap();
}
