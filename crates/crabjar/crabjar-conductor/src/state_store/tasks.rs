//! Task types and operations.

use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::StateStore;

/// Executable unit of work with dependency tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub goal_id: String,
    pub worker_id: Option<String>,
    pub command: String,
    pub dependencies: Vec<String>,
    pub status: TaskStatus,
    pub result: Option<String>,
    /// Phase 2: explicit capability requirements (e.g., ["gpu:nvidia:*"])
    pub required_capabilities: Vec<String>,
    /// Phase 2: minimum trust level required to execute this task
    pub trust_level_required: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Assigned,
    Running,
    Completed,
    Failed,
}

impl TaskStatus {
    fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Assigned => "assigned",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "assigned" => TaskStatus::Assigned,
            "running" => TaskStatus::Running,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            _ => TaskStatus::Pending,
        }
    }
}

fn row_to_task(row: &Row<'_>) -> rusqlite::Result<Task> {
    let deps_str: String = row.get(4)?;
    let required_caps_str: String = row.get(7).unwrap_or_else(|_| "[]".to_string());
    let trust_required: u8 = row.get(8).unwrap_or(0);

    Ok(Task {
        id: row.get(0)?,
        goal_id: row.get(1)?,
        worker_id: row.get(2)?,
        command: row.get(3)?,
        dependencies: serde_json::from_str(&deps_str)
            .map_err(|_e| rusqlite::Error::ExecuteReturnedResults)?,
        status: TaskStatus::from_str(&row.get::<_, String>(5)?),
        result: row.get(6)?,
        required_capabilities: serde_json::from_str(&required_caps_str)
            .map_err(|_e| rusqlite::Error::ExecuteReturnedResults)?,
        trust_level_required: trust_required,
    })
}

// ─── Task operations ──────────────────────────────────────────────

impl StateStore {
    pub fn create_task(&self, goal_id: &str, command: String) -> Result<Task, rusqlite::Error> {
        let task = Task {
            id: Uuid::new_v4().to_string(),
            goal_id: goal_id.to_string(),
            worker_id: None,
            command,
            dependencies: vec![],
            status: TaskStatus::Pending,
            result: None,
            required_capabilities: vec![],
            trust_level_required: 0,
        };

        let deps_json = serde_json::to_string(&task.dependencies)
            .map_err(|_e| rusqlite::Error::ExecuteReturnedResults)?;
        self.db.execute(
            "INSERT INTO tasks (id, goal_id, worker_id, command, dependencies, status) VALUES (?, ?, ?, ?, ?, ?)",
            params![
                task.id,
                task.goal_id,
                task.worker_id,
                task.command,
                deps_json,
                task.status.as_str()
            ],
        )?;

        Ok(task)
    }

    pub fn get_task(&self, id: &str) -> Result<Option<Task>, rusqlite::Error> {
        self.db
            .query_row(
                "SELECT id, goal_id, worker_id, command, dependencies, status, result, required_capabilities, trust_level_required FROM tasks WHERE id = ?",
                params![id],
                row_to_task,
            )
            .optional()
    }

    pub fn list_tasks_for_goal(&self, goal_id: &str) -> Result<Vec<Task>, rusqlite::Error> {
        let mut stmt = self.db.prepare(
            "SELECT id, goal_id, worker_id, command, dependencies, status, result, required_capabilities, trust_level_required FROM tasks WHERE goal_id = ?",
        )?;
        let rows = stmt.query_map(params![goal_id], row_to_task)?;
        rows.collect()
    }

    pub fn assign_task(&self, task_id: &str, worker_id: &str) -> Result<(), rusqlite::Error> {
        self.db.execute(
            "UPDATE tasks SET worker_id = ?, status = ? WHERE id = ?",
            params![worker_id, TaskStatus::Assigned.as_str(), task_id],
        )?;
        Ok(())
    }

    pub fn complete_task(
        &self,
        task_id: &str,
        result: Option<String>,
    ) -> Result<(), rusqlite::Error> {
        self.db.execute(
            "UPDATE tasks SET status = ?, result = ? WHERE id = ?",
            params![TaskStatus::Completed.as_str(), result, task_id],
        )?;
        Ok(())
    }

    pub fn fail_task(&self, task_id: &str, error: String) -> Result<(), rusqlite::Error> {
        self.db.execute(
            "UPDATE tasks SET status = ?, result = ? WHERE id = ?",
            params![TaskStatus::Failed.as_str(), Some(error), task_id],
        )?;
        Ok(())
    }

    /// Get the trust level required for a task (defaults to 0).
    pub fn get_task_trust_level(&self, task_id: &str) -> Result<u8, rusqlite::Error> {
        self.db
            .query_row(
                "SELECT trust_level_required FROM tasks WHERE id = ?",
                params![task_id],
                |r| r.get::<_, u8>(0),
            )
            .optional()?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    /// Get the capabilities required for a task (defaults to empty list).
    pub fn get_task_capabilities(&self, task_id: &str) -> Result<Vec<String>, rusqlite::Error> {
        let caps_str = self
            .db
            .query_row(
                "SELECT required_capabilities FROM tasks WHERE id = ?",
                params![task_id],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_else(|| "[]".to_string());

        serde_json::from_str(&caps_str).map_err(|_e| rusqlite::Error::ExecuteReturnedResults)
    }

    /// Set the trust level required for a task.
    pub fn set_task_trust_level(
        &self,
        task_id: &str,
        trust_level: u8,
    ) -> Result<(), rusqlite::Error> {
        self.db.execute(
            "UPDATE tasks SET trust_level_required = ? WHERE id = ?",
            params![trust_level, task_id],
        )?;
        Ok(())
    }

    /// Set the capabilities required for a task.
    pub fn set_task_capabilities(
        &self,
        task_id: &str,
        caps: &[String],
    ) -> Result<(), rusqlite::Error> {
        let caps_json =
            serde_json::to_string(caps).map_err(|_e| rusqlite::Error::ExecuteReturnedResults)?;
        self.db.execute(
            "UPDATE tasks SET required_capabilities = ? WHERE id = ?",
            params![caps_json, task_id],
        )?;
        Ok(())
    }

    /// Get pending tasks that have no dependencies or whose dependencies are all completed.
    pub fn get_ready_tasks(&self) -> Result<Vec<Task>, rusqlite::Error> {
        let sql = r"
            SELECT t.id, t.goal_id, t.worker_id, t.command, t.dependencies, t.status, t.result,
                   t.required_capabilities, t.trust_level_required
            FROM tasks t
            WHERE t.status = 'pending'
              AND (
                json_array_length(t.dependencies) = 0
                OR NOT EXISTS (
                    SELECT 1 FROM json_each(t.dependencies) AS dep
                    WHERE (SELECT status FROM tasks WHERE id = dep.value) != 'completed'
                )
              )
        ";
        let mut stmt = self.db.prepare(sql)?;
        let rows = stmt.query_map(params![], row_to_task)?;
        rows.collect()
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
    fn task_lifecycle() {
        let (store, _dir) = test_store();

        // Create a goal first (task depends on it via FK constraint)
        let goal = store.create_goal("Test".to_string()).unwrap();

        let task = store
            .create_task(&goal.id, "echo hello".to_string())
            .unwrap();

        assert_eq!(task.status, TaskStatus::Pending);

        store
            .complete_task(&task.id, Some("done".to_string()))
            .unwrap();
        let completed = store.get_task(&task.id).unwrap().unwrap();
        assert_eq!(completed.status, TaskStatus::Completed);
    }

    #[test]
    fn get_ready_tasks() {
        let (store, _dir) = test_store();
        let goal = store.create_goal("Ready tasks".to_string()).unwrap();
        store.create_task(&goal.id, "task1".to_string()).unwrap();
        store.create_task(&goal.id, "task2".to_string()).unwrap();

        let ready = store.get_ready_tasks().unwrap();
        assert_eq!(ready.len(), 2);
    }
}
