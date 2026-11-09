//! Action-related types for the guard system.
//!
//! Types: `ActionStatus`, `OutcomeStatus`, `ActionRequest`, `ActionOutcome`,
//! and typestate-wrapped `GuardedAction<State>` for compile-time state transition safety.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::marker::PhantomData;

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

/// Request to perform an action, gated by trust layer.
///
/// This is the plain data representation used for database serialization.
/// For compile-time state transition safety in application code, use
/// [`GuardedAction`] instead.
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

    /// Mark as approved. Prefer [`GuardedAction::approve`] for type-safe transitions.
    pub fn mark_approved(mut self) -> Self {
        self.status = ActionStatus::TrustApproved;
        self
    }

    /// Mark as denied. Prefer [`GuardedAction::deny`] for type-safe transitions.
    pub fn mark_denied(mut self) -> Self {
        self.status = ActionStatus::Denied;
        self
    }

    /// Mark as executed. Prefer [`GuardedAction::execute`] for type-safe transitions.
    pub fn mark_executed(mut self) -> Self {
        self.status = ActionStatus::Executed;
        self.resolved_at = Some(chrono::Utc::now().timestamp());
        self
    }

    /// Mark as interrupted. Prefer [`GuardedAction::interrupt`] for type-safe transitions.
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

// ─── Typestate markers ───────────────────────────────────────────────────────

/// Zero-sized type marker: action is waiting for approval.
pub struct Pending;
/// Zero-sized type marker: action has been approved, ready to execute.
pub struct TrustApproved;
/// Zero-sized type marker: action was denied (terminal).
pub struct Denied;
/// Zero-sized type marker: action executed successfully (terminal).
pub struct Executed;
/// Zero-sized type marker: action was interrupted (terminal).
pub struct Interrupted;

// ─── Typestate-wrapped action ────────────────────────────────────────────────

/// A typestate-wrapped action request that enforces valid state transitions
/// at compile time.
///
/// The state machine is:
/// ```text
/// Pending ──approve()──▶ TrustApproved ──execute()──▶ Executed     (terminal)
///    │                        │
///    └─────deny()─────────────┴──interrupt()──▶ Interrupted        (terminal)
///              │
///              ▼
///           Denied                                                (terminal)
/// ```
///
/// Invalid transitions (e.g., executing a denied action, approving an executed
/// action) are impossible — the compiler rejects them.
///
/// Convert to/from the plain [`ActionRequest`] for database serialization:
/// - `GuardedAction::new(request)` wraps a pending request
/// - `.into_inner()` unwraps to the plain representation
///
/// # Example
/// ```rust
/// use crabjar_guard::{ActionRequest, GuardedAction, Pending, TrustScore};
///
/// let request = ActionRequest::new("a1", "execute", "echo hi", 2, TrustScore::new(0.8));
/// let action: GuardedAction<Pending> = GuardedAction::new(request);
///
/// // Only approve() and deny() are available in Pending state
/// let approved = action.approve();
///
/// // Only execute() and interrupt() are available in TrustApproved state
/// let done = approved.execute();
///
/// // Terminal state: no further transitions possible
/// let plain: ActionRequest = done.into_inner();
/// assert_eq!(plain.status, crabjar_guard::ActionStatus::Executed);
/// ```
pub struct GuardedAction<State> {
    inner: ActionRequest,
    _state: PhantomData<State>,
}

impl fmt::Debug for GuardedAction<Pending> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GuardedAction<Pending>")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<State> GuardedAction<State> {
    /// Consume and return the plain [`ActionRequest`] for serialization or storage.
    pub fn into_inner(self) -> ActionRequest {
        self.inner
    }
}

impl<State> std::convert::AsRef<ActionRequest> for GuardedAction<State> {
    fn as_ref(&self) -> &ActionRequest {
        &self.inner
    }
}

impl GuardedAction<Pending> {
    /// Wrap a pending action request in a typestate guard.
    ///
    /// # Panics
    /// If the request's status is not [`ActionStatus::Pending`].
    pub fn new(request: ActionRequest) -> Self {
        assert!(
            matches!(request.status, ActionStatus::Pending),
            "GuardedAction::new requires a Pending action, got {:?}",
            request.status
        );
        Self {
            inner: request,
            _state: PhantomData,
        }
    }

    /// Approve the action. Only valid from [`Pending`] state.
    pub fn approve(self) -> GuardedAction<TrustApproved> {
        let mut inner = self.inner;
        inner.status = ActionStatus::TrustApproved;
        GuardedAction {
            inner,
            _state: PhantomData,
        }
    }

    /// Deny the action. Only valid from [`Pending`] state. Terminal transition.
    pub fn deny(self) -> GuardedAction<Denied> {
        let mut inner = self.inner;
        inner.status = ActionStatus::Denied;
        inner.resolved_at = Some(chrono::Utc::now().timestamp());
        GuardedAction {
            inner,
            _state: PhantomData,
        }
    }
}

impl GuardedAction<TrustApproved> {
    /// Execute the action. Only valid from [`TrustApproved`] state. Terminal transition.
    pub fn execute(self) -> GuardedAction<Executed> {
        let mut inner = self.inner;
        inner.status = ActionStatus::Executed;
        inner.resolved_at = Some(chrono::Utc::now().timestamp());
        GuardedAction {
            inner,
            _state: PhantomData,
        }
    }

    /// Interrupt the action. Only valid from [`TrustApproved`] state. Terminal transition.
    pub fn interrupt(self) -> GuardedAction<Interrupted> {
        let mut inner = self.inner;
        inner.status = ActionStatus::Interrupted;
        inner.resolved_at = Some(chrono::Utc::now().timestamp());
        GuardedAction {
            inner,
            _state: PhantomData,
        }
    }
}

// Terminal states: no state-changing methods, only inspection and unwrapping.
// (The blanket impl above provides as_ref() and into_inner().)

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
        let request =
            ActionRequest::new("test-1", "execute", "echo hello", 2, TrustScore::new(0.7));
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
        let request =
            ActionRequest::new("test-5", "execute", "echo", 2, TrustScore::new(0.5)).mark_denied();
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

    // ─── Typestate tests ─────────────────────────────────────────────────────

    #[test]
    fn guarded_action_new_requires_pending() {
        let request = ActionRequest::new("gs-1", "execute", "echo", 2, TrustScore::new(0.5));
        let action: GuardedAction<Pending> = GuardedAction::new(request);
        assert_eq!(action.as_ref().status, ActionStatus::Pending);
    }

    #[test]
    #[should_panic(expected = "GuardedAction::new requires a Pending action")]
    fn guarded_action_new_rejects_non_pending() {
        let request =
            ActionRequest::new("gs-2", "execute", "echo", 2, TrustScore::new(0.5)).mark_denied();
        let _action: GuardedAction<Pending> = GuardedAction::new(request);
    }

    #[test]
    fn guarded_action_approve_transitions_to_trust_approved() {
        let request = ActionRequest::new("gs-3", "execute", "echo", 2, TrustScore::new(0.8));
        let action: GuardedAction<Pending> = GuardedAction::new(request);
        let approved = action.approve();
        assert_eq!(approved.as_ref().status, ActionStatus::TrustApproved);
    }

    #[test]
    fn guarded_action_deny_transitions_to_denied() {
        let request = ActionRequest::new("gs-4", "execute", "echo", 2, TrustScore::new(0.3));
        let action: GuardedAction<Pending> = GuardedAction::new(request);
        let denied = action.deny();
        assert_eq!(denied.as_ref().status, ActionStatus::Denied);
        assert!(denied.as_ref().resolved_at.is_some());
    }

    #[test]
    fn guarded_action_execute_transitions_to_executed() {
        let request = ActionRequest::new("gs-5", "execute", "echo", 2, TrustScore::new(0.9));
        let action: GuardedAction<Pending> = GuardedAction::new(request);
        let executed = action.approve().execute();
        assert_eq!(executed.as_ref().status, ActionStatus::Executed);
        assert!(executed.as_ref().resolved_at.is_some());
    }

    #[test]
    fn guarded_action_interrupt_transitions_to_interrupted() {
        let request = ActionRequest::new("gs-6", "execute", "echo", 2, TrustScore::new(0.9));
        let action: GuardedAction<Pending> = GuardedAction::new(request);
        let interrupted = action.approve().interrupt();
        assert_eq!(interrupted.as_ref().status, ActionStatus::Interrupted);
        assert!(interrupted.as_ref().resolved_at.is_some());
    }

    #[test]
    fn guarded_action_into_inner_preserves_data() {
        let request = ActionRequest::new("gs-7", "execute", "echo hi", 3, TrustScore::new(0.7))
            .with_source("event-999");
        let action: GuardedAction<Pending> = GuardedAction::new(request);
        let plain = action.approve().execute().into_inner();
        assert_eq!(plain.id, "gs-7");
        assert_eq!(plain.action_type, "execute");
        assert_eq!(plain.payload, "echo hi");
        assert_eq!(plain.trust_layer, 3);
        assert_eq!(plain.status, ActionStatus::Executed);
        assert_eq!(plain.source_event_id, Some("event-999".to_string()));
    }

    #[test]
    fn guarded_action_as_ref_does_not_consume() {
        let request = ActionRequest::new("gs-8", "execute", "echo", 2, TrustScore::new(0.5));
        let action: GuardedAction<Pending> = GuardedAction::new(request);
        let ref_data = action.as_ref();
        assert_eq!(ref_data.id, "gs-8");
        // action is still usable after as_ref()
        let approved = action.approve();
        assert_eq!(approved.as_ref().status, ActionStatus::TrustApproved);
    }

    #[test]
    fn guarded_action_full_lifecycle() {
        // Pending → TrustApproved → Executed
        let request = ActionRequest::new(
            "gs-9",
            "execute",
            "rm -rf /tmp/test",
            4,
            TrustScore::new(0.95),
        );
        let pending: GuardedAction<Pending> = GuardedAction::new(request);
        assert_eq!(pending.as_ref().status, ActionStatus::Pending);

        let approved = pending.approve();
        assert_eq!(approved.as_ref().status, ActionStatus::TrustApproved);

        let executed = approved.execute();
        assert_eq!(executed.as_ref().status, ActionStatus::Executed);
        assert!(executed.as_ref().resolved_at.is_some());

        // Terminal state: can only inspect or unwrap
        let final_request = executed.into_inner();
        assert_eq!(final_request.status, ActionStatus::Executed);
    }

    #[test]
    fn guarded_action_denied_is_terminal() {
        let request =
            ActionRequest::new("gs-10", "execute", "dangerous cmd", 1, TrustScore::new(0.2));
        let pending: GuardedAction<Pending> = GuardedAction::new(request);
        let denied = pending.deny();
        assert_eq!(denied.as_ref().status, ActionStatus::Denied);

        // Terminal: no approve/execute/interrupt methods available (compile-time enforced)
        let final_request = denied.into_inner();
        assert_eq!(final_request.status, ActionStatus::Denied);
    }

    #[test]
    fn guarded_action_preserves_gate_result() {
        let mut request = ActionRequest::new("gs-11", "execute", "echo", 2, TrustScore::new(0.8));
        request.gate_result = Some("approved by policy engine".to_string());
        let action: GuardedAction<Pending> = GuardedAction::new(request);
        let executed = action.approve().execute();
        assert_eq!(
            executed.as_ref().gate_result,
            Some("approved by policy engine".to_string())
        );
    }
}
