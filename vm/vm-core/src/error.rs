//! Error types for VM lifecycle operations.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum VmError {
    #[error("VM not found: {0}")]
    NotFound(String),

    #[error("VM already exists: {0}")]
    AlreadyExists(String),

    #[error("VM state transition invalid: from={from} to={to}")]
    InvalidTransition { from: String, to: String },

    #[error("libvirt operation failed: {0}")]
    LibvirtError(#[from] anyhow::Error),

    #[error("Snapshot error: {0}")]
    SnapshotError(String),

    #[error("Timeout waiting for VM state change")]
    Timeout,

    #[error("Policy denied: {reason}")]
    PolicyDenied { reason: String },

    #[error("Disk operation failed: {0}")]
    DiskError(String),

    #[error("Invalid VM specification: {0}")]
    InvalidSpec(String),
}

pub type Result<T> = std::result::Result<T, VmError>;