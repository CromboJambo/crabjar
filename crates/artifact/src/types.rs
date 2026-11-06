use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error types for artifact operations.
#[derive(Debug, Error)]
pub enum ArtifactError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("Cargo command failed: {0}")]
    Cargo(String),

    #[error("Artifact verification failed: expected {expected}, got {actual}")]
    VerifyFailed { expected: String, actual: String },

    #[error("Invalid artifact format: {0}")]
    InvalidFormat(String),

    #[error("Binary not found at expected location")]
    BinaryNotFound,
}

/// Metadata for a content-addressed artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    /// Format version
    pub version: u32,
    /// Content-addressed hash of the entire tarball (binary + metadata)
    pub hash: String,
    /// Package name from Cargo.toml
    pub name: String,
    /// Target triple (e.g., x86_64-unknown-linux-musl)
    pub target: String,
    /// Build inputs for reproducibility
    pub inputs: ArtifactInputs,
    /// Output artifacts produced
    pub outputs: ArtifactOutputs,
}

/// Inputs that determine the artifact's content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactInputs {
    /// Hash of tracked source files (via git ls-files)
    pub source_hash: String,
    /// Hash of Cargo.lock
    pub lockfile_hash: String,
    /// Rust compiler version
    pub compiler_version: String,
    /// Enabled features
    pub features: Vec<String>,
}

/// Output artifacts produced by the build.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactOutputs {
    /// Primary binary name
    pub binary: Option<String>,
    /// Hash of the built binary (for reproducibility checking)
    pub binary_hash: Option<String>,
    /// Shared library name (if any)
    pub lib: Option<String>,
}

/// Declared dependency from cargo metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredDependency {
    /// Crate name
    pub name: String,
    /// Resolved version
    pub version: String,
    /// Source (registry URL or path)
    pub source: Option<String>,
    /// Registry checksum (if available)
    pub checksum: Option<String>,
}

/// Dependency graph for provenance tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// Root package name
    pub root: String,
    /// All resolved dependencies
    pub dependencies: Vec<DeclaredDependency>,
}
