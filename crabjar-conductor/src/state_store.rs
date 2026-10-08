//! SQLite-backed state store for conductor persistence.
//!
//! Provides durable storage for goals, tasks, and workers using embedded SQLite.
//! Schema migrations are handled automatically on open.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

/// High-level objective submitted by a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub description: String,
    pub status: GoalStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoalStatus {
    Decomposed,
    Running,
    Completed,
    Failed,
}

impl GoalStatus {
    fn as_str(&self) -> &'static str {
        match self {
            GoalStatus::Decomposed => "decomposed",
            GoalStatus::Running => "running",
            GoalStatus::Completed => "completed",
            GoalStatus::Failed => "failed",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "running" => GoalStatus::Running,
            "completed" => GoalStatus::Completed,
            "failed" => GoalStatus::Failed,
            _ => GoalStatus::Decomposed,
        }
    }
}

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

/// Registered fleet member with capability metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worker {
    pub id: String,
    pub name: String,
    pub ssh_key: Option<String>,
    pub capabilities: Vec<String>,
    pub status: WorkerStatus,
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

fn row_to_goal(row: &Row<'_>) -> rusqlite::Result<Goal> {
    let created_at_str: String = row.get(3)?;
    Ok(Goal {
        id: row.get(0)?,
        description: row.get(1)?,
        status: GoalStatus::from_str(&row.get::<_, String>(2)?),
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| rusqlite::Error::ExecuteReturnedResults)?
            .with_timezone(&Utc),
    })
}

fn row_to_task(row: &Row<'_>) -> rusqlite::Result<Task> {
    let deps_str: String = row.get(4)?;
    Ok(Task {
        id: row.get(0)?,
        goal_id: row.get(1)?,
        worker_id: row.get(2)?,
        command: row.get(3)?,
        dependencies: serde_json::from_str(&deps_str)
            .map_err(|e| rusqlite::Error::ExecuteReturnedResults)?,
        status: TaskStatus::from_str(&row.get::<_, String>(5)?),
        result: row.get(6)?,
    })
}

fn row_to_worker(row: &Row<'_>) -> rusqlite::Result<Worker> {
    let caps_str: String = row.get(3)?;
    Ok(Worker {
        id: row.get(0)?,
        name: row.get(1)?,
        ssh_key: row.get(2)?,
        capabilities: serde_json::from_str(&caps_str)
            .map_err(|e| rusqlite::Error::ExecuteReturnedResults)?,
        status: WorkerStatus::from_str(&row.get::<_, String>(4)?),
    })
}

/// SQLite-backed persistent state store.
pub struct StateStore {
    db: rusqlite::Connection,
}

impl StateStore {
    /// Open or create a state store at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let db = Connection::open(path)?;

        // Run migrations
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS goals (
                id TEXT PRIMARY KEY,
                description TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'decomposed',
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                goal_id TEXT NOT NULL REFERENCES goals(id),
                worker_id TEXT REFERENCES workers(id),
                command TEXT NOT NULL,
                dependencies TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'pending',
                result TEXT
            );

            CREATE TABLE IF NOT EXISTS workers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                ssh_key TEXT,
                capabilities TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'online'
            );",
        )?;

        Ok(Self { db })
    }

    // ─── Goal operations ──────────────────────────────────────────────

    pub fn create_goal(&self, description: String) -> Result<Goal, rusqlite::Error> {
        let goal = Goal {
            id: Uuid::new_v4().to_string(),
            description,
            status: GoalStatus::Decomposed,
            created_at: Utc::now(),
        };

        self.db.execute(
            "INSERT INTO goals (id, description, status, created_at) VALUES (?, ?, ?, ?)",
            params![
                goal.id,
                goal.description,
                goal.status.as_str(),
                goal.created_at.to_rfc3339()
            ],
        )?;

        Ok(goal)
    }

    pub fn get_goal(&self, id: &str) -> Result<Option<Goal>, rusqlite::Error> {
        self.db
            .query_row(
                "SELECT id, description, status, created_at FROM goals WHERE id = ?",
                params![id],
                row_to_goal,
            )
            .optional()
    }

    pub fn list_goals(&self) -> Result<Vec<Goal>, rusqlite::Error> {
        let mut stmt = self.db.prepare("SELECT id, description, status, created_at FROM goals")?;
        let rows = stmt.query_map(params![], row_to_goal)?;
        rows.collect()
    }

    pub fn update_goal_status(&self, id: &str, status: GoalStatus) -> Result<(), rusqlite::Error> {
        self.db.execute(
            "UPDATE goals SET status = ? WHERE id = ?",
            params![status.as_str(), id],
        )?;
        Ok(())
    }

    // ─── Task operations ──────────────────────────────────────────────

    pub fn create_task(&self, goal_id: &str, command: String) -> Result<Task, rusqlite::Error> {
        let task = Task {
            id: Uuid::new_v4().to_string(),
            goal_id: goal_id.to_string(),
            worker_id: None,
            command,
            dependencies: vec![],
            status: TaskStatus::Pending,
            result: None,
        };

        let deps_json = serde_json::to_string(&task.dependencies)
            .map_err(|e| rusqlite::Error::ExecuteReturnedResults)?;
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
                "SELECT id, goal_id, worker_id, command, dependencies, status, result FROM tasks WHERE id = ?",
                params![id],
                row_to_task,
            )
            .optional()
    }

    pub fn list_tasks_for_goal(&self, goal_id: &str) -> Result<Vec<Task>, rusqlite::Error> {
        let mut stmt = self.db.prepare(
            "SELECT id, goal_id, worker_id, command, dependencies, status, result FROM tasks WHERE goal_id = ?",
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

    pub fn complete_task(&self, task_id: &str, result: Option<String>) -> Result<(), rusqlite::Error> {
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

    /// Get pending tasks that have no dependencies or whose dependencies are all completed.
    pub fn get_ready_tasks(&self) -> Result<Vec<Task>, rusqlite::Error> {
        let sql = r#"
            SELECT t.id, t.goal_id, t.worker_id, t.command, t.dependencies, t.status, t.result
            FROM tasks t
            WHERE t.status = 'pending'
              AND (
                json_array_length(t.dependencies) = 0
                OR NOT EXISTS (
                    SELECT 1 FROM json_each(t.dependencies) AS dep
                    WHERE (SELECT status FROM tasks WHERE id = dep.value) != 'completed'
                )
              )
        "#;
        let mut stmt = self.db.prepare(sql)?;
        let rows = stmt.query_map(params![], row_to_task)?;
        rows.collect()
    }

    // ─── Worker operations ────────────────────────────────────────────

    pub fn register_worker(&self, name: String, capabilities: Vec<String>) -> Result<Worker, rusqlite::Error> {
        let worker = Worker {
            id: Uuid::new_v4().to_string(),
            name: name.clone(),
            ssh_key: None,
            capabilities,
            status: WorkerStatus::Online,
        };

        let caps_json = serde_json::to_string(&worker.capabilities)
            .map_err(|e| rusqlite::Error::ExecuteReturnedResults)?;
        self.db.execute(
            "INSERT OR REPLACE INTO workers (id, name, ssh_key, capabilities, status) VALUES (?, ?, ?, ?, ?)",
            params![
                worker.id,
                worker.name,
                worker.ssh_key,
                caps_json,
                worker.status.as_str()
            ],
        )?;

        Ok(worker)
    }

    pub fn get_worker(&self, id: &str) -> Result<Option<Worker>, rusqlite::Error> {
        self.db
            .query_row(
                "SELECT id, name, ssh_key, capabilities, status FROM workers WHERE id = ?",
                params![id],
                row_to_worker,
            )
            .optional()
    }

    pub fn get_worker_by_name(&self, name: &str) -> Result<Option<Worker>, rusqlite::Error> {
        self.db
            .query_row(
                "SELECT id, name, ssh_key, capabilities, status FROM workers WHERE name = ?",
                params![name],
                row_to_worker,
            )
            .optional()
    }

    pub fn list_workers(&self) -> Result<Vec<Worker>, rusqlite::Error> {
        let mut stmt = self.db.prepare("SELECT id, name, ssh_key, capabilities, status FROM workers")?;
        let rows = stmt.query_map(params![], row_to_worker)?;
        rows.collect()
    }

    pub fn update_worker_status(&self, id: &str, status: WorkerStatus) -> Result<(), rusqlite::Error> {
        self.db.execute(
            "UPDATE workers SET status = ? WHERE id = ?",
            params![status.as_str(), id],
        )?;
        Ok(())
    }

    pub fn heartbeat(&self, worker_id: &str) -> Result<(), rusqlite::Error> {
        self.db.execute(
            "UPDATE workers SET status = 'online' WHERE id = ?",
            params![worker_id],
        )?;
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
    fn create_and_get_goal() {
        let (store, _dir) = test_store();
        let goal = store.create_goal("Test goal".to_string()).unwrap();
        let fetched = store.get_goal(&goal.id).unwrap().unwrap();
        assert_eq!(fetched.description, "Test goal");
        assert_eq!(fetched.status, GoalStatus::Decomposed);
    }

    #[test]
    fn worker_registration() {
        let (store, _dir) = test_store();
        let worker = store.register_worker("ftw3".to_string(), vec!["gpu:nvidia:3070ti".to_string()]).unwrap();
        let fetched = store.get_worker(&worker.id).unwrap().unwrap();
        assert_eq!(fetched.name, "ftw3");
        assert_eq!(fetched.capabilities[0], "gpu:nvidia:3070ti");
    }

    #[test]
    fn task_lifecycle() {
        let (store, _dir) = test_store();
        
        // Create a goal first (task depends on it via FK constraint)
        let goal = store.create_goal("Test".to_string()).unwrap();
        
        // Register a fake worker so assign_task can reference it
        let worker = store.register_worker("worker-1".to_string(), vec![]).unwrap();
        
        let task = store.create_task(&goal.id, "echo hello".to_string()).unwrap();

        assert_eq!(task.status, TaskStatus::Pending);

        // Assign using the worker's UUID (FK constraint)
        store.assign_task(&task.id, &worker.id).unwrap();
        let assigned = store.get_task(&task.id).unwrap().unwrap();
        assert_eq!(assigned.status, TaskStatus::Assigned);

        store.complete_task(&task.id, Some("done".to_string())).unwrap();
        let completed = store.get_task(&task.id).unwrap().unwrap();
        assert_eq!(completed.status, TaskStatus::Completed);
    }

    #[test]
    fn persistence_across_instances() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("persist.db");

        // Write to store
        {
            let store = StateStore::open(&path).unwrap();
            store.create_goal("Persistent".to_string()).unwrap();
        }

        // Re-open and verify
        {
            let store = StateStore::open(&path).unwrap();
            let goals = store.list_goals().unwrap();
            assert_eq!(goals.len(), 1);
            assert_eq!(goals[0].description, "Persistent");
        }

        dir.close().unwrap();
    }
}