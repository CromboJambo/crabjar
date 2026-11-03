//! Error types for web search operations.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum WebSearchError {
    #[error("Backend selection failed")]
    BackendSelectionFailed,

    #[error("API request failed: {0}")]
    ApiRequestFailed(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Malformed response from backend")]
    MalformedResponse(String),

    #[error("Empty result set for query '{query}'")]
    EmptyResults { query: String },

    #[error("Timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Authentication failed for backend '{backend}'")]
    AuthFailed { backend: String },
}