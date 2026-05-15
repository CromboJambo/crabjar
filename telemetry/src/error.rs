use thiserror::Error;

#[derive(Debug, Error)]
pub enum FlightRecorderError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("git diff capture failed: {0}")]
    GitDiff(String),

    #[error("command capture failed: {0}")]
    CommandCapture(String),

    #[error("path not found: {0}")]
    NotFound(String),

    #[error("internal telemetry error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("FlightRecorder error: {0}")]
    FlightRecorder(#[from] FlightRecorderError),

    #[error("git diff capture failed: {0}")]
    GitDiff(String),

    #[error("command capture failed: {0}")]
    CommandCapture(String),

    #[error("path not found: {0}")]
    NotFound(String),

    #[error("internal telemetry error: {0}")]
    Internal(String),
}