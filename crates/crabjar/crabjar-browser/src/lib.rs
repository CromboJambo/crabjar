//! crabjar-browser: Browser automation via Obscura CDP server.
//!
//! Provides a structured API for web browsing, form filling, screenshots,
//! and content extraction through an Obscura headless browser speaking
//! the Chrome DevTools Protocol (CDP).
//!
//! Architecture:
//! - `ObscuraServer` manages the `obscura serve` subprocess lifecycle
//! - `CdpClient` speaks JSON-RPC over WebSocket to a specific page target
//! - `BrowserSession` is the high-level API that callers use

mod cdp;
mod server;

pub use cdp::CdpClient;
pub use server::{ObscuraServer, TargetInfo};

use anyhow::{Result, anyhow};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// Browser automation session backed by an Obscura CDP server.
pub struct BrowserSession {
    /// Shared CDP server (owned by the first session that starts it).
    server: Mutex<Option<ObscuraServer>>,
    /// CDP client connected via browser-level WebSocket with session routing.
    cdp: Mutex<Option<CdpClient>>,
    /// The page target ID for this session.
    page_id: Mutex<Option<String>>,
    /// Background reader task handle (kept alive while session is active).
    reader_handle: Mutex<Option<JoinHandle<()>>>,
}

impl Default for BrowserSession {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserSession {
    /// Create a new browser session (doesn't start Obscura yet).
    pub fn new() -> Self {
        Self {
            server: Mutex::new(None),
            cdp: Mutex::new(None),
            page_id: Mutex::new(None),
            reader_handle: Mutex::new(None),
        }
    }

    /// Start the Obscura CDP server and connect via browser-level WebSocket.
    pub async fn start(&mut self) -> Result<serde_json::Value> {
        // Spawn Obscura server if not already running
        let (port, ws_url) = {
            let mut server_guard = self.server.lock().await;
            if server_guard.is_none() {
                *server_guard = Some(ObscuraServer::spawn().await?);
            }
            let server = server_guard.as_ref().unwrap();
            (server.port, server.ws_url.clone())
        };

        // Connect via browser-level WebSocket and create a page target
        let (cdp_client, _event_rx, reader_handle) = CdpClient::connect_browser(&ws_url, "about:blank").await?;

        // Get the session ID as our page identifier
        let page_id = format!("session-{}", port);

        *self.page_id.lock().await = Some(page_id.clone());
        *self.cdp.lock().await = Some(cdp_client);
        *self.reader_handle.lock().await = Some(reader_handle);

        Ok(serde_json::json!({
            "status": "started",
            "port": port,
            "page_id": page_id,
        }))
    }

    /// Navigate to a URL.
    pub async fn navigate(&self, url: &str) -> Result<serde_json::Value> {
        let cdp_guard = self.cdp.lock().await;
        let cdp = cdp_guard.as_ref().ok_or_else(|| anyhow!("Browser not started"))?;

        // Enable Page domain to receive lifecycle events
        let _ = cdp.send("Page.enable", serde_json::json!({})).await;

        // Navigate
        cdp.send("Page.navigate", serde_json::json!({ "url": url })).await?;

        // Wait for load event (poll with timeout)
        drop(cdp_guard);
        self.wait_for_load().await?;

        // Get title and final URL
        let title = self.eval_string("document.title").await.unwrap_or_default();
        let final_url = self.eval_string("window.location.href").await.unwrap_or_else(|| url.to_string());

        Ok(serde_json::json!({
            "status": "success",
            "url": final_url,
            "title": title,
        }))
    }

    /// Evaluate JavaScript expression and return the result value.
    pub async fn evaluate(&self, expression: &str) -> Result<serde_json::Value> {
        let cdp_guard = self.cdp.lock().await;
        let cdp = cdp_guard.as_ref().ok_or_else(|| anyhow!("Browser not started"))?;

        let result = cdp
            .send("Runtime.evaluate", serde_json::json!({
                "expression": expression,
                "returnByValue": true,
            }))
            .await?;

        let value = result
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        Ok(serde_json::json!({
            "status": "success",
            "result": value,
        }))
    }

    /// Fill a form field by CSS selector.
    pub async fn fill(&self, selector: &str, value: &str) -> Result<serde_json::Value> {
        let cdp_guard = self.cdp.lock().await;
        let cdp = cdp_guard.as_ref().ok_or_else(|| anyhow!("Browser not started"))?;

        // Use JavaScript to set the value and dispatch events
        let escaped_selector = escape_js_string(selector);
        let escaped_value = escape_js_string(value);
        let js = format!(
            "(() => {{ const el = document.querySelector(\"{}\"); if (!el) return 'Element not found'; el.value = \"{}\"; el.dispatchEvent(new Event('input', {{bubbles: true}})); el.dispatchEvent(new Event('change', {{bubbles: true}})); return 'ok'; }})()",
            escaped_selector,
            escaped_value
        );

        let result = cdp
            .send("Runtime.evaluate", serde_json::json!({
                "expression": js,
                "returnByValue": true,
            }))
            .await?;

        let status = result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string();

        if status == "Element not found" {
            return Err(anyhow!("Element not found: {}", selector));
        }

        Ok(serde_json::json!({ "status": "success" }))
    }

    /// Click an element by CSS selector.
    pub async fn click(&self, selector: &str) -> Result<serde_json::Value> {
        let cdp_guard = self.cdp.lock().await;
        let cdp = cdp_guard.as_ref().ok_or_else(|| anyhow!("Browser not started"))?;

        // Use JavaScript to click the element
        let escaped_selector = escape_js_string(selector);
        let js = format!(
            "(() => {{ const el = document.querySelector(\"{}\"); if (!el) return 'Element not found'; el.click(); return 'ok'; }})()",
            escaped_selector
        );

        let result = cdp
            .send("Runtime.evaluate", serde_json::json!({
                "expression": js,
                "returnByValue": true,
            }))
            .await?;

        let status = result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string();

        if status == "Element not found" {
            return Err(anyhow!("Element not found: {}", selector));
        }

        Ok(serde_json::json!({ "status": "success" }))
    }

    /// Take a screenshot.
    pub async fn screenshot(&self, path: Option<&str>) -> Result<serde_json::Value> {
        let cdp_guard = self.cdp.lock().await;
        let cdp = cdp_guard.as_ref().ok_or_else(|| anyhow!("Browser not started"))?;

        let result = cdp
            .send("Page.captureScreenshot", serde_json::json!({
                "format": "png",
            }))
            .await?;

        let data = result
            .get("data")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow!("Screenshot did not return data"))?
            .to_string();

        if let Some(p) = path {
            std::fs::write(p, base64_decode(&data))
                .map_err(|e| anyhow!("Failed to write screenshot to {}: {}", p, e))?;
            Ok(serde_json::json!({ "status": "success", "path": p }))
        } else {
            Ok(serde_json::json!({ "status": "success", "data": data }))
        }
    }

    /// Get page HTML content.
    pub async fn content(&self) -> Result<serde_json::Value> {
        let cdp_guard = self.cdp.lock().await;
        let cdp = cdp_guard.as_ref().ok_or_else(|| anyhow!("Browser not started"))?;

        let result = cdp
            .send("Runtime.evaluate", serde_json::json!({
                "expression": "document.documentElement.outerHTML",
                "returnByValue": true,
            }))
            .await?;

        let html = result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow!("Failed to get page HTML"))?
            .to_string();

        Ok(serde_json::json!({ "status": "success", "html": html }))
    }

    /// Get text content of page or element.
    pub async fn text_content(&self, selector: Option<&str>) -> Result<serde_json::Value> {
        let cdp_guard = self.cdp.lock().await;
        let cdp = cdp_guard.as_ref().ok_or_else(|| anyhow!("Browser not started"))?;

        let expression = match selector {
            Some(s) => format!(
                "document.querySelector(\"{}\").textContent",
                escape_js_string(s)
            ),
            None => "document.body.textContent".to_string(),
        };

        let result = cdp
            .send("Runtime.evaluate", serde_json::json!({
                "expression": expression,
                "returnByValue": true,
            }))
            .await?;

        let text = result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow!("Failed to get text content"))?
            .to_string();

        Ok(serde_json::json!({ "status": "success", "text": text }))
    }

    /// Close the browser session (page target only; server stays up for reuse).
    pub async fn close(&mut self) -> Result<serde_json::Value> {
        // Close CDP connection
        let cdp_guard = self.cdp.lock().await;
        if let Some(cdp) = cdp_guard.as_ref() {
            let _ = cdp.close().await;
        }

        *self.cdp.lock().await = None;
        *self.page_id.lock().await = None;

        // Abort the reader task
        if let Some(handle) = self.reader_handle.lock().await.take() {
            handle.abort();
        }

        Ok(serde_json::json!({ "status": "closed" }))
    }

    /// Execute arbitrary JavaScript code in the page context.
    pub async fn execute_code(&self, code: &str) -> Result<serde_json::Value> {
        // For backward compatibility with browser_exec which passes Python code,
        // detect if it looks like Python (has 'await page.' or 'from playwright')
        // and translate to JS. Otherwise treat as JavaScript.
        let js_code = translate_python_to_js(code);

        let cdp_guard = self.cdp.lock().await;
        let cdp = cdp_guard.as_ref().ok_or_else(|| anyhow!("Browser not started"))?;

        let result = cdp
            .send("Runtime.evaluate", serde_json::json!({
                "expression": js_code,
                "returnByValue": true,
                "awaitPromise": true,
            }))
            .await?;

        let value = result
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        Ok(serde_json::json!({
            "status": "success",
            "result": value,
        }))
    }

    /// Wait for the page to finish loading (poll with timeout).
    async fn wait_for_load(&self) -> Result<()> {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);

        loop {
            if tokio::time::Instant::now() > deadline {
                return Err(anyhow!("Timed out waiting for page to load"));
            }

            let cdp_guard = self.cdp.lock().await;
            let cdp = cdp_guard.as_ref().ok_or_else(|| anyhow!("Browser not started"))?;

            let result = cdp
                .send("Runtime.evaluate", serde_json::json!({
                    "expression": "document.readyState",
                    "returnByValue": true,
                }))
                .await;

            drop(cdp_guard);

            if let Ok(v) = result {
                let state = v
                    .get("result")
                    .and_then(|r| r.get("value"))
                    .and_then(serde_json::Value::as_str)
                    .map(String::from);
                if state.as_deref() == Some("complete") {
                    return Ok(());
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    }

    /// Evaluate a JS expression and return the string result, or None on error.
    async fn eval_string(&self, expression: &str) -> Option<String> {
        let cdp_guard = self.cdp.lock().await;
        let cdp = cdp_guard.as_ref()?;
        let result = cdp
            .send("Runtime.evaluate", serde_json::json!({
                "expression": expression,
                "returnByValue": true,
            }))
            .await
            .ok()?;
        drop(cdp_guard);
        result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(serde_json::Value::as_str)
            .map(String::from)
    }
}

/// Execute a one-shot browser operation: start session, run code, close session.
pub async fn execute_code(code: &str) -> Result<serde_json::Value> {
    let mut session = BrowserSession::new();
    session.start().await?;

    let result = session.execute_code(code).await;

    // Close session (ignore errors — we already have our result)
    let _ = session.close().await;

    result
}

/// Translate simple Python/Playwright code to JavaScript for CDP evaluation.
fn translate_python_to_js(code: &str) -> String {
    if code.contains("from playwright") || code.contains("await page.") || code.contains("async_playwright") {
        return translate_playwright_python(code);
    }
    code.to_string()
}

/// Translate Python Playwright code to equivalent JavaScript.
fn translate_playwright_python(code: &str) -> String {
    let mut js = code.to_string();

    // Remove Python-specific imports and setup
    js = js.replace("from playwright.async_api import async_playwright, TimeoutError as PlaywrightTimeout", "");
    js = js.replace("import asyncio", "");
    js = js.replace("import json", "");
    js = js.replace("import base64", "");

    // Remove Python async function wrapper
    js = js.replace("async def main():", "(async () => {");
    if let Some(pos) = js.find("asyncio.run(main())") {
        js = format!("{}{})", &js[..pos], "})()");
    }

    // Translate Playwright API calls to browser JS equivalents
    js = js.replace("await async_playwright().start()", "null");
    js = js.replace("await pw.chromium.launch(", "null;");
    js = js.replace("await browser.new_context(", "null;");
    js = js.replace("await context.new_page()", "null");

    // Common Playwright → JS translations
    js = js.replace("await page.goto(", "location.href = ");
    js = js.replace("await page.title()", "document.title");
    js = js.replace("await page.content()", "document.documentElement.outerHTML");
    js = js.replace("await page.text_content(", "document.querySelector(");
    js = js.replace(".text_content()", ").textContent");
    js = js.replace("await page.evaluate(", "(");
    js = js.replace("await page.fill(", "(function(s,v){var el=document.querySelector(s);el.value=v;el.dispatchEvent(new Event('input',{bubbles:true}));el.dispatchEvent(new Event('change',{bubbles:true}))})(");
    js = js.replace("await page.click(", "document.querySelector(");

    // Remove trailing semicolons from Python statements that don't need them in JS
    let lines: Vec<&str> = js.lines().filter(|l| !l.trim().is_empty()).collect();
    lines.join("\n")
}

/// Escape a string for safe embedding in a JavaScript double-quoted string.
fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Simple base64 decoder (avoids adding a dependency for one function).
fn base64_decode(data: &str) -> Vec<u8> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = Vec::with_capacity(data.len() * 3 / 4);
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;

    for &byte in data.as_bytes() {
        if byte == b'=' {
            break;
        }
        if let Some(pos) = ALPHABET.iter().position(|&c| c == byte) {
            buffer = (buffer << 6) | pos as u32;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                output.push((buffer >> bits) as u8);
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_session_creation() {
        let session = BrowserSession::new();
        assert!(session.page_id.blocking_lock().is_none());
    }

    #[test]
    fn test_base64_decode() {
        // "hello" in base64
        let decoded = base64_decode("aGVsbG8=");
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_escape_js_string() {
        assert_eq!(escape_js_string("hello"), "hello");
        assert_eq!(escape_js_string("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_js_string("a\\b"), "a\\\\b");
    }

    #[tokio::test]
    async fn test_execute_code_without_server() {
        // Should fail gracefully if obscura is not available
        let result = execute_code("document.title").await;
        if let Err(e) = &result {
            assert!(e.to_string().contains("Obscura") || e.to_string().contains("obscura"));
        }
    }
}
