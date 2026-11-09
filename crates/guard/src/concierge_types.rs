//! Gate concierge types — extracted from concierge.rs.
//!
//! Types: `PendingQueueEntry`, `InterruptedLogEntry`, `GateConcierge`,
//! `ApprovalStore`, and the typestate-wrapped `ConciergeOutcome`.
use crate::action::{ActionRequest, ActionStatus, Denied, GuardedAction, Pending, TrustApproved};
use crate::fingerprint::ApprovalLease;
use crate::fingerprint::ApprovalScope;
use serde::{Deserialize, Serialize};

/// Pending queue entry for actions requiring review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingQueueEntry {
    pub id: String,
    pub gate_result_id: String,
    pub action_type: String,
    pub command: String,
    pub args: Vec<String>,
    pub trust_layer: u32,
    pub confidence: f64,
    pub source_event_id: Option<String>,
    pub queued_at: i64,
    pub reason: String,
}

/// Interrupted log entry for actions blocked by the gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptedLogEntry {
    pub id: String,
    pub gate_result_id: String,
    pub action_type: String,
    pub command: String,
    pub args: Vec<String>,
    pub trust_layer: u32,
    pub source_event_id: Option<String>,
    pub reason: String,
    pub logged_at: i64,
}

/// Wrapper for the approval store with a default in-memory implementation.
#[derive(Default)]
pub struct ApprovalStore {
    inner: crate::fingerprint::InMemoryApprovalStore,
}

impl ApprovalStore {
    pub fn new() -> Self {
        Self {
            inner: crate::fingerprint::InMemoryApprovalStore::new(),
        }
    }

    pub fn insert(&self, lease: ApprovalLease) {
        self.inner.insert(lease);
    }

    pub fn find_matching(
        &self,
        fingerprint: &crate::fingerprint::InvocationFingerprint,
        scope: &ApprovalScope,
    ) -> Option<ApprovalLease> {
        self.inner.find_matching(fingerprint, scope)
    }

    pub fn list_valid(&self) -> Vec<ApprovalLease> {
        self.inner.list_valid()
    }

    pub fn cleanup_expired(&self) {
        self.inner.cleanup_expired();
    }

    pub fn revoke_scope(&self, scope: &ApprovalScope) {
        self.inner.revoke_scope(scope);
    }
}

/// Gate concierge that enforces provenance boundaries on gate results.
///
/// Pending → PendingQueue (queued, not executed).
/// Interrupted → InterruptedLog (logged, returned, not proceeded).
/// No tool call path bypasses the gate.
#[derive(Default)]
pub struct GateConcierge {
    pub db: Option<crate::guard_db::GuardDb>,
    /// In-memory store for exact-invocation fingerprint approval leases.
    /// IronClaw's `ironclaw_approvals` uses this pattern to prevent
    /// approval smuggling: approving `cp src dst` does NOT approve `cp src malicious`.
    pub approval_store: ApprovalStore,
}

// ─── Typed concierge outcome ─────────────────────────────────────────────────

/// Typestate-wrapped outcome of a concierge enforcement decision.
///
/// Replaces the previous raw `(ActionStatus, Option<PendingQueueEntry>,
/// Option<InterruptedLogEntry>)` tuple with a sum type that carries the
/// action in its compile-time-verified state:
///
/// - [`ConciergeOutcome::Approved`] — trust layer cleared it; ready to execute.
///   The inner action is `GuardedAction<TrustApproved>`, so only `.execute()`
///   and `.interrupt()` are available.
/// - [`ConciergeOutcome::Queued`] — needs human review. The inner action is
///   still `GuardedAction<Pending>`; a future approval decision can call
///   `.approve()` or `.deny()`.
/// - [`ConciergeOutcome::Denied`] — blocked by the gate. Terminal state; no
///   further transitions possible.
///
/// This makes invalid post-concierge operations unrepresentable at compile time:
/// you cannot execute a denied action, approve an already-approved one, or
/// forget to handle a pending entry.
pub enum ConciergeOutcome {
    /// Trust layer approved; safe to execute.
    Approved(GuardedAction<TrustApproved>),
    /// Queued for human review; decision pending.
    Queued(GuardedAction<Pending>, PendingQueueEntry),
    /// Denied/interrupted by the gate. Terminal.
    Denied(GuardedAction<Denied>, InterruptedLogEntry),
}

impl ConciergeOutcome {
    /// Returns true only if this outcome is ready for execution.
    pub fn is_executable(&self) -> bool {
        matches!(self, Self::Approved(_))
    }

    /// Returns the action status (for telemetry/logging).
    pub fn status(&self) -> ActionStatus {
        match self {
            Self::Approved(a) => a.as_ref().status.clone(),
            Self::Queued(a, _) => a.as_ref().status.clone(),
            Self::Denied(a, _) => a.as_ref().status.clone(),
        }
    }

    /// Consume the outcome and return the plain [`ActionRequest`] for storage.
    pub fn into_action(self) -> ActionRequest {
        match self {
            Self::Approved(a) => a.into_inner(),
            Self::Queued(a, _) => a.into_inner(),
            Self::Denied(a, _) => a.into_inner(),
        }
    }

    /// Returns the pending queue entry if this is a `Queued` outcome.
    pub fn pending_entry(&self) -> Option<&PendingQueueEntry> {
        match self {
            Self::Queued(_, entry) => Some(entry),
            _ => None,
        }
    }

    /// Returns the interrupted log entry if this is a `Denied` outcome.
    pub fn interrupted_entry(&self) -> Option<&InterruptedLogEntry> {
        match self {
            Self::Denied(_, entry) => Some(entry),
            _ => None,
        }
    }
}

impl std::fmt::Debug for ConciergeOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Approved(a) => f
                .debug_struct("ConciergeOutcome::Approved")
                .field("action", a)
                .finish(),
            Self::Queued(a, entry) => f
                .debug_struct("ConciergeOutcome::Queued")
                .field("action", a)
                .field("pending_id", &entry.id)
                .finish(),
            Self::Denied(a, entry) => f
                .debug_struct("ConciergeOutcome::Denied")
                .field("action", a)
                .field("interrupted_id", &entry.id)
                .field("reason", &entry.reason)
                .finish(),
        }
    }
}
