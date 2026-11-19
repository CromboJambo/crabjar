//! Extended query methods for GuardDb — scope-aware reads and trust resolution.

use rusqlite::params;

use crate::action::{ActionRequest, ActionStatus};
use crate::guard_db::{GuardDb, GuardDbError};
use crate::guard_db_queries::{RawActionRow, raw_to_action_request};
use crate::guard_db_types::{TrustResolutionEntry};

impl GuardDb {
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
