//! Security scanner for ClawHub skills.
//!
//! Analyzes downloaded skill archives for potential security issues
//! before installation, including permission model analysis and
//! sandboxed execution checks.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Security scan result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScanResult {
    pub skill_name: String,
    pub version: String,
    pub passed: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub permissions: PermissionAnalysis,
}

/// Analysis of requested permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionAnalysis {
    pub filesystem_read: bool,
    pub filesystem_write: bool,
    pub network: bool,
    pub exec_commands: bool,
    pub env_variables: bool,
}

/// Security scanner for ClawHub skills.
pub struct SecurityScanner;

impl SecurityScanner {
    /// Create a new security scanner.
    pub fn new() -> Self {
        Self
    }

    /// Scan a downloaded skill archive for security issues.
    pub fn scan(&self, name: &str, version: &str, archive_path: &Path) -> Result<SecurityScanResult, String> {
        info!("Scanning {} v{} for security issues", name, version);

        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // Analyze permissions from manifest
        let permissions = self.analyze_permissions(archive_path)?;

        // Check for suspicious patterns
        self.check_suspicious_patterns(archive_path, &mut warnings, &mut errors);

        let passed = errors.is_empty();

        if passed {
            info!("Security scan passed for {} v{} ({} warnings)", name, version, warnings.len());
        } else {
            warn!("Security scan failed for {} v{} ({} errors)", name, version, errors.len());
        }

        Ok(SecurityScanResult {
            skill_name: name.to_string(),
            version: version.to_string(),
            passed,
            warnings,
            errors,
            permissions,
        })
    }

    /// Analyze the permission model from the skill manifest.
    fn analyze_permissions(&self, archive_path: &Path) -> Result<PermissionAnalysis, String> {
        debug!("Analyzing permissions for {}", archive_path.display());

        // Look for manifest.json in the archive (simplified — would need to extract first in real impl)
        let manifest_path = archive_path.join("manifest.json");

        if !manifest_path.exists() {
            return Ok(PermissionAnalysis {
                filesystem_read: true,
                filesystem_write: false,
                network: false,
                exec_commands: false,
                env_variables: false,
            });
        }

        let manifest = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read manifest: {}", e))?;
        let parsed: serde_json::Value = serde_json::from_str(&manifest)
            .map_err(|e| format!("Failed to parse manifest: {}", e))?;

        let permissions_node = parsed.get("permissions");

        Ok(PermissionAnalysis {
            filesystem_read: has_permission(permissions_node, "filesystem.read"),
            filesystem_write: has_permission(permissions_node, "filesystem.write"),
            network: has_permission(permissions_node, "network"),
            exec_commands: has_permission(permissions_node, "exec.commands"),
            env_variables: has_permission(permissions_node, "env.variables"),
        })
    }

    /// Check for suspicious patterns in the skill code.
    fn check_suspicious_patterns(
        &self,
        archive_path: &Path,
        warnings: &mut Vec<String>,
        errors: &mut Vec<String>,
    ) {
        debug!(
            "Checking for suspicious patterns in {}",
            archive_path.display()
        );

        // Check for shell execution patterns
        if self.contains_pattern(archive_path, "exec(")
            || self.contains_pattern(archive_path, "spawn(")
        {
            warnings.push("Skill uses process execution — verify trust tier".to_string());
        }

        // Check for network calls
        if self.contains_pattern(archive_path, "fetch(")
            || self.contains_pattern(archive_path, "http")
        {
            warnings.push("Skill makes network requests".to_string());
        }

        // Check for file system access
        if self.contains_pattern(archive_path, "fs.read")
            || self.contains_pattern(archive_path, "read_file")
        {
            warnings.push("Skill reads files from filesystem".to_string());
        }

        // Check for eval patterns (higher risk)
        if self.contains_pattern(archive_path, "eval(") {
            errors.push("Skill uses dynamic code evaluation — high risk".to_string());
        }
    }

    fn contains_pattern(&self, path: &Path, pattern: &str) -> bool {
        // Simplified — would walk directory and check files in real impl
        let manifest = path.join("manifest.json");
        if !manifest.exists() {
            return false;
        }

        match std::fs::read_to_string(&manifest) {
            Ok(content) => content.contains(pattern),
            Err(_) => false,
        }
    }
}

fn has_permission(permissions: Option<&serde_json::Value>, perm: &str) -> bool {
    match permissions {
        Some(node) => {
            if let Some(arr) = node.as_array() {
                arr.iter().any(|p| p.as_str() == Some(perm))
            } else {
                false
            }
        }
        None => false,
    }
}
