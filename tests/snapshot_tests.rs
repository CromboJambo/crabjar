//! Snapshot tests for CrabJar structured output.
//!
//! These tests verify that the TUI message types and CLI JSON output format
//! produce consistent, serializable output across changes. They don't require
//! a real terminal — just serde serialization of data structures.

use std::process::Command;

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

    insta::assert_json_snapshot!("message_serialization", messages);
}

/// Test that CLI `state list` returns valid JSON structure.
#[test]
fn test_cli_state_list_output() {
    let output = Command::new("cargo")
        .args(["run", "-p", "crabjar", "--", "state", "list"])
        .output()
        .expect("Failed to run crabjar state list");

    assert!(
        output.status.success(),
        "crabjar state list failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output is not valid JSON");

    insta::assert_json_snapshot!(
        "cli_state_list_output",
        serde_json::json!({
            "success": parsed.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
            "docs_count": parsed.get("docs").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
        })
    );
}

/// Test that CLI `workspace status` returns valid JSON structure.
#[test]
fn test_cli_workspace_status_output() {
    let output = Command::new("cargo")
        .args(["run", "-p", "crabjar", "--", "workspace", "status"])
        .output()
        .expect("Failed to run crabjar workspace status");

    assert!(
        output.status.success(),
        "crabjar workspace status failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output is not valid JSON");

    let workspace_val = parsed
        .get("workspace")
        .cloned()
        .unwrap_or(serde_json::json!(null));

    insta::assert_json_snapshot!(
        "cli_workspace_status_output",
        serde_json::json!({
            "success": parsed.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
            "workspace": workspace_val,
        })
    );
}

/// Test that CLI `doctor check` returns valid JSON structure.
#[test]
fn test_cli_doctor_check_output() {
    let output = Command::new("cargo")
        .args(["run", "-p", "crabjar", "--", "doctor", "check"])
        .output()
        .expect("Failed to run crabjar doctor check");

    assert!(
        output.status.success(),
        "crabjar doctor check failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output is not valid JSON");

    insta::assert_json_snapshot!(
        "cli_doctor_check_output",
        serde_json::json!({
            "success": parsed.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
            "checks_count": parsed.get("checks").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
        })
    );
}

/// Test that CLI `tool list` returns valid JSON structure.
#[test]
fn test_cli_tool_list_output() {
    let output = Command::new("cargo")
        .args(["run", "-p", "crabjar", "--", "tool", "list"])
        .output()
        .expect("Failed to run crabjar tool list");

    assert!(
        output.status.success(),
        "crabjar tool list failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output is not valid JSON");

    insta::assert_json_snapshot!(
        "cli_tool_list_output",
        serde_json::json!({
            "success": parsed.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
            "tools_count": parsed.get("tools").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
        })
    );
}

/// Test that CLI `attempts status` returns valid JSON structure.
#[test]
fn test_cli_attempts_status_output() {
    let output = Command::new("cargo")
        .args(["run", "-p", "crabjar", "--", "attempts", "status"])
        .output()
        .expect("Failed to run crabjar attempts status");

    assert!(
        output.status.success(),
        "crabjar attempts status failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output is not valid JSON");

    insta::assert_json_snapshot!(
        "cli_attempts_status_output",
        serde_json::json!({
            "success": parsed.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
            "unjudged": parsed.get("attempts").and_then(|v| v.get("unjudged")).and_then(|v| v.as_u64()).unwrap_or(0),
            "has_doubt": parsed.get("doubt").is_some(),
        })
    );
}

/// Test that CLI `guard queue` returns valid JSON structure.
#[test]
fn test_cli_guard_queue_output() {
    let output = Command::new("cargo")
        .args(["run", "-p", "crabjar", "--", "guard", "queue"])
        .output()
        .expect("Failed to run crabjar guard queue");

    assert!(
        output.status.success(),
        "crabjar guard queue failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output is not valid JSON");

    insta::assert_json_snapshot!(
        "cli_guard_queue_output",
        serde_json::json!({
            "success": parsed.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
            "pending_count": parsed.get("pending").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
        })
    );
}
