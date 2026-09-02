use rust_automation_framework::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    rust_automation_framework::init_logging();
    
    println!("🌐 浏览器自动化演示\n");
    
    // 创建有头浏览器（可见）
    println!("1. 启动浏览器...");
    let mut browser = BrowserAutomation::new_headed().await?;
    
    // 导航到示例网站
    println!("2. 导航到 example.com...");
    browser.navigate("https://example.com").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // 截图
    println!("3. 截图保存...");
    browser.screenshot(Some("example_page.png")).await?;
    
    // 获取页面内容
    println!("4. 获取页面内容...");
    let content = browser.get_content().await?;
    println!("   页面长度: {} 字符", content.len());
    
    // 执行 JavaScript
    println!("5. 执行 JavaScript...");
    let result = browser.execute_script("document.title").await?;
    println!("   页面标题: {}", result);
    
    // 滚动页面
    println!("6. 滚动页面...");
    browser.scroll(0, 300).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // 导航到搜索引擎演示表单操作
    println!("\n7. 导航到 DuckDuckGo 演示搜索...");
    browser.navigate("https://duckduckgo.com").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // 等待搜索框出现
    println!("8. 等待搜索框...");
    browser.wait_for_element("input[name='q']", 5000).await?;
    
    // 输入搜索关键词
    println!("9. 输入搜索关键词...");
    browser.type_text("input[name='q']", "Rust programming language").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // 截图最终状态
    println!("10. 最终截图...");
    browser.screenshot(Some("search_demo.png")).await?;
    
    println!("\n✅ 演示完成！");
    println!("   - 截图已保存: example_page.png, search_demo.png");
    
    // 关闭浏览器
    browser.close().await?;
    
    Ok(())
}
