//! SHA-256 checksum computation and verification.

use std::fs;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

/// Compute SHA-256 checksum of a file.
pub fn compute_sha256(path: &Path) -> Result<String, String> {
    let mut hasher = Sha256::new();
    let mut file = fs::File::open(path).map_err(|e| format!("failed to open {}: {}", path.display(), e))?;

    let mut buffer = vec![0u8; 8192];
    loop {
        let bytes_read = file.read(&mut buffer).map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Verify a file's checksum against an expected value.
pub fn verify_checksum(path: &Path, expected: &str) -> Result<bool, String> {
    let actual = compute_sha256(path)?;
    Ok(actual == expected.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_compute_checksum() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"hello world").unwrap();

        let checksum = compute_sha256(file.path()).unwrap();
        assert_eq!(checksum.len(), 64); // SHA-256 produces 32 bytes = 64 hex chars
    }

    #[test]
    fn test_verify_checksum() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"hello world").unwrap();

        let expected = compute_sha256(file.path()).unwrap();
        assert!(verify_checksum(file.path(), &expected).unwrap());
        assert!(!verify_checksum(file.path(), "wrong_checksum").unwrap());
    }
}