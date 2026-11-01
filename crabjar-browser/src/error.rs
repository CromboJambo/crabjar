//! Browser automation errors.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BrowserError {
    #[error("HTTP request failed: {0}")]
    HttpFailure(String),

    #[error("Browser operation timed out after {0}s")]
    Timeout(u64),

    #[error("Playwright subprocess failed: {0}")]
    SubprocessFailure(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("JavaScript evaluation error: {0}")]
    JsEvaluationError(String),

    #[error("Form field not found: {0}")]
    FieldNotFound(String),

    #[error("Screenshot capture failed: {0}")]
    ScreenshotFailure(String),

    #[error("Browser context creation failed: {0}")]
    ContextCreationFailed(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}
