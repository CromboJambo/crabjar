//! GuardDbError — error types for the guard database.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GuardDbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("database path not available: {0}")]
    PathError(String),

    #[error("schema initialization failed: {0}")]
    SchemaError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
