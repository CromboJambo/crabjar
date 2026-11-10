//! Persistence types for GuardDb — records read from or written to the database.

use serde::{Deserialize, Serialize};

/// A persisted trust resolution record.
#[derive(Debug, Clone)]
pub struct TrustResolutionEntry {
    pub id: i64,
    pub action_id: Option<String>,
    pub requested_layer: u32,
    pub requested_confidence: f64,
    pub requested_source: String,
    pub effective_layer: u32,
    pub effective_confidence: f64,
    pub effective_by: String,
    pub scope_actor: Option<String>,
    pub scope_target: Option<String>,
    pub applied_policies: String, // JSON array of policy strings
    pub resolved_at: i64,
}

/// A single state transition recorded in the action_transitions audit table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionTransition {
    pub id: i64,
    pub action_id: String,
    pub from_status: String,
    pub to_status: String,
    /// Authority that performed this transition (e.g., "guard", "user:alice", "arbiter")
    pub authority: String,
    /// Optional reason/context for the transition
    pub reason: Option<String>,
    pub created_at: i64,
}

/// Error returned when an action state transition is invalid.
#[derive(Debug, thiserror::Error)]
pub enum TransitionError {
    #[error("Action not found: {action_id}")]
    ActionNotFound { action_id: String },

    #[error("Unknown status value in database: '{status}' (action {action_id})")]
    UnknownStatus { action_id: String, status: String },

    #[error(
        "Invalid transition for action {action_id}: cannot go from {from} to {to}. \
         Valid transitions from {from}: {valid}"
    )]
    InvalidTransition {
        action_id: String,
        from: String,
        to: String,
        valid: String,
    },
    #[error(
        "Rehydration validation failed for action {action_id}: \
         transition log ends at {log_status} but row says {row_status}"
    )]
    StateMismatch {
        action_id: String,
        log_status: String,
        row_status: String,
    },

    #[error("Transition log is empty for action {action_id} — cannot validate state")]
    EmptyTransitionLog { action_id: String },
}

/// Validate that a transition from `from` to `to` is legal according to the
/// typestate machine defined in `GuardedAction`.
///
/// Legal transitions:
/// - Pending → TrustApproved (approve)
/// - Pending → Denied (deny)
/// - TrustApproved → Executed (execute)
/// - TrustApproved → Interrupted (interrupt)
pub fn validate_transition(from: &str, to: &str) -> Result<(), TransitionError> {
    use crate::action::ActionStatus;

    let from_status = parse_status(from)?;
    let to_status = parse_status(to)?;

    let valid_targets: Vec<ActionStatus> = match from_status {
        ActionStatus::Pending => vec![ActionStatus::TrustApproved, ActionStatus::Denied],
        ActionStatus::TrustApproved => vec![ActionStatus::Executed, ActionStatus::Interrupted],
        // Terminal states have no outgoing transitions
        ActionStatus::Denied | ActionStatus::Executed | ActionStatus::Interrupted => vec![],
    };

    if valid_targets.contains(&to_status) {
        Ok(())
    } else {
        let valid_str = valid_targets
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        Err(TransitionError::InvalidTransition {
            action_id: String::new(), // filled in by caller if needed
            from: from.to_string(),
            to: to.to_string(),
            valid: valid_str,
        })
    }
}

/// Parse a status string into ActionStatus, returning an error for unknown values.
/// Unlike the old silent fallback (`_ => Pending`), this fails loudly on corruption.
pub fn parse_status(status: &str) -> Result<crate::action::ActionStatus, TransitionError> {
    use crate::action::ActionStatus;

    match status {
        "pending" => Ok(ActionStatus::Pending),
        "trust-approved" => Ok(ActionStatus::TrustApproved),
        "denied" => Ok(ActionStatus::Denied),
        "executed" => Ok(ActionStatus::Executed),
        "interrupted" => Ok(ActionStatus::Interrupted),
        other => Err(TransitionError::UnknownStatus {
            action_id: String::new(),
            status: other.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::ActionStatus;

    #[test]
    fn valid_pending_to_approved() {
        assert!(validate_transition("pending", "trust-approved").is_ok());
    }

    #[test]
    fn valid_pending_to_denied() {
        assert!(validate_transition("pending", "denied").is_ok());
    }

    #[test]
    fn valid_approved_to_executed() {
        assert!(validate_transition("trust-approved", "executed").is_ok());
    }

    #[test]
    fn valid_approved_to_interrupted() {
        assert!(validate_transition("trust-approved", "interrupted").is_ok());
    }

    #[test]
    fn invalid_pending_to_executed() {
        let err = validate_transition("pending", "executed").unwrap_err();
        assert!(matches!(err, TransitionError::InvalidTransition { .. }));
    }

    #[test]
    fn invalid_denied_to_pending() {
        let err = validate_transition("denied", "pending").unwrap_err();
        assert!(matches!(err, TransitionError::InvalidTransition { .. }));
    }

    #[test]
    fn invalid_executed_to_anything() {
        let err = validate_transition("executed", "pending").unwrap_err();
        assert!(matches!(err, TransitionError::InvalidTransition { .. }));
    }

    #[test]
    fn unknown_status_fails_loudly() {
        let err = parse_status("corrupted-value").unwrap_err();
        assert!(matches!(err, TransitionError::UnknownStatus { .. }));
    }

    #[test]
    fn all_known_statuses_parse() {
        assert_eq!(parse_status("pending").unwrap(), ActionStatus::Pending);
        assert_eq!(
            parse_status("trust-approved").unwrap(),
            ActionStatus::TrustApproved
        );
        assert_eq!(parse_status("denied").unwrap(), ActionStatus::Denied);
        assert_eq!(parse_status("executed").unwrap(), ActionStatus::Executed);
        assert_eq!(
            parse_status("interrupted").unwrap(),
            ActionStatus::Interrupted
        );
    }
}
