//! Update detection for installed skills — check for newer versions, notify user.

use crate::models::{SkillCacheEntry, UpdateCheckResult};
use chrono::Utc;
use tracing::info;

/// Detects available updates for installed skills.
pub struct UpdateDetector {
    // In a real implementation, this would have HTTP client for registry queries
}

impl Default for UpdateDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateDetector {
    /// Create a new update detector.
    pub fn new() -> Self {
        info!("initializing update detector");
        Self {}
    }

    /// Check if a specific skill has an available update.
    /// In production, this queries the ClawHub/GitHub API for latest version.
    pub fn check_for_update(
        &self,
        entry: &SkillCacheEntry,
        latest_version: Option<&str>,
    ) -> UpdateCheckResult {
        let has_update = match latest_version {
            Some(latest) => self.version_greater_than(latest, &entry.version),
            None => false,
        };

        info!(
            skill = %entry.name,
            current = %entry.version,
            latest = ?latest_version,
            has_update,
            "checked for update"
        );

        UpdateCheckResult {
            name: entry.name.clone(),
            current_version: entry.version.clone(),
            latest_version: latest_version.map(|s| s.to_string()),
            has_update,
            updated_at: Utc::now(),
        }
    }

    /// Check for updates across all cached skills.
    pub fn check_all_updates(
        &self,
        entries: &[&SkillCacheEntry],
        latest_versions: &std::collections::HashMap<String, String>,
    ) -> Vec<UpdateCheckResult> {
        entries
            .iter()
            .map(|entry| {
                let latest = latest_versions.get(&entry.name).map(|s| s.as_str());
                self.check_for_update(entry, latest)
            })
            .collect()
    }

    /// Simple semver comparison (major.minor.patch).
    fn version_greater_than(&self, newer: &str, older: &str) -> bool {
        let new_parts = self.parse_version(newer);
        let old_parts = self.parse_version(older);

        for (n, o) in new_parts.iter().zip(old_parts.iter()) {
            if n > o {
                return true;
            }
            if n < o {
                return false;
            }
        }

        // If all parts equal, longer version wins (e.g., 1.0.0-rc.1 > 1.0.0)
        new_parts.len() > old_parts.len()
    }

    fn parse_version(&self, version: &str) -> Vec<u32> {
        version
            .split('.')
            .filter_map(|part| part.parse::<u32>().ok())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        let detector = UpdateDetector::new();

        // Basic semver comparisons
        assert!(detector.version_greater_than("1.1.0", "1.0.0"));
        assert!(detector.version_greater_than("2.0.0", "1.9.9"));
        assert!(!detector.version_greater_than("1.0.0", "1.0.0"));
        assert!(!detector.version_greater_than("1.0.0", "1.1.0"));

        // Pre-release handling (simplified)
        assert!(detector.version_greater_than("1.0.1", "1.0.0"));
    }

    #[test]
    fn test_check_for_update() {
        let detector = UpdateDetector::new();
        let entry = SkillCacheEntry {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            source: "clawhub".to_string(),
            install_path: "/tmp/test".to_string(),
            download_url: None,
            checksum: None,
            trust_tier: crate::models::TrustTier::Community,
            installed_at: Utc::now(),
            last_used: Utc::now(),
        };

        let result = detector.check_for_update(&entry, Some("1.1.0"));
        assert!(result.has_update);
        assert_eq!(result.current_version, "1.0.0");
        assert_eq!(result.latest_version.as_deref(), Some("1.1.0"));
    }

    #[test]
    fn test_check_for_update_no_latest() {
        let detector = UpdateDetector::new();
        let entry = SkillCacheEntry {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            source: "clawhub".to_string(),
            install_path: "/tmp/test".to_string(),
            download_url: None,
            checksum: None,
            trust_tier: crate::models::TrustTier::Community,
            installed_at: Utc::now(),
            last_used: Utc::now(),
        };

        let result = detector.check_for_update(&entry, None);
        assert!(!result.has_update);
    }
}
