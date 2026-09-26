//! Scheduler error types.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SchedulerError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Job not found: {0}")]
    JobNotFound(String),

    #[error("Invalid cron expression: {0}")]
    InvalidCron(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Job execution failed: {0}")]
    Execution(String),

    #[error("Guard denied execution: {0}")]
    GuardDenied(String),

    #[error("Worker pool full")]
    PoolFull,

    #[error("Watcher error: {0}")]
    Watcher(String),
}
