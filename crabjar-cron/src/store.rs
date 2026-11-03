//! Cron job `SQLite` storage layer.

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::Path;

use crate::types::{CreateJobRequest, CronJob, JobStatus, UpdateJobRequest};

/// Cron job store backed by `SQLite`.
pub struct CronStore {
    conn: Connection,
}

impl CronStore {
    /// Open or create the cron job database at the given path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path).context("Failed to open cron database")?;
        conn.execute_batch(Self::SCHEMA)?;
        Ok(CronStore { conn })
    }

    /// Create a new cron job.
    pub fn create_job(&self, req: &CreateJobRequest) -> Result<CronJob> {
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = Utc::now();

        self.conn.execute(
            "INSERT INTO jobs (id, name, schedule, prompt, skills, script, deliver, no_agent, status, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'scheduled', ?)",
            params![
                id,
                req.name,
                req.schedule,
                req.prompt,
                serde_json::to_string(&req.skills)?,
                req.script,
                req.deliver,
                i32::from(req.no_agent),
                created_at.to_rfc3339()
            ],
        )?;

        self.get_job(&id)
    }

    /// Get a job by ID.
    pub fn get_job(&self, id: &str) -> Result<CronJob> {
        let row = self.conn.query_row(
            "SELECT id, name, schedule, prompt, skills, script, deliver, no_agent, status, last_run_at, next_run_at, failure_count, paused_reason, created_at FROM jobs WHERE id = ?",
            params![id],
            |row| {
                let skills_str: String = row.get(4)?;
                let skills = serde_json::from_str(&skills_str)
                    .map_err(|_e| rusqlite::Error::ExecuteReturnedResults)?;

                Ok(CronJob {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    schedule: row.get(2)?,
                    prompt: row.get(3)?,
                    skills,
                    script: row.get(5)?,
                    deliver: row.get(6)?,
                    no_agent: row.get::<_, i32>(7)? == 1,
                    status: match row.get::<_, String>(8)?.as_str() {
                        "running" => JobStatus::Running,
                        "completed" => JobStatus::Completed,
                        "failed" => JobStatus::Failed,
                        "paused" => JobStatus::Paused,
                        _ => JobStatus::Scheduled,
                    },
                    last_run_at: row.get::<_, Option<String>>(9)?.map(|s| s.parse().unwrap()),
                    next_run_at: row.get::<_, Option<String>>(10)?.map(|s| s.parse().unwrap()),
                    failure_count: row.get(11)?,
                    paused_reason: row.get(12)?,
                    created_at: row.get::<_, String>(13)?.parse().unwrap(),
                })
            },
        )?;
        Ok(row)
    }

    /// List all cron jobs.
    pub fn list_jobs(&self) -> Result<Vec<CronJob>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, schedule, prompt, skills, script, deliver, no_agent, status, last_run_at, next_run_at, failure_count, paused_reason, created_at FROM jobs ORDER BY created_at DESC"
        )?;

        let jobs = stmt
            .query_map([], |row| {
                let skills_str: String = row.get(4)?;
                let skills = serde_json::from_str(&skills_str)
                    .map_err(|_e| rusqlite::Error::ExecuteReturnedResults)?;

                Ok(CronJob {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    schedule: row.get(2)?,
                    prompt: row.get(3)?,
                    skills,
                    script: row.get(5)?,
                    deliver: row.get(6)?,
                    no_agent: row.get::<_, i32>(7)? == 1,
                    status: match row.get::<_, String>(8)?.as_str() {
                        "running" => JobStatus::Running,
                        "completed" => JobStatus::Completed,
                        "failed" => JobStatus::Failed,
                        "paused" => JobStatus::Paused,
                        _ => JobStatus::Scheduled,
                    },
                    last_run_at: row.get::<_, Option<String>>(9)?.map(|s| s.parse().unwrap()),
                    next_run_at: row.get::<_, Option<String>>(10)?.map(|s| s.parse().unwrap()),
                    failure_count: row.get(11)?,
                    paused_reason: row.get(12)?,
                    created_at: row.get::<_, String>(13)?.parse().unwrap(),
                })
            })?
            .map(|r| r.map_err(anyhow::Error::from))
            .collect::<Result<Vec<_>>>()?;

        Ok(jobs)
    }

    /// Update a job's configuration.
    pub fn update_job(&self, id: &str, req: &UpdateJobRequest) -> Result<CronJob> {
        if let Some(schedule) = &req.schedule {
            self.conn.execute("UPDATE jobs SET schedule = ? WHERE id = ?", params![schedule, id])?;
        }
        if let Some(prompt) = &req.prompt {
            self.conn.execute("UPDATE jobs SET prompt = ? WHERE id = ?", params![prompt, id])?;
        }
        if let Some(name) = &req.name {
            self.conn.execute("UPDATE jobs SET name = ? WHERE id = ?", params![name, id])?;
        }
        if let Some(skills) = &req.skills {
            self.conn.execute("UPDATE jobs SET skills = ? WHERE id = ?", params![serde_json::to_string(skills)?, id])?;
        }
        if let Some(script) = &req.script {
            self.conn.execute("UPDATE jobs SET script = ? WHERE id = ?", params![script, id])?;
        }
        if let Some(deliver) = &req.deliver {
            self.conn.execute("UPDATE jobs SET deliver = ? WHERE id = ?", params![deliver, id])?;
        }
        if let Some(no_agent) = req.no_agent {
            self.conn.execute(
                "UPDATE jobs SET no_agent = ? WHERE id = ?",
                params![i32::from(no_agent), id],
            )?;
        }

        self.get_job(id)
    }

    /// Pause a job.
    pub fn pause_job(&self, id: &str, reason: Option<&str>) -> Result<()> {
        self.conn.execute(
            "UPDATE jobs SET status = 'paused', paused_reason = ? WHERE id = ?",
            params![reason, id],
        )?;
        Ok(())
    }

    /// Resume a job.
    pub fn resume_job(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE jobs SET status = 'scheduled', paused_reason = NULL WHERE id = ?",
            params![id],
        )?;
        Ok(())
    }

    /// Remove a job.
    pub fn remove_job(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM jobs WHERE id = ?", params![id])?;
        Ok(())
    }

    /// Mark a job as running.
    pub fn mark_running(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE jobs SET status = 'running', last_run_at = ? WHERE id = ?",
            params![Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    /// Mark a job as completed.
    pub fn mark_completed(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE jobs SET status = 'scheduled', failure_count = 0 WHERE id = ?",
            params![id],
        )?;
        Ok(())
    }

    /// Mark a job as failed.
    pub fn mark_failed(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE jobs SET status = 'scheduled', failure_count = failure_count + 1 WHERE id = ?",
            params![id],
        )?;
        Ok(())
    }

    /// `SQLite` schema for cron jobs.
    const SCHEMA: &str = "
        CREATE TABLE IF NOT EXISTS jobs (
            id TEXT PRIMARY KEY,
            name TEXT,
            schedule TEXT NOT NULL,
            prompt TEXT,
            skills TEXT DEFAULT '[]',
            script TEXT,
            deliver TEXT,
            no_agent INTEGER DEFAULT 0,
            status TEXT DEFAULT 'scheduled',
            last_run_at TEXT,
            next_run_at TEXT,
            failure_count INTEGER DEFAULT 0,
            paused_reason TEXT,
            created_at TEXT NOT NULL
        );
    ";
}