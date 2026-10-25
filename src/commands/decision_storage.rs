//! Decision Input and Criteria Storage
//!
//! Provides persistent storage for decision inputs, versioned criteria definitions,
//! and links between decisions and specific criteria versions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Timestamp type for storage operations.
pub type Timestamp = u64;

/// Source information for a decision input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// A criteria definition with versioning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Criteria {
    pub name: String,
    pub version: String,
    pub description: String,
    pub rules: Vec<String>,
}

/// A decision input with timestamp and source information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionInput {
    pub timestamp: Timestamp,
    pub source: Source,
    pub decision_name: String,
    pub content: String,
}

/// Link between a decision and a specific criteria version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionLink {
    pub decision_name: String,
    pub criteria_version: String,
    pub timestamp: Timestamp,
}

/// Historical criteria version for audit purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalCriteriaVersion {
    pub criteria: Criteria,
    pub version: String,
    pub timestamp: Timestamp,
    pub source: Source,
}

/// Decision storage module with persistent storage capabilities.
pub struct DecisionStorage {
    inputs: HashMap<String, Vec<DecisionInput>>,
    criteria: HashMap<String, Vec<Criteria>>,
    links: HashMap<String, Vec<DecisionLink>>,
}

impl DecisionStorage {
    /// Create a new decision storage instance.
    pub fn new() -> Self {
        Self {
            inputs: HashMap::new(),
            criteria: HashMap::new(),
            links: HashMap::new(),
        }
    }

    /// Store a decision input with timestamp and source information.
    pub fn store_decision_input(&mut self, input: DecisionInput) {
        let entry = self.inputs.entry(input.decision_name.clone()).or_default();
        entry.push(input);
    }

    /// Store a versioned criteria definition.
    pub fn store_criteria(&mut self, criteria: Criteria) {
        let entry = self.criteria.entry(criteria.name.clone()).or_default();
        entry.push(criteria);
    }

    /// Store a link between a decision and a specific criteria version.
    pub fn store_decision_link(&mut self, link: DecisionLink) {
        let entry = self.links.entry(link.decision_name.clone()).or_default();
        entry.push(link);
    }

    /// Retrieve historical criteria versions for audit purposes.
    pub fn retrieve_criteria_version(&self, criteria_name: &str, version: &str) -> Option<&Criteria> {
        if let Some(versions) = self.criteria.get(criteria_name) {
            for criteria in versions {
                if criteria.version == version {
                    return Some(criteria);
                }
            }
        }
        None
    }

    /// Rollback to a previous criteria version.
    pub fn rollback_to_previous_criteria(&self, criteria_name: &str, version: &str) -> Option<&Criteria> {
        self.retrieve_criteria_version(criteria_name, version)
    }

    /// Get all decision inputs for a specific decision.
    pub fn get_decision_inputs(&self, decision_name: &str) -> &[DecisionInput] {
        self.inputs.get(decision_name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get all criteria versions for a specific criteria name.
    pub fn get_criteria_versions(&self, criteria_name: &str) -> &[Criteria] {
        self.criteria.get(criteria_name).map(|v| v.as_slice()).unwrap_or(&[])
    }
}
