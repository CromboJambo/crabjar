//! Error types for durable object checkpointing.

use std::fmt;

/// Result type for checkpoint operations.
pub type CheckpointResult<T> = Result<T, CheckpointError>;

/// Errors that can occur during checkpoint operations.
#[derive(Debug)]
pub enum CheckpointError {
    /// SQLite database error.
    DbError(rusqlite::Error),
    /// File system error.
    IoError(std::io::Error),
    /// Serialization/deserialization error.
    SerdeError(serde_json::Error),
    /// Checkpoint not found.
    NotFound(String),
    /// Validation error for checkpoint data.
    Validation(String),
    /// Configuration error.
    Config(String),
}

impl fmt::Display for CheckpointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckpointError::DbError(e) => write!(f, "database error: {}", e),
            CheckpointError::IoError(e) => write!(f, "IO error: {}", e),
            CheckpointError::SerdeError(e) => write!(f, "serialization error: {}", e),
            CheckpointError::NotFound(id) => write!(f, "checkpoint not found: {}", id),
            CheckpointError::Validation(msg) => write!(f, "validation error: {}", msg),
            CheckpointError::Config(msg) => write!(f, "configuration error: {}", msg),
        }
    }
}

impl std::error::Error for CheckpointError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CheckpointError::DbError(e) => Some(e),
            CheckpointError::IoError(e) => Some(e),
            CheckpointError::SerdeError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for CheckpointError {
    fn from(e: rusqlite::Error) -> Self {
        CheckpointError::DbError(e)
    }
}

impl From<std::io::Error> for CheckpointError {
    fn from(e: std::io::Error) -> Self {
        CheckpointError::IoError(e)
    }
}

impl From<serde_json::Error> for CheckpointError {
    fn from(e: serde_json::Error) -> Self {
        CheckpointError::SerdeError(e)
    }
}
