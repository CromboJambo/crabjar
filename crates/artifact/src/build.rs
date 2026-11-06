use std::fs;
use std::path::Path;

use crate::hashing::{hash_file, hash_source_tree};
use crate::types::{ArtifactError, ArtifactInputs, ArtifactMetadata, ArtifactOutputs};

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
