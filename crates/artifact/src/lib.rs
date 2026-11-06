//! crabjar-artifact: Content-addressed artifact packaging for Rust projects.
//!
//! Provides Guix/Nix-style reproducible build artifacts that are self-contained,
//! versioned, and portable across machines.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
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
}

/// Metadata for a content-addressed artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    /// Format version
    pub version: u32,
    /// Content-addressed hash of the entire artifact
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
    /// Hash of the source tree
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
    /// Shared library name (if any)
    pub lib: Option<String>,
    /// Data files included
    pub data_files: Vec<String>,
}

/// Compute SHA-256 hash of a file.
pub fn hash_file(path: &Path) -> Result<String, ArtifactError> {
    let mut hasher = Sha256::new();
    let mut file = std::fs::File::open(path)?;
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hex::encode(hasher.finalize()))
}

/// Compute SHA-256 hash of a directory tree.
pub fn hash_directory(path: &Path) -> Result<String, ArtifactError> {
    let mut hasher = Sha256::new();
    walk_and_hash(path, &mut hasher)?;
    Ok(hex::encode(hasher.finalize()))
}

fn walk_and_hash(dir: &Path, hasher: &mut Sha256) -> Result<(), ArtifactError> {
    let entries = std::fs::read_dir(dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            // Hash filename + content
            hasher.update(path.file_name().unwrap().to_string_lossy().as_bytes());
            let file_hash = hash_file(&path)?;
            hasher.update(file_hash.as_bytes());
        } else if path.is_dir() {
            // Recurse into subdirectories
            hasher.update(path.file_name().unwrap().to_string_lossy().as_bytes());
            walk_and_hash(&path, hasher)?;
        }
    }
    Ok(())
}

/// Read Cargo.toml to extract package name.
pub fn read_package_name(cargo_toml_path: &Path) -> Result<String, ArtifactError> {
    let content = std::fs::read_to_string(cargo_toml_path)?;
    // Simple parsing - find [package] section and name field
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name") && trimmed.contains("=") {
            let parts: Vec<&str> = trimmed.split('=').collect();
            if parts.len() == 2 {
                let value = parts[1].trim().trim_matches('"');
                return Ok(value.to_string());
            }
        }
    }
    Err(ArtifactError::InvalidFormat(
        "Could not find package name in Cargo.toml".to_string(),
    ))
}

/// Get rustc version string.
pub fn get_rustc_version() -> Result<String, ArtifactError> {
    let output = std::process::Command::new("rustc")
        .arg("--version")
        .output()?;
    if !output.status.success() {
        return Err(ArtifactError::Cargo(
            "Failed to run rustc --version".to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Build a cargo artifact and package it.
pub fn build_artifact(
    project_dir: &Path,
    target_triple: Option<&str>,
) -> Result<ArtifactMetadata, ArtifactError> {
    let target = target_triple.unwrap_or("x86_64-unknown-linux-musl");

    // Read package name
    let cargo_toml = project_dir.join("Cargo.toml");
    let name = read_package_name(&cargo_toml)?;

    // Compute source and lockfile hashes
    let source_hash = hash_directory(project_dir)?;
    let lockfile = project_dir.join("Cargo.lock");
    let lockfile_hash = if lockfile.exists() {
        hash_file(&lockfile)?
    } else {
        "none".to_string()
    };

    // Get compiler version
    let compiler_version = get_rustc_version()?;

    // Build the project
    println!("Building {} for {}...", name, target);
    let build_result = std::process::Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--target")
        .arg(target)
        .current_dir(project_dir)
        .output()?;

    if !build_result.status.success() {
        let stderr = String::from_utf8_lossy(&build_result.stderr);
        return Err(ArtifactError::Cargo(format!(
            "cargo build failed: {}",
            stderr
        )));
    }

    // Locate built binary
    let target_dir = project_dir.join("target").join(target).join("release");
    let binary_path = target_dir.join(&name);
    let binary_name = if binary_path.exists() {
        Some(name.clone())
    } else {
        None
    };

    // Compute artifact hash (source + lockfile + compiler)
    let mut hasher = Sha256::new();
    hasher.update(source_hash.as_bytes());
    hasher.update(lockfile_hash.as_bytes());
    hasher.update(compiler_version.as_bytes());
    let artifact_hash = hex::encode(hasher.finalize());

    // Create metadata
    let metadata = ArtifactMetadata {
        version: 1,
        hash: format!("sha256:{}", artifact_hash),
        name,
        target: target.to_string(),
        inputs: ArtifactInputs {
            source_hash,
            lockfile_hash,
            compiler_version,
            features: vec![], // TODO: parse from Cargo.toml
        },
        outputs: ArtifactOutputs {
            binary: binary_name,
            lib: None,
            data_files: vec![],
        },
    };

    Ok(metadata)
}

/// Inspect an artifact file and return its metadata.
pub fn inspect_artifact(artifact_path: &Path) -> Result<ArtifactMetadata, ArtifactError> {
    let content = std::fs::read_to_string(artifact_path)?;
    let metadata: ArtifactMetadata = serde_json::from_str(&content)?;
    Ok(metadata)
}

/// Write artifact metadata to a .ca file.
pub fn write_artifact_metadata(
    path: &Path,
    metadata: &ArtifactMetadata,
) -> Result<(), ArtifactError> {
    let content = serde_json::to_string_pretty(metadata)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Verify artifact integrity by recomputing hash.
pub fn verify_artifact(artifact_path: &Path) -> Result<bool, ArtifactError> {
    let metadata = inspect_artifact(artifact_path)?;

    // Recompute hash from inputs
    let mut hasher = Sha256::new();
    hasher.update(metadata.inputs.source_hash.as_bytes());
    hasher.update(metadata.inputs.lockfile_hash.as_bytes());
    hasher.update(metadata.inputs.compiler_version.as_bytes());
    let computed_hash = hex::encode(hasher.finalize());

    // Compare with stored hash
    let expected = metadata
        .hash
        .strip_prefix("sha256:")
        .unwrap_or(&metadata.hash);
    Ok(computed_hash == expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_file() {
        // Create a temp file and hash it
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        let hash = hash_file(&file_path).unwrap();
        assert_eq!(hash.len(), 64); // SHA-256 hex is 64 chars
    }

    #[test]
    fn test_hash_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("file1.txt"), "content1").unwrap();
        std::fs::write(dir.path().join("file2.txt"), "content2").unwrap();

        let hash = hash_directory(dir.path()).unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_artifact_metadata_serialization() {
        let metadata = ArtifactMetadata {
            version: 1,
            hash: "sha256:test".to_string(),
            name: "test-package".to_string(),
            target: "x86_64-unknown-linux-musl".to_string(),
            inputs: ArtifactInputs {
                source_hash: "abc".to_string(),
                lockfile_hash: "def".to_string(),
                compiler_version: "rustc 1.80.0".to_string(),
                features: vec!["std".to_string()],
            },
            outputs: ArtifactOutputs {
                binary: Some("test-binary".to_string()),
                lib: None,
                data_files: vec![],
            },
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let restored: ArtifactMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "test-package");
    }
}
