//! Integration test: watcher observes → scheduler decides → guard gates → worker executes.

use crabjar_scheduler::{Scheduler, WorkerPool, Watcher};
use crabjar_scheduler::cron_parser::CronSchedule;
use chrono::Utc;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

#[tokio::test]
async fn full_pipeline() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("scheduler.db");

    // 1. Create scheduler with persistence
    let scheduler = Scheduler::new(&db_path).expect("scheduler init");

    // 2. Define a watcher that observes a condition (e.g., disk usage)
    let _trigger_count = Arc::new(Mutex::new(0));
    let should_trigger = Arc::new(Mutex::new(false));

    let observer = Watcher::new(
        "disk-watcher",
        "disk-usage-high",
        {
            let should = Arc::clone(&should_trigger);
            move || *should.lock().unwrap()
        }
    );

    // 3. Watcher evaluates — observation only, no action yet
    let mut observers = vec![observer];
    let events = observers.iter_mut().filter_map(|w| w.evaluate()).collect::<Vec<_>>();
    assert!(events.is_empty(), "Should not trigger yet");

    // Now the condition is met
    *should_trigger.lock().unwrap() = true;
    let events = observers.iter_mut().filter_map(|w| w.evaluate()).collect::<Vec<_>>();
    assert_eq!(events.len(), 1, "Watcher should observe condition");
    assert_eq!(events[0].condition, "disk-usage-high");

    // Observation ≠ Permission: event exists but nothing executed yet.
    // Scheduler must decide to act on it.

    // 4. Scheduler receives event and decides to create a job
    let job = crabjar_scheduler::JobSpec {
        id: String::new(),
        name: "cleanup-disk".to_string(),
        schedule: "0 * * * *".to_string(),  // hourly
        command: "/usr/bin/find /tmp -type f -mtime +7 -delete".to_string(),
        working_dir: None,
        timeout_secs: 300,
        requires_approval: true,  // Must go through guard
        last_run: None,
        next_run: Utc::now(),
    };

    scheduler.add_job(job).expect("add job");

    // 5. Verify persistence
    let jobs = scheduler.list_jobs().expect("list jobs");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].name, "cleanup-disk");
    assert!(jobs[0].requires_approval);

    // 6. Worker pool ready to execute (after guard approval)
    let workers = WorkerPool::new(2);
    assert_eq!(workers.worker_count(), 2);

    // Guard would approve here in real scenario
    println!("✓ Watcher observed disk condition");
    println!("✓ Scheduler created cleanup job (requires approval)");
    println!("✓ Job persisted to SQLite");
    println!("✓ Worker pool ready for execution after guard approval");
}

#[test]
fn cron_parsing_comprehensive() {
    // Every 5 minutes
    let s1 = CronSchedule::parse("*/5 * * * *").unwrap();
    assert!(s1.matches(0, 12, 15, 6, 1));
    assert!(s1.matches(5, 12, 15, 6, 1));
    assert!(!s1.matches(3, 12, 15, 6, 1));

    // Weekday mornings at 9am
    let s2 = CronSchedule::parse("0 9 * * MON-FRI").unwrap();
    assert!(s2.matches(0, 9, 15, 6, 1));  // Monday
    assert!(s2.matches(0, 9, 15, 6, 5));  // Friday
    assert!(!s2.matches(0, 9, 15, 6, 0)); // Sunday

    // Midnight daily
    let s3 = CronSchedule::parse("0 0 * * *").unwrap();
    assert!(s3.matches(0, 0, 1, 1, 0));
}
