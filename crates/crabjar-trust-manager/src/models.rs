//! Data models for trust management and skill caching.

use chrono::DateTime;
use serde::{Deserialize, Serialize};

/// Trust tiers for remote skills.
/// Maps to ClawHub's community/verified/official system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustTier {
    /// User-submitted, minimal review. Requires confirmation for medium/high risk actions.
    Community,
    /// Passed security scan, reviewed by CrabJar maintainers. Trusted by default.
    Verified,
    /// Maintained by CrabJar team or trusted partners. Highest trust, auto-approve policies.
    Official,
}

impl std::fmt::Display for TrustTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrustTier::Community => write!(f, "community"),
            TrustTier::Verified => write!(f, "verified"),
            TrustTier::Official => write!(f, "official"),
        }
    }
}

impl TrustTier {
    /// Parse trust tier from string.
    pub fn from_str(s: &str) -> Result<Self, crate::TrustManagerError> {
        match s.to_lowercase().as_str() {
            "community" => Ok(TrustTier::Community),
            "verified" => Ok(TrustTier::Verified),
            "official" => Ok(TrustTier::Official),
            _ => Err(crate::TrustManagerError::UnknownTrustTier(s.to_string())),
        }
    }

    /// Map trust tier to CrabJar execution policy.
    /// Community → low_risk default, requires confirmation for medium/high
    /// Verified → trusted by default (medium_risk allowed)
    /// Official → auto-approve all policies
    pub fn execution_policy(&self) -> &'static str {
        match self {
            TrustTier::Community => "low_risk",
            TrustTier::Verified => "medium_risk",
            TrustTier::Official => "auto_approve",
        }
    }

    /// Map trust tier to numeric trust layer (for tool registry).
    pub fn trust_layer(&self) -> u32 {
        match self {
            TrustTier::Community => 1,
            TrustTier::Verified => 2,
            TrustTier::Official => 3,
        }
    }
}

/// Cache entry for a downloaded skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCacheEntry {
    pub name: String,
    pub version: String,
    pub source: String, // "clawhub" or "github"
    pub install_path: String,
    pub download_url: Option<String>,
    pub checksum: Option<String>,
    pub trust_tier: TrustTier,
    pub installed_at: DateTime<chrono::Utc>,
    pub last_used: DateTime<chrono::Utc>,
}

/// Result of an update check for a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    pub name: String,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub has_update: bool,
    pub updated_at: DateTime<chrono::Utc>,
}
