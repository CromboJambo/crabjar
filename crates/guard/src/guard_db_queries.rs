use rusqlite::params;

use crate::action::{ActionRequest, ActionStatus};
use crate::guard_db::{GuardDb, GuardDbError};
use crate::guard_db_types::{
    ActionTransition, TransitionError, TrustResolutionEntry, parse_status, validate_transition,
};

// -- Action request persistence --

/// Raw row data from action_requests before status parsing.
type RawActionRow = (
    String,         // id
    Option<String>, // source_event_id
    Option<String>, // source_node_id
    String,         // action_type
    String,         // payload
    u32,            // trust_layer
    f64,            // confidence (stored as REAL in DB)
    String,         // status (raw string — parsed outside closure)
    Option<String>, // gate_result
    i64,            // requested_at
    Option<i64>,    // resolved_at
);

/// Convert a raw action row into an ActionRequest, validating the status field.
/// Returns `GuardDbError::UnknownStatus` on unrecognized status values instead of
/// silently falling back to Pending.
fn raw_to_action_request(raw: RawActionRow) -> Result<ActionRequest, GuardDbError> {
    let (
        id,
        source_event_id,
        source_node_id,
        action_type,
        payload,
        trust_layer,
        confidence,
        status_str,
        gate_result,
        requested_at,
        resolved_at,
    ) = raw;

    // Parse status — fail loudly on unknown values instead of silent Pending fallback
    let status = parse_status(&status_str).map_err(|e| match e {
        TransitionError::UnknownStatus { status, .. } => GuardDbError::UnknownStatus {
            value: status,
            action_id: id.clone(),
        },
        other => GuardDbError::SchemaError(other.to_string()),
    })?;

    Ok(ActionRequest {
        id,
        source_event_id,
        source_node_id,
        action_type,
        payload,
        trust_layer,
        confidence: crate::trust_types::TrustScore::new(confidence),
        status,
        gate_result,
        requested_at,
        resolved_at,
    })
}

impl GuardDb {
    pub fn verify_provenance(&self, source_event_id: &str) -> Result<bool, GuardDbError> {
        let conn = self.conn();
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM action_requests WHERE source_event_id = ?1)",
                params![source_event_id],
                |r| r.get(0),
            )
            .map_err(|_| GuardDbError::SchemaError("Provenance check failed".into()))?;
        Ok(exists)
    }

    pub fn read_action_requests(
        &self,
        status: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ActionRequest>, GuardDbError> {
        let conn = self.conn();

        // Read raw tuples inside the closure (only rusqlite::Error allowed there),
        // then parse/validate outside where we can return proper GuardDbErrors.
        let rows: Vec<RawActionRow> = if let Some(s) = status {
            let mut stmt = conn.prepare(
                "SELECT id, source_event_id, source_node_id, action_type, payload, trust_layer, \
                 confidence, status, gate_result, requested_at, resolved_at \
                 FROM action_requests WHERE status = ? ORDER BY requested_at DESC LIMIT ?",
            )?;
            stmt.query_map(params![s, limit as i64], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                ))
            })?
            .collect::<Result<_, _>>()?
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, source_event_id, source_node_id, action_type, payload, trust_layer, \
                 confidence, status, gate_result, requested_at, resolved_at \
                 FROM action_requests ORDER BY requested_at DESC LIMIT ?",
            )?;
            stmt.query_map(params![limit as i64], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                ))
            })?
            .collect::<Result<_, _>>()?
        };

        // Parse and validate each row outside the closure — unknown statuses become errors
        rows.into_iter().map(raw_to_action_request).collect()
    }

    /// Update action status with transition validation and audit logging.
    ///
    /// Validates that the transition is legal according to the typestate machine,
    /// records the transition in the `action_transitions` audit table, and updates
    /// the action row. Returns an error if the transition is invalid or the action
    /// doesn't exist.
    pub fn update_action_status(
        &self,
        action_id: &str,
        new_status: ActionStatus,
        authority: &str,
        reason: Option<&str>,
    ) -> Result<bool, GuardDbError> {
        let conn = self.conn();

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
        }

        Ok(rows_affected > 0)
    }

    /// Record an explicit action transition in the audit log without changing
    /// the action row status. Used for initial state recording (Pending) and
    /// external transitions that are applied separately.
    pub fn record_action_transition(
        &self,
        action_id: &str,
        from_status: &str,
        to_status: &str,
        authority: &str,
        reason: Option<&str>,
    ) -> Result<(), GuardDbError> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO action_transitions (action_id, from_status, to_status, authority, reason) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![action_id, from_status, to_status, authority, reason],
        )?;
        Ok(())
    }

    /// Read the full transition audit trail for an action.
    pub fn read_action_transitions(
        &self,
        action_id: &str,
    ) -> Result<Vec<ActionTransition>, GuardDbError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, action_id, from_status, to_status, authority, reason, created_at \
             FROM action_transitions WHERE action_id = ?1 ORDER BY created_at ASC, id ASC",
        )?;

        let transitions: Vec<ActionTransition> = stmt
            .query_map(params![action_id], |row| {
                Ok(ActionTransition {
                    id: row.get(0)?,
                    action_id: row.get(1)?,
                    from_status: row.get(2)?,
                    to_status: row.get(3)?,
                    authority: row.get(4)?,
                    reason: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        Ok(transitions)
    }

    /// Read and validate an action request by reconstructing its state from the
    /// transition log. This is the "validated rehydration" path — it ensures the
    /// current status is reachable via legal transitions from Pending.
    ///
    /// Returns an error if:
    /// - The action doesn't exist
    /// - The status string is unknown (corruption)
    /// - The transition log ends at a different state than the row claims
    /// - Any transition in the log is invalid
    pub fn read_action_request_validated(
        &self,
        action_id: &str,
    ) -> Result<(ActionRequest, Vec<ActionTransition>), GuardDbError> {
        let conn = self.conn();
        Self::read_action_request_validated_on_conn(&conn, action_id)
    }

    /// Internal version that operates on an existing connection (avoids mutex re-acquisition).
    pub(crate) fn read_action_request_validated_on_conn(
        conn: &rusqlite::Connection,
        action_id: &str,
    ) -> Result<(ActionRequest, Vec<ActionTransition>), GuardDbError> {
        // Read the action row as raw values first, then parse outside the closure
        // so we can return proper GuardDbErrors instead of trying to construct rusqlite errors.
        let row_data: (
            String,   // id
            Option<String>, // source_event_id
            Option<String>, // source_node_id
            String,   // action_type
            String,   // payload
            u32,      // trust_layer
            f64,      // confidence (REAL in DB)
            String,   // status
            String,   // gate_result
            i64,      // requested_at
            Option<i64>, // resolved_at
        ) = conn
            .query_row(
                "SELECT id, source_event_id, source_node_id, action_type, payload, trust_layer, \
                 confidence, status, gate_result, requested_at, resolved_at FROM action_requests WHERE id = ?1",
                params![action_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                    ))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => GuardDbError::SchemaError(format!(
                    "Action not found: {}",
                    action_id
                )),
                other => GuardDbError::from(other),
            })?;

        let (
            id,
            source_event_id,
            source_node_id,
            action_type,
            payload,
            trust_layer,
            confidence_val,
            status_str,
            gate_result_str,
            requested_at,
            resolved_at,
        ) = row_data;

        // Parse status — fail loudly on unknown values instead of silent Pending fallback
        let status = parse_status(&status_str).map_err(|e| {
            GuardDbError::SchemaError(format!("Invalid status for action {}: {}", action_id, e))
        })?;

        // Payload and gate_result are stored as strings — keep them as strings.
        // (ActionRequest.payload is String, ActionRequest.gate_result is Option<String>)
        let gate_result = if gate_result_str.is_empty() {
            None
        } else {
            Some(gate_result_str)
        };

        let request = ActionRequest {
            id,
            source_event_id,
            source_node_id,
            action_type,
            payload,
            trust_layer,
            confidence: crate::trust_types::TrustScore::new(confidence_val),
            status,
            gate_result,
            requested_at,
            resolved_at,
        };

        // Read the transition log directly from the connection
        let mut stmt = conn.prepare(
            "SELECT id, action_id, from_status, to_status, authority, reason, created_at \
             FROM action_transitions WHERE action_id = ?1 ORDER BY created_at ASC, id ASC",
        )?;

        let transitions: Vec<ActionTransition> = stmt
            .query_map(params![action_id], |row| {
                Ok(ActionTransition {
                    id: row.get(0)?,
                    action_id: row.get(1)?,
                    from_status: row.get(2)?,
                    to_status: row.get(3)?,
                    authority: row.get(4)?,
                    reason: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        if transitions.is_empty() {
            // No transition log — this is a legacy row (pre-schema-v2).
            // Accept it but flag it: the state can't be validated against history.
            // For now, trust the row's status field. This is the conservative backfill path.
            return Ok((request, transitions));
        }

        // Validate: replay transitions from Pending and check they end at the row's status
        let mut current = ActionStatus::Pending;
        for t in &transitions {
            validate_transition(&t.from_status, &t.to_status).map_err(|e| {
                GuardDbError::SchemaError(format!(
                    "Invalid transition in log for {}: {} ({} -> {})",
                    action_id, e, t.from_status, t.to_status
                ))
            })?;

            // Check continuity: each transition's from_status must match the previous to_status
            if t.from_status != current.to_string() {
                return Err(GuardDbError::SchemaError(format!(
                    "Transition log discontinuity for {}: expected from '{}' but got '{}'",
                    action_id, current, t.from_status
                )));
            }

            current = parse_status(&t.to_status).map_err(|e| {
                GuardDbError::SchemaError(format!("Unknown status in transition log: {}", e))
            })?;
        }

        // Final check: replayed state must match the row's status
        if current != request.status {
            return Err(GuardDbError::SchemaError(format!(
                "State mismatch for {}: transition log ends at '{}' but row says '{}'",
                action_id, current, request.status
            )));
        }

        Ok((request, transitions))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn persist_action_request_with_scope(
        &self,
        action_id: &str,
        source_event_id: Option<&str>,
        source_node_id: Option<&str>,
        action_type: &str,
        payload: &str,
        trust_layer: u32,
        confidence: f64,
        scope_actor: Option<&str>,
        scope_target: Option<&str>,
    ) -> Result<(), GuardDbError> {
        let conn = self.conn();
        eprintln!(
            "DEBUG persist: action_id={}, source_event_id={}",
            action_id,
            source_event_id.unwrap_or("NONE")
        );
        conn.execute(
            "INSERT INTO action_requests \
             (id, source_event_id, source_node_id, action_type, payload, trust_layer, \
              confidence, status, requested_at, scope_actor, scope_target) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', unixepoch(), ?8, ?9)",
            params![
                action_id,
                source_event_id,
                source_node_id,
                action_type,
                payload,
                trust_layer,
                confidence,
                scope_actor,
                scope_target,
            ],
        )?;
        eprintln!("DEBUG persist: row inserted");

        // Record the initial state in the transition audit log.
        // Every action starts at Pending — this gives us a complete history from creation.
        conn.execute(
            "INSERT INTO action_transitions (action_id, from_status, to_status, authority, reason) \
             VALUES (?1, 'none', 'pending', 'guard', 'action created')",
            params![action_id],
        )?;

        Ok(())
    }

    /// Read action requests with scope information.
    #[allow(clippy::type_complexity)]
    pub fn read_action_requests_with_scope(
        &self,
        status: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(ActionRequest, Option<String>, Option<String>)>, GuardDbError> {
        let conn = self.conn();

        // Raw row with scope fields appended
        type RawScopedRow = (RawActionRow, Option<String>, Option<String>);

        let rows: Vec<RawScopedRow> = if let Some(s) = status {
            let mut stmt = conn.prepare(
                "SELECT id, source_event_id, source_node_id, action_type, payload, trust_layer, \
                 confidence, status, gate_result, requested_at, resolved_at, scope_actor, scope_target \
                 FROM action_requests WHERE status = ? ORDER BY requested_at DESC LIMIT ?",
            )?;
            stmt.query_map(params![s, limit as i64], |row| {
                Ok((
                    (
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                    ),
                    row.get(11)?,
                    row.get(12)?,
                ))
            })?
            .collect::<Result<_, _>>()?
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, source_event_id, source_node_id, action_type, payload, trust_layer, \
                 confidence, status, gate_result, requested_at, resolved_at, scope_actor, scope_target \
                 FROM action_requests ORDER BY requested_at DESC LIMIT ?",
            )?;
            stmt.query_map(params![limit as i64], |row| {
                Ok((
                    (
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                    ),
                    row.get(11)?,
                    row.get(12)?,
                ))
            })?
            .collect::<Result<_, _>>()?
        };

        // Parse and validate each row outside the closure
        rows.into_iter()
            .map(|(raw, scope_actor, scope_target)| {
                Ok((raw_to_action_request(raw)?, scope_actor, scope_target))
            })
            .collect()
    }

    // -- Trust resolution persistence --

    /// Persist a trust resolution audit record.
    #[allow(clippy::too_many_arguments)]
    pub fn record_trust_resolution(
        &self,
        action_id: Option<&str>,
        requested_layer: u32,
        requested_confidence: f64,
        requested_source: &str,
        effective_layer: u32,
        effective_confidence: f64,
        effective_by: &str,
        scope_actor: Option<&str>,
        scope_target: Option<&str>,
        applied_policies: Vec<String>,
    ) -> Result<(), GuardDbError> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO trust_resolutions \
             (action_id, requested_layer, requested_confidence, requested_source, \
              effective_layer, effective_confidence, effective_by, \
              scope_actor, scope_target, applied_policies, resolved_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, unixepoch())",
            params![
                action_id,
                requested_layer,
                requested_confidence,
                requested_source,
                effective_layer,
                effective_confidence,
                effective_by,
                scope_actor,
                scope_target,
                serde_json::to_string(&applied_policies)
                    .map_err(|e| GuardDbError::SchemaError(e.to_string()))?,
            ],
        )?;
        Ok(())
    }

    /// List trust resolution records, optionally filtered by effective layer.
    pub fn list_trust_resolutions(
        &self,
        effective_layer: Option<u32>,
        limit: usize,
    ) -> Result<Vec<TrustResolutionEntry>, GuardDbError> {
        let conn = self.conn();
        let query = if effective_layer.is_some() {
            "SELECT id, action_id, requested_layer, requested_confidence, requested_source, \
             effective_layer, effective_confidence, effective_by, \
             scope_actor, scope_target, applied_policies, resolved_at \
             FROM trust_resolutions WHERE effective_layer = ? \
             ORDER BY resolved_at DESC LIMIT ?"
                .to_string()
        } else {
            "SELECT id, action_id, requested_layer, requested_confidence, requested_source, \
             effective_layer, effective_confidence, effective_by, \
             scope_actor, scope_target, applied_policies, resolved_at \
             FROM trust_resolutions ORDER BY resolved_at DESC LIMIT ?"
                .to_string()
        };

        let mut stmt = conn.prepare(&query)?;
        let entries: Vec<TrustResolutionEntry> = if let Some(el) = effective_layer {
            stmt.query_map(params![el, limit as i64], |row| {
                Ok(TrustResolutionEntry {
                    id: row.get(0)?,
                    action_id: row.get(1)?,
                    requested_layer: row.get(2)?,
                    requested_confidence: row.get(3)?,
                    requested_source: row.get(4)?,
                    effective_layer: row.get(5)?,
                    effective_confidence: row.get(6)?,
                    effective_by: row.get(7)?,
                    scope_actor: row.get(8)?,
                    scope_target: row.get(9)?,
                    applied_policies: row.get::<_, String>(10)?,
                    resolved_at: row.get(11)?,
                })
            })?
            .collect::<Result<_, _>>()?
        } else {
            let mut stmt2 = conn.prepare(&query)?;
            stmt2
                .query_map(params![limit as i64], |row| {
                    Ok(TrustResolutionEntry {
                        id: row.get(0)?,
                        action_id: row.get(1)?,
                        requested_layer: row.get(2)?,
                        requested_confidence: row.get(3)?,
                        requested_source: row.get(4)?,
                        effective_layer: row.get(5)?,
                        effective_confidence: row.get(6)?,
                        effective_by: row.get(7)?,
                        scope_actor: row.get(8)?,
                        scope_target: row.get(9)?,
                        applied_policies: row.get::<_, String>(10)?,
                        resolved_at: row.get(11)?,
                    })
                })?
                .collect::<Result<_, _>>()?
        };
        Ok(entries)
    }
}
