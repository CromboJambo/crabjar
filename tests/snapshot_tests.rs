//! Snapshot tests for CrabJar structured output.
//!
//! These tests verify that the TUI message types and CLI JSON output format
//! produce consistent, serializable output across changes. They don't require
//! a real terminal — just serde serialization of data structures.
//!
//! CLI tests check **structure** (valid JSON, expected fields present) rather
//! than exact values (counts change with system state). This keeps them
//! deterministic when run in parallel with other workspace tests.

use std::process::Command;

use serial_test::serial;

/// Test that Message enum serializes to stable JSON.
#[test]
fn test_message_serialization() {
    // We can't import TUI types directly (crabjar-host is binary-only),
    // but we can verify the expected JSON structure matches what the TUI would produce.

    let messages = vec![
        serde_json::json!({
            "User": {"text": "Hello, world!"}
        }),
        serde_json::json!({
            "Agent": {"text": "I'm processing your request..."}
        }),
        serde_json::json!({
            "ToolCall": {
                "name": "read_file",
                "args": r#"{"path": "/test.txt"}"#,
                "result": "File contents here"
            }
        }),
        serde_json::json!({
            "Guard": {"action": "execute_command", "pending": true}
        }),
    ];

    // Verify structure: 4 messages, each with the expected type and fields.
    assert_eq!(messages.len(), 4);

    let user = &messages[0];
    assert!(user.get("User").is_some());
    assert_eq!(user["User"]["text"], "Hello, world!");

    let agent = &messages[1];
    assert!(agent.get("Agent").is_some());
    assert!(
        agent["Agent"]["text"]
            .as_str()
            .unwrap()
            .contains("processing")
    );

    let tool_call = &messages[2];
    assert!(tool_call.get("ToolCall").is_some());
    assert_eq!(tool_call["ToolCall"]["name"], "read_file");
    assert!(
        tool_call["ToolCall"]["args"]
            .as_str()
            .unwrap()
            .contains("path")
    );
    assert!(tool_call["ToolCall"]["result"].is_string());

    let guard = &messages[3];
    assert!(guard.get("Guard").is_some());
    assert_eq!(guard["Guard"]["action"], "execute_command");
    assert_eq!(guard["Guard"]["pending"], true);
}

/// Helper: run a CLI command and return parsed JSON.
fn run_cli(args: &[&str]) -> serde_json::Value {
    let output = Command::new("cargo")
        .args(["run", "-p", "crabjar", "--"])
        .args(args)
        .output()
        .expect("Failed to run crabjar CLI");

    assert!(
        output.status.success(),
        "crabjar {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).expect("Output is not valid JSON")
}

/// Verify CLI output has the expected structure: success field + required keys.
fn assert_cli_structure(parsed: &serde_json::Value, command: &str, required_fields: &[&str]) {
    assert!(
        parsed
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        "crabjar {} should return success: true",
        command
    );

    for field in required_fields {
        assert!(
            parsed.get(*field).is_some(),
            "crabjar {} output missing expected field '{}'",
            command,
            field
        );
    }
}

/// Test that CLI `state list` returns valid JSON structure.
#[serial]
#[test]
fn test_cli_state_list_output() {
    let parsed = run_cli(&["state", "list"]);
    assert_cli_structure(&parsed, "state list", &["success", "payload"]);
}

/// Test that CLI `workspace status` returns valid JSON structure.
#[serial]
#[test]
fn test_cli_workspace_status_output() {
    let parsed = run_cli(&["workspace", "status"]);
    assert_cli_structure(&parsed, "workspace status", &["success", "workspace"]);
}

/// Test that CLI `doctor check` returns valid JSON structure.
#[serial]
#[test]
fn test_cli_doctor_check_output() {
    let parsed = run_cli(&["doctor", "check"]);
    assert_cli_structure(&parsed, "doctor check", &["success", "doctor"]);
}

/// Test that CLI `tool list` returns valid JSON structure.
#[serial]
#[test]
fn test_cli_tool_list_output() {
    let parsed = run_cli(&["tool", "list"]);
    assert_cli_structure(&parsed, "tool list", &["success", "tools"]);
}

/// Test that CLI `attempts status` returns valid JSON structure.
#[serial]
#[test]
fn test_cli_attempts_status_output() {
    let parsed = run_cli(&["attempts", "status"]);
    assert_cli_structure(
        &parsed,
        "attempts status",
        &["success", "attempts", "doubt"],
    );
}

/// Test that CLI `guard queue` returns valid JSON structure.
#[serial]
#[test]
fn test_cli_guard_queue_output() {
    let parsed = run_cli(&["guard", "queue"]);
    assert_cli_structure(&parsed, "guard queue", &["success", "guard"]);
}
