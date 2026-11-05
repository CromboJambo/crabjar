//! Gate concierge types — extracted from concierge.rs.
//!
//! Types: `PendingQueueEntry`, `InterruptedLogEntry`, `GateConcierge`, `ApprovalStore`.
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
