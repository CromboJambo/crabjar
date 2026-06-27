//! Action-related types for the guard system.
//!
//! Types: `ActionStatus`, `OutcomeStatus`, `ActionRequest`, `ActionOutcome`.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Status of an action request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionStatus {
    Pending,
    TrustApproved,
    Denied,
    Executed,
    Interrupted,
}

impl fmt::Display for ActionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionStatus::Pending => write!(f, "pending"),
            ActionStatus::TrustApproved => write!(f, "trust-approved"),
            ActionStatus::Denied => write!(f, "denied"),
            ActionStatus::Executed => write!(f, "executed"),
            ActionStatus::Interrupted => write!(f, "interrupted"),
        }
    }
}

/// Status of an action outcome record
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutcomeStatus {
    Executed,
    ExecutedTrustUpdateFailed,
}

impl fmt::Display for OutcomeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutcomeStatus::Executed => write!(f, "executed"),
            OutcomeStatus::ExecutedTrustUpdateFailed => write!(f, "executed-trust-update-failed"),
        }
    }
}

impl ActionStatus {
    pub fn is_pending(&self) -> bool {
        matches!(self, ActionStatus::Pending)
    }

    pub fn is_executed(&self) -> bool {
        matches!(self, ActionStatus::Executed)
    }

    pub fn is_denied(&self) -> bool {
        matches!(self, ActionStatus::Denied)
    }
}

/// Request to perform an action, gated by trust layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    pub id: String,
    /// Raw event ID from mirror-log — provenance of the triggering observation
    pub source_event_id: Option<String>,
    /// Memory node ID from mirror-guard — the derived knowledge authorizing this action
    pub source_node_id: Option<String>,
    pub action_type: String,
    pub payload: String,
    pub trust_layer: u32,
    pub confidence: super::TrustScore,
    pub status: ActionStatus,
    pub gate_result: Option<String>,
    pub requested_at: i64,
    pub resolved_at: Option<i64>,
}

impl ActionRequest {
    pub fn new(
        id: impl Into<String>,
        action_type: impl Into<String>,
        payload: impl Into<String>,
        trust_layer: u32,
        confidence: super::TrustScore,
    ) -> Self {
        Self {
            id: id.into(),
            source_event_id: None,
            source_node_id: None,
            action_type: action_type.into(),
            payload: payload.into(),
            trust_layer,
            confidence,
            status: ActionStatus::Pending,
            gate_result: None,
            requested_at: chrono::Utc::now().timestamp(),
            resolved_at: None,
        }
    }

    pub fn with_source(mut self, event_id: impl Into<String>) -> Self {
        self.source_event_id = Some(event_id.into());
        self
    }

    pub fn with_node(mut self, node_id: impl Into<String>) -> Self {
        self.source_node_id = Some(node_id.into());
        self
    }

    pub fn mark_approved(mut self) -> Self {
        self.status = ActionStatus::TrustApproved;
        self
    }

    pub fn mark_denied(mut self) -> Self {
        self.status = ActionStatus::Denied;
        self
    }

    pub fn mark_executed(mut self) -> Self {
        self.status = ActionStatus::Executed;
        self.resolved_at = Some(chrono::Utc::now().timestamp());
        self
    }

    pub fn mark_interrupted(mut self) -> Self {
        self.status = ActionStatus::Interrupted;
        self.resolved_at = Some(chrono::Utc::now().timestamp());
        self
    }
}

/// Outcome of an executed action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionOutcome {
    pub id: String,
    pub action_id: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub output_hash: Option<String>,
    pub residual: Option<String>,
    pub skill_residue: Option<String>,
    pub confidence_delta: f64,
    pub created_at: i64,
}

impl ActionOutcome {
    pub fn new(
        id: impl Into<String>,
        action_id: impl Into<String>,
        success: bool,
        confidence_delta: f64,
    ) -> Self {
        Self {
            id: id.into(),
            action_id: action_id.into(),
            success,
            exit_code: None,
            output_hash: None,
            residual: None,
            skill_residue: None,
            confidence_delta,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    pub fn with_exit_code(mut self, code: i32) -> Self {
        self.exit_code = Some(code);
        self
    }

    pub fn with_output_hash(mut self, hash: impl Into<String>) -> Self {
        self.output_hash = Some(hash.into());
        self
    }

    pub fn with_residual(mut self, residual: impl Into<String>) -> Self {
        self.residual = Some(residual.into());
        self
    }

    pub fn with_skill_residue(mut self, residue: impl Into<String>) -> Self {
        self.skill_residue = Some(residue.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust_types::TrustScore;

    #[test]
    fn action_status_display_pending() {
        assert_eq!(format!("{}", ActionStatus::Pending), "pending");
    }

    #[test]
    fn action_status_display_trust_approved() {
        assert_eq!(format!("{}", ActionStatus::TrustApproved), "trust-approved");
    }

    #[test]
    fn action_status_display_denied() {
        assert_eq!(format!("{}", ActionStatus::Denied), "denied");
    }

    #[test]
    fn action_status_display_executed() {
        assert_eq!(format!("{}", ActionStatus::Executed), "executed");
    }

    #[test]
    fn action_status_display_interrupted() {
        assert_eq!(format!("{}", ActionStatus::Interrupted), "interrupted");
    }

    #[test]
    fn outcome_status_display_executed() {
        assert_eq!(format!("{}", OutcomeStatus::Executed), "executed");
    }

    #[test]
    fn outcome_status_display_executed_trust_update_failed() {
        assert_eq!(
            format!("{}", OutcomeStatus::ExecutedTrustUpdateFailed),
            "executed-trust-update-failed"
        );
    }

    #[test]
    fn action_status_equality() {
        assert_eq!(ActionStatus::Pending, ActionStatus::Pending);
        assert_ne!(ActionStatus::Pending, ActionStatus::Denied);
    }

    #[test]
    fn outcome_status_equality() {
        assert_eq!(OutcomeStatus::Executed, OutcomeStatus::Executed);
        assert_ne!(
            OutcomeStatus::Executed,
            OutcomeStatus::ExecutedTrustUpdateFailed
        );
    }

    #[test]
    fn action_status_helpers() {
        assert!(ActionStatus::Pending.is_pending());
        assert!(!ActionStatus::Executed.is_pending());

        assert!(!ActionStatus::Pending.is_executed());
        assert!(ActionStatus::Executed.is_executed());

        assert!(!ActionStatus::Pending.is_denied());
        assert!(ActionStatus::Denied.is_denied());
    }

    #[test]
    fn action_request_new() {
        let request = ActionRequest::new(
            "test-1",
            "execute",
            "echo hello",
            2,
            TrustScore::new(0.7),
        );
        assert_eq!(request.id, "test-1");
        assert_eq!(request.action_type, "execute");
        assert_eq!(request.trust_layer, 2);
        assert_eq!(request.status, ActionStatus::Pending);
        assert!(request.source_event_id.is_none());
        assert!(request.source_node_id.is_none());
    }

    #[test]
    fn action_request_with_source() {
        let request = ActionRequest::new("test-2", "execute", "echo", 2, TrustScore::new(0.5))
            .with_source("event-123");
        assert_eq!(request.source_event_id, Some("event-123".to_string()));
    }

    #[test]
    fn action_request_with_node() {
        let request = ActionRequest::new("test-3", "execute", "echo", 2, TrustScore::new(0.5))
            .with_node("node-456");
        assert_eq!(request.source_node_id, Some("node-456".to_string()));
    }

    #[test]
    fn action_request_mark_approved() {
        let request = ActionRequest::new("test-4", "execute", "echo", 2, TrustScore::new(0.5))
            .mark_approved();
        assert_eq!(request.status, ActionStatus::TrustApproved);
    }

    #[test]
    fn action_request_mark_denied() {
        let request = ActionRequest::new("test-5", "execute", "echo", 2, TrustScore::new(0.5))
            .mark_denied();
        assert_eq!(request.status, ActionStatus::Denied);
    }

    #[test]
    fn action_request_mark_executed() {
        let request = ActionRequest::new("test-6", "execute", "echo", 2, TrustScore::new(0.5))
            .mark_executed();
        assert_eq!(request.status, ActionStatus::Executed);
        assert!(request.resolved_at.is_some());
    }

    #[test]
    fn action_request_mark_interrupted() {
        let request = ActionRequest::new("test-7", "execute", "echo", 2, TrustScore::new(0.5))
            .mark_interrupted();
        assert_eq!(request.status, ActionStatus::Interrupted);
        assert!(request.resolved_at.is_some());
    }

    #[test]
    fn action_outcome_new() {
        let outcome = ActionOutcome::new("out-1", "req-1", true, 0.1);
        assert_eq!(outcome.id, "out-1");
        assert_eq!(outcome.action_id, "req-1");
        assert!(outcome.success);
        assert_eq!(outcome.confidence_delta, 0.1);
        assert!(outcome.exit_code.is_none());
        assert!(outcome.output_hash.is_none());
    }

    #[test]
    fn action_outcome_with_all_fields() {
        let outcome = ActionOutcome::new("out-2", "req-2", false, -0.05)
            .with_exit_code(1)
            .with_output_hash("abc123")
            .with_residual("residual data")
            .with_skill_residue("skill residue");
        assert_eq!(outcome.exit_code, Some(1));
        assert_eq!(outcome.output_hash, Some("abc123".to_string()));
        assert_eq!(outcome.residual, Some("residual data".to_string()));
        assert_eq!(outcome.skill_residue, Some("skill residue".to_string()));
    }
}
