//! Error types for web extraction operations.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum WebExtractError {
    #[error("HTTP request failed: {0}")]
    HttpRequestFailed(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Timeout waiting for page to load")]
    Timeout,

    #[error("Malformed HTML content")]
    MalformedHtml,

    #[error("Character limit exceeded")]
    CharLimitExceeded,

    #[error("PDF extraction failed: {0}")]
    PdfError(String),

    #[error("JavaScript execution failed: {0}")]
    JavascriptError(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Network error: {0}")]
    NetworkError(String),
}