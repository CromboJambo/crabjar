//! Error types for GitHub client operations.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitHubError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("GitHub API error [{code}]: {message}")]
    Api { code: u16, message: String },

    #[error("Rate limit exceeded; retry after {retry_after}s")]
    RateLimited { retry_after: i64 },

    #[error("Authentication required but no token provided (set GITHUB_TOKEN)")]
    AuthRequired,

    #[error("Repository not found: {0}")]
    RepoNotFound(String),

    #[error("Skill manifest not found in repository: {0}")]
    ManifestNotFound(String),

    #[error("Git clone failed: {0}")]
    CloneFailed(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}
