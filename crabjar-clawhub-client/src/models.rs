//! Data models for ClawHub API interactions.

use serde::{Deserialize, Serialize};
use chrono::DateTime;

/// Skill information from ClawHub registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: Option<String>,
    pub author: String,
    pub versions: Vec<SkillVersion>,
    pub latest_version: String,
    pub download_count: u32,
    pub created_at: Option<DateTime<chrono::Utc>>,
    pub updated_at: Option<DateTime<chrono::Utc>>,
}

/// Specific version of a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVersion {
    pub version: String,
    pub download_url: String,
    pub checksum: Option<String>,
    pub published_at: Option<DateTime<chrono::Utc>>,
}

impl SkillInfo {
    /// Get the latest version info.
    pub fn latest(&self) -> Option<&SkillVersion> {
        self.versions.iter().find(|v| v.version == self.latest_version)
    }

    /// Get a specific version by semver.
    pub fn get_version(&self, version: &str) -> Option<&SkillVersion> {
        self.versions.iter().find(|v| v.version == version)
    }
}

/// Options for searching skills on ClawHub.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub query: String,
    pub limit: u32,
    pub offset: u32,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            query: String::new(),
            limit: 20,
            offset: 0,
        }
    }
}
