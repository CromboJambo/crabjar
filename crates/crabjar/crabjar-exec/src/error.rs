//! Error types for crabjar-exec.

use thiserror::Error;

/// Result type for execution operations.
pub type ExecResult<T> = std::result::Result<T, ExecError>;

/// Errors that can occur during code execution.
#[derive(Debug, Error)]
pub enum ExecError {
    /// The script itself had a runtime or compile error.
    #[error("Script execution error: {0}")]
    ScriptError(String),

    /// A tool call failed during execution.
    #[error("Tool '{tool}' failed: {message}")]
    ToolError {
        /// Name of the tool that failed.
        tool: String,
        /// Error message from the tool.
        message: String,
    },

    /// Maximum number of tool calls exceeded.
    #[error("Maximum tool call limit ({0}) exceeded")]
    ToolCallLimitExceeded(usize),

    /// Execution timeout was reached.
    #[error("Execution timed out after {0} seconds")]
    Timeout(u64),

    /// File operation failed.
    #[error("File operation failed: {path}: {source}")]
    FileError {
        /// Path that was being operated on.
        path: String,
        /// Underlying IO error.
        source: std::io::Error,
    },

    /// Network operation failed.
    #[error("Network operation failed: {0}")]
    NetworkError(String),

    /// Invalid arguments passed to a tool.
    #[error("Invalid arguments for '{tool}': {message}")]
    InvalidArgs {
        /// Name of the tool.
        tool: String,
        /// Description of what was wrong.
        message: String,
    },
}

impl ExecError {
    /// Create a file error from an IO error.
    pub fn file(path: impl Into<String>, source: std::io::Error) -> Self {
        Self::FileError {
            path: path.into(),
            source,
        }
    }
}
