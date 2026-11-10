//! Worker deployment persistence for fleet tasking.
//!
//! Tracks individual worker deployments (sandbox + agent + task) through their lifecycle.
//! Part of SUPERVISOR-DEPLOYMENT-READINESS Phase 1.

use chrono::Utc;
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::StateStore;

/// Lifecycle status of a worker deployment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeploymentStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

impl DeploymentStatus {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            DeploymentStatus::Pending => "pending",
            DeploymentStatus::Running => "running",
            DeploymentStatus::Completed => "completed",
            DeploymentStatus::Failed => "failed",
            DeploymentStatus::Cancelled => "cancelled",
            DeploymentStatus::Timeout => "timeout",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "running" => DeploymentStatus::Running,
            "completed" => DeploymentStatus::Completed,
            "failed" => DeploymentStatus::Failed,
            "cancelled" => DeploymentStatus::Cancelled,
            "timeout" => DeploymentStatus::Timeout,
            _ => DeploymentStatus::Pending,
        }
    }
}

/// A worker deployment: sandbox + agent executing a task spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerDeployment {
    pub id: String,
    pub supervisor_id: String,
    /// Full task spec as submitted (JSON).
    pub task_spec: serde_json::Value,
    /// Sandbox container ID (set when sandbox is created).
    pub sandbox_id: Option<String>,
    pub status: DeploymentStatus,
    /// Structured outcome on completion.
    pub outcome: Option<serde_json::Value>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub error: Option<String>,
}

fn row_to_deployment(row: &Row<'_>) -> rusqlite::Result<WorkerDeployment> {
    let task_spec_str: String = row.get(2)?;
    let outcome_str: Option<String> = row.get(5)?;
    Ok(WorkerDeployment {
        id: row.get(0)?,
        supervisor_id: row.get(1)?,
        task_spec: serde_json::from_str(&task_spec_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            )
        })?,
        sandbox_id: row.get(3)?,
        status: DeploymentStatus::from_str(&row.get::<_, String>(4)?),
        outcome: outcome_str.and_then(|s| serde_json::from_str(&s).ok()),
        created_at: row.get(6)?,
        completed_at: row.get(7)?,
        error: row.get(8)?,
    })
}

// ─── Deployment operations ────────────────────────────────────────

impl StateStore {
    /// Create a new pending deployment record.
    pub fn create_deployment(
        &self,
        supervisor_id: String,
        task_spec: serde_json::Value,
    ) -> Result<WorkerDeployment, rusqlite::Error> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let task_spec_str = serde_json::to_string(&task_spec).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            )
        })?;

        self.db.execute(
            "INSERT INTO worker_deployments (id, supervisor_id, task_spec, status, created_at) VALUES (?, ?, ?, 'pending', ?)",
            params![id, supervisor_id, task_spec_str, created_at],
        )?;

        Ok(WorkerDeployment {
            id,
            supervisor_id,
            task_spec,
            sandbox_id: None,
            status: DeploymentStatus::Pending,
            outcome: None,
            created_at,
            completed_at: None,
            error: None,
        })
    }

    /// Get a deployment by ID.
    pub fn get_deployment(&self, id: &str) -> Result<Option<WorkerDeployment>, rusqlite::Error> {
        self.db
            .query_row(
                "SELECT id, supervisor_id, task_spec, sandbox_id, status, outcome, created_at, completed_at, error FROM worker_deployments WHERE id = ?",
                params![id],
                row_to_deployment,
            )
            .optional()
    }

    /// List deployments filtered by supervisor and/or status.
    pub fn list_deployments(
        &self,
        supervisor_id: Option<&str>,
        status: Option<&DeploymentStatus>,
    ) -> Result<Vec<WorkerDeployment>, rusqlite::Error> {
        let mut sql = String::from(
            "SELECT id, supervisor_id, task_spec, sandbox_id, status, outcome, created_at, completed_at, error FROM worker_deployments WHERE 1=1",
        );

        if supervisor_id.is_some() {
            sql.push_str(" AND supervisor_id = ?");
        }
        if status.is_some() {
            sql.push_str(" AND status = ?");
        }
        sql.push_str(" ORDER BY created_at DESC");

        let mut stmt = self.db.prepare(&sql)?;

        // Build params dynamically
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(sid) = supervisor_id {
            param_values.push(Box::new(sid.to_string()));
        }
        if let Some(st) = status {
            param_values.push(Box::new(st.as_str().to_string()));
        }

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            param_values.iter().map(AsRef::as_ref).collect();
        let rows = stmt.query_map(params_refs.as_slice(), row_to_deployment)?;
        rows.collect()
    }

    /// Update deployment status and optional fields.
    pub fn update_deployment(
        &self,
        id: &str,
        status: &DeploymentStatus,
        sandbox_id: Option<&str>,
        outcome: Option<&serde_json::Value>,
        error: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let completed_at = if matches!(
            status,
            DeploymentStatus::Completed
                | DeploymentStatus::Failed
                | DeploymentStatus::Cancelled
                | DeploymentStatus::Timeout
        ) {
            Some(Utc::now().to_rfc3339())
        } else {
            None
        };

        let outcome_str = outcome.and_then(|v| serde_json::to_string(v).ok());

        self.db.execute(
            "UPDATE worker_deployments SET status = ?, sandbox_id = COALESCE(?, sandbox_id), outcome = ?, completed_at = COALESCE(?, completed_at), error = ? WHERE id = ?",
            params![status.as_str(), sandbox_id, outcome_str, completed_at, error, id],
        )?;
        Ok(())
    }

    /// Mark a deployment as running and record its sandbox ID.
    pub fn mark_deployment_running(
        &self,
        id: &str,
        sandbox_id: &str,
    ) -> Result<(), rusqlite::Error> {
        self.update_deployment(id, &DeploymentStatus::Running, Some(sandbox_id), None, None)
    }

    /// Mark a deployment as completed with an outcome.
    pub fn mark_deployment_completed(
        &self,
        id: &str,
        outcome: &serde_json::Value,
    ) -> Result<(), rusqlite::Error> {
        self.update_deployment(id, &DeploymentStatus::Completed, None, Some(outcome), None)
    }

    /// Mark a deployment as failed with an error message.
    pub fn mark_deployment_failed(&self, id: &str, error: &str) -> Result<(), rusqlite::Error> {
        self.update_deployment(id, &DeploymentStatus::Failed, None, None, Some(error))
    }

    /// Cancel a pending or running deployment.
    pub fn cancel_deployment(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.update_deployment(
            id,
            &DeploymentStatus::Cancelled,
            None,
            None,
            Some("cancelled by supervisor"),
        )
    }

    /// Count active (pending + running) deployments for a supervisor.
    pub fn count_active_deployments(&self, supervisor_id: &str) -> Result<i32, rusqlite::Error> {
        self.db.query_row(
            "SELECT COUNT(*) FROM worker_deployments WHERE supervisor_id = ? AND status IN ('pending', 'running')",
            params![supervisor_id],
            |r| r.get::<_, i32>(0),
        )
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
    fn create_and_get_deployment() {
        let (store, _dir) = test_store();
        let task_spec = serde_json::json!({
            "id": "test-task-001",
            "title": "Test deployment",
            "repo": "pesti",
            "context": {"description": "test"}
        });

        let dep = store
            .create_deployment("ftw3-supervisor".to_string(), task_spec.clone())
            .unwrap();
        assert_eq!(dep.supervisor_id, "ftw3-supervisor");
        assert_eq!(dep.status, DeploymentStatus::Pending);
        assert!(dep.sandbox_id.is_none());

        let fetched = store.get_deployment(&dep.id).unwrap().unwrap();
        assert_eq!(fetched.id, dep.id);
        assert_eq!(fetched.task_spec["title"], "Test deployment");
    }

    #[test]
    fn deployment_lifecycle() {
        let (store, _dir) = test_store();
        let task_spec = serde_json::json!({"id": "lifecycle-test"});
        let dep = store
            .create_deployment("ftw3-supervisor".to_string(), task_spec)
            .unwrap();

        // Mark running with sandbox
        store
            .mark_deployment_running(&dep.id, "sandbox-abc123")
            .unwrap();
        let d = store.get_deployment(&dep.id).unwrap().unwrap();
        assert_eq!(d.status, DeploymentStatus::Running);
        assert_eq!(d.sandbox_id.as_deref(), Some("sandbox-abc123"));

        // Mark completed with outcome
        let outcome = serde_json::json!({"status": "success", "artifacts": ["report.md"]});
        store.mark_deployment_completed(&dep.id, &outcome).unwrap();
        let d = store.get_deployment(&dep.id).unwrap().unwrap();
        assert_eq!(d.status, DeploymentStatus::Completed);
        assert!(d.completed_at.is_some());
        assert_eq!(d.outcome.unwrap()["status"], "success");
    }

    #[test]
    fn deployment_failure() {
        let (store, _dir) = test_store();
        let task_spec = serde_json::json!({"id": "fail-test"});
        let dep = store
            .create_deployment("lappy-supervisor".to_string(), task_spec)
            .unwrap();

        store
            .mark_deployment_failed(&dep.id, "timeout after 3600s")
            .unwrap();
        let d = store.get_deployment(&dep.id).unwrap().unwrap();
        assert_eq!(d.status, DeploymentStatus::Failed);
        assert_eq!(d.error.as_deref(), Some("timeout after 3600s"));
    }

    #[test]
    fn list_deployments_by_supervisor() {
        let (store, _dir) = test_store();
        let spec = serde_json::json!({"id": "x"});
        store
            .create_deployment("ftw3-supervisor".to_string(), spec.clone())
            .unwrap();
        store
            .create_deployment("lappy-supervisor".to_string(), spec.clone())
            .unwrap();
        store
            .create_deployment("ftw3-supervisor".to_string(), spec)
            .unwrap();

        let ftw3_deps = store
            .list_deployments(Some("ftw3-supervisor"), None)
            .unwrap();
        assert_eq!(ftw3_deps.len(), 2);

        let all = store.list_deployments(None, None).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn count_active_deployments() {
        let (store, _dir) = test_store();
        let spec = serde_json::json!({"id": "x"});
        let d1 = store
            .create_deployment("ftw3-supervisor".to_string(), spec.clone())
            .unwrap();
        let d2 = store
            .create_deployment("ftw3-supervisor".to_string(), spec)
            .unwrap();

        assert_eq!(
            store.count_active_deployments("ftw3-supervisor").unwrap(),
            2
        );

        // Mark one running, one completed
        store.mark_deployment_running(&d1.id, "sb-1").unwrap();
        store
            .mark_deployment_completed(&d2.id, &serde_json::json!({}))
            .unwrap();

        assert_eq!(
            store.count_active_deployments("ftw3-supervisor").unwrap(),
            1
        );
    }

    #[test]
    fn cancel_deployment() {
        let (store, _dir) = test_store();
        let spec = serde_json::json!({"id": "cancel-test"});
        let dep = store
            .create_deployment("ftw3-supervisor".to_string(), spec)
            .unwrap();

        store.cancel_deployment(&dep.id).unwrap();
        let d = store.get_deployment(&dep.id).unwrap().unwrap();
        assert_eq!(d.status, DeploymentStatus::Cancelled);
    }
}
