// Tests for ADR-044 redteam findings 1.3 and 1.4:
// - Finding 1.3: Truthful outcomes — success must be derived from exit code, not hardcoded true
// - Finding 1.4: Correct outcome persistence — outcomes must persist to the same DB as provenance

use serde_json::Value;
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

#[test]
fn success_true_when_exit_code_zero() {
    let temp = tempfile::tempdir().unwrap();

    std::fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "finding-1.3-success"
tool_execution_enabled = true
"#,
    )
    .unwrap();

    // Execute a command that exits with code 0
    let output = run_in(&temp, &["exec", "--command=echo hello", "--reason=test success derivation"]);
    assert!(output.status.success());

    let body = json_stdout(&output);

    // Finding 1.3: success must be derived from exit code, not hardcoded true
    assert_eq!(body["success"], true, "success should be true when exit code is 0");
    assert_eq!(body["exec"]["exit_code"], 0);
}

#[test]
fn success_false_when_nonzero_exit() {
    let temp = tempfile::tempdir().unwrap();

    std::fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "finding-1.3-failure"
tool_execution_enabled = true
"#,
    )
    .unwrap();

    // Execute a command that exits with non-zero code
    let output = run_in(&temp, &["exec", "--command=false", "--reason=test success derivation failure"]);
    assert!(output.status.success());

    let body = json_stdout(&output);

    // Finding 1.3: success must be derived from exit code (not hardcoded true)
    assert_eq!(body["success"], false, "success should be false when exit code is non-zero");
    assert_ne!(body["exec"]["exit_code"], 0);
}

#[test]
fn outcome_inserted_with_actual_id() {
    let temp = tempfile::tempdir().unwrap();

    std::fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "finding-1.4-persistence"
tool_execution_enabled = true
"#,
    )
    .unwrap();

    // Execute a command and capture the outcome_id from the response
    let output = run_in(&temp, &[
        "exec", "--command=echo outcome test",
        "--reason=test outcome persistence",
        "--trust-layer=4"
    ]);
    eprintln!("Test output status: {}", output.status);
    eprintln!("Test output stdout: {}", String::from_utf8_lossy(&output.stdout));
    eprintln!("Test output stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success());

    let body = json_stdout(&output);

    // Finding 1.4: outcome_id must be a real UUID that exists in the DB
    let outcome_id = body["exec"]["outcome_id"]
        .as_str()
        .expect("outcome_id should be a string");
    assert!(!outcome_id.is_empty(), "outcome_id should not be empty");
    
    // Verify it's a valid UUID format (basic check)
    assert!(outcome_id.contains("-"), "outcome_id should be a UUID with dashes");

    // Verify the outcome was inserted into the guard DB (same as provenance)
    let guard_db_path = temp.path().join("guard/guard.db");
    let conn = rusqlite::Connection::open(&guard_db_path).unwrap();
    
    let row = conn.query_row(
        "SELECT COUNT(*) as cnt FROM action_outcomes WHERE id = ?1",
        [&outcome_id],
        |r| r.get::<_, i64>("cnt")
    ).unwrap();
    
    assert_eq!(row, 1, "Outcome should exist in guard DB with the returned ID");
}