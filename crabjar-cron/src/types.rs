//! Cron job types and models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Job status values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    /// Job is scheduled and will run at the next tick.
    Scheduled,
    /// Job is currently running.
    Running,
    /// Job completed successfully.
    Completed,
    /// Job failed to execute.
    Failed,
    /// Job is paused (won't run until resumed).
    Paused,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Scheduled => write!(f, "scheduled"),
            JobStatus::Running => write!(f, "running"),
            JobStatus::Completed => write!(f, "completed"),
            JobStatus::Failed => write!(f, "failed"),
            JobStatus::Paused => write!(f, "paused"),
        }
    }
}

/// A scheduled cron job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    /// Unique identifier for the job.
    pub id: String,
    /// Human-friendly name (optional).
    pub name: Option<String>,
    /// Schedule expression (natural language or cron syntax).
    pub schedule: String,
    /// Self-contained prompt for the agent (required for agent-driven jobs).
    pub prompt: Option<String>,
    /// Ordered skill names to load before the cron prompt.
    pub skills: Vec<String>,
    /// Script path to run each tick (optional; required for no_agent jobs).
    pub script: Option<String>,
    /// Delivery target for job output.
    pub deliver: Option<String>,
    /// If true, run script only without LLM agent.
    pub no_agent: bool,
    /// Job status.
    pub status: JobStatus,
    /// When the job last ran (UTC).
    pub last_run_at: Option<DateTime<Utc>>,
    /// When the job is next scheduled to run (UTC).
    pub next_run_at: Option<DateTime<Utc>>,
    /// Number of consecutive failures.
    pub failure_count: i32,
    /// Reason for pausing (if paused).
    pub paused_reason: Option<String>,
    /// Created at timestamp (UTC).
    pub created_at: DateTime<Utc>,
}

/// Request to create a new cron job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJobRequest {
    pub schedule: String,
    pub prompt: Option<String>,
    pub name: Option<String>,
    pub skills: Vec<String>,
    pub script: Option<String>,
    pub deliver: Option<String>,
    pub no_agent: bool,
    pub paused: bool,
    pub paused_reason: Option<String>,
}

/// Request to update an existing cron job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateJobRequest {
    pub schedule: Option<String>,
    pub prompt: Option<String>,
    pub name: Option<String>,
    pub skills: Option<Vec<String>>,
    pub script: Option<String>,
    pub deliver: Option<String>,
    pub no_agent: Option<bool>,
}

/// Response from listing cron jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListJobsResponse {
    pub jobs: Vec<CronJob>,
}

/// Response from a single job operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobOperationResponse {
    pub success: bool,
    pub job_id: String,
    pub message: String,
}