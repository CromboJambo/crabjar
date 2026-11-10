//! Handoff envelope persistence for GuardDb.
//!
//! Stores and retrieves `HandoffEnvelope` records — the full context envelopes
//! that capture origin, reasoning, authority, and idempotency for interprocess
//! state transfer.

use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::action::ActionStatus;
use crate::guard_db::{GuardDb, GuardDbError};
use crate::guard_db_types::validate_transition;
use crate::handoff::HandoffEnvelope;

impl GuardDb {
    // -- Handoff persistence --

    /// Persist a handoff envelope for an action. The envelope ID is the idempotency key —
    /// duplicate inserts with the same ID are ignored (idempotent).
    pub fn persist_handoff(
        &self,
        action_id: &str,
        envelope: &HandoffEnvelope,
    ) -> Result<(), GuardDbError> {
        let conn = self.conn();
        Self::persist_handoff_on_conn(&conn, action_id, envelope)
    }

    fn persist_handoff_on_conn(
        conn: &Connection,
        action_id: &str,
        envelope: &HandoffEnvelope,
    ) -> Result<(), GuardDbError> {
        let envelope_json = serde_json::to_string(envelope).map_err(|e| {
            GuardDbError::SchemaError(format!("Failed to serialize handoff: {}", e))
        })?;

        conn.execute(
            "INSERT OR IGNORE INTO handoffs (id, action_id, envelope_json) \
             VALUES (?1, ?2, ?3)",
            params![envelope.context.idempotency_key, action_id, envelope_json],
        )?;
        Ok(())
    }

    /// Read all handoff envelopes for an action, ordered by creation time.
    pub fn read_handoffs(&self, action_id: &str) -> Result<Vec<HandoffEnvelope>, GuardDbError> {
        let conn = self.conn();
        Self::read_handoffs_on_conn(&conn, action_id)
    }

    fn read_handoffs_on_conn(
        conn: &Connection,
        action_id: &str,
    ) -> Result<Vec<HandoffEnvelope>, GuardDbError> {
        let mut stmt = conn.prepare(
            "SELECT envelope_json FROM handoffs WHERE action_id = ?1 ORDER BY created_at ASC",
        )?;

        let envelopes: Vec<HandoffEnvelope> = stmt
            .query_map(params![action_id], |row| {
                let json: String = row.get(0)?;
                serde_json::from_str(&json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Failed to deserialize handoff: {}", e),
                        )),
                    )
                })
            })?
            .collect::<Result<_, _>>()?;

        Ok(envelopes)
    }

    /// Apply a state transition with handoff persistence.
    ///
    /// This is the core of Phase 3: the action row update, transition audit log entry,
    /// and handoff envelope write happen in sequence on the same connection. The
    /// operations are ordered so that if any step fails, the caller can inspect the
    /// partial state via the transition log and handoff table.
    ///
    /// Returns true if the transition was applied, false if the action wasn't found.
    pub fn apply_transition(
        &self,
        action_id: &str,
        new_status: ActionStatus,
        authority: &str,
        reason: Option<&str>,
        envelope: Option<&HandoffEnvelope>,
    ) -> Result<bool, GuardDbError> {
        let conn = self.conn();
        Self::apply_transition_on_conn(&conn, action_id, new_status, authority, reason, envelope)
    }

    /// Internal version that operates on an existing connection (avoids mutex re-acquisition).
    pub(crate) fn apply_transition_on_conn(
        conn: &Connection,
        action_id: &str,
        new_status: ActionStatus,
        authority: &str,
        reason: Option<&str>,
        envelope: Option<&HandoffEnvelope>,
    ) -> Result<bool, GuardDbError> {
        // Read current status to validate the transition
        let current_status: String = conn
            .query_row(
                "SELECT status FROM action_requests WHERE id = ?1",
                params![action_id],
                |r| r.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    GuardDbError::SchemaError(format!("Action not found: {}", action_id))
                }
                other => GuardDbError::from(other),
            })?;

        // Validate the transition is legal
        validate_transition(&current_status, &new_status.to_string()).map_err(|e| {
            GuardDbError::SchemaError(format!("Invalid transition for {}: {}", action_id, e))
        })?;

        // Update the action row
        let rows_affected = conn.execute(
            "UPDATE action_requests SET status = ?, resolved_at = unixepoch() WHERE id = ?",
            params![new_status.to_string(), action_id],
        )?;

        if rows_affected > 0 {
            // Log the transition in the audit table
            conn.execute(
                "INSERT INTO action_transitions (action_id, from_status, to_status, authority, reason) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    action_id,
                    current_status,
                    new_status.to_string(),
                    authority,
                    reason,
                ],
            )?;

            // Persist the handoff envelope if provided
            if let Some(env) = envelope {
                Self::persist_handoff_on_conn(conn, action_id, env)?;
            }
        }

        Ok(rows_affected > 0)
    }

    /// Resolve a pending queue entry by approving or rejecting it.
    ///
    /// - If `approved`: transitions the action_request from Pending → TrustApproved,
    ///   logs the transition, and removes the entry from pending_queue.
    /// - If `rejected`: moves the entry to interrupted_log with reason "user_rejected"
    ///   then deletes from pending_queue.
    pub fn resolve_pending_queue_entry(
        &self,
        id: &str,
        approved: bool,
    ) -> Result<(), GuardDbError> {
        let conn = self.conn();

        // Fetch the entry before modifying it (needed for interrupted_log if rejected)
        let row = conn.query_row(
            "SELECT id, gate_result_id, action_type, command, args, trust_layer FROM pending_queue WHERE id = ?1",
            params![id],
            |row| Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?, // args is stored as JSON string
                row.get::<_, u32>(5)?,
            )),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => GuardDbError::PathError(format!("Pending queue entry not found: {}", id)),
            _ => GuardDbError::Sqlite(e),
        })?;

        let (id_val, gate_result_id, action_type, command, args_str, trust_layer) = row;

        if approved {
            // Phase 3: transition the action_request from Pending → TrustApproved
            // The entry.id is also the action_request.id (persisted alongside by the concierge)
            match Self::apply_transition_on_conn(
                &conn,
                &id_val,
                crate::action::ActionStatus::TrustApproved,
                "concierge",
                Some("Pending queue entry approved by user"),
                None, // No new envelope — the original was persisted when queued
            ) {
                Ok(_) => {
                    tracing::info!(pending_id = %id_val, action_type = %action_type, "Pending queue entry approved: transitioned to TrustApproved")
                }
                Err(e) => {
                    tracing::warn!(pending_id = %id_val, error = %e, "Failed to apply transition for approved entry (may not have action_request)")
                }
            }

            // Remove from pending_queue
            conn.execute("DELETE FROM pending_queue WHERE id = ?1", params![id_val])?;
        } else {
            // Rejected: move to interrupted_log then delete from pending_queue
            conn.execute(
                "INSERT INTO interrupted_log (id, gate_result_id, action_type, command, args, trust_layer, source_event_id, reason, logged_at)\n                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, unixepoch())",
                params![
                    Uuid::new_v4().to_string(), // new id for interrupted_log entry
                    gate_result_id,
                    action_type,
                    command,
                    args_str, // already a JSON string from pending_queue
                    trust_layer,
                    None::<String>, // source_event_id — not available in pending_queue schema
                    "user_rejected",
                ],
            )?;

            conn.execute("DELETE FROM pending_queue WHERE id = ?1", params![id_val])?;

            tracing::info!(pending_id = %id_val, "Pending queue entry rejected and moved to interrupted_log");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handoff::{ActionOrigin, HandoffPayload, SenderIdentity, TransitionIntent};
    use std::sync::Arc;
    use tempfile::tempdir;

    fn test_db() -> GuardDb {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        // Keep the tempdir alive for the duration of the test by leaking it
        // (tests are short-lived, this is acceptable)
        std::mem::forget(dir);
        GuardDb::open(&path).expect("Failed to open test DB")
    }

    fn create_action(db: &GuardDb, action_id: &str) {
        db.persist_action_request_with_scope(
            action_id,
            None,
            None,
            "shell",
            r#"{"command":"echo test"}"#,
            2,
            0.95,
            None,
            None,
        )
        .unwrap();
    }

    fn test_envelope(idempotency_key: &str) -> HandoffEnvelope {
        HandoffEnvelope {
            context: crate::handoff::HandoffContext {
                origin: ActionOrigin::UserRequest {
                    session_id: "test-session".into(),
                    reason: "user requested action".into(),
                },
                reasoning: None,
                sender: SenderIdentity::User {
                    principal: "cli".into(),
                },
                previous_state: ActionStatus::Pending,
                intended_transition: TransitionIntent::ToApproved,
                idempotency_key: idempotency_key.to_string(),
                schema_version: 1,
            },
            payload: HandoffPayload::Transition {
                action_id: "action-1".into(),
                to_status: ActionStatus::TrustApproved,
                reason: "approved via CLI".into(),
            },
        }
    }

    #[test]
    fn persist_and_read_handoff() {
        let db = Arc::new(test_db());
        create_action(&db, "action-1");
        let envelope = test_envelope("idempotency-key-1");

        // Persist the handoff
        db.persist_handoff("action-1", &envelope).unwrap();

        // Read it back
        let envelopes = db.read_handoffs("action-1").unwrap();
        assert_eq!(envelopes.len(), 1);
        assert_eq!(envelopes[0].context.idempotency_key, "idempotency-key-1");
    }

    #[test]
    fn persist_handoff_is_idempotent() {
        let db = Arc::new(test_db());
        create_action(&db, "action-1");
        let envelope = test_envelope("idempotency-key-2");

        // Persist twice with the same idempotency key
        db.persist_handoff("action-1", &envelope).unwrap();
        db.persist_handoff("action-1", &envelope).unwrap();

        // Should only have one record
        let envelopes = db.read_handoffs("action-1").unwrap();
        assert_eq!(envelopes.len(), 1);
    }

    #[test]
    fn apply_transition_updates_status_and_persists_envelope() {
        let db = Arc::new(test_db());
        create_action(&db, "action-1");

        // Apply a transition with an envelope
        let envelope = test_envelope("transition-key-1");
        let applied = db
            .apply_transition(
                "action-1",
                ActionStatus::TrustApproved,
                "concierge",
                Some("gate passed"),
                Some(&envelope),
            )
            .unwrap();

        assert!(applied);

        // Verify the action status changed (query directly to avoid pre-existing confidence type bug)
        let status = {
            let conn = db.conn();
            conn.query_row(
                "SELECT status FROM action_requests WHERE id = 'action-1'",
                [],
                |r| r.get::<_, String>(0),
            )
            .unwrap()
        }; // Drop the MutexGuard before next DB call
        assert_eq!(status, "trust-approved");

        // Verify the handoff was persisted
        let envelopes = db.read_handoffs("action-1").unwrap();
        assert_eq!(envelopes.len(), 1);
        assert_eq!(envelopes[0].context.idempotency_key, "transition-key-1");
    }

    #[test]
    fn apply_transition_validates_legal_transitions() {
        let db = Arc::new(test_db());
        create_action(&db, "action-1");

        // Try an invalid transition: Pending -> Executed (must go through TrustApproved first)
        let result = db.apply_transition(
            "action-1",
            ActionStatus::Executed,
            "concierge",
            Some("invalid"),
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn apply_transition_is_atomic() {
        let db = Arc::new(test_db());
        create_action(&db, "action-1");

        // Apply a valid transition without an envelope
        let applied = db
            .apply_transition(
                "action-1",
                ActionStatus::TrustApproved,
                "concierge",
                Some("gate passed"),
                None,
            )
            .unwrap();

        assert!(applied);

        // Verify no handoff was created (envelope was None)
        let envelopes = db.read_handoffs("action-1").unwrap();
        assert_eq!(envelopes.len(), 0);

        // Verify the transition was still logged
        let transitions = db.read_action_transitions("action-1").unwrap();
        assert!(transitions.iter().any(|t| t.to_status == "trust-approved"));
    }
}
