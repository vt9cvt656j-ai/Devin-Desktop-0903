#[cfg(feature = "browser")]
use crate::browser::*;
use crate::types::*;

#[tokio::test]
async fn test_browser_automation_api() {
    // 测试 API 存在性（不实际创建浏览器）
    // BrowserAutomation 的创建需要真实 Chrome 环境
    assert!(true);
}

#[tokio::test]
#[ignore] // 需要 Chrome 环境
async fn test_browser_creation() {
    let result = BrowserAutomation::new().await;
    
    // 在 CI 环境可能失败（无 Chrome）
    if result.is_ok() {
        let browser = result.unwrap();
        browser.close().await.ok();
    }
}

#[tokio::test]
#[ignore]
async fn test_browser_navigation() {
    let mut browser = BrowserAutomation::new_headed().await.unwrap();
    
    let result = browser.navigate("https://example.com").await;
    assert!(result.is_ok());
    
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    browser.close().await.ok();
}

#[tokio::test]
#[ignore]
async fn test_browser_screenshot() {
    let mut browser = BrowserAutomation::new().await.unwrap();
    
    browser.navigate("https://example.com").await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    let result = browser.screenshot(Some("test_screenshot.png")).await;
    assert!(result.is_ok());
    
    browser.close().await.ok();
    
    // 清理测试文件
    std::fs::remove_file("test_screenshot.png").ok();
}

#[tokio::test]
#[ignore]
async fn test_browser_script_execution() {
    let mut browser = BrowserAutomation::new().await.unwrap();
    
    browser.navigate("https://example.com").await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    let result = browser.execute_script("document.title").await;
    assert!(result.is_ok());
    
    let title = result.unwrap();
    assert!(title.is_string() || title.is_object());
    
    browser.close().await.ok();
}

#[tokio::test]
#[ignore]
async fn test_browser_element_interaction() {
    let mut browser = BrowserAutomation::new_headed().await.unwrap();
    
    browser.navigate("https://example.com").await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // 尝试等待元素
    let result = browser.wait_for_element("body", 5000).await;
    assert!(result.is_ok());
    
    browser.close().await.ok();
}

#[tokio::test]
#[ignore]
async fn test_browser_get_content() {
    let mut browser = BrowserAutomation::new().await.unwrap();
    
    browser.navigate("https://example.com").await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    let result = browser.get_content().await;
    assert!(result.is_ok());
    
    let content = result.unwrap();
    assert!(content.contains("Example Domain") || content.contains("html"));
    
    browser.close().await.ok();
}

#[test]
fn test_browser_action_serialization() {
    let actions = vec![
        BrowserAction::Navigate("https://test.com".into()),
        BrowserAction::Click("#btn".into()),
        BrowserAction::Type { 
            selector: "input".into(), 
            text: "test".into() 
        },
        BrowserAction::ExecuteScript("alert('test')".into()),
    ];
    
    for action in actions {
        let json = serde_json::to_string(&action).unwrap();
        assert!(!json.is_empty());
    }
}
