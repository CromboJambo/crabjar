//! Core scheduler — manages job lifecycle, persistence, and execution gating.
//!
//! Jobs are stored in SQLite for durability across restarts. Execution is gated
//! through crabjar-guard: every job must be approved before running.

use crate::error::SchedulerError;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Job specification with cron schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    pub id: String,
    pub name: String,
    /// Unix cron expression (5 fields)
    pub schedule: String,
    /// Command to execute
    pub command: String,
    /// Working directory for execution
    pub working_dir: Option<PathBuf>,
    /// Timeout in seconds (default: 3600)
    pub timeout_secs: u64,
    /// Require guard approval before execution?
    pub requires_approval: bool,
    /// Last run timestamp
    pub last_run: Option<DateTime<Utc>>,
    /// Next scheduled run
    pub next_run: DateTime<Utc>,
}

/// Scheduler manages job storage and lifecycle.
pub struct Scheduler {
    db: Connection,
    data_dir: PathBuf,
}

impl Scheduler {
    /// Create or open scheduler with SQLite persistence.
    pub fn new(data_dir: &Path) -> Result<Self, SchedulerError> {
        std::fs::create_dir_all(data_dir)?;
        let db_path = data_dir.join("scheduler.db");

        let mut flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE;
        if !db_path.exists() {
            flags |= OpenFlags::SQLITE_OPEN_CREATE;
        }

        let conn = Connection::open_with_flags(&db_path, flags)?;
        Self::init_schema(&conn)?;

        Ok(Self {
            db: conn,
            data_dir: data_dir.to_path_buf(),
        })
    }

    fn init_schema(conn: &Connection) -> Result<(), SchedulerError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                schedule TEXT NOT NULL,
                command TEXT NOT NULL,
                working_dir TEXT,
                timeout_secs INTEGER DEFAULT 3600,
                requires_approval INTEGER DEFAULT 1,
                last_run TEXT,
                next_run TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now'))
            );",
        )?;
        Ok(())
    }

    /// Register a new job.
    pub fn add_job(&self, spec: JobSpec) -> Result<(), SchedulerError> {
        let id = Uuid::new_v4().to_string();
        self.db.execute(
            "INSERT INTO jobs (id, name, schedule, command, working_dir, timeout_secs, requires_approval, next_run)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (&id, &spec.name, &spec.schedule, &spec.command,
             spec.working_dir.as_ref().map(|p| p.to_string_lossy()),
             spec.timeout_secs as i64,
             spec.requires_approval as i32,
             spec.next_run.to_rfc3339()),
        )?;
        Ok(())
    }

    /// List all registered jobs.
    pub fn list_jobs(&self) -> Result<Vec<JobSpec>, SchedulerError> {
        let mut stmt = self.db.prepare("SELECT * FROM jobs ORDER BY next_run")?;
        let rows = stmt.query_map([], |row| {
            Ok(JobSpec {
                id: row.get(0)?,
                name: row.get(1)?,
                schedule: row.get(2)?,
                command: row.get(3)?,
                working_dir: row.get::<_, Option<String>>(4)?.map(PathBuf::from),
                timeout_secs: row.get(5).unwrap_or(3600) as u64,
                requires_approval: row.get(6).unwrap_or(1) != 0,
                last_run: row.get(7).ok().flatten().and_then(|s: String| {
                    DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
                }),
                next_run: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?).unwrap()
                    .with_timezone(&Utc),
            })
        })?;

        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(row?);
        }
        Ok(jobs)
    }

    /// Get job by name.
    pub fn get_job(&self, name: &str) -> Result<Option<JobSpec>, SchedulerError> {
        let mut stmt = self.db.prepare("SELECT * FROM jobs WHERE name = ? LIMIT 1")?;
        let mut rows = stmt.query_map([name], |row| {
            Ok(JobSpec {
                id: row.get(0)?,
                name: row.get(1)?,
                schedule: row.get(2)?,
                command: row.get(3)?,
                working_dir: row.get::<_, Option<String>>(4)?.map(PathBuf::from),
                timeout_secs: row.get(5).unwrap_or(3600) as u64,
                requires_approval: row.get(6).unwrap_or(1) != 0,
                last_run: row.get(7).ok().flatten().and_then(|s: String| {
                    DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
                }),
                next_run: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?).unwrap()
                    .with_timezone(&Utc),
            })
        })?;

        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    /// Remove a job by name.
    pub fn remove_job(&self, name: &str) -> Result<(), SchedulerError> {
        self.db.execute("DELETE FROM jobs WHERE name = ?", [name])?;
        Ok(())
    }

    /// Update last run timestamp for a job.
    pub fn update_last_run(&self, name: &str, time: DateTime<Utc>) -> Result<(), SchedulerError> {
        self.db.execute(
            "UPDATE jobs SET last_run = ? WHERE name = ?",
            (time.to_rfc3339(), name),
        )?;
        Ok(())
    }

    /// Get data directory path.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_scheduler_create_and_list() {
        let dir = tempdir().unwrap();
        let scheduler = Scheduler::new(dir.path()).unwrap();

        assert_eq!(scheduler.list_jobs().unwrap().len(), 0);

        let spec = JobSpec {
            id: String::new(),
            name: "test-job".to_string(),
            schedule: "* * * * *".to_string(),
            command: "echo hello".to_string(),
            working_dir: None,
            timeout_secs: 60,
            requires_approval: false,
            last_run: None,
            next_run: Utc::now(),
        };

        scheduler.add_job(spec).unwrap();
        let jobs = scheduler.list_jobs().unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].name, "test-job");
    }

    #[test]
    fn test_scheduler_persistence() {
        let dir = tempdir().unwrap();

        // Create and add job
        {
            let scheduler = Scheduler::new(dir.path()).unwrap();
            let spec = JobSpec {
                id: String::new(),
                name: "persistent-job".to_string(),
                schedule: "*/5 * * * *".to_string(),
                command: "echo persistent".to_string(),
                working_dir: None,
                timeout_secs: 60,
                requires_approval: false,
                last_run: None,
                next_run: Utc::now(),
            };
            scheduler.add_job(spec).unwrap();
        }

        // Reopen and verify job persists
        {
            let scheduler = Scheduler::new(dir.path()).unwrap();
            let jobs = scheduler.list_jobs().unwrap();
            assert_eq!(jobs.len(), 1);
            assert_eq!(jobs[0].name, "persistent-job");
        }
    }

    #[test]
    fn test_scheduler_remove_job() {
        let dir = tempdir().unwrap();
        let scheduler = Scheduler::new(dir.path()).unwrap();

        let spec = JobSpec {
            id: String::new(),
            name: "removable-job".to_string(),
            schedule: "* * * * *".to_string(),
            command: "echo removable".to_string(),
            working_dir: None,
            timeout_secs: 60,
            requires_approval: false,
            last_run: None,
            next_run: Utc::now(),
        };

        scheduler.add_job(spec).unwrap();
        assert_eq!(scheduler.list_jobs().unwrap().len(), 1);

        scheduler.remove_job("removable-job").unwrap();
        assert_eq!(scheduler.list_jobs().unwrap().len(), 0);
    }
}
