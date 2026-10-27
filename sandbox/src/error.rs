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

    #[error("schema error: {0}")]
    Schema(#[from] SandboxSchemaError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("systemd-nspawn failed: {0}")]
    SystemdSpawn(String),

    #[error("dinit-container failed: {0}")]
    DinitSpawn(String),

    #[error("cgroup setup failed: {0}")]
    Cgroup(String),

    #[error("user creation failed: {0}")]
    UserCreation(String),

    #[error("path not found: {0}")]
    NotFound(String),

    #[error("internal sandbox error: {0}")]
    Internal(String),

    #[error("podman not installed or not in PATH: {0}")]
    PodmanNotInstalled(String),

    #[error("podman command failed: {0}")]
    PodmanCommandFailed(std::io::Error),

    #[error("container creation failed: {0}")]
    ContainerCreateFailed(String),

    #[error("container execution failed: {0}")]
    ContainerExecFailed(String),

    #[error("container start failed: {0}")]
    ContainerStartFailed(String),

    #[error("container stop failed: {0}")]
    ContainerStopFailed(String),

    #[error("container destroy failed: {0}")]
    ContainerDestroyFailed(String),

    #[error("container listing failed: {0}")]
    ContainerListFailed(String),
}
