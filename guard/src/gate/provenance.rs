use crate::guard_db::GuardDbError;
use chrono;
use tracing::warn;

/// Verify provenance: source_event_id exists in GuardDb action_requests AND is recent.
/// Addresses redteam finding #7: stale approvals should be rejected.
pub fn verify_provenance(conn: &rusqlite::Connection, id: &str) -> Result<bool, GuardDbError> {
    // Check existence and freshness in a single query
    // Approvals older than 24 hours are considered stale
    let exists_and_fresh: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM action_requests \n\
             WHERE source_event_id = ?1 AND requested_at > ?2)",
            rusqlite::params![id, chrono::Utc::now().timestamp() - 86400],
            |r| r.get(0),
        )
        .map_err(|_| GuardDbError::SchemaError("Provenance check failed".into()))?;

    if !exists_and_fresh {
        // Check if it exists but is stale
        let is_stale: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM action_requests \n\
                 WHERE source_event_id = ?1 AND requested_at <= ?2)",
                rusqlite::params![id, chrono::Utc::now().timestamp() - 86400],
                |r| r.get(0),
            )
            .map_err(|_| GuardDbError::SchemaError("Stale provenance check failed".into()))?;

        if is_stale {
            warn!(event_id = id, "Provenance exists but is stale (>24h)");
        }
    }

    Ok(exists_and_fresh)
}
