use crate::types::*;
use serde_json;

#[test]
fn test_mouse_button_serialization() {
    let buttons = vec![MouseButton::Left, MouseButton::Right, MouseButton::Middle];
    
    for button in buttons {
        let json = serde_json::to_string(&button).unwrap();
        let deserialized: MouseButton = serde_json::from_str(&json).unwrap();
        assert_eq!(button, deserialized);
    }
}

#[test]
fn test_coordinate_mode() {
    let modes = vec![CoordinateMode::Absolute, CoordinateMode::Relative];
    
    for mode in modes {
        let json = serde_json::to_string(&mode).unwrap();
        let _deserialized: CoordinateMode = serde_json::from_str(&json).unwrap();
        // 无法直接比较，检查序列化成功即可
        assert!(!json.is_empty());
    }
}

#[test]
fn test_mouse_action_serialization() {
    let actions = vec![
        MouseAction::Move { x: 100, y: 200, mode: CoordinateMode::Absolute },
        MouseAction::Click { button: MouseButton::Left },
        MouseAction::DoubleClick { button: MouseButton::Right },
        MouseAction::Scroll { delta_x: 0, delta_y: -5 },
        MouseAction::Drag { from_x: 10, from_y: 20, to_x: 100, to_y: 200 },
    ];
    
    for action in actions {
        let json = serde_json::to_string(&action).unwrap();
        let _deserialized: MouseAction = serde_json::from_str(&json).unwrap();
        // 验证序列化往返成功
        assert!(!json.is_empty());
    }
}

#[test]
fn test_key_serialization() {
    let keys = vec![
        Key::Character('a'),
        Key::String("hello".into()),
        Key::Return,
        Key::Control,
        Key::F1,
    ];
    
    for key in keys {
        let json = serde_json::to_string(&key).unwrap();
        let _deserialized: Key = serde_json::from_str(&json).unwrap();
        assert!(!json.is_empty());
    }
}

#[test]
fn test_keyboard_action_serialization() {
    let actions = vec![
        KeyboardAction::Text("Hello World".into()),
        KeyboardAction::Press(Key::Return),
        KeyboardAction::Combination(vec![Key::Control, Key::Character('c')]),
    ];
    
    for action in actions {
        let json = serde_json::to_string(&action).unwrap();
        let _deserialized: KeyboardAction = serde_json::from_str(&json).unwrap();
        assert!(!json.is_empty());
    }
}

#[test]
#[cfg(feature = "browser")]
fn test_browser_action_serialization() {
    let actions = vec![
        BrowserAction::Navigate("https://example.com".into()),
        BrowserAction::Click("#button".into()),
        BrowserAction::Type { selector: "input".into(), text: "test".into() },
        BrowserAction::ExecuteScript("console.log('test')".into()),
        BrowserAction::Screenshot { path: Some("test.png".into()) },
    ];
    
    for action in actions {
        let json = serde_json::to_string(&action).unwrap();
        let _deserialized: BrowserAction = serde_json::from_str(&json).unwrap();
        assert!(!json.is_empty());
    }
}

#[test]
fn test_automation_command_serialization() {
    let commands = vec![
        AutomationCommand::Mouse(MouseAction::Click { button: MouseButton::Left }),
        AutomationCommand::Keyboard(KeyboardAction::Text("test".into())),
        AutomationCommand::Wait(1000),
        AutomationCommand::Batch(vec![
            AutomationCommand::Wait(500),
            AutomationCommand::Mouse(MouseAction::Move { x: 10, y: 10, mode: CoordinateMode::Absolute }),
        ]),
    ];
    
    for cmd in commands {
        let json = serde_json::to_string(&cmd).unwrap();
        let _deserialized: AutomationCommand = serde_json::from_str(&json).unwrap();
        assert!(!json.is_empty());
    }
}

#[test]
fn test_execution_result() {
    let result = ExecutionResult {
        success: true,
        message: Some("操作成功".into()),
        data: Some(serde_json::json!({"key": "value"})),
    };
    
    let json = serde_json::to_string(&result).unwrap();
    let deserialized: ExecutionResult = serde_json::from_str(&json).unwrap();
    
    assert_eq!(deserialized.success, true);
    assert_eq!(deserialized.message, Some("操作成功".into()));
    assert!(deserialized.data.is_some());
}

#[test]
fn test_window_info() {
    let info = WindowInfo {
        title: "Test Window".into(),
        process_name: "test.exe".into(),
        x: 100,
        y: 200,
        width: 800,
        height: 600,
        is_visible: true,
        // 「是不是前台」和「可见」是两回事：可见的窗口可以有很多个，前台只有一个，
        // 而合成按键只进前台那个。原来两边共用 is_visible，Windows 上它恒为 true，
        // 于是「谁在前台」永远是枚举到的第一个窗口。
        is_frontmost: false,
        is_minimized: false,
    };
    
    let json = serde_json::to_string(&info).unwrap();
    let deserialized: WindowInfo = serde_json::from_str(&json).unwrap();
    
    assert_eq!(deserialized.title, "Test Window");
    assert_eq!(deserialized.width, 800);
}

#[test]
fn test_screen_info() {
    let info = ScreenInfo {
        width: 1920,
        height: 1080,
        scale_factor: 2.0,
    };
    
    let json = serde_json::to_string(&info).unwrap();
    let deserialized: ScreenInfo = serde_json::from_str(&json).unwrap();
    
    assert_eq!(deserialized.width, 1920);
    assert_eq!(deserialized.height, 1080);
    assert_eq!(deserialized.scale_factor, 2.0);
}

/// cmd 必须解释成「**这个平台的**主修饰键」，不是死板地映射到 Meta。
///
/// 错了的后果在 Windows 上是静默的：enigo 把 Meta 编成 VK_LWIN，模型照工具描述里的
/// 示例发 ["cmd","s"] 想保存，实际按下 Win+S 打开系统搜索框，紧接着 keyboard.type
/// 把内容打进了搜索框。而 RPC 回 ok、还带 delivered_to，三层都在说成功。
/// 想按真正的 Windows 键仍然有说法：win / super / meta。
#[test]
fn cmd_means_this_platforms_primary_modifier() {
    use crate::agent::parse_key_for_test as parse_key;
    let primary = parse_key("cmd").expect("cmd 必须能解析");
    let win_key = parse_key("win").expect("win 必须能解析");

    #[cfg(target_os = "macos")]
    {
        assert_eq!(format!("{primary:?}"), format!("{:?}", enigo::Key::Meta), "mac 上 cmd = Meta");
    }
    #[cfg(not(target_os = "macos"))]
    {
        assert_eq!(format!("{primary:?}"), format!("{:?}", enigo::Key::Control),
            "非 mac 平台上 cmd 必须是 Ctrl —— 映射成 Meta 会按出 Win 键");
        assert_ne!(format!("{primary:?}"), format!("{win_key:?}"),
            "cmd 和 win 在 Windows 上必须是两个不同的键");
    }
    // 两个平台上 win 都指真正的 Windows/Command 键位（Meta）。
    assert_eq!(format!("{win_key:?}"), format!("{:?}", enigo::Key::Meta));
    // 平台中立的别名也要认，文案里推荐的就是它。
    assert_eq!(format!("{:?}", parse_key("mod").unwrap()), format!("{primary:?}"));
}
