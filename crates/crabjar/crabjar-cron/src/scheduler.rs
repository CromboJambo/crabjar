//! Cron job scheduler — polls for due jobs and submits them to conductor.

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::time::{Duration, sleep};

use crate::store::CronStore;

/// Scheduler that periodically checks for due cron jobs.
pub struct CronScheduler {
    store: Arc<CronStore>,
    conductor_url: String,
    interval: Duration,
}

impl CronScheduler {
    /// Create a new scheduler.
    pub fn new(store: Arc<CronStore>, conductor_url: &str) -> Self {
        CronScheduler {
            store,
            conductor_url: conductor_url.to_string(),
            interval: Duration::from_mins(1),
        }
    }

    /// Run the scheduler loop.
    pub async fn run(&self) -> Result<()> {
        loop {
            self.tick().await?;
            sleep(self.interval).await;
        }
    }

    /// Single scheduler tick — find due jobs and execute them.
    async fn tick(&self) -> Result<()> {
        let jobs = self.store.list_jobs()?;
        let now = Utc::now();

        for job in jobs {
            if job.status.to_string() != "scheduled" {
                continue;
            }

            // Check if job is due (simplified: run every tick for now)
            // In production, parse the schedule expression to determine next_run_at
            if self.is_due(&job, &now) {
                self.execute_job(&job).await?;
            }
        }

        Ok(())
    }

    /// Check if a job is due to run.
    fn is_due(&self, job: &crate::types::CronJob, now: &DateTime<Utc>) -> bool {
        // Simplified: if no next_run_at or next_run_at is in the past, it's due
        match &job.next_run_at {
            Some(next) => *now >= *next,
            None => true,
        }
    }

    /// Execute a job by submitting it to conductor.
    async fn execute_job(&self, job: &crate::types::CronJob) -> Result<()> {
        println!(
            "[cron] Executing job {} ({})",
            job.id,
            job.name.as_deref().unwrap_or("unnamed")
        );

        // Submit goal to conductor via HTTP API
        let payload = serde_json::json!({
            "prompt": job.prompt.clone(),
            "skills": &job.skills,
            "name": format!("cron:{}", job.name.as_deref().unwrap_or(&job.id)),
        });

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/goals", self.conductor_url))
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            self.store.mark_completed(&job.id)?;
        } else {
            self.store.mark_failed(&job.id)?;
        }

        Ok(())
    }
}
