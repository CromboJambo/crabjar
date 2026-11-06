use rusqlite::params;

use crate::guard_db::{GuardDb, GuardDbError};

impl GuardDb {
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
