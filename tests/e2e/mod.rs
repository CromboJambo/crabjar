//! E2E smoke tests — fast regression gate (~30s).
//!
//! These verify the CLI binary runs and returns structured JSON for each
//! major subsystem. They run on every PR; full slice (tests/e2e/full.rs)
//! covers deeper integration paths.

use serde_json::Value;
use std::fs;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_crabjar")
}

fn run_in(temp: &tempfile::TempDir, args: &[&str]) -> std::process::Output {
    Command::new(binary())
        .current_dir(temp.path())
        .args(args)
        .output()
        .unwrap()
}

fn json_stdout(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Smoke test 1: `crabjar state list` — verifies CLI binary runs and returns JSON.
#[test]
fn smoke_state_list_returns_json() {
    let temp = tempfile::tempdir().unwrap();
    let docs_dir = temp.path().join("state-docs");
    fs::create_dir_all(&docs_dir).unwrap();

    // Index a minimal doc so list has something to return
    fs::write(
        docs_dir.join("smoke_state.md"),
        "---\nname: smoke_state\n---\n# Smoke\n",
    )
    .unwrap();

    let index_output = run_in(&temp, &["state", "index", "--docs-dir", "state-docs"]);
    assert!(
        index_output.status.success(),
        "index failed: {}",
        String::from_utf8_lossy(&index_output.stderr)
    );

    let output = run_in(&temp, &["state", "list"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert!(body["payload"]["docs"].is_array());
}

/// Smoke test 2: `crabjar workspace status` — verifies `.crabjar_config.toml` loading.
#[test]
fn smoke_workspace_status_returns_json() {
    let temp = tempfile::tempdir().unwrap();

    // No config → should return null workspace (soft-fail)
    let output = run_in(&temp, &["workspace", "status"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert_eq!(body["workspace"], Value::Null);

    // With config → should return workspace details
    fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "smoke-workspace"
description = "Smoke test workspace"
tool_execution_enabled = true
"#,
    )
    .unwrap();

    let output2 = run_in(&temp, &["workspace", "status"]);
    assert!(output2.status.success());

    let body2 = json_stdout(&output2);
    assert_eq!(body2["success"], true);
    assert_eq!(body2["workspace"]["name"], "smoke-workspace");
    assert_eq!(body2["workspace"]["tool_execution_enabled"], true);
}

/// Smoke test 3: Guard DB init + basic gate check (in-memory).
#[test]
fn smoke_guard_queue_returns_json() {
    let temp = tempfile::tempdir().unwrap();

    // Create a minimal config so guard commands work
    fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "guard-smoke"
tool_execution_enabled = true
"#,
    )
    .unwrap();

    let output = run_in(&temp, &["guard", "queue", "--status=pending"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert!(body["guard"]["queue"]["entries"].is_array());
}

/// Smoke test 4: Tool registry init + register/query cycle.
#[test]
fn smoke_tool_list_returns_json() {
    let temp = tempfile::tempdir().unwrap();

    // Create a minimal config so tool commands work
    fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "tool-smoke"
"#,
    )
    .unwrap();

    let output = run_in(&temp, &["tool", "list"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    // Response shape: {"tools": {"count": N, "tools": [...]}}
    assert!(body["tools"]["count"].is_number() || body["tools"]["tools"].is_array());
}

/// Smoke test 5: Knowledge store init + basic query.
#[test]
fn smoke_knowledge_query_returns_json() {
    let temp = tempfile::tempdir().unwrap();

    // Insert a knowledge entry, then query it back
    let insert_output = run_in(
        &temp,
        &[
            "knowledge",
            "insert",
            "--content=Smoke test knowledge entry",
            "--kind=context",
            "--tags=e2e,smoke",
        ],
    );
    assert!(insert_output.status.success());

    let insert_body = json_stdout(&insert_output);
    assert_eq!(insert_body["success"], true);
    // Response shape: {"data": {"id": N}, ...}
    assert!(insert_body["data"]["id"].is_number());

    // Query the entry back
    let query_output = run_in(
        &temp,
        &["knowledge", "query", "--tags=e2e,smoke"],
    );
    assert!(query_output.status.success());

    let query_body = json_stdout(&query_output);
    assert_eq!(query_body["success"], true);
    // Response shape: {"data": {"rows": [...]}, ...}
    assert!(query_body["data"]["rows"].is_array());
}

/// Smoke test 6: `crabjar doctor check` — verifies environment health.
#[test]
fn smoke_doctor_check_returns_json() {
    let temp = tempfile::tempdir().unwrap();

    // Create a minimal config so doctor can load workspace state
    fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "doctor-smoke"
"#,
    )
    .unwrap();

    let output = run_in(&temp, &["doctor", "check"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert!(body["doctor"]["ok"].is_boolean());
    assert!(body["doctor"]["checks"].is_array());

    // Each check should have name, ok, detail
    for check in body["doctor"]["checks"].as_array().unwrap() {
        assert!(check["check"].is_string(), "check missing 'check' field");
        assert!(check["ok"].is_boolean(), "check missing 'ok' field");
        assert!(check["detail"].is_string(), "check missing 'detail' field");
    }

    // Should include doubt block per CLI output contract
    assert!(body["doctor"]["doubt"].is_object());
}
