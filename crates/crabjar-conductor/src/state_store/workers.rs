//! Worker types and operations including trust level management.

use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth;

use super::StateStore;

/// Registered fleet member with capability metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worker {
    pub id: String,
    pub name: String,
    pub ssh_key: Option<String>,
    pub capabilities: Vec<String>,
    pub status: WorkerStatus,
    /// Trust level (0-3): earned through successful task completion.
    pub trust_level: u8,
    /// Count of successfully completed tasks.
    pub successful_tasks: i32,
    /// Count of failed tasks.
    pub failed_tasks: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkerStatus {
    Online,
    Busy,
    Offline,
}

impl WorkerStatus {
    fn as_str(&self) -> &'static str {
        match self {
            WorkerStatus::Online => "online",
            WorkerStatus::Busy => "busy",
            WorkerStatus::Offline => "offline",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "busy" => WorkerStatus::Busy,
            "offline" => WorkerStatus::Offline,
            _ => WorkerStatus::Online,
        }
    }
}

fn row_to_worker(row: &Row<'_>) -> rusqlite::Result<Worker> {
    let caps_str: String = row.get(3)?;
    Ok(Worker {
        id: row.get(0)?,
        name: row.get(1)?,
        ssh_key: row.get(2)?,
        capabilities: serde_json::from_str(&caps_str)
            .map_err(|_e| rusqlite::Error::ExecuteReturnedResults)?,
        status: WorkerStatus::from_str(&row.get::<_, String>(4)?),
        trust_level: row.get(5).unwrap_or(0u8),
        successful_tasks: row.get(6).unwrap_or(0i32),
        failed_tasks: row.get(7).unwrap_or(0i32),
    })
}

// ─── Worker operations ────────────────────────────────────────────

impl StateStore {
    pub fn register_worker(
        &self,
        name: String,
        capabilities: Vec<String>,
        ssh_key: Option<String>,
    ) -> Result<(Worker, String), rusqlite::Error> {
        let worker = Worker {
            id: Uuid::new_v4().to_string(),
            name: name.clone(),
            ssh_key: ssh_key.clone(),
            capabilities: capabilities.clone(),
            status: WorkerStatus::Online,
            trust_level: 0,
            successful_tasks: 0,
            failed_tasks: 0,
        };

        // Generate auth token for the worker
        let token = match auth::generate_token(&worker.id, &worker.name, &capabilities, 0) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("Failed to generate auth token: {}", e);
                String::new()
            }
        };

        let caps_json = serde_json::to_string(&worker.capabilities)
            .map_err(|_e| rusqlite::Error::ExecuteReturnedResults)?;
        self.db.execute(
            "INSERT OR REPLACE INTO workers (id, name, ssh_key, capabilities, status, auth_token) VALUES (?, ?, ?, ?, ?, ?)",
            params![
                worker.id,
                worker.name,
                ssh_key,
                caps_json,
                worker.status.as_str(),
                token.clone()
            ],
        )?;

        Ok((worker, token))
    }

    pub fn get_worker(&self, id: &str) -> Result<Option<Worker>, rusqlite::Error> {
        self.db
            .query_row(
                "SELECT id, name, ssh_key, capabilities, status, trust_level, successful_tasks, failed_tasks FROM workers WHERE id = ?",
                params![id],
                row_to_worker,
            )
            .optional()
    }

    pub fn get_worker_by_name(&self, name: &str) -> Result<Option<Worker>, rusqlite::Error> {
        self.db
            .query_row(
                "SELECT id, name, ssh_key, capabilities, status, trust_level, successful_tasks, failed_tasks FROM workers WHERE name = ?",
                params![name],
                row_to_worker,
            )
            .optional()
    }

    pub fn list_workers(&self) -> Result<Vec<Worker>, rusqlite::Error> {
        let mut stmt = self
            .db
            .prepare("SELECT id, name, ssh_key, capabilities, status, trust_level, successful_tasks, failed_tasks FROM workers")?;
        let rows = stmt.query_map(params![], row_to_worker)?;
        rows.collect()
    }

    pub fn update_worker_status(
        &self,
        id: &str,
        status: WorkerStatus,
    ) -> Result<(), rusqlite::Error> {
        self.db.execute(
            "UPDATE workers SET status = ? WHERE id = ?",
            params![status.as_str(), id],
        )?;
        Ok(())
    }

    /// Set a worker's trust level directly (for testing or manual override).
    pub fn set_worker_trust_level(&self, id: &str, level: u8) -> Result<(), rusqlite::Error> {
        self.db.execute(
            "UPDATE workers SET trust_level = ? WHERE id = ?",
            params![level, id],
        )?;
        Ok(())
    }

    pub fn heartbeat(&self, worker_id: &str) -> Result<(), rusqlite::Error> {
        let now = Utc::now().to_rfc3339();

        // Check for idle decay first
        self.apply_idle_decay(worker_id)?;

        // Update status and last heartbeat
        self.db.execute(
            "UPDATE workers SET status = 'online', last_heartbeat = ? WHERE id = ?",
            params![now, worker_id],
        )?;
        Ok(())
    }

    /// For testing: set the last heartbeat timestamp directly.
    pub fn update_worker_last_heartbeat(
        &self,
        worker_id: &str,
        time: &DateTime<Utc>,
    ) -> Result<(), rusqlite::Error> {
        self.db.execute(
            "UPDATE workers SET last_heartbeat = ? WHERE id = ?",
            params![time.to_rfc3339(), worker_id],
        )?;
        Ok(())
    }

    /// Apply idle decay: if worker hasn't been active in >30 days, drop trust level.
    fn apply_idle_decay(&self, worker_id: &str) -> Result<(), rusqlite::Error> {
        // Get last heartbeat and current trust level
        let row = self
            .db
            .query_row(
                "SELECT last_heartbeat, trust_level FROM workers WHERE id = ?",
                params![worker_id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, u8>(1)?)),
            )
            .optional()?;

        if let Some((last_hb, current_level)) = row {
            if last_hb.is_empty() {
                return Ok(()); // No heartbeat recorded yet
            }

            if let Ok(last_time) = DateTime::parse_from_rfc3339(&last_hb) {
                let now = Utc::now();
                // Convert parsed time to UTC for subtraction
                let last_utc = last_time.with_timezone(&Utc);
                let elapsed = (now - last_utc).num_days();

                if elapsed > 30 && current_level > 0 {
                    // Idle for >30 days, drop one trust level
                    self.db.execute(
                        "UPDATE workers SET trust_level = ? WHERE id = ?",
                        params![current_level.saturating_sub(1), worker_id],
                    )?;
                }
            } else {
                // Invalid timestamp, skip decay
            }
        }

        Ok(())
    }

    /// Record a successful task completion and potentially upgrade trust level.
    pub fn record_task_success(&self, worker_id: &str) -> Result<u8, rusqlite::Error> {
        // Increment success count
        self.db.execute(
            "UPDATE workers SET successful_tasks = successful_tasks + 1 WHERE id = ?",
            params![worker_id],
        )?;

        // Check if worker should be upgraded
        let new_level = self.should_upgrade_trust(worker_id)?;
        if new_level > 0 {
            self.db.execute(
                "UPDATE workers SET trust_level = ? WHERE id = ?",
                params![new_level, worker_id],
            )?;
        }

        Ok(new_level)
    }

    /// Record a task failure and potentially downgrade trust level.
    pub fn record_task_failure(&self, worker_id: &str) -> Result<u8, rusqlite::Error> {
        // Increment failure count
        self.db.execute(
            "UPDATE workers SET failed_tasks = failed_tasks + 1 WHERE id = ?",
            params![worker_id],
        )?;

        // Check if worker should be downgraded
        let new_level = self.should_downgrade_trust(worker_id)?;
        if new_level >= 0 {
            self.db.execute(
                "UPDATE workers SET trust_level = ? WHERE id = ?",
                params![new_level, worker_id],
            )?;
            return Ok(new_level as u8);
        }

        // Return current level if no downgrade
        let row = self.db.query_row(
            "SELECT trust_level FROM workers WHERE id = ?",
            params![worker_id],
            |r| r.get::<_, u8>(0),
        )?;
        Ok(row)
    }

    /// Check if worker qualifies for trust level upgrade.
    fn should_upgrade_trust(&self, worker_id: &str) -> Result<u8, rusqlite::Error> {
        let row = self.db.query_row(
            "SELECT trust_level, successful_tasks FROM workers WHERE id = ?",
            params![worker_id],
            |r| Ok((r.get::<_, u8>(0)?, r.get::<_, i32>(1)?)),
        )?;

        let (current_level, successes) = row;

        match current_level {
            0 if successes >= 5 => Ok(1),
            1 if successes >= 20 => Ok(2),
            2 if successes >= 50 => Ok(3),
            _ => Ok(0), // No upgrade
        }
    }

    /// Check if worker should be downgraded due to failures.
    fn should_downgrade_trust(&self, worker_id: &str) -> Result<i8, rusqlite::Error> {
        let row = self.db.query_row(
            "SELECT trust_level, successful_tasks, failed_tasks FROM workers WHERE id = ?",
            params![worker_id],
            |r| Ok((r.get::<_, u8>(0)?, r.get::<_, i32>(1)?, r.get::<_, i32>(2)?)),
        )?;

        let (current_level, successes, failures) = row;

        // Downgrade if failure rate exceeds 50% of recent tasks
        let total = successes + failures;
        if total > 10 && (f64::from(failures) / f64::from(total)) > 0.5 {
            return Ok((current_level.saturating_sub(1)) as i8);
        }

        Ok(-1) // No downgrade
    }

    /// Generate a new auth token for a worker (after trust level change).
    pub fn refresh_worker_token(&self, worker_id: &str) -> Result<String, rusqlite::Error> {
        let worker = self
            .get_worker(worker_id)?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;

        let token = match auth::generate_token(
            &worker.id,
            &worker.name,
            &worker.capabilities,
            worker.trust_level,
        ) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("Failed to refresh auth token: {}", e);
                return Err(rusqlite::Error::ExecuteReturnedResults);
            }
        };

        self.db.execute(
            "UPDATE workers SET auth_token = ? WHERE id = ?",
            params![token.clone(), worker_id],
        )?;

        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_store() -> (StateStore, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let store = StateStore::open(&path).unwrap();
        (store, dir)
    }

    #[test]
    fn worker_registration() {
        let (store, _dir) = test_store();
        unsafe { std::env::set_var("CRABJAR_AUTH_SECRET", "test-secret") };
        let (worker, token) = store
            .register_worker(
                "ftw3".to_string(),
                vec!["gpu:nvidia:3070ti".to_string()],
                None,
            )
            .unwrap();
        assert!(!token.is_empty());
        let fetched = store.get_worker(&worker.id).unwrap().unwrap();
        assert_eq!(fetched.name, "ftw3");
        assert_eq!(fetched.capabilities[0], "gpu:nvidia:3070ti");
        assert_eq!(fetched.trust_level, 0);
    }

    #[test]
    fn trust_upgrade_threshold() {
        let (store, _dir) = test_store();
        unsafe { std::env::set_var("CRABJAR_AUTH_SECRET", "test-secret") };
        let (worker, _token) = store
            .register_worker("upgrader".to_string(), vec![], None)
            .unwrap();

        // 4 successes - no upgrade yet
        for _ in 0..4 {
            store.record_task_success(&worker.id).unwrap();
        }
        let w = store.get_worker(&worker.id).unwrap().unwrap();
        assert_eq!(w.trust_level, 0);

        // 5th success triggers upgrade to level 1
        store.record_task_success(&worker.id).unwrap();
        let w = store.get_worker(&worker.id).unwrap().unwrap();
        assert_eq!(w.trust_level, 1);
    }
}
