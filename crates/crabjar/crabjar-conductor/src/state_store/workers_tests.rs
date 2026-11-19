#[cfg(test)]
mod tests {
    use super::super::*;
    use tempfile::tempdir;

    fn test_store() -> (StateStore, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let store = StateStore::open(&path).unwrap();
        (store, dir)
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
    fn trust_upgrade_threshold() {
        let (store, _dir) = test_store();
        unsafe { std::env::set_var("CRABJAR_AUTH_SECRET", "test-secret") };
        let (worker, _token) = store
            .register_worker("upgrader".to_string(), vec![], None)
            .unwrap();

        // 4 successes - no upgrade yet
        for _ in 0..4 {
            store.record_task_success(&worker.id).unwrap();
        }
        let w = store.get_worker(&worker.id).unwrap().unwrap();
        assert_eq!(w.trust_level, 0);

        // 5th success triggers upgrade to level 1
        store.record_task_success(&worker.id).unwrap();
        let w = store.get_worker(&worker.id).unwrap().unwrap();
        assert_eq!(w.trust_level, 1);
    }

    #[test]
    fn heartbeat_with_resources_persists() {
        let (store, _dir) = test_store();
        unsafe { std::env::set_var("CRABJAR_AUTH_SECRET", "test-secret") };
        let (worker, _token) = store
            .register_worker(
                "res-worker".to_string(),
                vec!["gpu:nvidia:3090".to_string()],
                None,
            )
            .unwrap();

        // Initially no resources reported
        let w = store.get_worker(&worker.id).unwrap().unwrap();
        assert!(w.resources.is_none());

        // Heartbeat with resources
        let res = WorkerResources {
            max_ram_mb: 65536,
            available_ram_mb: 49152,
            gpu_count: 1,
            gpus_available: 1,
            reported_at: chrono::Utc::now().to_rfc3339(),
        };
        store
            .heartbeat_with_resources(&worker.id, Some(&res))
            .unwrap();

        // Verify persisted
        let w = store.get_worker(&worker.id).unwrap().unwrap();
        assert!(w.resources.is_some());
        let stored = w.resources.unwrap();
        assert_eq!(stored.max_ram_mb, 65536);
        assert_eq!(stored.available_ram_mb, 49152);
        assert_eq!(stored.gpu_count, 1);
        assert_eq!(stored.gpus_available, 1);

        // Heartbeat without resources — should not clear existing snapshot
        store.heartbeat_with_resources(&worker.id, None).unwrap();
        let w = store.get_worker(&worker.id).unwrap().unwrap();
        assert!(w.resources.is_some());
        assert_eq!(w.resources.unwrap().max_ram_mb, 65536);

        // Update resources with new values
        let updated_res = WorkerResources {
            available_ram_mb: 32768,
            gpus_available: 0,
            ..res.clone()
        };
        store
            .update_worker_resources(&worker.id, &updated_res)
            .unwrap();
        let w = store.get_worker(&worker.id).unwrap().unwrap();
        assert_eq!(w.resources.as_ref().unwrap().available_ram_mb, 32768);
        assert_eq!(w.resources.as_ref().unwrap().gpus_available, 0);
    }

    #[test]
    fn worker_resources_can_satisfy_integration() {
        let (store, _dir) = test_store();
        unsafe { std::env::set_var("CRABJAR_AUTH_SECRET", "test-secret") };
        let (worker, _token) = store
            .register_worker("sched-worker".to_string(), vec![], None)
            .unwrap();

        // Worker reports limited resources
        let res = WorkerResources {
            max_ram_mb: 8192,
            available_ram_mb: 4096,
            gpu_count: 0,
            gpus_available: 0,
            reported_at: chrono::Utc::now().to_rfc3339(),
        };
        store
            .heartbeat_with_resources(&worker.id, Some(&res))
            .unwrap();

        let w = store.get_worker(&worker.id).unwrap().unwrap();
        let stored_res = w.resources.unwrap();

        // Small task fits
        let small_task = crate::task_spec::TaskResources {
            memory_mb: 2048,
            gpu: false,
            ..Default::default()
        };
        assert!(stored_res.can_satisfy(&small_task));

        // Large task doesn't fit (needs more RAM than available)
        let big_task = crate::task_spec::TaskResources {
            memory_mb: 8192,
            gpu: false,
            ..Default::default()
        };
        assert!(!stored_res.can_satisfy(&big_task));

        // GPU task doesn't fit (no GPUs)
        let gpu_task = crate::task_spec::TaskResources {
            memory_mb: 1024,
            gpu: true,
            ..Default::default()
        };
        assert!(!stored_res.can_satisfy(&gpu_task));
    }
}
