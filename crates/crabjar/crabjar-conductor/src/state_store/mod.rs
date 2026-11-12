//! SQLite-backed state store for conductor persistence.
//!
//! Provides durable storage for goals, tasks, workers, and decisions using embedded SQLite.
//! Schema migrations are handled automatically on open.

use std::path::Path;

// Submodules split by entity type (500 LoC rule)
pub mod decisions;
pub mod deployments;
pub mod goals;
pub mod supervisors;
pub mod tasks;
pub mod workers;

// Re-export public types for backward compatibility
pub use decisions::{DecisionFrequency, DecisionRecord, PromotedRule};
pub use deployments::{DeploymentStatus, WorkerDeployment};
pub use goals::{Goal, GoalStatus};
pub use supervisors::SupervisorStatus;
pub use tasks::{Task, TaskStatus};
pub use workers::{Worker, WorkerStatus};

/// SQLite-backed persistent state store.
pub struct StateStore {
    db: rusqlite::Connection,
}

impl StateStore {
    /// Get a reference to the underlying database connection.
    pub fn db(&self) -> &rusqlite::Connection {
        &self.db
    }

    /// Open or create a state store at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let db = rusqlite::Connection::open(path)?;

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
                trust_level_required INTEGER NOT NULL DEFAULT 0,
                resources_json TEXT NOT NULL DEFAULT '{}'
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
            CREATE INDEX IF NOT EXISTS idx_decision_rules_criteria ON decision_rules(criteria_file);

            -- Worker deployments (SUPERVISOR-DEPLOYMENT-READINESS Phase 1)
            CREATE TABLE IF NOT EXISTS worker_deployments (
                id              TEXT PRIMARY KEY,
                supervisor_id   TEXT NOT NULL,
                task_spec       JSON NOT NULL,
                sandbox_id      TEXT,
                status          TEXT NOT NULL DEFAULT 'pending',
                outcome         JSON,
                created_at      TEXT NOT NULL,
                completed_at    TEXT,
                error           TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_deployments_supervisor ON worker_deployments(supervisor_id);
            CREATE INDEX IF NOT EXISTS idx_deployments_status ON worker_deployments(status);

            -- Supervisor readiness status (SUPERVISOR-DEPLOYMENT-READINESS Phase 2)
            CREATE TABLE IF NOT EXISTS supervisor_status (
                id              TEXT PRIMARY KEY,
                ready           BOOLEAN NOT NULL DEFAULT false,
                health          JSON,
                last_heartbeat  TEXT,
                active_deployments INTEGER NOT NULL DEFAULT 0
            );",
        )?;

        // Migration: add resources_json column to existing tasks tables (idempotent)
        let _ = db.execute(
            "ALTER TABLE tasks ADD COLUMN resources_json TEXT NOT NULL DEFAULT '{}'",
            [],
        );

        Ok(Self { db })
    }
}
