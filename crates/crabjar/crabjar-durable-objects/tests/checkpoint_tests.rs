//! Tests for durable object checkpointing infrastructure.

use chrono::Utc;
use serde_json::json;
use std::fs;
use std::time::Duration;

use crabjar_durable_objects::{CheckpointConfig, CheckpointStore};

#[test]
fn test_checkpoint_store_open_and_create() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("checkpoints.db");

    let store = CheckpointStore::open(&db_path).unwrap();

    // Create a checkpoint
    let payload = json!({"step": 42, "data": vec![1, 2, 3]});
    let id = store
        .create_checkpoint("task-1", payload.clone(), None)
        .unwrap();

    assert!(!id.is_empty());

    // Verify checkpoint exists
    let meta = store.get_checkpoint(&id).unwrap().unwrap();
    assert_eq!(meta.task_id, "task-1");
    assert_eq!(meta.size_bytes > 0, true);
}

#[test]
fn test_latest_checkpoint() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("checkpoints.db");

    let store = CheckpointStore::open(&db_path).unwrap();

    // Create multiple checkpoints for same task
    store
        .create_checkpoint("task-1", json!({"step": 1}), None)
        .unwrap();
    std::thread::sleep(Duration::from_millis(10));
    store
        .create_checkpoint("task-1", json!({"step": 2}), None)
        .unwrap();

    // Get latest
    let latest = store.get_latest_checkpoint("task-1").unwrap().unwrap();
    assert_eq!(latest.task_id, "task-1");
}

#[test]
fn test_list_checkpoints() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("checkpoints.db");

    let store = CheckpointStore::open(&db_path).unwrap();

    // Create checkpoints for different tasks
    store
        .create_checkpoint("task-1", json!({"step": 1}), None)
        .unwrap();
    store
        .create_checkpoint("task-2", json!({"step": 1}), None)
        .unwrap();
    std::thread::sleep(Duration::from_millis(10));
    store
        .create_checkpoint("task-1", json!({"step": 2}), None)
        .unwrap();

    // List checkpoints for task-1
    let checkpoints = store.list_checkpoints("task-1").unwrap();
    assert_eq!(checkpoints.len(), 2);
}

#[test]
fn test_checkpoint_config() {
    let config = CheckpointConfig::new("/tmp/test.db");

    assert_eq!(config.checkpoint_interval, Duration::from_secs(300));
    assert_eq!(config.retention_count, 10);
    assert_eq!(config.compress, true);
}
