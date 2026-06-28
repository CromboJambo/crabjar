use rusqlite::params;

use crate::action::ActionRequest;
use crate::action::ActionStatus;
use crate::guard_db::{GuardDb, GuardDbError};
use crate::guard_db_types::TrustResolutionEntry;

// -- Action request persistence --

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

        let query = if status.is_some() {
            "SELECT id, source_event_id, source_node_id, action_type, payload, trust_layer, confidence, status, gate_result, requested_at, resolved_at FROM action_requests WHERE status = ? ORDER BY requested_at DESC LIMIT ?"
                .to_string()
        } else {
            "SELECT id, source_event_id, source_node_id, action_type, payload, trust_layer, confidence, status, gate_result, requested_at, resolved_at FROM action_requests ORDER BY requested_at DESC LIMIT ?"
                .to_string()
        };

        let mut stmt = conn.prepare(&query)?;

        let entries: Vec<ActionRequest> = if let Some(s) = status {
            stmt.query_map(params![s, limit as i64], |row| {
                Ok(ActionRequest {
                    id: row.get(0)?,
                    source_event_id: row.get(1)?,
                    source_node_id: row.get(2)?,
                    action_type: row.get(3)?,
                    payload: row.get(4)?,
                    trust_layer: row.get(5)?,
                    confidence: crate::trust_types::TrustScore::new(
                        row.get::<_, String>(6)?
                            .parse::<f64>()
                            .map_err(|_e| GuardDbError::SchemaError(_e.to_string()))
                            .map_err(|_e| rusqlite::Error::QueryReturnedNoRows)?,
                    ),
                    status: match row.get::<_, String>(7)?.as_str() {
                        "pending" => ActionStatus::Pending,
                        "trust-approved" => ActionStatus::TrustApproved,
                        "denied" => ActionStatus::Denied,
                        "executed" => ActionStatus::Executed,
                        "interrupted" => ActionStatus::Interrupted,
                        _ => ActionStatus::Pending,
                    },
                    gate_result: row.get(8)?,
                    requested_at: row.get(9)?,
                    resolved_at: row.get(10)?,
                })
            })?
            .collect::<Result<_, _>>()?
        } else {
            let mut stmt2 = conn.prepare(&query)?;
            stmt2
                .query_map(params![limit as i64], |row| {
                    Ok(ActionRequest {
                        id: row.get(0)?,
                        source_event_id: row.get(1)?,
                        source_node_id: row.get(2)?,
                        action_type: row.get(3)?,
                        payload: row.get(4)?,
                        trust_layer: row.get(5)?,
                        confidence: crate::trust_types::TrustScore::new(
                            row.get::<_, String>(6)?
                                .parse::<f64>()
                                .map_err(|_e| GuardDbError::SchemaError(_e.to_string()))
                                .map_err(|_e| rusqlite::Error::QueryReturnedNoRows)?,
                        ),
                        status: match row.get::<_, String>(7)?.as_str() {
                            "pending" => ActionStatus::Pending,
                            "trust-approved" => ActionStatus::TrustApproved,
                            "denied" => ActionStatus::Denied,
                            "executed" => ActionStatus::Executed,
                            "interrupted" => ActionStatus::Interrupted,
                            _ => ActionStatus::Pending,
                        },
                        gate_result: row.get(8)?,
                        requested_at: row.get(9)?,
                        resolved_at: row.get(10)?,
                    })
                })?
                .collect::<Result<_, _>>()?
        };
        Ok(entries)
    }

    pub fn update_action_status(
        &self,
        action_id: &str,
        new_status: ActionStatus,
    ) -> Result<(), GuardDbError> {
        let conn = self.conn();
        conn.execute(
            "UPDATE action_requests SET status = ?, resolved_at = unixepoch() WHERE id = ?",
            params![format!("{}", new_status), action_id],
        )?;
        Ok(())
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
                confidence.to_string(),
                scope_actor,
                scope_target,
            ],
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

        let query = if status.is_some() {
            "SELECT id, source_event_id, source_node_id, action_type, payload, trust_layer, \
             confidence, status, gate_result, requested_at, resolved_at, scope_actor, scope_target \
             FROM action_requests WHERE status = ? ORDER BY requested_at DESC LIMIT ?"
                .to_string()
        } else {
            "SELECT id, source_event_id, source_node_id, action_type, payload, trust_layer, \
             confidence, status, gate_result, requested_at, resolved_at, scope_actor, scope_target \
             FROM action_requests ORDER BY requested_at DESC LIMIT ?"
                .to_string()
        };

        let mut stmt = conn.prepare(&query)?;

        let entries: Vec<(ActionRequest, Option<String>, Option<String>)> = if let Some(s) = status
        {
            stmt.query_map(params![s, limit as i64], |row| {
                Ok((
                    ActionRequest {
                        id: row.get(0)?,
                        source_event_id: row.get(1)?,
                        source_node_id: row.get(2)?,
                        action_type: row.get(3)?,
                        payload: row.get(4)?,
                        trust_layer: row.get(5)?,
                        confidence: crate::trust_types::TrustScore::new(
                            row.get::<_, String>(6)?
                                .parse::<f64>()
                                .map_err(|_e| GuardDbError::SchemaError(_e.to_string()))
                                .map_err(|_e| rusqlite::Error::QueryReturnedNoRows)?,
                        ),
                        status: match row.get::<_, String>(7)?.as_str() {
                            "pending" => ActionStatus::Pending,
                            "trust-approved" => ActionStatus::TrustApproved,
                            "denied" => ActionStatus::Denied,
                            "executed" => ActionStatus::Executed,
                            "interrupted" => ActionStatus::Interrupted,
                            _ => ActionStatus::Pending,
                        },
                        gate_result: row.get(8)?,
                        requested_at: row.get(9)?,
                        resolved_at: row.get(10)?,
                    },
                    row.get(11)?,
                    row.get(12)?,
                ))
            })?
            .collect::<Result<_, _>>()?
        } else {
            let mut stmt2 = conn.prepare(&query)?;
            stmt2
                .query_map(params![limit as i64], |row| {
                    Ok((
                        ActionRequest {
                            id: row.get(0)?,
                            source_event_id: row.get(1)?,
                            source_node_id: row.get(2)?,
                            action_type: row.get(3)?,
                            payload: row.get(4)?,
                            trust_layer: row.get(5)?,
                            confidence: crate::trust_types::TrustScore::new(
                                row.get::<_, String>(6)?
                                    .parse::<f64>()
                                    .map_err(|_e| GuardDbError::SchemaError(_e.to_string()))
                                    .map_err(|_e| rusqlite::Error::QueryReturnedNoRows)?,
                            ),
                            status: match row.get::<_, String>(7)?.as_str() {
                                "pending" => ActionStatus::Pending,
                                "trust-approved" => ActionStatus::TrustApproved,
                                "denied" => ActionStatus::Denied,
                                "executed" => ActionStatus::Executed,
                                "interrupted" => ActionStatus::Interrupted,
                                _ => ActionStatus::Pending,
                            },
                            gate_result: row.get(8)?,
                            requested_at: row.get(9)?,
                            resolved_at: row.get(10)?,
                        },
                        row.get(11)?,
                        row.get(12)?,
                    ))
                })?
                .collect::<Result<_, _>>()?
        };
        Ok(entries)
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
