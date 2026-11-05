//! Plugin process types: spawn errors, handle, restart policy.

use std::path::PathBuf;
use thiserror::Error;

/// Policy for handling process crashes.
#[derive(Debug, Clone, Default)]
pub enum RestartPolicy {
    /// Never restart (default).
    #[default]
    NoRestart,
    /// Restart up to N times with exponential backoff.
    Retry { max_retries: u32 },
}

/// Errors that can occur when spawning a plugin process.
#[derive(Error, Debug)]
pub enum SpawnError {
    #[error("startup timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("process not found: {0}")]
    ProcessNotFound(PathBuf),

    #[error("permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("spawn failed: {0}")]
    SpawnFailed(String),
}

/// Handle to a running plugin process.
#[derive(Debug, Clone)]
pub struct PluginHandle {
    pub pid: Option<u32>,
    pub name: String,
    /// Time when the process was spawned (for startup latency tracking).
    pub spawn_time: std::time::Instant,
}
