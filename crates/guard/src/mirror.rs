//! Session mirror: durable state persistence behind a replaceable contract.
//!
//! This is the glass boundary between crabjar's authorization model (inside)
//! and whatever durable store we use to survive machine lifecycle changes (outside).
//!
//! ## Design principles
//!
//! - **GuardDb is the authority.** The mirror is a backup, not a co-authority.
//! - **Writes are fire-and-forget with retry.** A failed mirror write never blocks
//!   a local decision. Reconciliation catches up later.
//! - **Recovery never authorizes.** A pending action stays pending across any
//!   number of crashes, restarts, and reconciliations.
//! - **Idempotent convergence.** Running reconciliation N times converges to the
//!   same state. Every write is keyed by action_id (upsert semantics).
//!
//! ## What gets mirrored
//!
//! - `ActionRequest` rows with status = Pending (and their status changes)
//! - The corresponding `HandoffEnvelope` (full context for rehydration)
//!
//! What does NOT get mirrored: memory graph, trust layers, anneal config,
//! PID trust — these are machine-local and not needed for session continuity.
//!
//! ## Implementation
//!
//! The trait is defined here (in guard, the stable substrate). Concrete
//! implementations live outside:
//! - `HttpSessionMirror` — talks to a celld worker via HTTP + JSON
//! - Future: MinIO-backed store, PostgreSQL, whatever replaces celld
//!
//! See `specs/experiments/guarddb-celld-consistency.md` for the full design.

use crate::action::{ActionRequest, ActionStatus};
use crate::handoff::HandoffEnvelope;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for session mirror operations.
#[derive(Debug, Error)]
pub enum MirrorError {
    /// The remote store is unreachable (network error, service down).
    #[error("mirror unreachable: {0}")]
    Unreachable(String),

    /// The remote store returned an unexpected response.
    #[error("unexpected mirror response: {0}")]
    BadResponse(String),

    /// The action was not found in the mirror.
    #[error("action not found in mirror: {0}")]
    NotFound(String),

    /// Serialization/deserialization failure.
    #[error("serialization error: {0}")]
    Serde(String),
}

/// A pending action as stored in the durable mirror.
///
/// This is a flat, serializable representation that doesn't depend on
/// GuardDb internals. The `envelope` field carries the full handoff context
/// needed to rehydrate the action into a fresh GuardDb after machine loss.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirroredAction {
    /// Unique action ID (matches GuardDb's `action_requests.id`).
    pub action_id: String,
    /// Current status. Only "pending" actions are actively mirrored;
    /// decided actions are updated once and left alone.
    pub status: String,
    /// The action type (e.g., "execute", "file_write").
    pub action_type: String,
    /// The payload/command being authorized.
    pub payload: String,
    /// When the action was first requested.
    pub requested_at: i64,
    /// When the action was decided (null if still pending).
    pub decided_at: Option<i64>,
    /// Who/what made the decision (null if still pending).
    pub decided_by: Option<String>,
    /// Full handoff envelope for rehydration after machine loss.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envelope: Option<HandoffEnvelope>,
}

impl MirroredAction {
    /// Create a mirrored action from an ActionRequest and optional envelope.
    pub fn from_request(request: &ActionRequest, envelope: Option<&HandoffEnvelope>) -> Self {
        Self {
            action_id: request.id.clone(),
            status: request.status.to_string(),
            action_type: request.action_type.clone(),
            payload: request.payload.clone(),
            requested_at: request.requested_at,
            decided_at: request.resolved_at,
            decided_by: None, // Filled in by the decision path
            envelope: envelope.cloned(),
        }
    }

    /// Check if this action is still pending.
    pub fn is_pending(&self) -> bool {
        self.status == "pending"
    }
}

/// The session mirror contract.
///
/// Implementations must be idempotent: calling any method multiple times
/// with the same arguments produces the same result. This is what makes
/// reconciliation safe to run repeatedly without side effects.
pub trait SessionMirror {
    /// Mirror a pending action to the durable store.
    ///
    /// Called after GuardDb records a new pending action. If this fails,
    /// the caller should log a warning and continue — GuardDb remains the
    /// source of truth. The next reconciliation will retry the mirror write.
    fn mirror_pending(&self, action: &MirroredAction) -> Result<(), MirrorError>;

    /// Update the status of a previously mirrored action.
    ///
    /// Called when a decision is made (approve/deny/execute/interrupt).
    /// Same fire-and-forget semantics as `mirror_pending`.
    fn update_status(
        &self,
        action_id: &str,
        status: &ActionStatus,
        decided_by: Option<&str>,
    ) -> Result<(), MirrorError>;

    /// List all pending actions in the durable store.
    ///
    /// Used during reconciliation to detect drift and during recovery to
    /// rehydrate a lost GuardDb. Returns an empty list if the store is
    /// unreachable (caller should check `is_available()` first).
    fn list_pending(&self) -> Result<Vec<MirroredAction>, MirrorError>;

    /// Get a specific action's state from the mirror.
    ///
    /// Returns `Ok(None)` if the action doesn't exist in the mirror.
    fn get_action(&self, action_id: &str) -> Result<Option<MirroredAction>, MirrorError>;

    /// Check if the mirror is reachable.
    ///
    /// Used to decide whether to attempt reconciliation or just operate
    /// on GuardDb alone. Implementations should make this cheap (e.g.,
    /// a HEAD request or connection check), not a full list operation.
    fn is_available(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust_types::TrustScore;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// In-memory mock for testing the reconciliation logic without HTTP.
    #[derive(Debug)]
    pub struct MockMirror {
        actions: Arc<Mutex<HashMap<String, MirroredAction>>>,
        available: Arc<Mutex<bool>>,
    }

    impl Default for MockMirror {
        fn default() -> Self {
            Self {
                actions: Arc::new(Mutex::new(HashMap::new())),
                available: Arc::new(Mutex::new(true)), // Available by default
            }
        }
    }

    impl MockMirror {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn set_available(&self, available: bool) {
            *self.available.lock().unwrap() = available;
        }
    }

    impl SessionMirror for MockMirror {
        fn mirror_pending(&self, action: &MirroredAction) -> Result<(), MirrorError> {
            if !*self.available.lock().unwrap() {
                return Err(MirrorError::Unreachable("mock offline".into()));
            }
            self.actions
                .lock()
                .unwrap()
                .insert(action.action_id.clone(), action.clone());
            Ok(())
        }

        fn update_status(
            &self,
            action_id: &str,
            status: &ActionStatus,
            decided_by: Option<&str>,
        ) -> Result<(), MirrorError> {
            if !*self.available.lock().unwrap() {
                return Err(MirrorError::Unreachable("mock offline".into()));
            }
            let mut actions = self.actions.lock().unwrap();
            if let Some(action) = actions.get_mut(action_id) {
                action.status = status.to_string();
                action.decided_at = Some(chrono::Utc::now().timestamp());
                action.decided_by = decided_by.map(|s| s.to_string());
            }
            Ok(())
        }

        fn list_pending(&self) -> Result<Vec<MirroredAction>, MirrorError> {
            if !*self.available.lock().unwrap() {
                return Err(MirrorError::Unreachable("mock offline".into()));
            }
            Ok(self
                .actions
                .lock()
                .unwrap()
                .values()
                .filter(|a| a.is_pending())
                .cloned()
                .collect())
        }

        fn get_action(&self, action_id: &str) -> Result<Option<MirroredAction>, MirrorError> {
            if !*self.available.lock().unwrap() {
                return Err(MirrorError::Unreachable("mock offline".into()));
            }
            Ok(self.actions.lock().unwrap().get(action_id).cloned())
        }

        fn is_available(&self) -> bool {
            *self.available.lock().unwrap()
        }
    }

    #[test]
    fn mirrored_action_from_request() {
        let request =
            ActionRequest::new("test-1", "execute", "echo hello", 2, TrustScore::new(0.8));
        let mirrored = MirroredAction::from_request(&request, None);
        assert_eq!(mirrored.action_id, "test-1");
        assert_eq!(mirrored.status, "pending");
        assert_eq!(mirrored.action_type, "execute");
        assert_eq!(mirrored.payload, "echo hello");
        assert!(mirrored.decided_at.is_none());
        assert!(mirrored.decided_by.is_none());
        assert!(mirrored.is_pending());
    }

    #[test]
    fn mirrored_action_from_decided_request() {
        let request = ActionRequest::new("test-2", "execute", "echo", 2, TrustScore::new(0.8))
            .mark_executed();
        let mirrored = MirroredAction::from_request(&request, None);
        assert_eq!(mirrored.status, "executed");
        assert!(mirrored.decided_at.is_some());
        assert!(!mirrored.is_pending());
    }

    #[test]
    fn mock_mirror_roundtrip() {
        let mirror = MockMirror::new();
        let request = ActionRequest::new("mock-1", "execute", "echo hi", 2, TrustScore::new(0.9));
        let action = MirroredAction::from_request(&request, None);

        mirror.mirror_pending(&action).unwrap();

        let fetched = mirror.get_action("mock-1").unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().status, "pending");

        mirror
            .update_status("mock-1", &ActionStatus::Executed, Some("test"))
            .unwrap();

        let updated = mirror.get_action("mock-1").unwrap().unwrap();
        assert_eq!(updated.status, "executed");
        assert_eq!(updated.decided_by, Some("test".to_string()));
    }

    #[test]
    fn mock_mirror_unreachable() {
        let mirror = MockMirror::new();
        mirror.set_available(false);

        let request = ActionRequest::new("mock-2", "execute", "echo", 2, TrustScore::new(0.5));
        let action = MirroredAction::from_request(&request, None);

        assert!(mirror.mirror_pending(&action).is_err());
        assert!(!mirror.is_available());
    }
}
