//! crabjar-artifact: Content-addressed artifact packaging for Rust projects.
//!
//! Provides Guix/Nix-style reproducible build artifacts that are self-contained,
//! versioned, and portable across machines. Artifacts include the binary itself,
//! addressed by the hash of the complete tarball (binary + metadata).

pub mod build;
pub mod hashing;
pub mod operations;
pub mod types;

// Re-export public API for convenience
pub use build::{build_artifact, get_rustc_version, read_package_name};
pub use hashing::{hash_data, hash_file, hash_source_tree};
pub use operations::{
    diff_artifacts, inspect_artifact, install_artifact, rebuild_check, verify_artifact,
};
pub use types::{
    ArtifactError, ArtifactInputs, ArtifactMetadata, ArtifactOutputs, DeclaredDependency,
    DependencyGraph,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_hash_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "hello world").unwrap();

        let hash = hash_file(&file_path).unwrap();
        assert_eq!(hash.len(), 64); // SHA-256 hex is 64 chars
    }

    #[test]
    fn test_hash_source_tree_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("file1.txt"), "content1").unwrap();
        fs::write(dir.path().join("file2.txt"), "content2").unwrap();

        let hash1 = hash_source_tree(dir.path()).unwrap();
        let hash2 = hash_source_tree(dir.path()).unwrap();
        assert_eq!(hash1, hash2); // Same content should produce same hash
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
                binary_hash: Some("hash123".to_string()),
                lib: None,
            },
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let restored: ArtifactMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "test-package");
        assert_eq!(restored.outputs.binary_hash.as_ref().unwrap(), "hash123");
    }
}
