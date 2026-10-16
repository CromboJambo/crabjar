//! Error types for ClawHub client operations.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClawHubError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("ClawHub API error: {0}")]
    Api(String),

    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    #[error("Version not found: {0}")]
    VersionNotFound(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Security scan failed: {0}")]
    ScanFailed(String),

    #[error("Invalid semver: {0}")]
    InvalidVersion(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
