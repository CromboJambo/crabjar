use rusqlite::params;

use crate::concierge_types::{InterruptedLogEntry, PendingQueueEntry};
use crate::guard_db::{GuardDb, GuardDbError};
use crate::trust_types::AnnealConfig;

impl GuardDb {
    // -- Anneal config helpers --

    pub fn load_anneal_config(&self) -> Result<AnnealConfig, GuardDbError> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT key, value FROM anneal_config")?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut config = AnnealConfig::default();
        for row in rows {
            let (key, value) = row?;
            match key.as_str() {
                "decay_rate" => config.decay_rate = value.parse().unwrap_or(config.decay_rate),
                "reinforce_threshold" => {
                    config.reinforce_threshold = value.parse().unwrap_or(config.reinforce_threshold)
                }
                "anneal_interval_seconds" => {
                    config.anneal_interval_seconds =
                        value.parse().unwrap_or(config.anneal_interval_seconds)
                }
                "max_anneal_passes" => {
                    config.max_anneal_passes = value.parse().unwrap_or(config.max_anneal_passes)
                }
                "confidence_floor" => {
                    config.confidence_floor = value.parse().unwrap_or(config.confidence_floor)
                }
                "auto_anneal_enabled" => {
                    config.auto_anneal_enabled = value.parse().unwrap_or(config.auto_anneal_enabled)
                }
                _ => {}
            }
        }

        Ok(config)
    }

    pub fn save_anneal_config(&self, config: &AnnealConfig) -> Result<(), GuardDbError> {
        let conn = self.conn();
        conn.execute(
            "INSERT OR REPLACE INTO anneal_config (key, value) VALUES ('decay_rate', ?1)",
            params![config.decay_rate.to_string()],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO anneal_config (key, value) VALUES ('reinforce_threshold', ?1)",
            params![config.reinforce_threshold.to_string()],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO anneal_config (key, value) VALUES ('anneal_interval_seconds', ?1)",
            params![config.anneal_interval_seconds.to_string()],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO anneal_config (key, value) VALUES ('max_anneal_passes', ?1)",
            params![config.max_anneal_passes.to_string()],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO anneal_config (key, value) VALUES ('confidence_floor', ?1)",
            params![config.confidence_floor.to_string()],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO anneal_config (key, value) VALUES ('auto_anneal_enabled', ?1)",
            params![config.auto_anneal_enabled.to_string()],
        )?;
        Ok(())
    }

    pub fn persist_pending_queue_entry(
        &self,
        entry: &PendingQueueEntry,
    ) -> Result<(), GuardDbError> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO pending_queue (id, gate_result_id, action_type, command, args, trust_layer, confidence, source_event_id, queued_at, reason)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                entry.id,
                entry.gate_result_id,
                entry.action_type,
                entry.command,
                serde_json::to_string(&entry.args)
                    .map_err(|e| GuardDbError::SchemaError(e.to_string()))?,
                entry.trust_layer,
                entry.confidence.to_string(),
                entry.source_event_id,
                entry.queued_at,
                entry.reason,
            ],
        )?;
        Ok(())
    }

    pub fn persist_interrupted_log_entry(
        &self,
        entry: &InterruptedLogEntry,
    ) -> Result<(), GuardDbError> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO interrupted_log (id, gate_result_id, action_type, command, args, trust_layer, source_event_id, reason, logged_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                entry.id,
                entry.gate_result_id,
                entry.action_type,
                entry.command,
                serde_json::to_string(&entry.args)
                    .map_err(|e| GuardDbError::SchemaError(e.to_string()))?,
                entry.trust_layer,
                entry.source_event_id,
                entry.reason,
                entry.logged_at,
            ],
        )?;
        Ok(())
    }

    pub fn read_pending_queue(&self) -> Result<Vec<PendingQueueEntry>, GuardDbError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, gate_result_id, action_type, command, args, trust_layer, confidence, source_event_id, queued_at, reason FROM pending_queue ORDER BY queued_at DESC",
        )?;
        let entries: Vec<PendingQueueEntry> = stmt
            .query_map([], |row| {
                let args_str: String = row.get(4)?;
                let args: Vec<String> = serde_json::from_str(&args_str)
                    .map_err(|e| GuardDbError::SchemaError(e.to_string()))
                    .map_err(|_e| rusqlite::Error::QueryReturnedNoRows)?;
                Ok(PendingQueueEntry {
                    id: row.get(0)?,
                    gate_result_id: row.get(1)?,
                    action_type: row.get(2)?,
                    command: row.get(3)?,
                    args,
                    trust_layer: row.get(5)?,
                    confidence: row.get::<_, f64>(6)?,
                    source_event_id: row.get(7)?,
                    queued_at: row.get(8)?,
                    reason: row.get(9)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(entries)
    }

    pub fn read_interrupted_log(&self) -> Result<Vec<InterruptedLogEntry>, GuardDbError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, gate_result_id, action_type, command, args, trust_layer, source_event_id, reason, logged_at FROM interrupted_log ORDER BY logged_at DESC",
        )?;
        let entries: Vec<InterruptedLogEntry> = stmt
            .query_map([], |row| {
                let args_str: String = row.get(4)?;
                let args: Vec<String> = serde_json::from_str(&args_str)
                    .map_err(|e| GuardDbError::SchemaError(e.to_string()))
                    .map_err(|_e| rusqlite::Error::QueryReturnedNoRows)?;
                Ok(InterruptedLogEntry {
                    id: row.get(0)?,
                    gate_result_id: row.get(1)?,
                    action_type: row.get(2)?,
                    command: row.get(3)?,
                    args,
                    trust_layer: row.get(5)?,
                    source_event_id: row.get(6)?,
                    reason: row.get(7)?,
                    logged_at: row.get(8)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(entries)
    }

    /// Persist a revoked entry to the revoked_log.
    pub fn persist_revoked_entry(&self, entry: &InterruptedLogEntry) -> Result<(), GuardDbError> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO revoked_log (id, gate_result_id, action_type, command, args, trust_layer, source_event_id, reason, logged_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                entry.id,
                entry.gate_result_id,
                entry.action_type,
                entry.command,
                serde_json::to_string(&entry.args)
                    .map_err(|e| GuardDbError::SchemaError(e.to_string()))?,
                entry.trust_layer,
                entry.source_event_id,
                entry.reason,
                entry.logged_at,
            ],
        )?;
        Ok(())
    }

    /// Grant or update a PID trust record.
    pub fn grant_pid_trust(
        &self,
        pid: i32,
        trust_layer: u32,
        auto_grant: bool,
    ) -> Result<(), GuardDbError> {
        let conn = self.conn();
        conn.execute(
            "INSERT OR REPLACE INTO pid_trust (pid, trust_layer, auto_grant, last_use)
             VALUES (?1, ?2, ?3, unixepoch())",
            params![pid, trust_layer, auto_grant],
        )?;
        Ok(())
    }

    /// Revoke a PID's trust (drop to layer 0).
    pub fn revoke_pid_trust(&self, pid: i32) -> Result<Option<(u32, i64)>, GuardDbError> {
        let conn = self.conn();
        let old = conn.query_row(
            "SELECT trust_layer, last_use FROM pid_trust WHERE pid = ?1",
            params![pid],
            |row| Ok((row.get::<_, u32>(0)?, row.get::<_, i64>(1)?)),
        );

        match old {
            Ok((old_layer, last_use)) => {
                conn.execute(
                    "UPDATE pid_trust SET trust_layer = 0, last_use = unixepoch() WHERE pid = ?1",
                    params![pid],
                )?;
                Ok(Some((old_layer, last_use)))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(GuardDbError::Sqlite(e)),
        }
    }

    /// Get PID trust record.
    pub fn get_pid_trust(
        &self,
        pid: i32,
    ) -> Result<Option<crate::trust_types::PidTrustRecord>, GuardDbError> {
        let conn = self.conn();
        let row = conn.query_row(
            "SELECT pid, trust_layer, use_count, last_use, auto_grant, decay_interval, decay_rate
             FROM pid_trust WHERE pid = ?1",
            params![pid],
            |row| {
                Ok((
                    row.get::<_, i32>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, u64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, bool>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, f64>(6)?,
                ))
            },
        );

        match row {
            Ok((pid, trust_layer, use_count, last_use, auto_grant, decay_interval, decay_rate)) => {
                Ok(Some(crate::trust_types::PidTrustRecord {
                    pid,
                    trust_layer,
                    use_count,
                    last_use,
                    auto_grant,
                    decay_interval,
                    decay_rate,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(GuardDbError::Sqlite(e)),
        }
    }

    /// Increment use count for a PID and optionally bump trust layer.
    pub fn record_pid_use(&self, pid: i32, new_confidence: f64) -> Result<(), GuardDbError> {
        let conn = self.conn();
        // Increment use count
        conn.execute(
            "UPDATE pid_trust SET use_count = use_count + 1, last_use = unixepoch() WHERE pid = ?1",
            params![pid],
        )?;

        // If confidence is high enough, bump trust layer
        let layer_threshold = match new_confidence {
            0.8.. => 3,
            0.5.. => 2,
            0.2.. => 1,
            _ => 0,
        };

        conn.execute(
            "UPDATE pid_trust SET trust_layer = MAX(trust_layer, ?1) WHERE pid = ?2",
            params![layer_threshold, pid],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_open_and_init_schema() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("guard.db");
        let db = GuardDb::open(&db_path).unwrap();
        assert!(db_path.exists());

        let conn = db.conn();
        let count: i64 = conn
            .query_row("SELECT count(*) FROM trust_layers", [], |r| r.get(0))
            .unwrap();
        assert!(count >= 4);
    }

    #[test]
    fn test_from_mirror_path() {
        let path = std::path::PathBuf::from("/some/dir/mirror.db");
        let guard_path = GuardDb::from_mirror_path(&path);
        assert_eq!(guard_path, std::path::PathBuf::from("/some/dir/guard.db"));
    }

    #[test]
    fn test_load_default_anneal_config() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("guard.db");
        let db = GuardDb::open(&db_path).unwrap();

        let config = db.load_anneal_config().unwrap();
        assert_eq!(config.decay_rate, 0.02);
        assert!(config.auto_anneal_enabled);
    }

    #[test]
    fn test_persist_and_list_trust_resolutions() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("guard.db");
        let db = GuardDb::open(&db_path).unwrap();

        db.record_trust_resolution(
            Some("act-1"),
            3,
            0.85,
            "agent-1",
            2,
            0.72,
            "project-policy:crabjar",
            Some("user:alice"),
            Some("project:crabjar"),
            vec!["user:max_cap:3".to_string()],
        )
        .unwrap();

        let entries = db.list_trust_resolutions(None, 10).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].requested_layer, 3);
        assert_eq!(entries[0].effective_layer, 2);
        assert_eq!(entries[0].effective_by, "project-policy:crabjar");
    }
}
