//! UI Automation 完整演示
//! 
//! 展示如何使用 UI Automation API 查找和操作桌面应用的界面元素

use rust_automation_framework::Agent;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 UI Automation 演示");
    println!("============================================================\n");

    let mut agent = Agent::new()?;
    agent.system_init()?;

    // 第1步：打开计算器应用
    println!("📱 第1步：打开计算器应用");
    #[cfg(target_os = "macos")]
    {
        agent.keyboard_combo(vec!["cmd", "space"])?;
        thread::sleep(Duration::from_millis(500));
        agent.keyboard_type("Calculator")?;
        thread::sleep(Duration::from_millis(300));
        agent.keyboard_press("return")?;
        println!("   ✅ 已启动计算器\n");
    }
    #[cfg(target_os = "windows")]
    {
        agent.keyboard_combo(vec!["win"])?;
        thread::sleep(Duration::from_millis(500));
        agent.keyboard_type("Calculator")?;
        thread::sleep(Duration::from_millis(300));
        agent.keyboard_press("return")?;
        println!("   ✅ 已启动计算器\n");
    }

    // 等待应用完全启动
    thread::sleep(Duration::from_secs(2));

    // 第2步：查找窗口中的元素
    println!("🔍 第2步：查找 UI 元素");
    
    #[cfg(target_os = "macos")]
    let app_name = "Calculator";
    #[cfg(target_os = "windows")]
    let app_name = "计算器";

    match agent.desktop_find_element(app_name, "7") {
        Ok(Some(element)) => {
            println!("   ✅ 找到元素: {}", element.name);
            println!("      类型: {}", element.element_type);
            println!("      位置: ({}, {})", element.x, element.y);
            println!("      可见: {}", element.is_visible);
            println!("      启用: {}\n", element.is_enabled);

            // 第3步：点击元素
            println!("👆 第3步：点击按钮");
            agent.desktop_click_element(&element)?;
            println!("   ✅ 已点击按钮 '7'\n");
            thread::sleep(Duration::from_millis(500));

            // 继续点击其他按钮演示计算
            if let Ok(Some(plus)) = agent.desktop_find_element(app_name, "+") {
                agent.desktop_click_element(&plus)?;
                println!("   ✅ 已点击按钮 '+'\n");
                thread::sleep(Duration::from_millis(300));
            }

            if let Ok(Some(three)) = agent.desktop_find_element(app_name, "3") {
                agent.desktop_click_element(&three)?;
                println!("   ✅ 已点击按钮 '3'\n");
                thread::sleep(Duration::from_millis(300));
            }

            if let Ok(Some(equals)) = agent.desktop_find_element(app_name, "=") {
                agent.desktop_click_element(&equals)?;
                println!("   ✅ 已点击按钮 '='");
                println!("   📊 结果应该显示: 10\n");
            }
        }
        Ok(None) => {
            println!("   ⚠️  未找到元素 '7'");
            println!("   💡 可能原因：");
            println!("      • 应用窗口未完全加载");
            println!("      • 需要辅助功能权限（macOS）");
            println!("      • 应用不支持 UI Automation\n");
        }
        Err(e) => {
            println!("   ❌ 查找元素出错: {}", e);
            #[cfg(target_os = "macos")]
            println!("\n   💡 macOS 用户需要授予辅助功能权限：");
            println!("      系统偏好设置 → 安全性与隐私 → 隐私 → 辅助功能");
            println!("      添加：终端（Terminal）或 iTerm\n");
        }
    }

    // 第4步：查找所有按钮
    println!("🔍 第4步：枚举所有按钮");
    match agent.desktop_find_elements_by_type(app_name, "button") {
        Ok(elements) => {
            println!("   ✅ 找到 {} 个按钮:", elements.len());
            for (i, elem) in elements.iter().take(10).enumerate() {
                println!("      {}. {} ({})", i + 1, elem.name, elem.element_type);
            }
            if elements.len() > 10 {
                println!("      ... 还有 {} 个按钮", elements.len() - 10);
            }
        }
        Err(e) => {
            println!("   ❌ 枚举按钮失败: {}", e);
        }
    }

    println!("\n============================================================");
    println!("✅ UI Automation 演示完成！\n");

    println!("💡 核心能力：");
    println!("   • 按名称查找界面元素");
    println!("   • 按类型枚举所有元素（button/textbox/menu）");
    println!("   • 获取元素属性（位置/大小/可见性/启用状态）");
    println!("   • 模拟点击操作");
    println!("   • 输入文本到输入框");
    
    println!("\n⚠️  注意事项：");
    println!("   • macOS 需要在「系统偏好设置 → 辅助功能」授予权限");
    println!("   • 应用必须实现 Accessibility/UI Automation API");
    println!("   • 元素名称可能随应用版本/语言改变");

    Ok(())
}
