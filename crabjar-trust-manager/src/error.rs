//! Error types for crabjar-trust-manager.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TrustManagerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("unknown trust tier: {0}")]
    UnknownTrustTier(String),

    #[error("skill not found in cache: {0}")]
    SkillNotFoundInCache(String),

    #[error("cache directory error: {0}")]
    CacheDirError(String),

    #[error("version comparison failed: {0}")]
    VersionComparison(String),

    #[error("HTTP request failed: {0}")]
    Http(String),
}
