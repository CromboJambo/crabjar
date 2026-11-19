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

impl<State> fmt::Debug for GuardedAction<State> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GuardedAction")
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
#[path = "action_tests.rs"]
mod action_tests;
