//! Skill manifest schema and parsing.
//!
//! Defines the standard format for skill manifests (manifest.json) that
//! crabjar-skills discovers and loads from the filesystem.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A tool definition within a skill manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique tool name (e.g., `cargo_check`).
    pub name: String,
    /// Human-readable description of what the tool does.
    #[serde(default)]
    pub description: String,
    /// Optional JSON Schema for tool parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    /// Execution policy (`low_risk`, `medium_risk`, `high_risk`). Defaults to `low_risk`.
    #[serde(default = "default_execution_policy")]
    pub execution_policy: String,
    /// Trust layer (0-4). Higher is more trusted. Defaults to 3.
    #[serde(default = "default_trust_layer")]
    pub trust_layer: u32,
}

fn default_execution_policy() -> String {
    "low_risk".to_string()
}

fn default_trust_layer() -> u32 {
    3
}

/// A dependency declaration within a skill manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Name of the dependency (crabjar crate name or binary).
    pub name: String,
    /// Optional version constraint (semver range).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Whether this is a required dependency. Defaults to true.
    #[serde(default = "default_required")]
    pub required: bool,
}

fn default_required() -> bool {
    true
}

/// Context requirements for a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRequirements {
    /// Required environment variables.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env_vars: Vec<String>,
    /// Required filesystem paths (directories or files).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<PathBuf>,
}

/// The standard skill manifest format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    /// Unique skill name (lowercase, hyphens allowed).
    pub name: String,
    /// Semantic version of the skill.
    #[serde(default = "default_version")]
    pub version: String,
    /// Human-readable description of the skill.
    #[serde(default)]
    pub description: String,
    /// Author or organization responsible for the skill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// License identifier (e.g., "MIT", "Apache-2.0").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Tools provided by this skill.
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
    /// Dependencies of this skill.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Dependency>,
    /// Context requirements for execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_requirements: Option<ContextRequirements>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

impl SkillManifest {
    /// Parse a skill manifest from JSON content.
    pub fn parse(content: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(content)
    }

    /// Validate the manifest structure.
    #[must_use]
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.name.is_empty() {
            errors.push("name is required".to_string());
        }

        if !self
            .name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit())
        {
            errors.push("name must be lowercase alphanumeric with hyphens only".to_string());
        }

        for tool in &self.tools {
            if tool.name.is_empty() {
                errors.push("tool name is required".to_string());
            }
            if !matches!(
                tool.execution_policy.as_str(),
                "low_risk" | "medium_risk" | "high_risk"
            ) {
                errors.push(format!(
                    "invalid execution policy '{}' for tool '{}'",
                    tool.execution_policy, tool.name
                ));
            }
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_manifest() {
        let content = r#"{"name": "test-skill"}"#;
        let manifest = SkillManifest::parse(content).unwrap();
        assert_eq!(manifest.name, "test-skill");
        assert_eq!(manifest.version, "0.1.0");
        assert!(manifest.tools.is_empty());
    }

    #[test]
    fn test_parse_full_manifest() {
        let content = r#"{
            "name": "cargo-tools",
            "version": "1.0.0",
            "description": "Cargo CLI tools",
            "author": "crabjar",
            "tools": [
                {
                    "name": "cargo_check",
                    "description": "Run cargo check",
                    "execution_policy": "low_risk"
                }
            ]
        }"#;
        let manifest = SkillManifest::parse(content).unwrap();
        assert_eq!(manifest.name, "cargo-tools");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.tools.len(), 1);
        assert_eq!(manifest.tools[0].name, "cargo_check");
    }

    #[test]
    fn test_validate_name_format() {
        let manifest = SkillManifest {
            name: "Valid-Name".to_string(),
            version: "0.1.0".to_string(),
            description: String::new(),
            author: None,
            license: None,
            tools: Vec::new(),
            dependencies: Vec::new(),
            context_requirements: None,
        };
        let errors = manifest.validate();
        assert!(errors.iter().any(|e| e.contains("lowercase")));
    }

    #[test]
    fn test_validate_execution_policy() {
        let manifest = SkillManifest {
            name: "test".to_string(),
            version: "0.1.0".to_string(),
            description: String::new(),
            author: None,
            license: None,
            tools: vec![ToolDefinition {
                name: "tool".to_string(),
                description: String::new(),
                parameters: None,
                execution_policy: "invalid_policy".to_string(),
                trust_layer: 3,
            }],
            dependencies: Vec::new(),
            context_requirements: None,
        };
        let errors = manifest.validate();
        assert!(errors.iter().any(|e| e.contains("execution policy")));
    }
}
