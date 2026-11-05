use crate::gate_result::GateResult;
use crate::guard_db::GuardDbError;
use crate::risk_config::RiskConfig;
use chrono;
use tracing::warn;

/// Check PID trust layer (Option B).
/// Returns Revoked if trust has decayed below the action's trust layer.
pub fn check_pid_trust(
    conn: &rusqlite::Connection,
    pid: i32,
    command: &str,
    risk_config: &RiskConfig,
) -> Result<Option<GateResult>, GuardDbError> {
    let row = conn.query_row(
        "SELECT trust_layer, use_count, last_use, auto_grant, decay_interval, decay_rate\n\
         FROM pid_trust WHERE pid = ?1",
        rusqlite::params![pid],
        |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, bool>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, f64>(5)?,
            ))
        },
    );

    let (current_layer, _use_count, last_use, auto_grant, decay_interval, decay_rate) = match row {
        Ok(r) => r,
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            return Ok(Some(GateResult::Pending));
        }
        Err(e) => return Err(GuardDbError::Sqlite(e)),
    };

    // Compute decayed trust layer
    let elapsed = chrono::Utc::now().timestamp() - last_use;
    let effective_layer = if elapsed > decay_interval {
        let decayed = (decay_rate * elapsed as f64).min(1.0);
        let new_confidence = (current_layer as f64 / 3.0) * (1.0 - decayed);
        (new_confidence * 3.0) as u32
    } else {
        current_layer
    };

    // If trust has dropped below the action's layer, revoke
    if effective_layer < (risk_config.confidence_floor * 3.0) as u32 {
        let reason = format!(
            "PID {} trust decayed from layer {} to {} (auto_grant={})",
            pid, current_layer, effective_layer, auto_grant
        );
        warn!(pid, command, %reason, "PID trust revoked");
        return Ok(Some(GateResult::Revoked { reason }));
    }

    Ok(None)
}
