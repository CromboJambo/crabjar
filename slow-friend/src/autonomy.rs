//! Earned Autonomy System — tracks action-class history and determines
//! permission levels based on approval patterns. Implements ADR-007:
//! topological uncertainty model where autonomy is determined by the density
//! of supporting relationships around a proposed action.

use crate::action::{Action, ActionClass, ActionResult, PermissionLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Per-action-class approval history record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionClassRecord {
    pub class: ActionClass,
    /// Total attempts of this action class.
    pub total_attempts: u64,
    /// Approved by human.
    pub approved: u64,
    /// Rejected by human.
    pub rejected: u64,
    /// Reversed after-the-fact.
    pub reversed: u64,
    /// Current earned permission level.
    pub permission: PermissionLevel,
}

impl ActionClassRecord {
    fn new(class: ActionClass) -> Self {
        Self {
            class,
            total_attempts: 0,
            approved: 0,
            rejected: 0,
            reversed: 0,
            permission: PermissionLevel::Ask,
        }
    }
}

/// Topological autonomy level derived from approval history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutonomyLevel {
    /// Known path (dense support) → ACT autonomously.
    Act,
    /// Weak path (some support) → PROPOSE to user.
    Propose,
    /// New path (no prior experience) → ASK user.
    Ask,
}

/// Tracks approval history per action class and determines earned autonomy.
pub struct AutonomyTracker {
    records: HashMap<ActionClass, ActionClassRecord>,
    /// Threshold for promoting from Ask to Propose.
    propose_threshold: u64,
    /// Threshold for promoting from Propose to Autonomous.
    autonomous_threshold: u64,
}

impl AutonomyTracker {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
            propose_threshold: 3,
            autonomous_threshold: 10,
        }
    }

    /// Record the outcome of an action attempt.
    pub fn record_outcome(&mut self, action: &Action, result: &ActionResult) {
        let record = match self.records.get_mut(&action.class) {
            Some(r) => r,
            None => {
                let new_record = ActionClassRecord::new(action.class.clone());
                self.records.insert(action.class.clone(), new_record);
                self.records.get_mut(&action.class).unwrap()
            }
        };

        record.total_attempts += 1;
        match result {
            ActionResult::Success { .. } => {
                record.approved += 1;
            }
            ActionResult::Failure { .. } | ActionResult::Rejected { .. } => {
                record.rejected += 1;
            }
        }

        // Re-evaluate permission level based on approval ratio
        let approval_ratio = if record.total_attempts > 0 {
            record.approved as f64 / record.total_attempts as f64
        } else {
            0.0
        };

        record.permission = if record.approved >= self.autonomous_threshold
            && approval_ratio >= 0.9
        {
            PermissionLevel::Autonomous
        } else if record.approved >= self.propose_threshold && approval_ratio >= 0.75 {
            PermissionLevel::Notify
        } else {
            PermissionLevel::Ask
        };
    }

    /// Determine the autonomy level for an action class based on history.
    pub fn autonomy_level(&self, class: &ActionClass) -> AutonomyLevel {
        match self.records.get(class) {
            Some(record) => match record.permission {
                PermissionLevel::Autonomous => AutonomyLevel::Act,
                PermissionLevel::Notify => AutonomyLevel::Propose,
                PermissionLevel::Ask => AutonomyLevel::Ask,
            },
            None => AutonomyLevel::Ask,
        }
    }

    /// Get the approval history for an action class (for inspectable provenance).
    pub fn get_record(&self, class: &ActionClass) -> Option<&ActionClassRecord> {
        self.records.get(class)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_action_classes_start_at_ask() {
        let tracker = AutonomyTracker::new();
        assert_eq!(
            tracker.autonomy_level(&ActionClass::DiskCleanup),
            AutonomyLevel::Ask
        );
    }

    #[test]
    fn earns_autonomy_through_approvals() {
        let mut tracker = AutonomyTracker::new();

        // Record 12 successful outcomes (above autonomous threshold of 10)
        for _ in 0..12 {
            let action = Action {
                class: ActionClass::DiskCleanup,
                description: "Test".to_string(),
                parameters: serde_json::json!({}),
            };
            tracker.record_outcome(&action, &ActionResult::Success {
                details: "ok".to_string(),
            });
        }

        assert_eq!(
            tracker.autonomy_level(&ActionClass::DiskCleanup),
            AutonomyLevel::Act
        );
    }
}
