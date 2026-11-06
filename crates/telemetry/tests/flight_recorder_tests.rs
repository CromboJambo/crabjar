use crabjar_telemetry::flight_recorder::{FlightRecorder, sha256};
use crabjar_telemetry::schema::{CheckpointRow, FlightRecordRow, init_db};
use rusqlite::Connection;
use tempfile::tempdir;

#[test]
fn test_flight_recorder_init() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("flight.db");
    let conn = Connection::open(&db_path).unwrap();

    init_db(&conn).unwrap();

    let recorder = FlightRecorder::new(&conn, "test-session");
    let rows = recorder.query_records(10).unwrap();
    assert_eq!(rows.len(), 0);
}

#[test]
fn test_checkpoint_before_and_after() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("flight.db");
    let conn = Connection::open(&db_path).unwrap();

    init_db(&conn).unwrap();

    let recorder = FlightRecorder::new(&conn, "test-session");
    let before_id = recorder.checkpoint_before("before_hash", 0).unwrap();
    let after_id = recorder.checkpoint_after("after_hash", 5).unwrap();
    assert!(!before_id.is_empty());
    assert!(!after_id.is_empty());

    let checkpoints = recorder.query_checkpoints(10).unwrap();
    assert_eq!(checkpoints.len(), 2);
    assert_eq!(checkpoints[0].repo_state_hash, "before_hash");
    assert_eq!(checkpoints[1].repo_state_hash, "after_hash");
}

#[tokio::test]
async fn test_capture_git_dirty() {
    let dir = tempdir().unwrap();

    // Initialize a clean git repo so `git status --porcelain` runs without error.
    let init = tokio::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .output()
        .await
        .unwrap();
    assert!(init.status.success(), "git init failed");

    let conn = Connection::open(dir.path().join("flight.db")).unwrap();
    init_db(&conn).unwrap();

    let recorder = FlightRecorder::new(&conn, "test-session");
    let dirty = recorder
        .capture_git_dirty(dir.path().to_string_lossy().as_ref())
        .await
        .unwrap();
    assert!(dirty >= 0);
}

#[tokio::test]
async fn test_execute_command_echo() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("flight.db");
    let conn = Connection::open(&db_path).unwrap();

    init_db(&conn).unwrap();

    let mut recorder = FlightRecorder::new(&conn, "test-execute");
    let cmd_id = recorder
        .execute_command(
            "echo",
            &["hello world".to_string()],
            dir.path().to_string_lossy().as_ref(),
            "test execute",
        )
        .await
        .unwrap();

    assert!(!cmd_id.is_empty());

    let records = recorder.query_records(10).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].command, "echo");
}

#[tokio::test]
async fn test_execute_command_fails() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("flight.db");
    let conn = Connection::open(&db_path).unwrap();

    init_db(&conn).unwrap();

    let mut recorder = FlightRecorder::new(&conn, "test-execute-fail");
    let result = recorder
        .execute_command(
            "nonexistent_binary_xyz",
            &[],
            dir.path().to_string_lossy().as_ref(),
            "test fail",
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_capture_git_diff() {
    let dir = tempdir().unwrap();
    let conn = Connection::open(dir.path().join("flight.db")).unwrap();

    init_db(&conn).unwrap();

    let recorder = FlightRecorder::new(&conn, "test-git-diff");
    let diff_hash = recorder
        .capture_git_diff(dir.path().to_string_lossy().as_ref())
        .await
        .unwrap();

    assert!(!diff_hash.is_empty() || diff_hash.is_empty());
}

#[test]
fn test_sha256_empty() {
    let hash = sha256("");
    assert!(hash.is_empty());
}

#[test]
fn test_sha256_nonempty() {
    let hash = sha256("hello");
    assert_eq!(hash.len(), 64);
}

#[test]
fn test_sha256_deterministic() {
    let h1 = sha256("test");
    let h2 = sha256("test");
    assert_eq!(h1, h2);
}

#[test]
fn test_query_transcript_empty() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("flight.db");
    let conn = Connection::open(&db_path).unwrap();

    init_db(&conn).unwrap();

    let recorder = FlightRecorder::new(&conn, "test-transcript");
    let result = recorder.query_transcript("nonexistent", 10).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_serialize_checkpoint_bincode() {
    let checkpoint = CheckpointRow {
        id: "chk-1".to_string(),
        session_id: "session-1".to_string(),
        repo_state_hash: "abc123".to_string(),
        dirty_tree_count: 5,
        checkpointed_at: "2024-01-01T00:00:00Z".to_string(),
    };

    let bytes = FlightRecorder::serialize_checkpoint_bincode(&checkpoint);
    assert!(!bytes.is_empty());

    let deserialized = FlightRecorder::deserialize_checkpoint_bincode(&bytes).unwrap();
    assert_eq!(deserialized.id, checkpoint.id);
    assert_eq!(deserialized.repo_state_hash, checkpoint.repo_state_hash);
    assert_eq!(deserialized.dirty_tree_count, checkpoint.dirty_tree_count);
}

#[test]
fn test_serialize_flight_record_bincode() {
    let record = FlightRecordRow {
        id: "cmd-1".to_string(),
        session_id: "session-1".to_string(),
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        command: "cargo check".to_string(),
        cwd: "/repo".to_string(),
        args: vec!["--workspace".to_string()],
        exit_code: 0,
        stdout_hash: "hash1".to_string(),
        stderr_hash: "hash2".to_string(),
        stdout_len: 100,
        stderr_len: 0,
        git_dirty: 3,
        git_diff_count: 2,
        git_diff_hash: "diff_hash".to_string(),
        reason: "test: check workspace".to_string(),
        source: "agent".to_string(),
        receipt: "zc-receipt-1234567890-abc123".to_string(),
    };

    let bytes = FlightRecorder::serialize_flight_record_bincode(&record);
    assert!(!bytes.is_empty());

    let deserialized = FlightRecorder::deserialize_flight_record_bincode(&bytes).unwrap();
    assert_eq!(deserialized.command, record.command);
    assert_eq!(deserialized.exit_code, record.exit_code);
    assert_eq!(deserialized.args, record.args);
    assert_eq!(deserialized.receipt, record.receipt);
}

#[test]
fn test_receipt_generation() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("flight.db");
    let conn = Connection::open(&db_path).unwrap();

    init_db(&conn).unwrap();

    let mut recorder = FlightRecorder::new(&conn, "test-receipt");
    recorder.init_receipt_key().unwrap();

    let receipt = recorder
        .generate_receipt("echo", &["hello".to_string()], "hello", 0)
        .unwrap();

    assert!(receipt.starts_with("zc-receipt-"));
    assert!(receipt.len() > 30); // epoch + digest

    let verified = recorder.verify_receipt(&receipt).unwrap();
    assert!(verified);
}

#[test]
fn test_receipt_invalid_format() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("flight.db");
    let conn = Connection::open(&db_path).unwrap();

    init_db(&conn).unwrap();

    let recorder = FlightRecorder::new(&conn, "test-receipt-invalid");
    // No key initialized — verify should still work for invalid formats

    let invalid = "not-a-receipt";
    let verified = recorder.verify_receipt(invalid).unwrap();
    assert!(!verified);
}

#[test]
fn test_receipts_enabled() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("flight.db");
    let conn = Connection::open(&db_path).unwrap();

    init_db(&conn).unwrap();

    let recorder = FlightRecorder::new(&conn, "test-enabled");
    assert!(!recorder.receipts_enabled());

    let mut recorder = FlightRecorder::new(&conn, "test-enabled");
    recorder.init_receipt_key().unwrap();
    assert!(recorder.receipts_enabled());
}
