//! Scanner errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScannerError {
    #[error("file not found: {0}")]
    FileNotFound(String),

    #[error("checksum mismatch for {0}: expected {1}, got {2}")]
    ChecksumMismatch(String, String, String),

    #[error("failed to read file {0}: {1}")]
    ReadError(String, String),

    #[error("archive extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("manifest validation failed: {0}")]
    ManifestValidation(String),
}