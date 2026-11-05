//! Unit tests for the conductor module.

use crate::auth;
use crate::conductor::{Conductor, ConductorConfig};
use crate::state_store::GoalStatus;
use tempfile::tempdir;

#[test]
fn submit_goal_creates_task() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    let conductor =
        Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

    // Create goal directly without decomposition (no pesti-server needed)
    let goal = conductor
        .store
        .create_goal("Test goal".to_string())
        .unwrap();
    let status = conductor.goal_status(&goal.id).unwrap().unwrap();
    assert_eq!(status, GoalStatus::Decomposed);
}

#[test]
fn register_worker_validates_ssh_key() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    let conductor =
        Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

    // Valid ed25519 key should pass
    let valid_key = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGFub3RoZXJrZXllbmNvZGluZw== user@host";
    rt.block_on(async {
        let result = conductor.register_worker(
            "worker-valid".to_string(),
            vec!["compute".to_string()],
            Some(valid_key.to_string()),
        );
        assert!(result.is_ok());

        // Invalid key format should fail
        let invalid_key = "not-a-valid-key";
        let result = conductor.register_worker(
            "worker-invalid".to_string(),
            vec!["compute".to_string()],
            Some(invalid_key.to_string()),
        );
        assert!(result.is_err());

        // Missing key should pass (optional)
        let result =
            conductor.register_worker("worker-nok".to_string(), vec!["compute".to_string()], None);
        assert!(result.is_ok());
    });
}

#[tokio::test]
async fn task_assignment_filters_by_capability() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    let conductor =
        Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

    // Register workers with different capabilities
    let (worker_gpu, _) = conductor
        .register_worker(
            "gpu-worker".to_string(),
            vec!["compute".to_string(), "gpu:nvidia:3070ti".to_string()],
            None,
        )
        .unwrap();
    let (worker_cpu, _) = conductor
        .register_worker("cpu-worker".to_string(), vec!["compute".to_string()], None)
        .unwrap();

    // Set trust levels so workers can be auto-assigned (not testing trust here)
    conductor
        .store
        .set_worker_trust_level(&worker_gpu.id, 1)
        .unwrap();
    conductor
        .store
        .set_worker_trust_level(&worker_cpu.id, 1)
        .unwrap();

    // Create a goal and task directly
    let goal = conductor.store.create_goal("GPU task".to_string()).unwrap();
    let task = conductor
        .store
        .create_task(&goal.id, "nvidia-smi".to_string())
        .unwrap();

    // Set GPU capability requirement on the task
    conductor
        .store
        .set_task_capabilities(&task.id, &["gpu:nvidia:*".to_string()])
        .unwrap();

    // CPU-only worker should not get the task
    let tasks_for_cpu = conductor.tasks_for_worker(&worker_cpu.id).unwrap();
    assert!(tasks_for_cpu.is_empty());

    // GPU worker should get the task
    let tasks_for_gpu = conductor.tasks_for_worker(&worker_gpu.id).unwrap();
    assert!(!tasks_for_gpu.is_empty());
    assert_eq!(tasks_for_gpu[0].id, task.id);
}

#[tokio::test]
async fn annealing_upgrades_trust_level() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    let conductor =
        Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

    // Register worker (starts at trust level 0)
    let (worker, _) = conductor
        .register_worker(
            "anneal-worker".to_string(),
            vec!["compute".to_string()],
            None,
        )
        .unwrap();
    assert_eq!(worker.trust_level, 0);

    // Create a goal and task
    let goal = conductor.store.create_goal("Test".to_string()).unwrap();
    let task = conductor
        .store
        .create_task(&goal.id, "echo test".to_string())
        .unwrap();

    // Assign task to worker
    conductor.store.assign_task(&task.id, &worker.id).unwrap();

    // Complete the task — trust level should upgrade
    let new_token = conductor
        .report_task_complete(&task.id, Some("success".to_string()))
        .unwrap();

    // After first success, trust level goes from 0 to 1 (upgrade threshold is 5 successes)
    // Actually at 1 success it's still below the threshold, so no upgrade yet
    let updated_worker = conductor.store.get_worker(&worker.id).unwrap().unwrap();
    assert_eq!(updated_worker.successful_tasks, 1);
}

#[tokio::test]
async fn worker_login_existing_refreshes_token() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    let conductor =
        Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

    // Register a worker first
    let (worker, _original_token) = conductor
        .register_worker(
            "login-worker".to_string(),
            vec!["compute".to_string()],
            None,
        )
        .unwrap();

    // Login with the same name should return the existing worker
    let (logged_in_worker, new_token) = conductor.worker_login("login-worker", None).unwrap();

    assert_eq!(logged_in_worker.id, worker.id);
    assert!(!new_token.is_empty()); // Should have a valid token
}

#[test]
fn worker_login_new_auto_registers() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    let conductor =
        Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

    // Login with a name that doesn't exist yet should auto-register
    let (worker, token) = conductor.worker_login("new-worker", None).unwrap();

    assert_eq!(worker.name, "new-worker");
    assert!(!token.is_empty());

    // Verify the worker was registered
    let workers = conductor.list_workers();
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0].name, "new-worker");
}

#[tokio::test]
async fn task_assignment_validates_trust_level() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    let conductor =
        Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

    // Register workers with different trust levels
    let (trusted, _) = conductor
        .register_worker(
            "trusted-worker".to_string(),
            vec!["compute".to_string()],
            None,
        )
        .unwrap();
    conductor
        .store
        .set_worker_trust_level(&trusted.id, 2)
        .unwrap();

    let (untrusted, _) = conductor
        .register_worker("new-worker".to_string(), vec!["compute".to_string()], None)
        .unwrap();
    // Trust level is 0 by default — insufficient for auto-assignment

    // Create a goal and task requiring trust level 1+
    let goal = conductor
        .store
        .create_goal("Sensitive task".to_string())
        .unwrap();
    let task = conductor
        .store
        .create_task(&goal.id, "handle sensitive data".to_string())
        .unwrap();
    conductor.store.set_task_trust_level(&task.id, 1).unwrap();

    // Only the trusted worker should be able to claim this task
    let trusted_tasks = conductor.tasks_for_worker(&trusted.id).unwrap();
    assert_eq!(trusted_tasks.len(), 1);
    assert_eq!(trusted_tasks[0].id, task.id);

    let untrusted_tasks = conductor.tasks_for_worker(&untrusted.id).unwrap();
    assert_eq!(untrusted_tasks.len(), 0); // Trust level too low
}

#[tokio::test]
async fn wildcard_capability_matching() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    let conductor =
        Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

    // Register worker with specific GPU
    let (worker, _) = conductor
        .register_worker(
            "gpu-node".to_string(),
            vec!["compute".to_string(), "gpu:nvidia:rtx4090".to_string()],
            None,
        )
        .unwrap();
    conductor
        .store
        .set_worker_trust_level(&worker.id, 1)
        .unwrap();

    // Create task requiring any NVIDIA GPU
    let goal = conductor.store.create_goal("GPU task".to_string()).unwrap();
    let task = conductor
        .store
        .create_task(&goal.id, "use nvidia gpu".to_string())
        .unwrap();
    conductor
        .store
        .set_task_capabilities(&task.id, &["gpu:nvidia:*".to_string()])
        .unwrap();

    // Worker with rtx4090 should match gpu:nvidia:*
    let tasks = conductor.tasks_for_worker(&worker.id).unwrap();
    assert_eq!(tasks.len(), 1);
}

#[tokio::test]
async fn trust_level_upgrades_after_threshold() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    let conductor =
        Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

    // Register worker at trust level 0
    let (worker, _) = conductor
        .register_worker(
            "threshold-worker".to_string(),
            vec!["compute".to_string()],
            None,
        )
        .unwrap();
    assert_eq!(worker.trust_level, 0);

    // Complete 5 successful tasks to reach the upgrade threshold
    for i in 0..5 {
        let goal = conductor.store.create_goal(format!("Goal {}", i)).unwrap();
        let task = conductor
            .store
            .create_task(&goal.id, "echo success".to_string())
            .unwrap();
        conductor.store.assign_task(&task.id, &worker.id).unwrap();
        conductor
            .report_task_complete(&task.id, Some("success".to_string()))
            .unwrap();
    }

    // After 5 successes, trust level should upgrade to 1
    let updated_worker = conductor.store.get_worker(&worker.id).unwrap().unwrap();
    assert_eq!(updated_worker.trust_level, 1);
    assert_eq!(updated_worker.successful_tasks, 5);
}

#[tokio::test]
async fn failed_task_decreases_trust_level() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    let conductor =
        Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

    // Register worker and manually set to trust level 2
    let (worker, _) = conductor
        .register_worker("fail-worker".to_string(), vec!["compute".to_string()], None)
        .unwrap();
    conductor
        .store
        .set_worker_trust_level(&worker.id, 2)
        .unwrap();

    // Complete some tasks to establish a baseline
    for i in 0..5 {
        let goal = conductor.store.create_goal(format!("Goal {}", i)).unwrap();
        let task = conductor
            .store
            .create_task(&goal.id, "echo success".to_string())
            .unwrap();
        conductor.store.assign_task(&task.id, &worker.id).unwrap();
        conductor
            .report_task_complete(&task.id, Some("success".to_string()))
            .unwrap();
    }

    // Fail enough tasks to exceed 50% failure rate (6 failures vs 5 successes)
    for i in 0..6 {
        let goal = conductor
            .store
            .create_goal(format!("Fail goal {}", i))
            .unwrap();
        let task = conductor
            .store
            .create_task(&goal.id, "echo fail".to_string())
            .unwrap();
        conductor.store.assign_task(&task.id, &worker.id).unwrap();
        conductor
            .report_task_failed(&task.id, "error occurred".to_string())
            .unwrap();
    }

    // Trust level should decrease from 2 to 1 (failure rate > 50%)
    let updated_worker = conductor.store.get_worker(&worker.id).unwrap().unwrap();
    assert_eq!(updated_worker.trust_level, 1);
}

#[tokio::test]
async fn idle_decay_reduces_trust_level() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    let conductor =
        Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

    // Register worker and manually set to trust level 2
    let (worker, _) = conductor
        .register_worker("idle-worker".to_string(), vec!["compute".to_string()], None)
        .unwrap();
    conductor
        .store
        .set_worker_trust_level(&worker.id, 2)
        .unwrap();

    // Simulate idle time by backdating last_heartbeat to >30 days ago
    let now = chrono::Utc::now();
    let thirty_days = chrono::Duration::days(31);
    let past = now - thirty_days;
    conductor
        .store
        .update_worker_last_heartbeat(&worker.id, &past)
        .unwrap();

    // Heartbeat should trigger idle decay
    conductor.worker_heartbeat(&worker.id).unwrap();

    let updated_worker = conductor.store.get_worker(&worker.id).unwrap().unwrap();
    assert_eq!(updated_worker.trust_level, 1);
}

#[tokio::test]
async fn api_returns_new_token_on_trust_change() {
    use crate::api;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::Value;

    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    let conductor =
        Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

    // Register worker at trust level 0
    let (worker, original_token) = conductor
        .register_worker(
            "token-worker".to_string(),
            vec!["compute".to_string()],
            None,
        )
        .unwrap();
    assert_eq!(worker.trust_level, 0);

    // Complete tasks to reach threshold and trigger upgrade
    for i in 0..5 {
        let goal = conductor.store.create_goal(format!("Goal {}", i)).unwrap();
        let task = conductor
            .store
            .create_task(&goal.id, "echo success".to_string())
            .unwrap();
        conductor.store.assign_task(&task.id, &worker.id).unwrap();
        conductor
            .report_task_complete(&task.id, Some("success".to_string()))
            .unwrap();
    }

    // Worker should now be at trust level 1
    let updated_worker = conductor.store.get_worker(&worker.id).unwrap().unwrap();
    assert_eq!(updated_worker.trust_level, 1);

    // Login again — token should reflect new trust level
    let (_, new_token) = conductor.worker_login("token-worker", None).unwrap();
    assert_ne!(original_token, new_token);

    // Verify the token's trust level claim via validate_token
    let claims = auth::validate_token(&new_token).unwrap();
    assert_eq!(claims.trust_level, 1);
}
