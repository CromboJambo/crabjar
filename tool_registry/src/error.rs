use crate::schema::ToolRegistrySchemaError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolRegistryError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("schema error: {0}")]
    Schema(#[from] ToolRegistrySchemaError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("tool discovery failed: {0}")]
    Discovery(String),

    #[error("tool registration failed: {0}")]
    Registration(String),

    #[error("path not found: {0}")]
    NotFound(String),

    #[error("internal tool registry error: {0}")]
    Internal(String),
}
