//! Error types for crabjar-browser-vault.

use thiserror::Error;

/// Errors that can occur during browser vault operations.
#[derive(Error, Debug)]
pub enum VaultError {
    /// The Bitwarden CLI is not installed or not found on PATH.
    #[error("Bitwarden CLI not found: {0}")]
    CliNotFound(String),

    /// The Bitwarden CLI returned a non-zero exit code.
    #[error("Bitwarden CLI error (exit {code}): {stderr}")]
    CliError { code: i32, stderr: String },

    /// The requested credential was not found.
    #[error("Credential not found: {0}")]
    NotFound(String),

    /// Failed to parse CLI output as JSON.
    #[error("Failed to parse Bitwarden response: {0}")]
    ParseError(#[from] serde_json::Error),

    /// The vault is not unlocked or authenticated.
    #[error("Vault is locked or not authenticated: {0}")]
    Locked(String),

    /// Configuration error (missing URL, invalid format, etc.).
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Timeout waiting for CLI response.
    #[error("Timeout after {seconds}s waiting for Bitwarden CLI")]
    Timeout { seconds: u64 },
}

/// Result type alias for vault operations.
pub type VaultResult<T> = Result<T, VaultError>;
