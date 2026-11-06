use std::fs;
use std::path::{Path, PathBuf};

use crate::build::build_artifact;
use crate::hashing::hash_file;
use crate::types::{ArtifactError, ArtifactMetadata};

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
    use crate::build::read_package_name;
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
