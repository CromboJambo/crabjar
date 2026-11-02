//! Example: Using crabjar-browser for web automation.

use crabjar_browser::BrowserSession;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create and start browser session
    let mut browser = BrowserSession::new();
    let result = browser.start().await?;
    println!("Browser started: {}", result["status"]);

    // Navigate to a website
    let result = browser.navigate("https://example.com").await?;
    println!("Navigated to: {}", result["url"]);
    println!("Page title: {}", result["title"]);

    // Evaluate JavaScript
    let result = browser.evaluate("document.title").await?;
    println!("JS evaluation: {}", result["result"]);

    // Get page text content
    let result = browser.text_content(None).await?;
    let text = result["text"].as_str().unwrap_or("");
    println!("Page text (first 100 chars): {}", &text[..100.min(text.len())]);

    // Take screenshot
    let result = browser.screenshot(None).await?;
    println!("Screenshot captured: {} bytes", result["data"].as_str().unwrap_or("").len());

    // Close browser
    let result = browser.close().await?;
    println!("Browser closed: {}", result["status"]);

    Ok(())
}