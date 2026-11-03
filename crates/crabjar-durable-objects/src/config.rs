//! Checkpoint configuration options.

use std::time::Duration;

/// Configuration for checkpointing behavior.
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    /// Interval between automatic checkpoints.
    pub checkpoint_interval: Duration,
    /// Number of checkpoints to retain per task.
    pub retention_count: u32,
    /// Maximum age of checkpoints before pruning.
    pub max_age: Duration,
    /// Enable compression for checkpoint payloads.
    pub compress: bool,
    /// Path to SQLite metadata database.
    pub db_path: String,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            checkpoint_interval: Duration::from_secs(300), // 5 minutes
            retention_count: 10,
            max_age: Duration::from_secs(86400 * 7), // 7 days
            compress: true,
            db_path: String::from("checkpoints.db"),
        }
    }
}

impl CheckpointConfig {
    /// Create a new checkpoint configuration.
    pub fn new(db_path: impl Into<String>) -> Self {
        Self {
            db_path: db_path.into(),
            ..Self::default()
        }
    }

    /// Set the interval between automatic checkpoints.
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.checkpoint_interval = interval;
        self
    }

    /// Set the number of checkpoints to retain per task.
    pub fn with_retention(mut self, count: u32) -> Self {
        self.retention_count = count;
        self
    }

    /// Set maximum age for checkpoint retention.
    pub fn with_max_age(mut self, age: Duration) -> Self {
        self.max_age = age;
        self
    }

    /// Enable or disable compression.
    pub fn with_compression(mut self, compress: bool) -> Self {
        self.compress = compress;
        self
    }
}
