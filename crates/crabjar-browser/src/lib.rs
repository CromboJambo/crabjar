//! crabjar-browser: Browser automation via Playwright subprocess.
//!
//! Provides a structured API for web browsing, form filling, screenshots,
//! and content extraction through a Python/Playwright subprocess wrapper.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use tokio::sync::Mutex;

/// Browser automation session backed by a Playwright subprocess.
pub struct BrowserSession {
    child: Mutex<Option<Child>>,
    request_id: i32,
}

#[derive(Serialize, Deserialize)]
struct Request {
    id: i32,
    method: String,
    params: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct Response {
    id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl BrowserSession {
    /// Create a new browser session (doesn't start Playwright yet).
    pub fn new() -> Self {
        Self {
            child: Mutex::new(None),
            request_id: 0,
        }
    }

    /// Start the Playwright subprocess and browser instance.
    pub async fn start(&mut self) -> Result<serde_json::Value> {
        let wrapper_path = find_wrapper_script()?;

        let mut child = Command::new("python3")
            .arg(&wrapper_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn Playwright subprocess: {}", e))?;

        let stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to get stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to get stdout"))?;

        // Store child process
        *self.child.lock().await = Some(child);

        // Send start command
        self.send_request("start", HashMap::new()).await
    }

    /// Navigate to a URL.
    pub async fn navigate(&mut self, url: &str) -> Result<serde_json::Value> {
        let mut params = HashMap::new();
        params.insert("url".to_string(), serde_json::json!(url));
        self.send_request("navigate", params).await
    }

    /// Evaluate JavaScript expression.
    pub async fn evaluate(&mut self, expression: &str) -> Result<serde_json::Value> {
        let mut params = HashMap::new();
        params.insert("expression".to_string(), serde_json::json!(expression));
        self.send_request("evaluate", params).await
    }

    /// Fill a form field.
    pub async fn fill(&mut self, selector: &str, value: &str) -> Result<serde_json::Value> {
        let mut params = HashMap::new();
        params.insert("selector".to_string(), serde_json::json!(selector));
        params.insert("value".to_string(), serde_json::json!(value));
        self.send_request("fill", params).await
    }

    /// Click an element.
    pub async fn click(&mut self, selector: &str) -> Result<serde_json::Value> {
        let mut params = HashMap::new();
        params.insert("selector".to_string(), serde_json::json!(selector));
        self.send_request("click", params).await
    }

    /// Take a screenshot.
    pub async fn screenshot(&mut self, path: Option<&str>) -> Result<serde_json::Value> {
        let mut params = HashMap::new();
        if let Some(p) = path {
            params.insert("path".to_string(), serde_json::json!(p));
        }
        self.send_request("screenshot", params).await
    }

    /// Get page HTML content.
    pub async fn content(&mut self) -> Result<serde_json::Value> {
        self.send_request("content", HashMap::new()).await
    }

    /// Get text content of page or element.
    pub async fn text_content(&mut self, selector: Option<&str>) -> Result<serde_json::Value> {
        let mut params = HashMap::new();
        if let Some(s) = selector {
            params.insert("selector".to_string(), serde_json::json!(s));
        }
        self.send_request("text_content", params).await
    }

    /// Close the browser session.
    pub async fn close(&mut self) -> Result<serde_json::Value> {
        self.send_request("close", HashMap::new()).await
    }

    async fn send_request(
        &mut self,
        method: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let child = self.child.lock().await;
        let mut child = child.as_ref().ok_or_else(|| anyhow!("Browser not started"))?.try_clone();

        // Increment request ID
        self.request_id += 1;
        let id = self.request_id;

        // Build request JSON
        let request = Request {
            id,
            method: method.to_string(),
            params,
        };
        let request_json = serde_json::to_string(&request)?;

        // Send request
        let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to get stdin"))?;
        stdin.write_all(format!("{}\n", request_json).as_bytes()).await?;
        stdin.flush().await?;

        // Read response
        let mut stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to get stdout"))?;
        let mut line = String::new();
        stdout.read_line(&mut line).await?;

        let response: Response = serde_json::from_str(line.trim())?;

        if let Some(error) = response.error {
            return Err(anyhow!("Browser error: {}", error));
        }

        Ok(response.result.unwrap_or(serde_json::json!({})))
    }
}

fn find_wrapper_script() -> Result<std::path::PathBuf> {
    // Look for wrapper script relative to this crate's directory
    let possible_paths = [
        std::path::PathBuf::from("browser_wrapper.py"),
        std::path::PathBuf::from("../crabjar-browser/browser_wrapper.py"),
        std::path::PathBuf::from("/home/crombo/projects/active/crabjar/crates/crabjar-browser/browser_wrapper.py"),
    ];

    for path in possible_paths {
        if path.exists() {
            return Ok(path);
        }
    }

    Err(anyhow!("Could not find browser_wrapper.py"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_browser_session_creation() {
        let session = BrowserSession::new();
        assert!(session.request_id == 0);
    }

    #[tokio::test]
    async fn test_start_and_close() {
        let mut session = BrowserSession::new();
        let result = session.start().await;
        assert!(result.is_ok());

        let result = session.close().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_navigate() {
        let mut session = BrowserSession::new();
        session.start().await.unwrap();

        let result = session.navigate("https://example.com").await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result["status"], "success");

        session.close().await.unwrap();
    }
}