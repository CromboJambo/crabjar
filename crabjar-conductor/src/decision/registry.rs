//! Decision capability registry — declarative registration of JEV decision types.
//!
//! Each decision capability is defined by a JSON manifest that declares:
//! - What triggers it (observation patterns)
//! - What criteria file to use
//! - Whether it requires the bounded KEV-9B model or can use embedding similarity
//! - What context factors should be included
//!
//! This makes decision capabilities first-class, discoverable entities that the
//! orchestrator knows about and can invoke declaratively.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// A registered decision capability definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionCapability {
    /// Unique identifier for this decision type (e.g., "upgrade-impact-evaluation")
    pub name: String,

    /// Human-readable description of what this decision evaluates
    pub description: String,

    /// Path to the JEV criteria JSON file that defines options and evaluation factors
    pub criteria_file: PathBuf,

    /// Observation patterns that should trigger this decision type.
    /// Simple substring matching for now; could evolve to regex or semantic matching.
    #[serde(default)]
    pub triggers: Vec<String>,

    /// Context factors that should be included when evaluating this decision.
    /// These are looked up from the agent's memory/context and injected into the prompt.
    #[serde(default)]
    pub context_factors: Vec<String>,

    /// If true, always route to the bounded KEV-9B model regardless of embedding similarity.
    /// Use for high-stakes decisions where nuanced reasoning matters more than speed.
    #[serde(default)]
    pub requires_bounded_model: bool,

    /// Embedding similarity threshold below which to escalate to KEV-9B.
    /// Only applies if requires_bounded_model is false. Default: 0.75.
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f64,

    /// Endpoint for the bounded decision model (KEV-9B). If null, uses default from config.
    #[serde(default)]
    pub model_endpoint: Option<String>,

    /// Maximum context window size for the bounded model evaluation. Default: 8192.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

fn default_similarity_threshold() -> f64 {
    0.75
}

fn default_max_tokens() -> usize {
    8192
}

/// Registry of all decision capabilities known to the conductor.
pub struct DecisionRegistry {
    /// Map from decision name to capability definition
    capabilities: HashMap<String, DecisionCapability>,

    /// Reverse index: trigger pattern → set of decision names that should be considered
    trigger_index: HashMap<String, Vec<String>>,
}

impl DecisionRegistry {
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
            trigger_index: HashMap::new(),
        }
    }

    /// Register a decision capability from its definition.
    pub fn register(&mut self, capability: DecisionCapability) {
        // Validate that criteria file exists
        if !capability.criteria_file.exists() {
            warn!(
                "Criteria file not found for decision '{}': {}",
                capability.name,
                capability.criteria_file.display()
            );
        }

        // Index trigger patterns
        for trigger in &capability.triggers {
            self.trigger_index
                .entry(trigger.clone())
                .or_insert_with(Vec::new)
                .push(capability.name.clone());
        }

        info!(
            "Registered decision capability '{}' with {} triggers",
            capability.name,
            capability.triggers.len()
        );

        self.capabilities.insert(capability.name.clone(), capability);
    }

    /// Load and register all decision capabilities from a directory.
    /// Each .json file in the directory is treated as a capability definition.
    pub fn load_from_directory<P: AsRef<Path>>(&mut self, dir: P) -> Result<usize, String> {
        let dir = dir.as_ref();

        if !dir.exists() {
            return Err(format!("Decision capabilities directory not found: {}", dir.display()));
        }

        let mut count = 0;

        for entry in std::fs::read_dir(dir).map_err(|e| format!("Failed to read directory {}: {}", dir.display(), e))? {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "json") {
                match self.load_from_file(&path) {
                    Ok(capability) => {
                        self.register(capability);
                        count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to load decision capability from {}: {}", path.display(), e);
                    }
                }
            }
        }

        info!(
            "Loaded {} decision capabilities from {}",
            count,
            dir.display()
        );

        Ok(count)
    }

    /// Load a single decision capability definition from a file.
    pub fn load_from_file<P: AsRef<Path>>(&self, path: P) -> Result<DecisionCapability, String> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            format!("Failed to read decision capability file {}: {}", path.display(), e)
        })?;

        serde_json::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse decision capability file {}: {}",
                path.display(),
                e
            )
        })
    }

    /// Find decision capabilities that should be triggered by an observation.
    pub fn find_matches(&self, observation: &str) -> Vec<&DecisionCapability> {
        let mut matches = Vec::new();

        for (trigger, decision_names) in &self.trigger_index {
            if observation.contains(trigger.as_str()) {
                for name in decision_names {
                    if let Some(capability) = self.capabilities.get(name) {
                        // Avoid duplicates
                        if !matches.iter().any(|m: &&DecisionCapability| m.name == capability.name) {
                            matches.push(capability);
                        }
                    }
                }
            }
        }

        matches
    }

    /// Get a specific decision capability by name.
    pub fn get(&self, name: &str) -> Option<&DecisionCapability> {
        self.capabilities.get(name)
    }

    /// List all registered decision capabilities.
    pub fn list_capabilities(&self) -> Vec<&DecisionCapability> {
        let mut caps = self.capabilities.values().collect::<Vec<_>>();
        caps.sort_by(|a, b| a.name.cmp(&b.name));
        caps
    }

    /// Get the total number of registered capabilities.
    pub fn count(&self) -> usize {
        self.capabilities.len()
    }

    /// Serialize registry state for API exposure.
    pub fn to_json(&self) -> serde_json::Value {
        let capabilities: Vec<serde_json::Value> = self
            .list_capabilities()
            .iter()
            .map(|cap| {
                serde_json::json!({
                    "name": cap.name,
                    "description": cap.description,
                    "triggers": cap.triggers,
                    "context_factors": cap.context_factors,
                    "requires_bounded_model": cap.requires_bounded_model,
                    "similarity_threshold": cap.similarity_threshold,
                })
            })
            .collect();

        serde_json::json!({
            "count": self.count(),
            "capabilities": capabilities
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_registry_new() {
        let registry = DecisionRegistry::new();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_register_and_get() {
        let mut registry = DecisionRegistry::new();

        let capability = DecisionCapability {
            name: "test-decision".to_string(),
            description: "Test decision capability".to_string(),
            criteria_file: PathBuf::from("/tmp/test.json"),
            triggers: vec!["upgrade".to_string()],
            context_factors: Vec::new(),
            requires_bounded_model: false,
            similarity_threshold: 0.75,
            model_endpoint: None,
            max_tokens: 8192,
        };

        registry.register(capability);

        assert_eq!(registry.count(), 1);
        let retrieved = registry.get("test-decision");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().triggers.len(), 1);
    }

    #[test]
    fn test_trigger_matching() {
        let mut registry = DecisionRegistry::new();

        let cap1 = DecisionCapability {
            name: "upgrade-eval".to_string(),
            description: "Evaluate upgrades".to_string(),
            criteria_file: PathBuf::from("/tmp/upgrade.json"),
            triggers: vec!["upgrading".to_string()],
            context_factors: Vec::new(),
            requires_bounded_model: false,
            similarity_threshold: 0.75,
            model_endpoint: None,
            max_tokens: 8192,
        };

        let cap2 = DecisionCapability {
            name: "security-eval".to_string(),
            description: "Evaluate security".to_string(),
            criteria_file: PathBuf::from("/tmp/security.json"),
            triggers: vec!["vulnerability".to_string()],
            context_factors: Vec::new(),
            requires_bounded_model: false,
            similarity_threshold: 0.75,
            model_endpoint: None,
            max_tokens: 8192,
        };

        registry.register(cap1);
        registry.register(cap2);

        // Test matching
        let matches = registry.find_matches("Service is upgrading to v2.0");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "upgrade-eval");

        let matches = registry.find_matches("Found a vulnerability in auth module");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "security-eval");

        // No match
        let matches = registry.find_matches("Just doing routine maintenance");
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_load_from_directory() {
        let dir = tempdir().unwrap();

        // Create a capability definition file
        let cap_file = dir.path().join("test-cap.json");
        fs::write(
            &cap_file,
            r#"{
                "name": "test-from-file",
                "description": "Loaded from directory",
                "criteria_file": "/tmp/criteria.json",
                "triggers": ["test-trigger"]
            }"#,
        )
        .unwrap();

        let mut registry = DecisionRegistry::new();
        let count = registry.load_from_directory(dir.path()).unwrap();

        assert_eq!(count, 1);
        assert!(registry.get("test-from-file").is_some());
    }

    #[test]
    fn test_to_json() {
        let mut registry = DecisionRegistry::new();

        let capability = DecisionCapability {
            name: "json-test".to_string(),
            description: "Test JSON serialization".to_string(),
            criteria_file: PathBuf::from("/tmp/test.json"),
            triggers: vec!["trigger1".to_string()],
            context_factors: vec!["factor1".to_string()],
            requires_bounded_model: true,
            similarity_threshold: 0.8,
            model_endpoint: Some("http://example.com".to_string()),
            max_tokens: 4096,
        };

        registry.register(capability);

        let json = registry.to_json();
        assert_eq!(json["count"], 1);
        assert_eq!(json["capabilities"][0]["name"], "json-test");
        assert_eq!(json["capabilities"][0]["requires_bounded_model"], true);
    }
}
