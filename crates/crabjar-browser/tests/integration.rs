//! Integration tests for crabjar-browser.
//! These tests exercise the actual Playwright subprocess wrapper.

use crabjar_browser::BrowserSession;

#[tokio::test]
async fn test_start_and_close() {
    let mut session = BrowserSession::new();
    
    // Start browser
    let result = session.start().await;
    assert!(result.is_ok(), "Failed to start browser: {:?}", result.err());
    
    let result = result.unwrap();
    assert_eq!(result["status"], "started");
    
    // Close browser
    let result = session.close().await;
    assert!(result.is_ok(), "Failed to close browser: {:?}", result.err());
}

#[tokio::test]
async fn test_navigate_and_get_title() {
    let mut session = BrowserSession::new();
    
    // Start browser
    session.start().await.unwrap();
    
    // Navigate to example.com
    let result = session.navigate("https://example.com").await;
    assert!(result.is_ok(), "Failed to navigate: {:?}", result.err());
    
    let result = result.unwrap();
    assert_eq!(result["status"], "success");
    assert!(result["title"].as_str().unwrap().contains("Example"));
    
    // Close browser
    session.close().await.unwrap();
}

#[tokio::test]
async fn test_evaluate_javascript() {
    let mut session = BrowserSession::new();
    
    // Start browser
    session.start().await.unwrap();
    
    // Navigate to a page
    session.navigate("https://example.com").await.unwrap();
    
    // Evaluate JavaScript
    let result = session.evaluate("document.title").await;
    assert!(result.is_ok(), "Failed to evaluate: {:?}", result.err());
    
    let result = result.unwrap();
    assert_eq!(result["status"], "success");
    assert!(result["result"].as_str().unwrap().contains("Example"));
    
    // Close browser
    session.close().await.unwrap();
}

#[tokio::test]
async fn test_get_text_content() {
    let mut session = BrowserSession::new();
    
    // Start browser
    session.start().await.unwrap();
    
    // Navigate to a page
    session.navigate("https://example.com").await.unwrap();
    
    // Get text content
    let result = session.text_content(None).await;
    assert!(result.is_ok(), "Failed to get text content: {:?}", result.err());
    
    let result = result.unwrap();
    assert_eq!(result["status"], "success");
    assert!(result["text"].as_str().unwrap().contains("Example Domain"));
    
    // Close browser
    session.close().await.unwrap();
}

#[tokio::test]
async fn test_screenshot() {
    let mut session = BrowserSession::new();
    
    // Start browser
    session.start().await.unwrap();
    
    // Navigate to a page
    session.navigate("https://example.com").await.unwrap();
    
    // Take screenshot (base64)
    let result = session.screenshot(None).await;
    assert!(result.is_ok(), "Failed to take screenshot: {:?}", result.err());
    
    let result = result.unwrap();
    assert_eq!(result["status"], "success");
    assert!(result["data"].as_str().is_some(), "Screenshot data missing");
    
    // Close browser
    session.close().await.unwrap();
}