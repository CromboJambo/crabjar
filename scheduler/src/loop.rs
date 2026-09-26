//! Scheduler main loop — checks for due jobs, triggers execution through guard.

use crate::{Scheduler, WorkerPool};
use chrono::Utc;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

pub struct SchedulerLoop {
    scheduler: Arc<Scheduler>,
    workers: Arc<std::sync::Mutex<WorkerPool>>,
}

impl SchedulerLoop {
    pub fn new(scheduler: Scheduler, worker_count: usize) -> Self {
        Self {
            scheduler: Arc::new(scheduler),
            workers: Arc::new(std::sync::Mutex::new(WorkerPool::new(worker_count))),
        }
    }

    /// Main loop: check for due jobs every interval.
    pub async fn run(self, tick_secs: u64) {
        loop {
            self.tick().await;
            sleep(Duration::from_secs(tick_secs)).await;
        }
    }

    /// Single evaluation cycle.
    async fn tick(&self) {
        let now = Utc::now();

        match self.scheduler.list_jobs() {
            Ok(jobs) => {
                for job in jobs {
                    if job.next_run <= now {
                        tracing::info!(job_name = %job.name, "Job due");

                        // Check guard approval if required
                        if job.requires_approval {
                            tracing::info!(job_name = %job.name, "Requires approval — submitting to guard");
                            // Submit to crabjar-guard pipeline here
                            continue;
                        }

                        // Execute via worker pool
                        let scheduler = Arc::clone(&self.scheduler);
                        let workers = Arc::clone(&self.workers);
                        let job_name = job.name.clone();
                        let job_command = job.command.clone();

                        if let Some(handle) = workers.lock().unwrap().spawn(move || {
                            tracing::info!(job_name = %job_name, "Executing");
                            // Execute command...
                            format!("Executed: {}", job_command)
                        }) {
                            tokio::spawn(async move {
                                match handle.await {
                                    Ok(output) => {
                                        tracing::info!(job_name = %job_name, output = %output, "Job completed");
                                        let _ = scheduler.update_last_run(&job_name, Utc::now());
                                    }
                                    Err(e) => {
                                        tracing::error!(job_name = %job_name, error = %e, "Job failed");
                                    }
                                }
                            });
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to list jobs");
            }
        }
    }
}
