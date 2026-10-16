//! Error types for crabjar-skills.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SkillError {
    #[error("skill directory not found: {0}")]
    DirectoryNotFound(PathBuf),

    #[error("failed to read skill manifest: {source}")]
    ReadManifest {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("invalid skill manifest JSON: {0}")]
    ParseManifest(#[from] serde_json::Error),

    #[error("skill validation failed: {0:?}")]
    Validation(Vec<String>),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
