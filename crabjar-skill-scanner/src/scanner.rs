//! Skill security scanner — analyzes downloaded skill packages for security issues.

use std::fs;
use std::path::Path;

use walkdir::WalkDir;

use crate::checksum::{compute_sha256, verify_checksum};
use crate::models::{ScanFinding, ScanResult, Severity};
use tracing::{debug, info, warn};

/// Maximum file size per skill (10MB).
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum number of files per skill.
const MAX_FILES: usize = 100;

/// Skill security scanner.
pub struct SkillScanner;

impl SkillScanner {
    /// Create a new skill scanner.
    pub fn new() -> Self {
        Self
    }

    /// Scan a downloaded skill package for security issues.
    pub fn scan(
        &self,
        name: &str,
        version: &str,
        package_path: &Path,
    ) -> Result<ScanResult, String> {
        info!("Scanning {} v{} for security issues", name, version);

        let mut result = ScanResult::new(name, version);

        // Verify manifest exists
        self.check_manifest(package_path, &mut result)?;

        // Walk all files and check sizes
        self.check_file_sizes(package_path, &mut result)?;

        // Check for binary files
        self.check_binary_files(package_path, &mut result);

        // Check for network access patterns
        self.check_network_patterns(package_path, &mut result);

        // Check for filesystem write patterns
        self.check_filesystem_patterns(package_path, &mut result);

        // Check for environment variable access
        self.check_env_patterns(package_path, &mut result);

        info!("Scan complete: {}", result.summary());
        Ok(result)
    }

    /// Scan and verify checksum against expected value.
    pub fn scan_with_checksum(
        &self,
        name: &str,
        version: &str,
        package_path: &Path,
        expected_checksum: &str,
    ) -> Result<ScanResult, String> {
        let mut result = self.scan(name, version, package_path)?;

        // Verify checksum — compute on the directory contents if it's a directory,
        // or on the file directly if it's a file (e.g., downloaded archive)
        let checksum_target = if package_path.is_dir() {
            // For directories, compute checksum of manifest.json as representative artifact
            let manifest_path = package_path.join("manifest.json");
            if manifest_path.exists() {
                Some(manifest_path)
            } else {
                None
            }
        } else {
            Some(package_path.to_path_buf())
        };

        match checksum_target {
            Some(target) => match compute_sha256(&target) {
                Ok(actual) => {
                    if actual == expected_checksum.to_lowercase() {
                        result.checksum_verified = true;
                        info!("Checksum verified for {} v{}", name, version);
                    } else {
                        result.add_finding(ScanFinding {
                            severity: Severity::Critical,
                            category: "checksum".to_string(),
                            message: format!(
                                "Checksum mismatch: expected {}, got {}",
                                expected_checksum, actual
                            ),
                            file: None,
                        });
                    }
                }
                Err(e) => {
                    result.add_finding(ScanFinding {
                        severity: Severity::Error,
                        category: "checksum".to_string(),
                        message: format!("Failed to compute checksum: {}", e),
                        file: None,
                    });
                }
            },
            None => {
                warn!("No manifest.json found for directory-based checksum verification");
            }
        }

        Ok(result)
    }

    fn check_manifest(&self, package_path: &Path, result: &mut ScanResult) -> Result<(), String> {
        let manifest_path = package_path.join("manifest.json");

        if !manifest_path.exists() {
            result.add_finding(ScanFinding {
                severity: Severity::Error,
                category: "manifest".to_string(),
                message: "manifest.json not found in skill package".to_string(),
                file: None,
            });
        } else {
            // Validate manifest is valid JSON
            match fs::read_to_string(&manifest_path) {
                Ok(content) => {
                    if let Err(e) = serde_json::from_str::<serde_json::Value>(&content) {
                        result.add_finding(ScanFinding {
                            severity: Severity::Error,
                            category: "manifest".to_string(),
                            message: format!("Invalid JSON in manifest.json: {}", e),
                            file: Some("manifest.json".to_string()),
                        });
                    } else {
                        debug!("Manifest validation passed");
                    }
                }
                Err(e) => {
                    result.add_finding(ScanFinding {
                        severity: Severity::Error,
                        category: "manifest".to_string(),
                        message: format!("Failed to read manifest.json: {}", e),
                        file: Some("manifest.json".to_string()),
                    });
                }
            }
        }

        Ok(())
    }

    fn check_file_sizes(&self, package_path: &Path, result: &mut ScanResult) -> Result<(), String> {
        let mut files_scanned = 0;
        let mut total_size: u64 = 0;

        for entry in WalkDir::new(package_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            files_scanned += 1;
            let size = entry
                .metadata()
                .map_err(|e| format!("failed to read metadata: {}", e))?
                .len();
            total_size += size;

            if size > MAX_FILE_SIZE {
                result.add_finding(ScanFinding {
                    severity: Severity::Warning,
                    category: "file_size".to_string(),
                    message: format!("File exceeds 10MB limit: {} bytes", size),
                    file: Some(entry.path().to_string_lossy().to_string()),
                });
            }
        }

        if files_scanned > MAX_FILES {
            result.add_finding(ScanFinding {
                severity: Severity::Warning,
                category: "file_count".to_string(),
                message: format!(
                    "Skill contains {} files (limit: {})",
                    files_scanned, MAX_FILES
                ),
                file: None,
            });
        }

        result.files_scanned = files_scanned;
        result.total_size_bytes = total_size;

        Ok(())
    }

    fn check_binary_files(&self, package_path: &Path, result: &mut ScanResult) {
        for entry in WalkDir::new(package_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            let extension = path.extension().and_then(|e| e.to_str());

            // Skip known binary types (fonts, images, etc.)
            match extension {
                Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg")
                | Some("woff") | Some("woff2") | Some("ttf") | Some("otf") => continue,
                _ => {}
            }

            // Check file size — non-source files > 1KB are suspicious
            if let Ok(metadata) = path.metadata() {
                if metadata.len() > 1024 {
                    // Try to read as text to determine if it's binary
                    match fs::read_to_string(path) {
                        Err(_) => {
                            result.add_finding(ScanFinding {
                                severity: Severity::Warning,
                                category: "binary_file".to_string(),
                                message: format!(
                                    "Non-text file > 1KB detected ({} bytes)",
                                    metadata.len()
                                ),
                                file: Some(path.to_string_lossy().to_string()),
                            });
                        }
                        Ok(_) => {} // It's text
                    }
                }
            }
        }
    }

    fn check_network_patterns(&self, package_path: &Path, result: &mut ScanResult) {
        let patterns = [
            "curl ",
            "wget ",
            "http://",
            "https://",
            "fetch(",
            "XMLHttpRequest",
        ];

        for entry in WalkDir::new(package_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            match fs::read_to_string(path) {
                Ok(content) => {
                    for pattern in &patterns {
                        if content.contains(pattern) {
                            result.add_finding(ScanFinding {
                                severity: Severity::Info,
                                category: "network".to_string(),
                                message: format!("Network access pattern detected: {}", pattern),
                                file: Some(path.to_string_lossy().to_string()),
                            });
                            break;
                        }
                    }
                }
                Err(_) => {} // Skip binary files
            }
        }
    }

    fn check_filesystem_patterns(&self, package_path: &Path, result: &mut ScanResult) {
        let patterns = ["fs.write", "writeFile", "open(", "createWriteStream"];

        for entry in WalkDir::new(package_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            match fs::read_to_string(path) {
                Ok(content) => {
                    for pattern in &patterns {
                        if content.contains(pattern) {
                            result.add_finding(ScanFinding {
                                severity: Severity::Info,
                                category: "filesystem".to_string(),
                                message: format!("Filesystem write pattern detected: {}", pattern),
                                file: Some(path.to_string_lossy().to_string()),
                            });
                            break;
                        }
                    }
                }
                Err(_) => {} // Skip binary files
            }
        }
    }

    fn check_env_patterns(&self, package_path: &Path, result: &mut ScanResult) {
        let patterns = ["process.env", "HOME", "SSH_AUTH_SOCK", "PATH"];

        for entry in WalkDir::new(package_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            match fs::read_to_string(path) {
                Ok(content) => {
                    for pattern in &patterns {
                        if content.contains(pattern) {
                            result.add_finding(ScanFinding {
                                severity: Severity::Info,
                                category: "environment".to_string(),
                                message: format!(
                                    "Environment variable access detected: {}",
                                    pattern
                                ),
                                file: Some(path.to_string_lossy().to_string()),
                            });
                            break;
                        }
                    }
                }
                Err(_) => {} // Skip binary files
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    fn create_test_skill(dir: &Path) -> Result<(), String> {
        // Create manifest.json
        let manifest = dir.join("manifest.json");
        let mut f = fs::File::create(&manifest).map_err(|e| e.to_string())?;
        f.write_all(b"{\"name\": \"test-skill\", \"version\": \"1.0.0\"}")
            .map_err(|e| e.to_string())?;

        // Create a simple script
        let script = dir.join("index.js");
        let mut f = fs::File::create(&script).map_err(|e| e.to_string())?;
        f.write_all(b"console.log('hello');")
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    #[test]
    fn test_scan_valid_skill() {
        let scanner = SkillScanner::new();
        let dir = tempdir().unwrap();
        create_test_skill(dir.path()).unwrap();

        let result = scanner.scan("test-skill", "1.0.0", dir.path()).unwrap();
        assert!(result.passed);
        assert_eq!(result.files_scanned, 2);
    }

    #[test]
    fn test_scan_missing_manifest() {
        let scanner = SkillScanner::new();
        let dir = tempdir().unwrap();

        // No manifest.json
        let script = dir.path().join("index.js");
        fs::write(&script, "console.log('hello');").unwrap();

        let result = scanner.scan("test-skill", "1.0.0", dir.path()).unwrap();
        assert!(!result.passed);
        assert!(result.findings.iter().any(|f| f.category == "manifest"));
    }

    #[test]
    fn test_scan_network_patterns() {
        let scanner = SkillScanner::new();
        let dir = tempdir().unwrap();

        // Create manifest and script with network call
        fs::write(dir.path().join("manifest.json"), "{\"name\": \"test\"}").unwrap();
        fs::write(
            dir.path().join("index.js"),
            "const response = fetch('https://example.com');",
        )
        .unwrap();

        let result = scanner.scan("test-skill", "1.0.0", dir.path()).unwrap();
        assert!(result.findings.iter().any(|f| f.category == "network"));
    }
}
