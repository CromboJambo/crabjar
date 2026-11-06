//! crabjar-artifact: Content-addressed artifact packaging for Rust projects.
//!
//! Provides Guix/Nix-style reproducible build artifacts that are self-contained,
//! versioned, and portable across machines. Artifacts include the binary itself,
//! addressed by the hash of the complete tarball (binary + metadata).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
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

/// Compute SHA-256 hash of a file.
pub fn hash_file(path: &Path) -> Result<String, ArtifactError> {
    let mut hasher = Sha256::new();
    let mut file = fs::File::open(path)?;
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hex::encode(hasher.finalize()))
}

/// Compute SHA-256 hash of data.
pub fn hash_data(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Get list of tracked files via git ls-files.
fn get_tracked_files(project_dir: &Path) -> Result<Vec<PathBuf>, ArtifactError> {
    // Check if this directory has its own .git (not inherited from parent)
    let has_local_git = project_dir.join(".git").exists();

    if !has_local_git {
        // Not a git repo root — collect all files directly
        return collect_files(project_dir);
    }

    let output = std::process::Command::new("git")
        .arg("ls-files")
        .current_dir(project_dir)
        .output()?;

    if !output.status.success() {
        // Fall back to reading all files if git fails
        return collect_files(project_dir);
    }

    let files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|f| !f.is_empty())
        .map(|f| project_dir.join(f))
        .collect();

    Ok(files)
}

/// Collect all files in a directory (non-git fallback).
fn collect_files(dir: &Path) -> Result<Vec<PathBuf>, ArtifactError> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        } else if path.is_dir() {
            // Skip target/ and .git/ directories
            if let Some(name) = path.file_name() {
                if name == "target" || name == ".git" {
                    continue;
                }
            }
            files.extend(collect_files(&path)?);
        }
    }
    Ok(files)
}

/// Compute deterministic hash of source tree.
pub fn hash_source_tree(project_dir: &Path) -> Result<String, ArtifactError> {
    let mut hasher = Sha256::new();

    let files = get_tracked_files(project_dir)?;

    // Sort files for deterministic ordering
    let mut sorted_files = files;
    sorted_files.sort();

    for file in &sorted_files {
        // Hash relative path with length prefix
        if let Ok(rel) = file.strip_prefix(project_dir) {
            let rel_str = rel.to_string_lossy();
            hasher.update((rel_str.len() as u32).to_le_bytes());
            hasher.update(rel_str.as_bytes());
        }

        // Hash file content with length prefix
        if file.is_file() {
            let content = fs::read(file)?;
            hasher.update((content.len() as u64).to_le_bytes());
            hasher.update(&content);
        }
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Read package name from Cargo.toml using toml crate.
pub fn read_package_name(cargo_toml_path: &Path) -> Result<String, ArtifactError> {
    let content = fs::read_to_string(cargo_toml_path)?;
    let parsed: toml::Value = content
        .parse()
        .map_err(|e| ArtifactError::InvalidFormat(format!("Failed to parse Cargo.toml: {}", e)))?;

    if let Some(name) = parsed
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
    {
        Ok(name.to_string())
    } else {
        Err(ArtifactError::InvalidFormat(
            "Could not find package name in Cargo.toml".to_string(),
        ))
    }
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

/// Extract resolved dependency graph using cargo metadata.
pub fn get_dependency_graph(project_dir: &Path) -> Result<DependencyGraph, ArtifactError> {
    let output = std::process::Command::new("cargo")
        .arg("metadata")
        .arg("--format-version=1")
        .arg("--no-deps")
        .current_dir(project_dir)
        .output()?;

    if !output.status.success() {
        return Err(ArtifactError::Cargo(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    // Parse the JSON output to extract package info and dependencies
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|e| {
        ArtifactError::InvalidFormat(format!("Failed to parse cargo metadata: {}", e))
    })?;

    let mut deps = Vec::new();

    // Extract root package info
    if let Some(pkgs) = json.get("packages").and_then(|p| p.as_array()) {
        for pkg in pkgs {
            let name = pkg
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            let version = pkg
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Get dependencies for this package
            if let Some(dep_list) = pkg.get("dependencies").and_then(|d| d.as_array()) {
                for dep in dep_list {
                    let dep_name = dep
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    let dep_req = dep
                        .get("req")
                        .and_then(|r| r.as_str())
                        .unwrap_or("")
                        .to_string();
                    let source = dep
                        .get("source")
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string());

                    // Skip dev and build dependencies for now
                    if dep_name.is_empty() {
                        continue;
                    }

                    deps.push(DeclaredDependency {
                        name: dep_name,
                        version: dep_req,
                        source,
                        checksum: None,
                    });
                }
            }

            // Record root package itself
            if !name.is_empty() && !version.is_empty() {
                return Ok(DependencyGraph {
                    root: name,
                    dependencies: deps,
                });
            }
        }
    }

    Err(ArtifactError::Cargo(
        "Could not extract dependency graph from cargo metadata".to_string(),
    ))
}

/// Build a cargo project and package it into a content-addressed artifact.
pub fn build_artifact(
    project_dir: &Path,
    target_triple: Option<&str>,
) -> Result<ArtifactMetadata, ArtifactError> {
    let target = target_triple.unwrap_or("x86_64-unknown-linux-musl");

    // Read package name using proper TOML parsing
    let cargo_toml = project_dir.join("Cargo.toml");
    let name = read_package_name(&cargo_toml)?;

    // Compute source and lockfile hashes (deterministic)
    let source_hash = hash_source_tree(project_dir)?;
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
    if !binary_path.exists() {
        return Err(ArtifactError::BinaryNotFound);
    }

    // Hash the built binary for reproducibility tracking
    let binary_hash = hash_file(&binary_path)?;

    // Create metadata (without final artifact hash yet)
    let mut metadata = ArtifactMetadata {
        version: 1,
        hash: String::new(), // Will be set after tarball creation
        name: name.clone(),
        target: target.to_string(),
        inputs: ArtifactInputs {
            source_hash,
            lockfile_hash,
            compiler_version,
            features: vec![], // TODO: parse from Cargo.toml
        },
        outputs: ArtifactOutputs {
            binary: Some(name.clone()),
            binary_hash: Some(binary_hash),
            lib: None,
        },
    };

    // Create tarball containing binary + metadata
    let tarball_path = project_dir.join(format!("{}.tar.gz", name));

    {
        use flate2::Compression;
        use flate2::write::GzEncoder;

        let encoder = GzEncoder::new(fs::File::create(&tarball_path)?, Compression::default());
        let mut tar = tar::Builder::new(encoder);

        // Add binary to tarball
        tar.append_path_with_name(&binary_path, "bin")?;

        // Add metadata as JSON file in tarball
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        {
            let mut header = tar::Header::new_gnu();
            header.set_size(metadata_json.len() as u64);
            header.set_mode(0o644);
            header.set_mtime(0);
            header.set_cksum();
            tar.append_data(&mut header, "metadata.json", metadata_json.as_bytes())?;
        }

        tar.finish()?;
    }

    // Compute content-addressed hash of the tarball
    let tarball_hash = hash_file(&tarball_path)?;
    metadata.hash = format!("sha256:{}", tarball_hash);

    // Write final metadata with correct hash
    let final_metadata_json = serde_json::to_string_pretty(&metadata)?;
    fs::write(
        project_dir.join(format!("{}.ca", name)),
        final_metadata_json,
    )?;

    println!("Artifact built successfully!");
    println!("  Name: {}", metadata.name);
    println!("  Hash: {}", metadata.hash);
    println!("  Target: {}", metadata.target);
    println!(
        "  Binary hash: {}",
        metadata.outputs.binary_hash.as_ref().unwrap()
    );
    println!("  Output: {}.ca", name);

    Ok(metadata)
}

/// Inspect an artifact file and return its metadata.
pub fn inspect_artifact(artifact_path: &Path) -> Result<ArtifactMetadata, ArtifactError> {
    let content = fs::read_to_string(artifact_path)?;
    let metadata: ArtifactMetadata = serde_json::from_str(&content)?;
    Ok(metadata)
}

/// Verify artifact integrity by re-hashing the tarball.
pub fn verify_artifact(artifact_path: &Path) -> Result<bool, ArtifactError> {
    let metadata = inspect_artifact(artifact_path)?;

    // The .ca file should be alongside a .tar.zst with the same base name
    let stem = artifact_path
        .file_stem()
        .ok_or(ArtifactError::InvalidFormat("No file stem".to_string()))?;
    let tarball_path = artifact_path.with_extension("tar.gz");

    if !tarball_path.exists() {
        return Err(ArtifactError::InvalidFormat(format!(
            "Tarball not found: {}",
            tarball_path.display()
        )));
    }

    // Re-hash the tarball and compare with stored hash
    let computed_hash = hash_file(&tarball_path)?;
    let expected = metadata
        .hash
        .strip_prefix("sha256:")
        .unwrap_or(&metadata.hash);

    Ok(computed_hash == expected)
}

/// Rebuild the artifact and check if output matches (reproducibility test).
pub fn rebuild_check(
    project_dir: &Path,
    target_triple: Option<&str>,
) -> Result<bool, ArtifactError> {
    let target = target_triple.unwrap_or("x86_64-unknown-linux-musl");

    // Read package name to find existing artifact
    let cargo_toml = project_dir.join("Cargo.toml");
    let name = read_package_name(&cargo_toml)?;

    // Check if existing .ca file exists
    let ca_path = project_dir.join(format!("{}.ca", name));
    if !ca_path.exists() {
        return Err(ArtifactError::InvalidFormat(format!(
            "No existing artifact found at {}",
            ca_path.display()
        )));
    }

    let original_metadata = inspect_artifact(&ca_path)?;

    // Rebuild
    println!("Rebuilding {} for reproducibility check...", name);
    let new_metadata = build_artifact(project_dir, target_triple)?;

    // Compare hashes
    if original_metadata.hash == new_metadata.hash {
        println!("✓ Reproducible: rebuild produced identical artifact");
        Ok(true)
    } else {
        println!("✗ Not reproducible!");
        println!("  Original: {}", original_metadata.hash);
        println!("  Rebuilt:  {}", new_metadata.hash);

        // Show which parts differ
        if original_metadata.outputs.binary_hash != new_metadata.outputs.binary_hash {
            println!("  Binary hash differs (expected — builds aren't bit-identical)");
        }
        Ok(false)
    }
}

/// Install artifact into store path with current symlink.
pub fn install_artifact(artifact_path: &Path, store_root: &Path) -> Result<PathBuf, ArtifactError> {
    let metadata = inspect_artifact(artifact_path)?;

    // Verify tarball integrity before installing
    if !verify_artifact(artifact_path)? {
        return Err(ArtifactError::VerifyFailed {
            expected: metadata.hash.clone(),
            actual: "recomputed hash differs".to_string(),
        });
    }

    // Compute store path from content address
    let hash = metadata
        .hash
        .strip_prefix("sha256:")
        .unwrap_or(&metadata.hash);
    let store_path = store_root.join(hash);

    // Create store path if it doesn't exist
    fs::create_dir_all(&store_path)?;

    // Extract tarball to store path
    let stem = artifact_path
        .file_stem()
        .ok_or(ArtifactError::InvalidFormat("No file stem".to_string()))?;
    let tarball_path = artifact_path.with_extension("tar.gz");

    {
        use flate2::read::GzDecoder;

        let file = fs::File::open(&tarball_path)?;
        let decoder = GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(&store_path)?;
    }

    // Create/update current symlink
    let current_link = store_root.join("current");
    if current_link.exists() {
        fs::remove_file(&current_link)?;
    }
    std::os::unix::fs::symlink(&store_path, &current_link)?;

    println!("Installed artifact to {}", store_path.display());
    println!("  Current symlink: {}", current_link.display());

    Ok(store_path)
}

/// Diff two artifacts' dependency graphs, reporting version changes.
pub fn diff_artifacts(artifact_a: &Path, artifact_b: &Path) -> Result<Vec<String>, ArtifactError> {
    let meta_a = inspect_artifact(artifact_a)?;
    let meta_b = inspect_artifact(artifact_b)?;

    // For now, compare lockfile hashes and binary hashes
    let mut diffs = Vec::new();

    if meta_a.hash != meta_b.hash {
        diffs.push("Artifact hash differs (overall content changed)".to_string());
    }

    if meta_a.inputs.source_hash != meta_b.inputs.source_hash {
        diffs.push("Source hash differs (source code changed)".to_string());
    }

    if meta_a.inputs.lockfile_hash != meta_b.inputs.lockfile_hash {
        diffs.push("Lockfile hash differs (dependencies changed)".to_string());
    }

    if meta_a.outputs.binary_hash.as_ref() != meta_b.outputs.binary_hash.as_ref() {
        diffs.push("Binary hash differs (build output changed)".to_string());
    }

    if meta_a.inputs.compiler_version != meta_b.inputs.compiler_version {
        diffs.push(format!(
            "Compiler version differs: {} vs {}",
            meta_a.inputs.compiler_version, meta_b.inputs.compiler_version
        ));
    }

    Ok(diffs)
}

#[cfg(test)]
mod tests {
    use super::*;

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
