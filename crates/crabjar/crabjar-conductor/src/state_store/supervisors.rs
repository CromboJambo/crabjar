//! Supervisor readiness and heartbeat tracking.
//!
//! Part of SUPERVISOR-DEPLOYMENT-READINESS Phase 2 (supervisor status endpoint),
//! but the table is created in Phase 1 migrations for schema stability.

use chrono::Utc;
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use super::StateStore;

/// Readiness and health state of a fleet supervisor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorStatus {
    pub id: String,
    pub ready: bool,
    /// Health check results (network, disk, podman, etc.).
    pub health: Option<serde_json::Value>,
    pub last_heartbeat: Option<String>,
    pub active_deployments: i32,
}

fn row_to_supervisor(row: &Row<'_>) -> rusqlite::Result<SupervisorStatus> {
    let health_str: Option<String> = row.get(2)?;
    Ok(SupervisorStatus {
        id: row.get(0)?,
        ready: row.get(1)?,
        health: health_str.and_then(|s| serde_json::from_str(&s).ok()),
        last_heartbeat: row.get(3)?,
        active_deployments: row.get(4)?,
    })
}

// ─── Supervisor status operations ─────────────────────────────────

impl StateStore {
    /// Upsert supervisor status (insert or update on conflict).
    pub fn upsert_supervisor_status(
        &self,
        status: &SupervisorStatus,
    ) -> Result<(), rusqlite::Error> {
        let health_str = status
            .health
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok());

        self.db.execute(
            "INSERT INTO supervisor_status (id, ready, health, last_heartbeat, active_deployments)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                 ready = excluded.ready,
                 health = COALESCE(excluded.health, health),
                 last_heartbeat = excluded.last_heartbeat,
                 active_deployments = excluded.active_deployments",
            params![
                status.id,
                status.ready,
                health_str,
                status.last_heartbeat,
                status.active_deployments
            ],
        )?;
        Ok(())
    }

    /// Get supervisor status by ID.
    pub fn get_supervisor_status(
        &self,
        id: &str,
    ) -> Result<Option<SupervisorStatus>, rusqlite::Error> {
        self.db
            .query_row(
                "SELECT id, ready, health, last_heartbeat, active_deployments FROM supervisor_status WHERE id = ?",
                params![id],
                row_to_supervisor,
            )
            .optional()
    }

    /// List all supervisor statuses.
    pub fn list_supervisor_statuses(&self) -> Result<Vec<SupervisorStatus>, rusqlite::Error> {
        let mut stmt = self.db.prepare(
            "SELECT id, ready, health, last_heartbeat, active_deployments FROM supervisor_status",
        )?;
        let rows = stmt.query_map([], row_to_supervisor)?;
        rows.collect()
    }

    /// Record a heartbeat from a supervisor and update its readiness.
    pub fn record_heartbeat(
        &self,
        id: &str,
        ready: bool,
        health: &serde_json::Value,
    ) -> Result<(), rusqlite::Error> {
        let now = Utc::now().to_rfc3339();
        let health_str = serde_json::to_string(&health).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            )
        })?;

        // Get current active deployments count
        let active = self.count_active_deployments(id).unwrap_or(0);

        self.db.execute(
            "INSERT INTO supervisor_status (id, ready, health, last_heartbeat, active_deployments)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                 ready = excluded.ready,
                 health = excluded.health,
                 last_heartbeat = excluded.last_heartbeat,
                 active_deployments = excluded.active_deployments",
            params![id, ready, health_str, now, active],
        )?;
        Ok(())
    }

    /// Remove a supervisor's status record.
    pub fn remove_supervisor_status(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.db
            .execute("DELETE FROM supervisor_status WHERE id = ?", params![id])?;
        Ok(())
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
    fn upsert_and_get_supervisor() {
        let (store, _dir) = test_store();
        let status = SupervisorStatus {
            id: "ftw3-supervisor".to_string(),
            ready: true,
            health: Some(serde_json::json!({"network": "ok", "disk": "ok"})),
            last_heartbeat: Some(Utc::now().to_rfc3339()),
            active_deployments: 0,
        };

        store.upsert_supervisor_status(&status).unwrap();
        let fetched = store
            .get_supervisor_status("ftw3-supervisor")
            .unwrap()
            .unwrap();
        assert_eq!(fetched.id, "ftw3-supervisor");
        assert!(fetched.ready);
    }

    #[test]
    fn upsert_updates_existing() {
        let (store, _dir) = test_store();
        let s1 = SupervisorStatus {
            id: "lappy-supervisor".to_string(),
            ready: true,
            health: None,
            last_heartbeat: Some("2026-11-08T00:00:00Z".to_string()),
            active_deployments: 0,
        };
        store.upsert_supervisor_status(&s1).unwrap();

        // Update to not ready
        let s2 = SupervisorStatus {
            id: "lappy-supervisor".to_string(),
            ready: false,
            health: Some(serde_json::json!({"error": "podman down"})),
            last_heartbeat: Some("2026-11-08T01:00:00Z".to_string()),
            active_deployments: 0,
        };
        store.upsert_supervisor_status(&s2).unwrap();

        let fetched = store
            .get_supervisor_status("lappy-supervisor")
            .unwrap()
            .unwrap();
        assert!(!fetched.ready);
        assert_eq!(
            fetched.last_heartbeat.as_deref(),
            Some("2026-11-08T01:00:00Z")
        );

        // Only one record
        let all = store.list_supervisor_statuses().unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn heartbeat_records_status() {
        let (store, _dir) = test_store();
        let health = serde_json::json!({
            "network": {"tailscale": true, "github": true},
            "disk": {"used_percent": 42.0},
            "podman": {"running": true}
        });

        store
            .record_heartbeat("ftw3-supervisor", true, &health)
            .unwrap();
        let s = store
            .get_supervisor_status("ftw3-supervisor")
            .unwrap()
            .unwrap();
        assert!(s.ready);
        assert!(s.last_heartbeat.is_some());
        assert_eq!(s.health.unwrap()["podman"]["running"], true);
    }

    #[test]
    fn remove_supervisor() {
        let (store, _dir) = test_store();
        store
            .record_heartbeat("sovnode-supervisor", true, &serde_json::json!({}))
            .unwrap();
        assert!(
            store
                .get_supervisor_status("sovnode-supervisor")
                .unwrap()
                .is_some()
        );

        store
            .remove_supervisor_status("sovnode-supervisor")
            .unwrap();
        assert!(
            store
                .get_supervisor_status("sovnode-supervisor")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn list_all_supervisors() {
        let (store, _dir) = test_store();
        store
            .record_heartbeat("ftw3", true, &serde_json::json!({}))
            .unwrap();
        store
            .record_heartbeat("lappy", false, &serde_json::json!({}))
            .unwrap();

        let all = store.list_supervisor_statuses().unwrap();
        assert_eq!(all.len(), 2);
    }
}
