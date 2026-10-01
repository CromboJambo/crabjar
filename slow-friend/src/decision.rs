//! Tier 3: Jev Decision Layer — expensive semantic reasoning invoked only when
//! cheaper checks detect something genuinely ambiguous that requires judgment.

use crate::action::{Action, ActionClass};
use crate::interpretation::Interpretation;
use serde::{Deserialize, Serialize};

/// A decision made by the Jev layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JevDecision {
    /// Not worth acting on after all — go back to sleep.
    Sleep,
    /// Propose an action for CrabJar execution (requires guard/concierge approval).
    Execute(Action),
}

/// The Jev decision engine — expensive semantic reasoning that should only be
/// invoked when structural interpretation finds something meaningful but
/// ambiguous. This is where LLM inference or complex policy evaluation happens.
pub trait JevEngine: Send + Sync {
    /// Given an interpreted event, decide whether to act and what action to take.
    fn decide(&self, interpretation: &Interpretation) -> JevDecision;

    /// Optional: explain the reasoning behind a decision (for audit trails).
    fn explain(&self, _interpretation: &Interpretation, _decision: &JevDecision) -> String {
        "No explanation available".to_string()
    }
}

/// Simple rule-based Jev engine for testing/demonstration.
pub struct RuleBasedJevEngine;

impl RuleBasedJevEngine {
    pub fn new() -> Self {
        Self
    }
}

impl JevEngine for RuleBasedJevEngine {
    fn decide(&self, interpretation: &Interpretation) -> JevDecision {
        // Simple rule: high severity disk events trigger cleanup action
        let is_disk = interpretation.source_event.source.contains("disk")
            || interpretation.source_event.event_type.contains("disk");

        if interpretation.severity > 0.5 && is_disk {
            JevDecision::Execute(Action {
                class: ActionClass::DiskCleanup,
                description: "Clean up old temp files".to_string(),
                parameters: serde_json::json!({"target": "/tmp"}),
            })
        } else {
            JevDecision::Sleep
        }
    }

    fn explain(&self, interpretation: &Interpretation, decision: &JevDecision) -> String {
        match decision {
            JevDecision::Sleep => format!(
                "Severity {} below action threshold or non-disk event",
                interpretation.severity
            ),
            JevDecision::Execute(action) => format!(
                "High severity disk event; proposing {}",
                action.description
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpretation::{HeuristicInterpreter, InterpretationEngine};
    use crate::sensor::SensorEvent;

    #[test]
    fn decides_to_act_on_severe_disk_events() {
        let jev = RuleBasedJevEngine::new();
        let interp = HeuristicInterpreter::new().interpret(SensorEvent {
            source: "disk".to_string(),
            event_type: "high_usage".to_string(),
            timestamp: 0,
            payload: serde_json::json!({"usage": 95.0}),
        });

        match jev.decide(&interp) {
            JevDecision::Execute(action) => {
                assert_eq!(action.class, ActionClass::DiskCleanup);
            }
            JevDecision::Sleep => panic!("Expected action for severe disk event"),
        }
    }
}
