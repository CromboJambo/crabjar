//! SQLite-backed state store for conductor persistence.
//!
//! Provides durable storage for goals, tasks, and workers using embedded SQLite.
//! Schema migrations are handled automatically on open.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

use crate::auth;

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
    let required_caps_str: String = row.get(7).unwrap_or_else(|_| "[]".to_string());
    let trust_required: u8 = row.get(8).unwrap_or(0);

    Ok(Task {
        id: row.get(0)?,
        goal_id: row.get(1)?,
        worker_id: row.get(2)?,
        command: row.get(3)?,
        dependencies: serde_json::from_str(&deps_str)
            .map_err(|e| rusqlite::Error::ExecuteReturnedResults)?,
        status: TaskStatus::from_str(&row.get::<_, String>(5)?),
        result: row.get(6)?,
        required_capabilities: serde_json::from_str(&required_caps_str)
            .map_err(|e| rusqlite::Error::ExecuteReturnedResults)?,
        trust_level_required: trust_required,
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
        trust_level: row.get(5).unwrap_or(0u8),
        successful_tasks: row.get(6).unwrap_or(0i32),
        failed_tasks: row.get(7).unwrap_or(0i32),
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
                result TEXT,
                required_capabilities TEXT NOT NULL DEFAULT '[]',
                trust_level_required INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS workers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                ssh_key TEXT,
                capabilities TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'online',
                trust_level INTEGER NOT NULL DEFAULT 0,
                successful_tasks INTEGER NOT NULL DEFAULT 0,
                failed_tasks INTEGER NOT NULL DEFAULT 0,
                last_heartbeat TEXT NOT NULL DEFAULT '',
                auth_token TEXT UNIQUE,
                token_expires_at TEXT
            );

            CREATE TABLE IF NOT EXISTS decisions (
                id TEXT PRIMARY KEY,
                observation_hash TEXT NOT NULL,
                criteria_file TEXT NOT NULL,
                chosen_option TEXT NOT NULL,
                confidence REAL NOT NULL,
                cache_hit INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_decisions_criteria ON decisions(criteria_file);
            CREATE INDEX IF NOT EXISTS idx_decisions_hash ON decisions(observation_hash);

            -- Promoted decision rules (JEV Phase 1 Milestone 3)
            CREATE TABLE IF NOT EXISTS decision_rules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pattern_id TEXT NOT NULL,
                criteria_file TEXT NOT NULL,
                recommended_option TEXT NOT NULL,
                avg_confidence REAL NOT NULL,
                occurrence_count INTEGER NOT NULL,
                promoted_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_decision_rules_criteria ON decision_rules(criteria_file);",
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
        let mut stmt = self
            .db
            .prepare("SELECT id, description, status, created_at FROM goals")?;
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
            required_capabilities: vec![],
            trust_level_required: 0,
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

        serde_json::from_str(&caps_str).map_err(|e| rusqlite::Error::ExecuteReturnedResults)
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
            serde_json::to_string(caps).map_err(|e| rusqlite::Error::ExecuteReturnedResults)?;
        self.db.execute(
            "UPDATE tasks SET required_capabilities = ? WHERE id = ?",
            params![caps_json, task_id],
        )?;
        Ok(())
    }

    /// Get pending tasks that have no dependencies or whose dependencies are all completed.
    pub fn get_ready_tasks(&self) -> Result<Vec<Task>, rusqlite::Error> {
        let sql = r#"
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
        "#;
        let mut stmt = self.db.prepare(sql)?;
        let rows = stmt.query_map(params![], row_to_task)?;
        rows.collect()
    }

    // ─── Worker operations ────────────────────────────────────────────

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
            .map_err(|e| rusqlite::Error::ExecuteReturnedResults)?;
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

            match DateTime::parse_from_rfc3339(&last_hb) {
                Ok(last_time) => {
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
                }
                Err(_) => {
                    // Invalid timestamp, skip decay
                }
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
        if total > 10 && (failures as f64 / total as f64) > 0.5 {
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

    // ─── Decision history operations ──────────────────────────────────

    /// Log a JEV decision to the history store.
    pub fn log_decision(
        &self,
        observation_hash: &str,
        criteria_file: &str,
        chosen_option: &str,
        confidence: f64,
        cache_hit: bool,
    ) -> Result<(), rusqlite::Error> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();

        self.db.execute(
            "INSERT INTO decisions (id, observation_hash, criteria_file, chosen_option, confidence, cache_hit, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                id,
                observation_hash,
                criteria_file,
                chosen_option,
                confidence,
                if cache_hit { 1 } else { 0 },
                created_at
            ],
        )?;

        Ok(())
    }

    /// Get decision history for a specific criteria file.
    pub fn get_decisions_for_criteria(
        &self,
        criteria_file: &str,
        limit: i32,
    ) -> Result<Vec<DecisionRecord>, rusqlite::Error> {
        let mut stmt = self.db.prepare(
            "SELECT id, observation_hash, chosen_option, confidence, cache_hit, created_at FROM decisions WHERE criteria_file = ? ORDER BY created_at DESC LIMIT ?",
        )?;
        let rows = stmt.query_map(params![criteria_file, limit], |row| {
            Ok(DecisionRecord {
                id: row.get(0)?,
                observation_hash: row.get(1)?,
                chosen_option: row.get(2)?,
                confidence: row.get(3)?,
                cache_hit: row.get::<_, i32>(4)? == 1,
                created_at: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    /// Get decision frequency statistics for rule extraction.
    /// Returns option counts grouped by observation hash pattern for a criteria file.
    pub fn get_decision_frequencies(
        &self,
        criteria_file: &str,
    ) -> Result<Vec<DecisionFrequency>, rusqlite::Error> {
        let sql = r#"
            SELECT chosen_option, COUNT(*) as count, AVG(confidence) as avg_confidence
            FROM decisions
            WHERE criteria_file = ?
            GROUP BY chosen_option
            ORDER BY count DESC
        "#;
        let mut stmt = self.db.prepare(sql)?;
        let rows = stmt.query_map(params![criteria_file], |row| {
            Ok(DecisionFrequency {
                option: row.get(0)?,
                count: row.get(1)?,
                avg_confidence: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    /// Get total decision count for a criteria file.
    pub fn get_decision_count(&self, criteria_file: &str) -> Result<i64, rusqlite::Error> {
        let row = self.db.query_row(
            "SELECT COUNT(*) FROM decisions WHERE criteria_file = ?",
            params![criteria_file],
            |r| r.get::<_, i64>(0),
        )?;
        Ok(row)
    }

    // ─── Promoted rule operations ─────────────────────────────────────

    /// Store a promoted decision rule.
    pub fn promote_rule(
        &self,
        pattern_id: &str,
        criteria_file: &str,
        recommended_option: &str,
        avg_confidence: f64,
        occurrence_count: i64,
    ) -> Result<i64, rusqlite::Error> {
        let promoted_at = Utc::now().to_rfc3339();
        let result = self.db.execute(
            "INSERT INTO decision_rules (pattern_id, criteria_file, recommended_option, avg_confidence, occurrence_count, promoted_at) VALUES (?, ?, ?, ?, ?, ?)",
            params![pattern_id, criteria_file, recommended_option, avg_confidence, occurrence_count, promoted_at],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    /// Get all promoted rules for a specific criteria file.
    pub fn get_promoted_rules(
        &self,
        criteria_file: &str,
    ) -> Result<Vec<PromotedRule>, rusqlite::Error> {
        let mut stmt = self.db.prepare(
            "SELECT id, pattern_id, recommended_option, avg_confidence, occurrence_count, promoted_at FROM decision_rules WHERE criteria_file = ? ORDER BY promoted_at DESC",
        )?;
        let rows = stmt.query_map(params![criteria_file], |row| {
            Ok(PromotedRule {
                id: row.get(0)?,
                pattern_id: row.get(1)?,
                recommended_option: row.get(2)?,
                avg_confidence: row.get(3)?,
                occurrence_count: row.get(4)?,
                promoted_at: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    /// Check if a rule exists for the given pattern and criteria.
    pub fn has_rule(&self, pattern_id: &str, criteria_file: &str) -> Result<bool, rusqlite::Error> {
        let row = self.db.query_row(
            "SELECT COUNT(*) FROM decision_rules WHERE pattern_id = ? AND criteria_file = ?",
            params![pattern_id, criteria_file],
            |r| r.get::<_, i64>(0),
        )?;
        Ok(row > 0)
    }

    /// Get the recommended option for a given pattern and criteria.
    pub fn get_rule_option(
        &self,
        pattern_id: &str,
        criteria_file: &str,
    ) -> Result<Option<String>, rusqlite::Error> {
        self.db.query_row(
            "SELECT recommended_option FROM decision_rules WHERE pattern_id = ? AND criteria_file = ?",
            params![pattern_id, criteria_file],
            |r| r.get::<_, String>(0),
        ).optional()
    }
}

/// A promoted decision rule stored in the database.
#[derive(Debug, Clone)]
pub struct PromotedRule {
    pub id: i64,
    pub pattern_id: String,
    pub recommended_option: String,
    pub avg_confidence: f64,
    pub occurrence_count: i64,
    pub promoted_at: String,
}

/// A single decision record from history.
#[derive(Debug, Clone)]
pub struct DecisionRecord {
    pub id: String,
    pub observation_hash: String,
    pub chosen_option: String,
    pub confidence: f64,
    pub cache_hit: bool,
    pub created_at: String,
}

/// Decision frequency statistics for rule extraction.
#[derive(Debug, Clone)]
pub struct DecisionFrequency {
    pub option: String,
    pub count: i64,
    pub avg_confidence: f64,
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
    fn task_lifecycle() {
        let (store, _dir) = test_store();
        unsafe { std::env::set_var("CRABJAR_AUTH_SECRET", "test-secret") };

        // Create a goal first (task depends on it via FK constraint)
        let goal = store.create_goal("Test".to_string()).unwrap();

        // Register a fake worker so assign_task can reference it
        let (worker, _token) = store
            .register_worker("worker-1".to_string(), vec![], None)
            .unwrap();

        let task = store
            .create_task(&goal.id, "echo hello".to_string())
            .unwrap();

        assert_eq!(task.status, TaskStatus::Pending);

        // Assign using the worker's UUID (FK constraint)
        store.assign_task(&task.id, &worker.id).unwrap();
        let assigned = store.get_task(&task.id).unwrap().unwrap();
        assert_eq!(assigned.status, TaskStatus::Assigned);

        store
            .complete_task(&task.id, Some("done".to_string()))
            .unwrap();
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
