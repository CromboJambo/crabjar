use crate::fingerprint::{ApprovalLease, ApprovalScope, InvocationFingerprint};
use crate::GateResult;
use crate::guard_db::{GuardDb, GuardDbError};
use crate::types::ActionStatus;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use uuid::Uuid;

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

/// Gate concierge that enforces provenance boundaries on gate results.
///
/// Pending → PendingQueue (queued, not executed).
/// Interrupted → InterruptedLog (logged, returned, not proceeded).
/// No tool call path bypasses the gate.
#[derive(Default)]
pub struct GateConcierge {
    pub db: Option<GuardDb>,
    /// In-memory store for exact-invocation fingerprint approval leases.
    /// IronClaw's `ironclaw_approvals` uses this pattern to prevent
    /// approval smuggling: approving `cp src dst` does NOT approve `cp src malicious`.
    pub approval_store: ApprovalStore,
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
        fingerprint: &InvocationFingerprint,
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

impl GateConcierge {
    pub fn new() -> Self {
        Self {
            db: None,
            approval_store: ApprovalStore::new(),
        }
    }

    pub fn with_db(mut self, db: GuardDb) -> Self {
        self.db = Some(db);
        self
    }

    pub fn with_approval_store(mut self, store: ApprovalStore) -> Self {
        self.approval_store = store;
        self
    }

    #[allow(clippy::too_many_arguments)]
    /// Enforce a gate result through provenance boundaries.
    /// Returns a boundary_enforced status and any queued/logged entries.
    pub fn enforce(
        &mut self,
        gate_result: GateResult,
        action_type: &str,
        command: &str,
        args: &[String],
        trust_layer: u32,
        confidence: f64,
        source_event_id: Option<String>,
    ) -> (
        ActionStatus,
        Option<PendingQueueEntry>,
        Option<InterruptedLogEntry>,
    ) {
        let gate_result_id = Uuid::new_v4().to_string();

        match gate_result {
            GateResult::Proceed => {
                info!(
                    gate_result_id = %gate_result_id,
                    action_type = %action_type,
                    "Gate concierge: Proceed — action authorized"
                );
                (ActionStatus::TrustApproved, None, None)
            }
            GateResult::Pending => {
                let reason =
                    "Action requires review: trust layer below auto-execute threshold".to_string();
                let entry = PendingQueueEntry {
                    id: Uuid::new_v4().to_string(),
                    gate_result_id: gate_result_id.clone(),
                    action_type: action_type.to_string(),
                    command: command.to_string(),
                    args: args.to_vec(),
                    trust_layer,
                    confidence,
                    source_event_id,
                    queued_at: chrono::Utc::now().timestamp(),
                    reason,
                };
                if let Some(db) = &self.db
                    && let Err(e) = db.persist_pending_queue_entry(&entry)
                {
                    error!(
                        gate_result_id = %gate_result_id,
                        "Failed to persist pending queue entry: {}", e
                    );
                }
                warn!(
                    gate_result_id = %gate_result_id,
                    action_type = %action_type,
                    command = %command,
                    "Gate concierge: Pending → PendingQueue"
                );
                (ActionStatus::Pending, Some(entry), None)
            }
            GateResult::Interrupted { reason } => {
                let entry = InterruptedLogEntry {
                    id: Uuid::new_v4().to_string(),
                    gate_result_id: gate_result_id.clone(),
                    action_type: action_type.to_string(),
                    command: command.to_string(),
                    args: args.to_vec(),
                    trust_layer,
                    source_event_id,
                    reason: reason.clone(),
                    logged_at: chrono::Utc::now().timestamp(),
                };
                if let Some(db) = &self.db
                    && let Err(e) = db.persist_interrupted_log_entry(&entry)
                {
                    error!(
                        gate_result_id = %gate_result_id,
                        "Failed to persist interrupted log entry: {}", e
                    );
                }
                error!(
                    gate_result_id = %gate_result_id,
                    action_type = %action_type,
                    command = %command,
                    reason = %reason,
                    "Gate concierge: Interrupted → InterruptedLog"
                );
                (ActionStatus::Denied, None, Some(entry))
            }
            GateResult::DryRun => {
                info!(
                    gate_result_id = %gate_result_id,
                    action_type = %action_type,
                    "Gate concierge: DryRun — no execution"
                );
                (ActionStatus::Denied, None, None)
            }
            GateResult::Revoked { reason } => {
                let entry = InterruptedLogEntry {
                    id: Uuid::new_v4().to_string(),
                    gate_result_id: gate_result_id.clone(),
                    action_type: action_type.to_string(),
                    command: command.to_string(),
                    args: args.to_vec(),
                    trust_layer,
                    source_event_id: source_event_id.clone(),
                    reason: reason.clone(),
                    logged_at: chrono::Utc::now().timestamp(),
                };
                if let Some(db) = &self.db
                    && let Err(e) = db.persist_revoked_entry(&entry)
                {
                    error!(
                        gate_result_id = %gate_result_id,
                        "Failed to persist revoked entry: {}", e
                    );
                }
                info!(
                    gate_result_id = %gate_result_id,
                    action_type = %action_type,
                    reason = %reason,
                    "Gate concierge: Revoked — guided exit"
                );
                (ActionStatus::Denied, None, Some(entry))
            }
        }
    }

    /// Return the pending queue entries from GuardDb.
    pub fn pending_queue(&self) -> Result<Vec<PendingQueueEntry>, GuardDbError> {
        if let Some(db) = &self.db {
            db.read_pending_queue()
        } else {
            Ok(Vec::new())
        }
    }

    /// Return the interrupted log entries from GuardDb.
    pub fn interrupted_log(&self) -> Result<Vec<InterruptedLogEntry>, GuardDbError> {
        if let Some(db) = &self.db {
            db.read_interrupted_log()
        } else {
            Ok(Vec::new())
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Approve a pending action with exact-invocation fingerprint.
    ///
    /// IronClaw's `ironclaw_approvals` pattern: the approval is tied to
    /// the exact SHA-256 fingerprint of the command+args, not a pattern.
    /// Approving `cp src dst` does NOT approve `cp src malicious`.
    ///
    /// Returns the approval lease if granted, or an error if the action
    /// doesn't match any existing approval.
    pub fn approve_with_fingerprint(
        &mut self,
        action_id: &str,
        command: &str,
        args: &[String],
        source_event_id: Option<String>,
        ttl_seconds: u64,
        granted_by: &str,
        is_persistent: bool,
    ) -> Result<ApprovalLease, String> {
        let fingerprint = InvocationFingerprint::from_command(command, args);

        let scope = ApprovalScope::new(
            None, // project context — would come from scope resolution
            None, // user context — would come from scope resolution
            source_event_id,
        );

        let lease = if is_persistent {
            ApprovalLease::persistent(fingerprint.clone(), scope.clone(), granted_by.to_string())
        } else {
            ApprovalLease::new(fingerprint.clone(), scope.clone(), ttl_seconds, granted_by.to_string())
        };

        // Store the lease so it can be matched later
        self.approval_store.insert(lease.clone());

        info!(
            action_id = %action_id,
            fingerprint = %lease.fingerprint,
            ttl = ttl_seconds,
            "Fingerprint approval granted"
        );

        Ok(lease)
    }

    /// Check if a pending action matches an existing approval lease.
    ///
    /// IronClaw's key insight: fingerprint matching is scope-isolated.
    /// A fingerprint approved in one scope does NOT match in another.
    pub fn check_fingerprint_approval(
        &self,
        command: &str,
        args: &[String],
        source_event_id: Option<String>,
    ) -> Option<ApprovalLease> {
        let fingerprint = InvocationFingerprint::from_command(command, args);
        let scope = ApprovalScope::new(None, None, source_event_id);
        self.approval_store.find_matching(&fingerprint, &scope)
    }

    /// Clean up expired approval leases.
    pub fn cleanup_expired_approvals(&self) {
        self.approval_store.cleanup_expired();
    }

    /// List all valid approval leases (for audit/debug).
    pub fn list_valid_approvals(&self) -> Vec<ApprovalLease> {
        self.approval_store.list_valid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_concierge_proceed() {
        let dir = tempdir().unwrap();
        let db = GuardDb::open(dir.path().join("guard.db")).unwrap();
        let mut concierge = GateConcierge::default().with_db(db);
        let (status, pending, interrupted) = concierge.enforce(
            GateResult::Proceed,
            "echo",
            "echo",
            &["hello".to_string()],
            3,
            0.9,
            Some("evt-1".to_string()),
        );
        assert_eq!(status, ActionStatus::TrustApproved);
        assert!(pending.is_none());
        assert!(interrupted.is_none());
    }

    #[test]
    fn test_concierge_pending_to_queue() {
        let dir = tempdir().unwrap();
        let db = GuardDb::open(dir.path().join("guard.db")).unwrap();
        let mut concierge = GateConcierge::default().with_db(db);
        let (status, pending, interrupted) = concierge.enforce(
            GateResult::Pending,
            "git_commit",
            "git",
            &["commit".to_string(), "-m".to_string(), "test".to_string()],
            2,
            0.5,
            Some("evt-2".to_string()),
        );
        assert_eq!(status, ActionStatus::Pending);
        assert!(pending.is_some());
        assert!(interrupted.is_none());
        let queue = concierge.pending_queue().unwrap();
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_concierge_interrupted_to_log() {
        let dir = tempdir().unwrap();
        let db = GuardDb::open(dir.path().join("guard.db")).unwrap();
        let mut concierge = GateConcierge::default().with_db(db);
        let (status, pending, interrupted) = concierge.enforce(
            GateResult::Interrupted {
                reason: "High-risk command detected".to_string(),
            },
            "delete",
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
            3,
            0.9,
            Some("evt-3".to_string()),
        );
        assert_eq!(status, ActionStatus::Denied);
        assert!(pending.is_none());
        assert!(interrupted.is_some());
        let log = concierge.interrupted_log().unwrap();
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn test_concierge_dry_run() {
        let dir = tempdir().unwrap();
        let db = GuardDb::open(dir.path().join("guard.db")).unwrap();
        let mut concierge = GateConcierge::default().with_db(db);
        let (status, pending, interrupted) = concierge.enforce(
            GateResult::DryRun,
            "echo",
            "echo",
            &["hello".to_string()],
            0,
            0.0,
            None,
        );
        assert_eq!(status, ActionStatus::Denied);
        assert!(pending.is_none());
        assert!(interrupted.is_none());
    }

    #[test]
    fn test_concierge_no_bypass() {
        let dir = tempdir().unwrap();
        let db = GuardDb::open(dir.path().join("guard.db")).unwrap();
        let mut concierge = GateConcierge::default().with_db(db);
        let (status, pending, interrupted) = concierge.enforce(
            GateResult::Pending,
            "run_command",
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
            2,
            0.5,
            Some("evt-4".to_string()),
        );
        assert_eq!(status, ActionStatus::Pending);
        assert!(pending.is_some());
        assert!(interrupted.is_none());
        assert!(status != ActionStatus::TrustApproved);
    }

    #[test]
    fn test_approve_with_fingerprint() {
        let mut concierge = GateConcierge::new();
        let lease = concierge
            .approve_with_fingerprint(
                "action-1",
                "rm",
                &["-rf".to_string(), "/tmp/test".to_string()],
                Some("evt-1".to_string()),
                3600,
                "user",
                false,
            )
            .unwrap();

        assert!(!lease.persistent);
        assert!(lease.is_valid());
    }

    #[test]
    fn test_check_fingerprint_approval_no_match() {
        let concierge = GateConcierge::new();
        let result = concierge.check_fingerprint_approval(
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
            Some("evt-1".to_string()),
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_fingerprint_prevents_smuggling() {
        let mut concierge = GateConcierge::new();

        // Approve the exact command
        let lease = concierge
            .approve_with_fingerprint(
                "action-1",
                "cp",
                &["src".to_string(), "dst".to_string()],
                Some("evt-1".to_string()),
                3600,
                "user",
                false,
            )
            .unwrap();

        // Try to check a different command with same fingerprint check
        let result = concierge.check_fingerprint_approval(
            "cp",
            &["src".to_string(), "malicious".to_string()],
            Some("evt-1".to_string()),
        );
        assert!(result.is_none());

        // Exact match should work
        let result = concierge.check_fingerprint_approval(
            "cp",
            &["src".to_string(), "dst".to_string()],
            Some("evt-1".to_string()),
        );
        assert!(result.is_some());
        assert_eq!(result.unwrap().fingerprint, lease.fingerprint);
    }

    #[test]
    fn test_cleanup_expired_approvals() {
        let mut concierge = GateConcierge::new();

        // Add a valid lease
        concierge
            .approve_with_fingerprint(
                "action-1",
                "echo",
                &["hello".to_string()],
                Some("evt-1".to_string()),
                3600,
                "user",
                false,
            )
            .unwrap();

        // Add an expired lease (TTL=0 means expires_at == now, is_valid checks now < expires_at)
        concierge
            .approve_with_fingerprint(
                "action-2",
                "echo",
                &["world".to_string()],
                Some("evt-2".to_string()),
                0,
                "user",
                false,
            )
            .unwrap();

        // Only the valid lease is valid (TTL=0 expires immediately)
        let valid = concierge.list_valid_approvals();
        assert_eq!(valid.len(), 1);

        concierge.cleanup_expired_approvals();

        let valid = concierge.list_valid_approvals();
        assert_eq!(valid.len(), 1);
    }
}
