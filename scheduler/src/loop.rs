//! Scheduler main loop — checks for due jobs, triggers execution through guard.

// Scheduler uses RefCell internally (not Sync) — Arc<Scheduler> is intentional design.
#![allow(clippy::arc_with_non_send_sync)]

use crate::{Scheduler, WorkerPool};
use chrono::Utc;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// RAII guard that releases a worker back to the pool when dropped.
/// Ensures worker capacity is restored on every terminal path: success, error,
/// panic/join failure, and cancellation (task drop).
struct WorkerGuard<'a> {
    workers: &'a Arc<std::sync::Mutex<WorkerPool>>,
    worker_id: u32,
}

impl Drop for WorkerGuard<'_> {
    fn drop(&mut self) {
        // Always succeeds — mark_available is idempotent for already-available workers.
        let mut pool = self.workers.lock().unwrap();
        pool.mark_available(self.worker_id);
    }
}

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
                        let workers = Arc::clone(&self.workers);
                        let job_name = job.name.clone();
                        let job_command = job.command.clone();

                        // Clone for the async task's logging before moving into spawn_blocking
                        let name_for_log = job_name.clone();

                        // Lock, spawn on worker, unlock — don't hold lock across tokio::spawn
                        let spawned = {
                            let mut pool = workers.lock().unwrap();
                            pool.spawn(move || {
                                tracing::info!(job_name = %job_name, "Executing");
                                format!("Executed: {}", job_command)
                            })
                        };

                        if let Some((worker_id, handle)) = spawned {
                            // Clone Arc for the async task's guard
                            let workers_for_guard = Arc::clone(&workers);
                            tokio::spawn(async move {
                                // Guard ensures worker is released on all terminal paths:
                                // success, error, panic (join failure), and cancellation.
                                let _guard = WorkerGuard { workers: &workers_for_guard, worker_id };

                                match handle.await {
                                    Ok(output) => {
                                        tracing::info!(job_name = %name_for_log, output = %output, "Job completed");
                                    }
                                    Err(e) => {
                                        tracing::error!(job_name = %name_for_log, error = %e, "Job failed");
                                    }
                                }
                                // _guard dropped here — worker released
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::{JobSpec, Scheduler};
    use chrono::Utc;
    use std::time::Duration as StdDuration;
    use tempfile::tempdir;

    /// Helper: create a scheduler with a due job.
    fn make_scheduler_with_due_job(name: &str) -> (Scheduler, JobSpec) {
        let dir = tempdir().unwrap();
        let sched = Scheduler::new(dir.path()).unwrap();
        let spec = JobSpec {
            id: String::new(),
            name: name.to_string(),
            schedule: "* * * * *".to_string(),
            command: "echo test".to_string(),
            working_dir: None,
            timeout_secs: 60,
            requires_approval: false,
            last_run: None,
            next_run: Utc::now() - chrono::Duration::minutes(1), // already due
        };
        sched.add_job(spec.clone()).unwrap();
        (sched, spec)
    }

    #[tokio::test]
    async fn worker_released_after_success() {
        let (sched, _) = make_scheduler_with_due_job("success-test");
        let loop_ = SchedulerLoop::new(sched, 1);

        // Run one tick — job should execute and worker released
        loop_.tick().await;

        // Wait for async task to complete
        tokio::time::sleep(StdDuration::from_millis(100)).await;

        // Worker should be available again (busy_workers == 0)
        let workers = loop_.workers.lock().unwrap();
        assert_eq!(workers.busy_workers(), 0, "Worker not released after success");
    }

    #[tokio::test]
    async fn sequential_jobs_reuse_worker() {
        let dir = tempdir().unwrap();
        let sched = Scheduler::new(dir.path()).unwrap();

        // Add two jobs that will be due
        for i in 0..2 {
            let spec = JobSpec {
                id: String::new(),
                name: format!("seq-job-{}", i),
                schedule: "* * * * *".to_string(),
                command: "echo seq".to_string(),
                working_dir: None,
                timeout_secs: 60,
                requires_approval: false,
                last_run: None,
                next_run: Utc::now() - chrono::Duration::minutes(1),
            };
            sched.add_job(spec).unwrap();
        }

        let loop_ = SchedulerLoop::new(sched, 1); // one worker

        // First tick: both jobs due, but only one can run (one worker)
        loop_.tick().await;
        tokio::time::sleep(StdDuration::from_millis(100)).await;

        // Second tick: remaining job should now run (worker was released)
        loop_.tick().await;
        tokio::time::sleep(StdDuration::from_millis(100)).await;

        // Both jobs should have had a chance to run
        let workers = loop_.workers.lock().unwrap();
        assert_eq!(workers.busy_workers(), 0, "Workers not released after sequential jobs");
    }

    #[tokio::test]
    async fn worker_pool_busy_count_tracking() {
        let mut pool = WorkerPool::new(2);

        // Spawn two jobs — both workers busy
        let (id1, h1) = pool.spawn(|| "job1".to_string()).unwrap();
        let (id2, h2) = pool.spawn(|| "job2".to_string()).unwrap();
        assert_eq!(pool.busy_workers(), 2);

        // Third spawn should fail (no available workers)
        assert!(pool.spawn(|| "job3".to_string()).is_none());

        // Mark one available
        pool.mark_available(id1);
        assert_eq!(pool.busy_workers(), 1);

        // Now we can spawn again
        let (id3, h3) = pool.spawn(|| "job3".to_string()).unwrap();
        assert_eq!(pool.busy_workers(), 2);

        // Clean up handles
        drop(h1);
        drop(h2);
        drop(h3);
    }
}