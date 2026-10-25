//! Decision Versioning System
//!
//! Provides versioned corrections to decisions, maintaining an immutable audit trail.
//! Each correction creates a new decision record linked to its parent via parent_decision_id.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of corrections that can be applied to a decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CorrectionType {
    /// The decision was based on incorrect facts.
    Factual,
    /// A constraint was missing or incorrect.
    Constraint,
    /// User preference differs from what was assumed.
    Preference,
    /// Policy or rule governing the decision has changed.
    Policy,
    /// The outcome of the previous decision was unsatisfactory.
    Outcome,
}

impl std::fmt::Display for CorrectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CorrectionType::Factual => write!(f, "factual"),
            CorrectionType::Constraint => write!(f, "constraint"),
            CorrectionType::Preference => write!(f, "preference"),
            CorrectionType::Policy => write!(f, "policy"),
            CorrectionType::Outcome => write!(f, "outcome"),
        }
    }
}

/// A correction applied to a decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionCorrection {
    /// The ID of the decision being corrected.
    pub parent_decision_id: i64,
    /// The type of correction.
    pub correction_type: CorrectionType,
    /// Explanation for why the correction is needed.
    pub reason: String,
    /// The new decision option selected after correction.
    pub new_decision_option: String,
}

/// Decision versioning manager that handles creating and tracking decision versions.
pub struct DecisionVersioner {
    conn: Connection,
}

impl DecisionVersioner {
    /// Create a new DecisionVersioner with the given database connection.
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    /// Apply a correction to a decision, creating a new version.
    ///
    /// Returns the ID of the new decision record.
    pub fn apply_correction(&self, correction: &DecisionCorrection) -> Result<i64, Box<dyn std::error::Error>> {
        // Insert the corrected decision as a new record linked to the parent
        let result = self.conn.execute(
            "INSERT INTO decisions (
                observation_preview, observation_hash, decision_option, confidence,
                method, components_invoked, computation_time_ms, outcome, created_at,
                criteria_version, decision_inputs, parent_decision_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                "Corrected decision",
                format!("corrected_{}", correction.parent_decision_id),
                correction.new_decision_option,
                0.95_f64, // High confidence for corrected decisions
                "correction",
                serde_json::to_string(&vec!["correction_engine"])?,
                0_u64,
                None::<String>,
                chrono::Utc::now().timestamp(),
                "",
                serde_json::to_string(&correction.reason)?,
                correction.parent_decision_id
            ],
        );

        match result {
            Ok(_) => Ok(self.conn.last_insert_rowid()),
            Err(e) => Err(Box::new(e)),
        }
    }

    /// Retrieve the full version history of a decision.
    ///
    /// Returns a vector of decision IDs from oldest to newest.
    pub fn get_version_history(&self, decision_id: i64) -> Result<Vec<i64>, Box<dyn std::error::Error>> {
        let mut history = Vec::new();
        let mut current_id = decision_id;

        loop {
            history.push(current_id);

            // Find the parent decision
            let statement = self.conn.prepare("SELECT id FROM decisions WHERE parent_decision_id = ?")?;
            let rows = statement.query_map(params![current_id], |row| row.get::<_, i64>(0))?;

            let mut found_parent = false;
            for row in rows {
                current_id = row?;
                history.push(current_id);
                found_parent = true;
                break;
            }

            if !found_parent {
                break;
            }
        }

        history.reverse();
        Ok(history)
    }

    /// Identify the latest version of a decision.
    ///
    /// Returns the ID of the most recent decision in the version chain.
    pub fn get_latest_version(&self, initial_decision_id: i64) -> Result<i64, Box<dyn std::error::Error>> {
        let history = self.get_version_history(initial_decision_id)?;
        Ok(*history.last().unwrap())
    }

    /// Compare two versions of a decision to identify changes.
    pub fn compare_versions(&self, older_id: i64, newer_id: i64) -> Result<DecisionDiff, Box<dyn std::error::Error>> {
        let older = self.get_decision(older_id)?;
        let newer = self.get_decision(newer_id)?;

        Ok(DecisionDiff {
            decision_option_changed: older.decision_option != newer.decision_option,
            confidence_changed: (older.confidence - newer.confidence).abs() > 0.01,
            method_changed: older.method != newer.method,
            parent_id: Some(newer.parent_decision_id),
        })
    }

    fn get_decision(&self, id: i64) -> Result<DecisionRecord, Box<dyn std::error::Error>> {
        let statement = self.conn.prepare(
            "SELECT id, observation_preview, observation_hash, decision_option, confidence, method, components_invoked, computation_time_ms, outcome, created_at FROM decisions WHERE id = ?",
        )?;

        let row = statement.query_row(params![id], |row| {
            Ok(DecisionRecord {
                id: row.get(0)?,
                observation_preview: row.get(1)?,
                observation_hash: row.get(2)?,
                decision_option: row.get(3)?,
                confidence: row.get(4)?,
                method: row.get(5)?,
                components_invoked: serde_json::from_str(&row.get::<_, String>(6)?)?,
                computation_time_ms: row.get(7)?,
                outcome: row.get(8)?,
                created_at: row.get(9)?,
            })
        });

        match row {
            Ok(record) => Ok(record),
            Err(e) => Err(Box::new(e)),
        }
    }
}

/// Represents the differences between two decision versions.
#[derive(Debug, Clone)]
pub struct DecisionDiff {
    pub decision_option_changed: bool,
    pub confidence_changed: bool,
    pub method_changed: bool,
    pub parent_id: Option<i64>,
}
