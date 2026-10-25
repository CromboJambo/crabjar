//! Audit Trail Query API
//!
//! Provides comprehensive query functions for the decision provenance system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Represents a single audit entry in the decision trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: i32,
    pub criteria_id: i32,
    pub version: i32,
    pub input_params: HashMap<String, String>,
    pub input_name: String,
    pub output_name: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub output_params: HashMap<String, String>,
    pub status: AuditStatus,
}

/// Status of an audit entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditStatus {
    Success,
    Failure,
    Skipped,
    Ignored,
}

/// The decision provenance system with audit trail capabilities.
pub struct DecisionProvenanceSystem {
    pub entries: HashMap<i32, AuditEntry>,
    pub criteria_versions: HashMap<i32, Vec<i32>>,
    pub decisions: HashMap<i32, HashMap<i32, AuditEntry>>,
}

impl DecisionProvenanceSystem {
    /// Initialize the system.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            criteria_versions: HashMap::new(),
            decisions: HashMap::new(),
        }
    }

    /// Add an audit entry to the system.
    pub fn add_entry(&mut self, entry: AuditEntry) {
        let id = entry.id;
        let version = entry.version;
        let criteria_id = entry.criteria_id;

        // Track versions for each criteria
        let versions = self.criteria_versions.entry(criteria_id).or_default();
        if !versions.contains(&version) {
            versions.push(version);
        }

        // Store the entry
        self.entries.insert(id, entry.clone());

        // Index by decision ID and version
        let decision_entries = self.decisions.entry(id).or_default();
        decision_entries.insert(version, entry);
    }

    /// List all decisions by criteria ID.
    pub fn list_by_criteria_id(&self, criteria_id: i32) -> Vec<&AuditEntry> {
        let mut results = Vec::new();
        for entry in self.entries.values() {
            if entry.criteria_id == criteria_id {
                results.push(entry);
            }
        }
        results
    }

    /// Get the full version history of a specific decision.
    pub fn get_version_history(&self, decision_id: i32) -> HashMap<i32, &AuditEntry> {
        let mut history = HashMap::new();
        if let Some(entries) = self.decisions.get(&decision_id) {
            for (version, entry) in entries {
                history.insert(*version, entry);
            }
        }
        history
    }

    /// Find all decisions made with a particular criteria version.
    pub fn find_by_criteria_version(&self, criteria_id: i32, version: i32) -> Vec<&AuditEntry> {
        let mut results = Vec::new();
        for entry in self.entries.values() {
            if entry.criteria_id == criteria_id && entry.version == version {
                results.push(entry);
            }
        }
        results
    }

    /// Search decisions by input parameters.
    pub fn search_by_input(&self, param_name: &str, param_value: &str) -> Vec<&AuditEntry> {
        let mut results = Vec::new();
        for entry in self.entries.values() {
            if let Some(value) = entry.input_params.get(param_name) {
                if *value == param_value {
                    results.push(entry);
                }
            }
        }
        results
    }

    /// Export audit trail as JSON.
    pub fn export_json(&self) -> serde_json::Value {
        let mut entries = Vec::new();
        for (id, entry) in &self.entries {
            entries.push(serde_json::to_value(entry).unwrap());
        }
        serde_json::json!({
            "total_entries": self.entries.len(),
            "entries": entries
        })
    }
}
