//! Decision Provenance Logging
//!
//! Provides comprehensive logging with criteria versioning, input tracking,
//! and parent decision linking for full audit trail capabilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A decision input that was used to make a decision.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionInput {
    pub id: String,
    pub name: String,
    pub value: String,
}

/// A stored decision with full provenance information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Decision {
    pub id: String,
    pub criteria_id: String,
    pub criteria_version: u64,
    pub decision_inputs: Vec<String>,
    pub decision_result: String,
    pub parent_decision_id: Option<String>,
    pub created_at: String,
}

impl Decision {
    /// Create a new decision with provenance fields.
    pub fn new(
        criteria_id: String,
        criteria_version: u64,
        decision_inputs: Vec<String>,
        decision_result: String,
        parent_decision_id: Option<String>,
        created_at: String,
    ) -> Self {
        Self {
            id: format!("dec-{}", Uuid::new_v4()),
            criteria_id,
            criteria_version,
            decision_inputs,
            decision_result,
            parent_decision_id,
            created_at,
        }
    }
}

/// Mock evidence store for testing provenance logging.
pub struct MockEvidenceStore {
    decisions: HashMap<String, Decision>,
    version_history: HashMap<String, Vec<(u64, String)>>,
}

impl MockEvidenceStore {
    pub fn new() -> Self {
        Self {
            decisions: HashMap::new(),
            version_history: HashMap::new(),
        }
    }

    /// Store a decision with provenance fields.
    pub fn store_decision(
        &mut self,
        criteria_id: String,
        criteria_version: u64,
        decision_inputs: Vec<String>,
        decision_result: String,
        parent_decision_id: Option<String>,
    ) -> String {
        let id = format!("dec-{}", Uuid::new_v4());
        let decision = Decision::new(
            criteria_id.clone(),
            criteria_version,
            decision_inputs,
            decision_result,
            parent_decision_id,
            chrono::Utc::now().to_rfc3339(),
        );

        self.decisions.insert(id.clone(), decision);

        // Track version history for criteria
        let entry = self.version_history.entry(criteria_id).or_default();
        entry.push((criteria_version, id.clone()));

        id
    }

    pub fn get_decision(&self, id: &str) -> Option<&Decision> {
        self.decisions.get(id)
    }

    pub fn get_all_decisions(&self) -> Vec<&Decision> {
        self.decisions.values().collect()
    }

    pub fn get_criteria_version_history(&self, criteria_id: &str) -> Vec<(u64, String)> {
        self.version_history
            .get(criteria_id)
            .cloned()
            .unwrap_or_default()
    }
}
