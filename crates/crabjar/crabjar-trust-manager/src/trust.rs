//! Trust tier management — maps external registry trust levels to CrabJar execution policies.

use crate::models::TrustTier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// Trust policy configuration for a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTrustPolicy {
    pub name: String,
    pub trust_tier: TrustTier,
    pub execution_policy: String,
    pub requires_confirmation: bool,
}

/// Manages trust tier assignments and policy mapping for remote skills.
pub struct TrustManager {
    policies: HashMap<String, SkillTrustPolicy>,
}

impl Default for TrustManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TrustManager {
    /// Create a new trust manager with empty policy map.
    pub fn new() -> Self {
        info!("initializing trust manager");
        Self {
            policies: HashMap::new(),
        }
    }

    /// Assign a trust tier to a skill and compute its execution policy.
    pub fn assign_trust(&mut self, name: &str, tier: TrustTier) -> SkillTrustPolicy {
        let policy = match tier {
            TrustTier::Community => SkillTrustPolicy {
                name: name.to_string(),
                trust_tier: tier,
                execution_policy: "low_risk".to_string(),
                requires_confirmation: true,
            },
            TrustTier::Verified => SkillTrustPolicy {
                name: name.to_string(),
                trust_tier: tier,
                execution_policy: "medium_risk".to_string(),
                requires_confirmation: false,
            },
            TrustTier::Official => SkillTrustPolicy {
                name: name.to_string(),
                trust_tier: tier,
                execution_policy: "auto_approve".to_string(),
                requires_confirmation: false,
            },
        };

        info!(
            skill = %name,
            tier = %tier,
            policy = %policy.execution_policy,
            "assigned trust tier"
        );

        self.policies.insert(name.to_string(), policy.clone());
        policy
    }

    /// Get the trust policy for a skill.
    pub fn get_policy(&self, name: &str) -> Option<&SkillTrustPolicy> {
        self.policies.get(name)
    }

    /// Check if a skill requires user confirmation before execution.
    pub fn requires_confirmation(&self, name: &str) -> bool {
        self.policies
            .get(name)
            .map(|p| p.requires_confirmation)
            .unwrap_or(true) // Default to requiring confirmation
    }

    /// Check if a specific action is allowed for a skill based on its trust tier.
    pub fn action_allowed(&self, name: &str, action_risk: &str) -> bool {
        let policy = match self.policies.get(name) {
            Some(p) => p,
            None => return true, // Unknown skills default to allowed (conservative)
        };

        match (policy.trust_tier, action_risk) {
            (TrustTier::Community, "low_risk") => true,
            (TrustTier::Community, _) => false,
            (TrustTier::Verified, "low_risk" | "medium_risk") => true,
            (TrustTier::Verified, _) => false,
            (TrustTier::Official, _) => true,
        }
    }

    /// List all managed skills with their trust policies.
    pub fn list_policies(&self) -> Vec<&SkillTrustPolicy> {
        self.policies.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_community_tier_requires_confirmation() {
        let mut tm = TrustManager::new();
        let policy = tm.assign_trust("community-skill", TrustTier::Community);
        assert!(policy.requires_confirmation);
        assert_eq!(policy.execution_policy, "low_risk");
    }

    #[test]
    fn test_verified_tier_no_confirmation() {
        let mut tm = TrustManager::new();
        let policy = tm.assign_trust("verified-skill", TrustTier::Verified);
        assert!(!policy.requires_confirmation);
        assert_eq!(policy.execution_policy, "medium_risk");
    }

    #[test]
    fn test_official_tier_auto_approve() {
        let mut tm = TrustManager::new();
        let policy = tm.assign_trust("official-skill", TrustTier::Official);
        assert!(!policy.requires_confirmation);
        assert_eq!(policy.execution_policy, "auto_approve");
    }

    #[test]
    fn test_action_allowed_community() {
        let mut tm = TrustManager::new();
        tm.assign_trust("community-skill", TrustTier::Community);
        assert!(tm.action_allowed("community-skill", "low_risk"));
        assert!(!tm.action_allowed("community-skill", "medium_risk"));
    }

    #[test]
    fn test_action_allowed_verified() {
        let mut tm = TrustManager::new();
        tm.assign_trust("verified-skill", TrustTier::Verified);
        assert!(tm.action_allowed("verified-skill", "low_risk"));
        assert!(tm.action_allowed("verified-skill", "medium_risk"));
        assert!(!tm.action_allowed("verified-skill", "high_risk"));
    }

    #[test]
    fn test_action_allowed_official() {
        let mut tm = TrustManager::new();
        tm.assign_trust("official-skill", TrustTier::Official);
        assert!(tm.action_allowed("official-skill", "low_risk"));
        assert!(tm.action_allowed("official-skill", "medium_risk"));
        assert!(tm.action_allowed("official-skill", "high_risk"));
    }
}
