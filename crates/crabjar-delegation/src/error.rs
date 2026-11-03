//! Error types for the delegation framework.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DelegationError {
    #[error("Failed to spawn subagent: {0}")]
    SpawnFailed(String),

    #[error("Subagent task timed out after {seconds}s")]
    Timeout { seconds: u64 },

    #[error("Subagent task failed with exit code {code}")]
    ExitCode { code: i32 },

    #[error("Concurrency limit reached (max {limit})")]
    ConcurrencyLimit { limit: usize },

    #[error("Invalid delegation request: {0}")]
    InvalidRequest(String),

    #[error("Result aggregation failed: {0}")]
    AggregationFailed(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl DelegationError {
    /// Convert to a `TaskResult` error message.
    #[must_use]
    pub fn to_task_error(&self) -> String {
        match self {
            DelegationError::SpawnFailed(msg) => format!("Spawn failed: {msg}"),
            DelegationError::Timeout { seconds } => format!("Timed out after {seconds}s"),
            DelegationError::ExitCode { code } => format!("Exit code {code}"),
            DelegationError::ConcurrencyLimit { limit } => format!("Concurrency limit: {limit}"),
            DelegationError::InvalidRequest(msg) => format!("Invalid request: {msg}"),
            DelegationError::AggregationFailed(msg) => format!("Aggregation failed: {msg}"),
            DelegationError::Serialization(err) => format!("Serialization error: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_to_task_error() {
        let err = DelegationError::Timeout { seconds: 60 };
        assert_eq!(err.to_task_error(), "Timed out after 60s");
    }
}