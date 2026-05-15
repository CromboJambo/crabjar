use thiserror::Error;

#[derive(Debug, Error)]
pub enum SandboxSchemaError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("schema initialization failed: {0}")]
    SchemaError(String),
}

#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("systemd-nspawn failed: {0}")]
    SystemdSpawn(String),

    #[error("cgroup setup failed: {0}")]
    Cgroup(String),

    #[error("user creation failed: {0}")]
    UserCreation(String),

    #[error("path not found: {0}")]
    NotFound(String),

    #[error("internal sandbox error: {0}")]
    Internal(String),
}