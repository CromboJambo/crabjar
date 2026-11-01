//! Playwright subprocess integration for browser automation.

use std::process::{Command, Stdio};

use super::error::BrowserError;
use super::models::*;
use super::DEFAULT_TIMEOUT_SECS;

/// Playwright client that executes browser operations via subprocess.
pub struct PlaywrightClient {
    timeout_secs: u64,
}

impl PlaywrightClient {
    /// Create a new Playwright client with default timeout.
    pub fn new() -> Self {
        Self {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }

    /// Navigate to URL and extract content.
    pub async fn navigate(&self, url: &str) -> Result<NavigationResult, BrowserError> {
        let script = format!(r#"
import asyncio
from playwright.async_api import async_playwright
import json

async def main():
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        try:
            page = await browser.new_page()
            await page.goto("{}", timeout={})
            title = await page.title()
            content = await page.inner_text("body")
            result = {{"url": page.url, "title": title, "content": content}}
            print(json.dumps(result))
        finally:
            await browser.close()

asyncio.run(main())
"#, url, self.timeout_secs * 1000);

        let output = self.execute_script(&script)?;
        serde_json::from_str(&output).map_err(BrowserError::SerializationError)
    }

    /// Evaluate JavaScript expression on page.
    pub async fn evaluate(&self, url: &str, expression: &str) -> Result<EvaluationResult, BrowserError> {
        let script = format!(r#"
import asyncio
from playwright.async_api import async_playwright
import json

async def main():
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        try:
            page = await browser.new_page()
            await page.goto("{}", timeout={})
            result = await page.evaluate("({})")
            print(json.dumps({{"value": result}}))
        finally:
            await browser.close()

asyncio.run(main())
"#, url, self.timeout_secs * 1000, expression);

        let output = self.execute_script(&script)?;
        serde_json::from_str(&output).map_err(BrowserError::SerializationError)
    }

    /// Fill form fields and optionally submit.
    pub async fn fill_form(
        &self,
        url: &str,
        fields: &[(&str, &str)],
        submit: bool,
    ) -> Result<NavigationResult, BrowserError> {
        // Build field filling code
        let mut fill_code = String::new();
        for (name, value) in fields {
            fill_code.push_str(&format!(
                "await page.fill('input[name=\"{}\"]', '{}')\n",
                name, value
            ));
        }

        if submit {
            fill_code.push_str("await page.click('button[type=\"submit\"]')\n");
        }

        let script = format!(r#"
import asyncio
from playwright.async_api import async_playwright
import json

async def main():
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        try:
            page = await browser.new_page()
            await page.goto("{}", timeout={})
{}
            title = await page.title()
            content = await page.inner_text("body")
            result = {{"url": page.url, "title": title, "content": content}}
            print(json.dumps(result))
        finally:
            await browser.close()

asyncio.run(main())
"#, url, self.timeout_secs * 1000, fill_code);

        let output = self.execute_script(&script)?;
        serde_json::from_str(&output).map_err(BrowserError::SerializationError)
    }

    /// Capture screenshot of page.
    pub async fn screenshot(&self, url: &str) -> Result<ScreenshotResult, BrowserError> {
        let script = format!(r#"
import asyncio
from playwright.async_api import async_playwright
import base64

async def main():
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        try:
            page = await browser.new_page()
            await page.goto("{}", timeout={})
            screenshot = await page.screenshot(type="png")
            print(base64.b64encode(screenshot).decode())
        finally:
            await browser.close()

asyncio.run(main())
"#, url, self.timeout_secs * 1000);

        let output = self.execute_script(&script)?;
        let data = base64_decode(&output.trim());
        Ok(ScreenshotResult {
            data,
            format: "png".to_string(),
        })
    }

    /// Execute a Playwright Python script and return its stdout.
    fn execute_script(&self, script: &str) -> Result<String, BrowserError> {
        // Use the playwright virtual environment's Python interpreter
        let python_path = std::env::var("PLAYWRIGHT_PYTHON").unwrap_or_else(|_| {
            "/home/crombo/.hermes/tools/playwright-venv/bin/python".to_string()
        });

        let child = Command::new(&python_path)
            .arg("-c")
            .arg(script)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| BrowserError::SubprocessFailure(format!("Failed to spawn python3: {}", e)))?;

        let output = child.wait_with_output()
            .map_err(|e| BrowserError::SubprocessFailure(format!("Failed to wait for subprocess: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BrowserError::SubprocessFailure(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    }
}

/// Simple base64 decode implementation.
fn base64_decode(input: &str) -> Vec<u8> {
    
    let alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;

    for ch in input.chars() {
        if let Some(pos) = alphabet.find(ch) {
            buf = (buf << 6) | pos as u32;
            bits += 6;
            if bits >= 8 {
                result.push((buf >> (bits - 8)) as u8);
                bits -= 8;
            }
        }
    }

    result
}