//! Error types for crabjar-vision.

use thiserror::Error;

/// Result type alias for vision operations.
pub type VisionResult<T> = std::result::Result<T, VisionError>;

/// Errors that can occur during vision analysis.
#[derive(Debug, Error)]
pub enum VisionError {
    /// Failed to load or download the image.
    #[error("Failed to load image '{image_source}': {reason}")]
    ImageLoadError {
        /// The source path or URL that failed.
        image_source: String,
        /// Human-readable reason for the failure.
        reason: String,
    },

    /// Failed to send the HTTP request to the LLM endpoint.
    #[error("Request error: {0}")]
    RequestError(String),

    /// The LLM endpoint returned an HTTP error status.
    #[error("HTTP {status}: {body}")]
    HttpError {
        /// HTTP status code.
        status: u16,
        /// Response body from the server.
        body: String,
    },

    /// Failed to parse the API response.
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Invalid configuration.
    #[error("Configuration error: {0}")]
    ConfigError(String),
}
