//! Browser automation via Playwright subprocess integration.
//!
//! Provides safe browser operations through structured API:
//! - Navigation and content extraction
//! - JavaScript evaluation
//! - Form filling and submission
//! - Screenshot capture

mod error;
mod models;
mod playwright_client;

pub use error::BrowserError;
pub use models::*;
pub use playwright_client::PlaywrightClient;

/// Default timeout for browser operations (30 seconds).
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Navigate to URL and extract content.
pub async fn navigate(url: &str) -> Result<NavigationResult, BrowserError> {
    let client = PlaywrightClient::new();
    client.navigate(url).await
}

/// Evaluate JavaScript expression on page.
pub async fn evaluate(url: &str, expression: &str) -> Result<EvaluationResult, BrowserError> {
    let client = PlaywrightClient::new();
    client.evaluate(url, expression).await
}

/// Fill form fields and optionally submit.
pub async fn fill_form(
    url: &str,
    fields: &[(&str, &str)],
    submit: bool,
) -> Result<NavigationResult, BrowserError> {
    let client = PlaywrightClient::new();
    client.fill_form(url, fields, submit).await
}

/// Capture screenshot of page.
pub async fn screenshot(url: &str) -> Result<ScreenshotResult, BrowserError> {
    let client = PlaywrightClient::new();
    client.screenshot(url).await
}