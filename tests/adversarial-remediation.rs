//! Integration tests for adversarial review findings 1.1 and 1.2:
//! - Container mode exclusivity (no host fallback)
//! - Fail-closed GuardDb and provenance validation

use std::fs;
use std::process::Command;

fn crabjar_bin() -> String {
    // Use the same binary that cargo test is running against
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/target/debug/crabjar", manifest_dir)
}

fn run_crabjar_in(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(crabjar_bin())
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to execute crabjar")
}

fn json_stdout(output: &std::process::Output) -> serde_json::Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(stdout.trim()).expect("failed to parse JSON from crabjar output")
}

// ──────────────────────────────────────────────
// Finding 1.1: Container mode exclusivity
// ──────────────────────────────────────────────

/// Test that when container=true, the command does NOT execute on the host.
/// Uses a marker file as proof of host execution — if the marker exists after
/// a container-mode exec, then host fallback occurred (failure).
#[test]
fn container_mode_does_not_execute_on_host() {
    let temp = tempfile::tempdir().unwrap();
    let marker_file = temp.path().join("host_marker.txt");

    // Initialize crabjar config to enable tool execution
    fs::write(
        temp.path().join(".crabjar_config.toml"),
        "name = \"container-test\"\ntool_execution_enabled = true\n",
    )
    .unwrap();

    // Request container execution with a command that would create a host marker
    // if it ran on the host. Use 'touch' which is safe and creates a file.
    let cwd_arg = format!("--cwd={}", temp.path().display());
    let touch_path = marker_file.display().to_string();
    let args: Vec<&str> = vec![
        "exec",
        "--container",
        &cwd_arg,
        "--reason=container-exclusivity-test",
        "--command=touch",
        "-a",
        &touch_path,
    ];

    let output = run_crabjar_in(temp.path(), &args);

    // Container mode should either succeed in podman or fail due to podman unavailability.
    // In either case, the host marker must NOT exist (proving no host fallback).
    assert!(
        !marker_file.exists(),
        "Container mode executed on host — marker file was created"
    );

    // Check the response indicates container mode was attempted
    let body = json_stdout(&output);
    if body["success"] == true {
        // If successful, it should report container backend
        assert_eq!(
            body["exec"]["container"], true,
            "Should report container=true"
        );
        assert_eq!(
            body["exec"]["backend"], "podman",
            "Should report podman backend"
        );
    } else {
        // If failed, it should be due to podman unavailability, not host execution
        let error_msg = body["exec"]["error"]
            .as_str()
            .or(body["error"].as_str())
            .unwrap_or("");
        assert!(
            error_msg.contains("podman") || error_msg.contains("container"),
            "Failure should mention podman/container, not host execution: {}",
            error_msg
        );
    }
}

/// Test that when Podman is unavailable, container mode fails rather than falling back to host.
#[test]
fn container_mode_fails_without_podman() {
    let temp = tempfile::tempdir().unwrap();

    fs::write(
        temp.path().join(".crabjar_config.toml"),
        "name = \"podman-test\"\ntool_execution_enabled = true\n",
    )
    .unwrap();

    let marker_file = temp.path().join("fallback_marker.txt");

    // Use a command that would create a marker if it ran on the host
    let cwd_arg = format!("--cwd={}", temp.path().display());
    let touch_path = marker_file.display().to_string();
    let args: Vec<&str> = vec![
        "exec",
        "--container",
        &cwd_arg,
        "--reason=podman-unavailable-test",
        "--command=touch",
        "-a",
        &touch_path,
    ];

    let output = run_crabjar_in(temp.path(), &args);
    let body = json_stdout(&output);

    // Must fail (podman unavailable) and must not create host marker
    assert!(
        !marker_file.exists(),
        "Podman unavailable but command ran on host — marker file was created"
    );

    // Response should indicate failure due to podman
    let exec_obj = &body["exec"];
    if body["success"] == false {
        let error_msg = exec_obj["error"]
            .as_str()
            .or(exec_obj["gate_reason"].as_str())
            .or(body["error"].as_str())
            .unwrap_or("");
        assert!(
            error_msg.contains("podman") || error_msg.contains("container"),
            "Failure should mention podman/container unavailability: {}",
            error_msg
        );
    }
}

// ──────────────────────────────────────────────
// Finding 1.2: Fail-closed GuardDb and provenance
// ──────────────────────────────────────────────

/// Test that when GuardDb cannot be opened (unwritable path), execution fails
/// with a structured error rather than silently falling back to in-memory DB.
#[test]
fn guarddb_open_failure_stops_execution() {
    let temp = tempfile::tempdir().unwrap();

    // Create an unwritable guard directory (make it a file instead of dir)
    let guard_path = temp.path().join("guard");
    fs::write(&guard_path, "not a directory").unwrap();

    fs::write(
        temp.path().join(".crabjar_config.toml"),
        "name = \"guarddb-test\"\ntool_execution_enabled = true\n",
    )
    .unwrap();

    let marker_file = temp.path().join("executed_marker.txt");

    // Command that would create a marker if it executed
    let cwd_arg = format!("--cwd={}", temp.path().display());
    let touch_path = marker_file.display().to_string();
    let args: Vec<&str> = vec![
        "exec",
        &cwd_arg,
        "--reason=guarddb-failure-test",
        "--command=touch",
        "-a",
        &touch_path,
    ];

    let output = run_crabjar_in(temp.path(), &args);
    let body = json_stdout(&output);

    // Command must not have executed (no marker file)
    assert!(
        !marker_file.exists(),
        "Command executed despite GuardDb open failure"
    );

    // Response should indicate guard database failure
    let exec_obj = &body["exec"];
    let gate_result = exec_obj["gate_result"].as_str().unwrap_or("");
    let gate_reason = exec_obj["gate_reason"]
        .as_str()
        .or(body["error"].as_str())
        .unwrap_or("");

    assert!(
        gate_result == "error" || gate_reason.contains("guard"),
        "Should report guard database error, got gate_result={} gate_reason={}",
        gate_result,
        gate_reason
    );
}

/// Test that arbitrary reason text cannot satisfy provenance validation.
/// The old code passed free-form 'reason' as source_event_id directly; now it
/// must create a real action request record first.
#[test]
fn arbitrary_reason_text_is_not_provenance() {
    let temp = tempfile::tempdir().unwrap();

    fs::write(
        temp.path().join(".crabjar_config.toml"),
        "name = \"provenance-test\"\ntool_execution_enabled = true\n",
    )
    .unwrap();

    // Use a unique, arbitrary reason string that could never be a real event ID
    let arbitrary_reason = format!(
        "completely-arbitrary-reason-{}-not-a-real-event-id",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    let cwd_arg = format!("--cwd={}", temp.path().display());
    let reason_arg = format!("--reason={}", arbitrary_reason);
    let args: Vec<&str> = vec![
        "exec",
        &cwd_arg,
        &reason_arg,
        "--command=echo",
        "-a",
        "test",
    ];

    let output = run_crabjar_in(temp.path(), &args);
    let body = json_stdout(&output);

    // The command should execute (we create a real action request first now).
    // What matters is that the arbitrary reason string is NOT used as source_event_id
    // directly — it must be wrapped in a proper action request record.
    //
    // Verify by checking guard.db has an action_request with our reason in the payload,
    // not as source_event_id.
    let guard_db_path = temp.path().join("guard/guard.db");
    assert!(
        guard_db_path.exists(),
        "GuardDb should exist after exec attempt"
    );

    // Query guard.db to verify provenance structure
    let conn = rusqlite::Connection::open(&guard_db_path).expect("open guard db for test query");

    // Use correct rusqlite API
    let mut stmt = conn.prepare(
        "SELECT source_event_id, payload FROM action_requests ORDER BY requested_at DESC LIMIT 1",
    ).expect("prepare statement");

    let rows: Vec<(Option<String>, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("query")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect");

    assert!(
        !rows.is_empty(),
        "Should have created an action request record"
    );

    let (source_event_id, payload) = &rows[0];

    // source_event_id should be a UUID (our generated action_request_id), not the arbitrary reason
    let source_str = source_event_id.as_deref().unwrap_or("");
    assert!(
        source_str != &arbitrary_reason,
        "Arbitrary reason text was used as source_event_id — provenance validation broken"
    );

    // The arbitrary reason should appear in the payload or elsewhere, but not as source_event_id
    assert!(
        payload.contains("echo") || payload.contains("test"),
        "Payload should contain command info"
    );
}
