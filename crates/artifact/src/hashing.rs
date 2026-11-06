use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use crate::types::ArtifactError;

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
