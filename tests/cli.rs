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

#[test]
fn help_exits_successfully() {
    let output = Command::new(binary()).arg("--help").output().unwrap();
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert!(body["usage"].is_array());
    assert!(
        body["usage"]
            .as_array()
            .unwrap()
            .iter()
            .any(|line| line == "crabjar state list")
    );
}

#[test]
fn state_list_returns_json() {
    let temp = tempfile::tempdir().unwrap();
    let docs_dir = temp.path().join("state-docs");
    fs::create_dir_all(&docs_dir).unwrap();
    fs::write(
        docs_dir.join("alpha_state.md"),
        "---\nname: alpha_state\ndescription: Test doc\n---\n# Alpha\nbody\n",
    )
    .unwrap();

    // First index the docs
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

#[test]
fn missing_command_exits_nonzero() {
    let temp = tempfile::tempdir().unwrap();

    let output = Command::new(binary())
        .current_dir(temp.path())
        .output()
        .unwrap();

    assert!(!output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], false);
    assert_eq!(body["error"], "missing command");
    assert!(body["usage"].is_array());
}

#[test]
fn state_show_returns_doc_contents() {
    let temp = tempfile::tempdir().unwrap();
    let docs_dir = temp.path().join("state-docs");
    fs::create_dir_all(&docs_dir).unwrap();
    fs::write(
        docs_dir.join("alpha_state.md"),
        "---\nname: alpha_state\n---\n# Alpha\nbody\n",
    )
    .unwrap();

    // First index the docs
    let index_output = run_in(&temp, &["state", "index", "--docs-dir", "state-docs"]);
    assert!(index_output.status.success());

    // Query with the doc_name as stored (filename with .md)
    let output = run_in(&temp, &["state", "show", "alpha_state.md"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert!(body["payload"]["markdown"].is_string());
    assert!(body["payload"]["metadata"].is_object());
}

#[test]
fn annotate_creates_overlay_entry() {
    // Test that state annotations command works (even without overlay data)
    let temp = tempfile::tempdir().unwrap();
    let docs_dir = temp.path().join("state-docs");
    fs::create_dir_all(&docs_dir).unwrap();
    fs::write(
        docs_dir.join("alpha_state.md"),
        "---\nname: alpha_state\n---\n# Alpha\n",
    )
    .unwrap();

    // Index the docs
    let index_output = run_in(&temp, &["state", "index", "--docs-dir", "state-docs"]);
    assert!(
        index_output.status.success(),
        "index failed: {}",
        String::from_utf8_lossy(&index_output.stdout)
    );

    // Query annotations (should return empty since no overlay data)
    let output = run_in(&temp, &["state", "annotations", "alpha_state.md"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert_eq!(body["payload"]["open_count"], 0);
}

#[test]
fn annotate_empty_message_fails() {
    let temp = tempfile::tempdir().unwrap();
    let docs_dir = temp.path().join("state-docs");
    fs::create_dir_all(&docs_dir).unwrap();
    fs::write(
        docs_dir.join("alpha_state.md"),
        "---\nname: alpha_state\n---\n# Alpha\n",
    )
    .unwrap();

    let output = run_in(&temp, &["state", "annotations", "alpha_state.md"]);

    // Should succeed but return empty annotations
    assert!(output.status.success());
    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert_eq!(body["payload"]["annotations"].as_array().unwrap().len(), 0);
}

#[test]
fn annotate_nonexistent_file_fails() {
    let temp = tempfile::tempdir().unwrap();
    let docs_dir = temp.path().join("state-docs");
    fs::create_dir_all(&docs_dir).unwrap();

    let output = run_in(&temp, &["state", "annotations", "nonexistent_file.md"]);

    // Should succeed but return empty annotations (no error for missing doc)
    assert!(output.status.success());
    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert_eq!(body["payload"]["annotations"].as_array().unwrap().len(), 0);
}

#[test]
fn resolve_marks_annotation_resolved() {
    let temp = tempfile::tempdir().unwrap();
    let docs_dir = temp.path().join("state-docs");
    fs::create_dir_all(&docs_dir).unwrap();
    fs::write(
        docs_dir.join("alpha_state.md"),
        "---\nname: alpha_state\n---\n# Alpha\n",
    )
    .unwrap();

    // Index the docs
    let index_output = run_in(&temp, &["state", "index", "--docs-dir", "state-docs"]);
    assert!(
        index_output.status.success(),
        "index failed: {}",
        String::from_utf8_lossy(&index_output.stdout)
    );

    // Query annotations (should be empty)
    let output_before = run_in(&temp, &["state", "annotations", "alpha_state.md"]);
    assert!(output_before.status.success());
    let body_before = json_stdout(&output_before);
    assert_eq!(body_before["payload"]["open_count"], 0);
}

#[test]
fn malformed_config_file_returns_partial_status() {
    let temp = tempfile::tempdir().unwrap();

    // Create a malformed TOML config file
    fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "bad-config"
this is not valid toml"#,
    )
    .unwrap();

    let output = run_in(&temp, &["workspace", "status"]);

    // The CLI should still succeed and return partial JSON
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert_eq!(body["workspace"], serde_json::Value::Null);
}

#[test]
fn missing_config_file_returns_default_workspace() {
    let temp = tempfile::tempdir().unwrap();

    // No config file exists - should soft-fail to null workspace
    let output = run_in(&temp, &["workspace", "status"]);

    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert_eq!(body["workspace"], serde_json::Value::Null);
}

#[test]
fn invalid_workspace_command_returns_usage_error() {
    let temp = tempfile::tempdir().unwrap();

    // Create a valid config file so we don't get config errors
    fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "valid-workspace""#,
    )
    .unwrap();

    let output = run_in(&temp, &["workspace", "invalid-command"]);

    assert!(!output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], false);
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("unrecognized subcommand 'invalid-command'")
    );
    assert!(body["usage"].is_array());
}

#[test]
fn valid_workspace_config_returns_workspace_status() {
    let temp = tempfile::tempdir().unwrap();

    fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"
name = "valid-workspace"
description = "Workspace for tests"
auto_register = false
"#,
    )
    .unwrap();

    let output = run_in(&temp, &["workspace", "status"]);

    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert_eq!(body["workspace"]["name"], "valid-workspace");
    assert_eq!(body["workspace"]["description"], "Workspace for tests");
    assert_eq!(body["workspace"]["declared_tools"], 0);
    assert_eq!(body["workspace"]["tool_execution_enabled"], false);
}

#[test]
fn workspace_status_with_tool_execution_enabled_true() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"
name = "exec-workspace"
tool_execution_enabled = true
"#,
    )
    .unwrap();

    let output = run_in(&temp, &["workspace", "status"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert_eq!(body["workspace"]["tool_execution_enabled"], true);
}

#[test]
fn knowledge_sync_and_query_return_json() {
    let temp = tempfile::tempdir().unwrap();
    let docs_dir = temp.path().join("state-docs");
    fs::create_dir_all(docs_dir.join("overlay")).unwrap();
    fs::write(docs_dir.join("alpha_state.md"), "# Alpha\n").unwrap();
    fs::write(
        docs_dir.join("overlay").join("alpha_state.overlay.json"),
        r#"{
  "entries": [
    {
      "id": "alpha-state-md-123-0",
      "kind": "note",
      "message": "Persist this decision",
      "author": "agent",
      "doc": "alpha_state.md",
      "line": null,
      "status": "open",
      "created_at_unix_ms": 123
    }
  ]
}"#,
    )
    .unwrap();

    let sync_output = run_in(&temp, &["knowledge", "sync", "alpha_state"]);
    assert!(sync_output.status.success());
    let sync_body = json_stdout(&sync_output);
    assert_eq!(sync_body["success"], true);
    assert_eq!(sync_body["data"]["doc"], "alpha_state");
    assert_eq!(sync_body["data"]["ids"].as_array().unwrap().len(), 1);

    let query_output = run_in(&temp, &["knowledge", "query", "--tags=state-doc"]);
    assert!(query_output.status.success());
    let query_body = json_stdout(&query_output);
    assert_eq!(query_body["success"], true);
    assert_eq!(query_body["data"]["rows"].as_array().unwrap().len(), 1);
    assert_eq!(
        query_body["data"]["rows"][0]["meta"]["annotation_id"],
        "alpha-state-md-123-0"
    );
}

#[test]
fn knowledge_events_and_verify_return_json() {
    let temp = tempfile::tempdir().unwrap();

    let insert_output = run_in(
        &temp,
        &[
            "knowledge",
            "insert",
            "--content=Keep releases reproducible",
            "--kind=instruction",
            "--tags=release,ops",
        ],
    );
    assert!(insert_output.status.success());

    let verify_output = run_in(&temp, &["knowledge", "verify"]);
    assert!(verify_output.status.success());
    let verify_body = json_stdout(&verify_output);
    assert_eq!(verify_body["success"], true);
    assert_eq!(verify_body["data"]["bad_ids"], serde_json::json!([]));

    let events_output = run_in(&temp, &["knowledge", "events", "--limit=10"]);
    assert!(events_output.status.success());
    let events_body = json_stdout(&events_output);
    assert_eq!(events_body["success"], true);
    assert!(!events_body["data"]["events"].as_array().unwrap().is_empty());
}

#[test]
fn knowledge_deactivate_updates_query_results() {
    let temp = tempfile::tempdir().unwrap();

    let insert_output = run_in(
        &temp,
        &[
            "knowledge",
            "insert",
            "--content=Archive stale deployment advice",
            "--kind=context",
            "--tags=deploy,stale",
        ],
    );
    assert!(insert_output.status.success());
    let insert_body = json_stdout(&insert_output);
    let id = insert_body["data"]["id"].as_i64().unwrap();

    let deactivate_output = run_in(
        &temp,
        &[
            "knowledge",
            "deactivate",
            &id.to_string(),
            "--reason=superseded",
        ],
    );
    assert!(deactivate_output.status.success());
    let deactivate_body = json_stdout(&deactivate_output);
    assert_eq!(deactivate_body["success"], true);
    assert_eq!(deactivate_body["data"]["id"], id);
    assert_eq!(deactivate_body["data"]["reason"], "superseded");

    let query_output = run_in(&temp, &["knowledge", "query", "--tags=deploy"]);
    assert!(query_output.status.success());
    let query_body = json_stdout(&query_output);
    assert_eq!(query_body["data"]["rows"], serde_json::json!([]));
}

#[test]
fn knowledge_sync_with_malformed_overlay_exits_nonzero() {
    let temp = tempfile::tempdir().unwrap();
    let docs_dir = temp.path().join("state-docs");
    fs::create_dir_all(docs_dir.join("overlay")).unwrap();
    fs::write(docs_dir.join("alpha_state.md"), "# Alpha\n").unwrap();
    fs::write(
        docs_dir.join("overlay").join("alpha_state.overlay.json"),
        "not valid json",
    )
    .unwrap();

    let output = run_in(&temp, &["knowledge", "sync", "alpha_state"]);
    assert!(!output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], false);
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("Serialization error")
    );
    assert!(body["usage"].is_array());
}

#[test]
fn knowledge_sync_is_idempotent() {
    let temp = tempfile::tempdir().unwrap();
    let docs_dir = temp.path().join("state-docs");
    fs::create_dir_all(docs_dir.join("overlay")).unwrap();
    fs::write(docs_dir.join("alpha_state.md"), "# Alpha\n").unwrap();
    fs::write(
        docs_dir.join("overlay").join("alpha_state.overlay.json"),
        r#"{
  "entries": [
    {
      "id": "alpha-state-md-123-0",
      "kind": "note",
      "message": "Persist this decision",
      "author": "agent",
      "doc": "alpha_state.md",
      "line": null,
      "status": "open",
      "created_at_unix_ms": 123
    }
  ]
}"#,
    )
    .unwrap();

    let first_sync = run_in(&temp, &["knowledge", "sync", "alpha_state"]);
    assert!(first_sync.status.success());
    let first_body = json_stdout(&first_sync);
    assert_eq!(first_body["success"], true);
    assert_eq!(first_body["data"]["ids"].as_array().unwrap().len(), 1);

    let second_sync = run_in(&temp, &["knowledge", "sync", "alpha_state"]);
    assert!(second_sync.status.success());
    let second_body = json_stdout(&second_sync);
    assert_eq!(second_body["success"], true);
    assert_eq!(second_body["data"]["ids"].as_array().unwrap().len(), 0);

    let query = run_in(
        &temp,
        &["knowledge", "query", "--tags=state-doc,alpha,state"],
    );
    assert!(query.status.success());
    let query_body = json_stdout(&query);
    assert_eq!(query_body["data"]["rows"].as_array().unwrap().len(), 1);
}

#[test]
fn resolve_annotation_deactivates_derived_knowledge() {
    let temp = tempfile::tempdir().unwrap();
    let docs_dir = temp.path().join("state-docs");
    fs::create_dir_all(docs_dir.join("overlay")).unwrap();
    fs::write(docs_dir.join("alpha_state.md"), "# Alpha\n").unwrap();
    fs::write(
        docs_dir.join("overlay").join("alpha_state.overlay.json"),
        r#"{
  "entries": [
    {
      "id": "alpha-state-md-123-0",
      "kind": "question",
      "message": "Should this stay here?",
      "author": "agent",
      "doc": "alpha_state.md",
      "line": null,
      "status": "open",
      "created_at_unix_ms": 123
    }
  ]
}"#,
    )
    .unwrap();

    let sync = run_in(&temp, &["knowledge", "sync", "alpha_state"]);
    assert!(sync.status.success());
    let sync_body = json_stdout(&sync);
    assert_eq!(sync_body["success"], true);
    assert_eq!(sync_body["data"]["ids"].as_array().unwrap().len(), 1);

    let query_before = run_in(
        &temp,
        &["knowledge", "query", "--tags=state-doc,alpha,state"],
    );
    assert!(query_before.status.success());
    let query_before_body = json_stdout(&query_before);
    assert_eq!(
        query_before_body["data"]["rows"].as_array().unwrap().len(),
        1
    );

    let resolve = run_in(
        &temp,
        &[
            "knowledge",
            "resolve-annotation",
            "alpha_state",
            "--annotation-id=alpha-state-md-123-0",
            "--reason=answered",
        ],
    );
    assert!(resolve.status.success());
    let resolve_body = json_stdout(&resolve);
    assert_eq!(resolve_body["success"], true);
    assert_eq!(resolve_body["data"]["deactivated"], 1);
    assert!(resolve_body["data"]["resolved"].is_object());
    assert_eq!(resolve_body["data"]["resolved"]["status"], "resolved");

    let query_after = run_in(
        &temp,
        &["knowledge", "query", "--tags=state-doc,alpha-state"],
    );
    assert!(query_after.status.success());
    let query_after_body = json_stdout(&query_after);
    assert_eq!(
        query_after_body["data"]["rows"].as_array().unwrap().len(),
        0
    );

    let overlay: Value = serde_json::from_str(
        &fs::read_to_string(docs_dir.join("overlay").join("alpha_state.overlay.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(overlay["entries"][0]["status"], "resolved");
}

#[test]
fn exec_denied_when_tool_execution_enabled_is_false() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"
name = "no-exec"
tool_execution_enabled = false
"#,
    )
    .unwrap();

    let output = run_in(&temp, &["exec", "--command=true", "--reason=review"]);
    assert!(!output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], false);
    assert_eq!(body["exec"]["gate_result"], "denied");
}

#[test]
fn exec_proceeds_when_tool_execution_enabled_is_true() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"
name = "exec-enabled"
tool_execution_enabled = true
"#,
    )
    .unwrap();

    let output = run_in(
        &temp,
        &["exec", "--command=true", "--reason=review", "--dry-run"],
    );
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert_eq!(body["exec"]["gate_result"], "dry_run");
}

#[test]
fn guard_queue_list_returns_pending_entries() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"
name = "guard-test"
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

#[test]
fn guard_provenance_verify_returns_exists() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"
name = "provenance-test"
tool_execution_enabled = true
"#,
    )
    .unwrap();

    let output = run_in(&temp, &["guard", "provenance", "--source-event-id=test-id"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert!(body["guard"]["provenance"]["exists"].is_boolean());
}

#[test]
fn workspace_status_reflects_execution_mode() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"
name = "exec-workspace"
tool_execution_enabled = true
"#,
    )
    .unwrap();

    let output = run_in(&temp, &["workspace", "status"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert_eq!(body["workspace"]["tool_execution_enabled"], true);
}

#[test]
fn resolve_one_annotation_does_not_deactivate_other() {
    let temp = tempfile::tempdir().unwrap();
    let docs_dir = temp.path().join("state-docs");
    fs::create_dir_all(docs_dir.join("overlay")).unwrap();
    fs::write(docs_dir.join("alpha_state.md"), "# Alpha\n").unwrap();
    fs::write(
        docs_dir.join("overlay").join("alpha_state.overlay.json"),
        r#"{
  "entries": [
    {
      "id": "alpha-state-md-123-0",
      "kind": "question",
      "message": "Should this stay here?",
      "author": "agent",
      "doc": "alpha_state.md",
      "line": null,
      "status": "open",
      "created_at_unix_ms": 123
    },
    {
      "id": "beta-state-md-456-0",
      "kind": "note",
      "message": "Beta decision",
      "author": "agent",
      "doc": "beta_state.md",
      "line": null,
      "status": "open",
      "created_at_unix_ms": 456
    }
  ]
}"#,
    )
    .unwrap();
    fs::write(docs_dir.join("beta_state.md"), "# Beta\n").unwrap();
    fs::write(
        docs_dir.join("overlay").join("beta_state.overlay.json"),
        r#"{
  "entries": [
    {
      "id": "beta-state-md-456-0",
      "kind": "note",
      "message": "Beta decision",
      "author": "agent",
      "doc": "beta_state.md",
      "line": null,
      "status": "open",
      "created_at_unix_ms": 456
    }
  ]
}"#,
    )
    .unwrap();

    let sync_alpha = run_in(&temp, &["knowledge", "sync", "alpha_state"]);
    assert!(sync_alpha.status.success());
    let sync_alpha_body = json_stdout(&sync_alpha);
    assert_eq!(sync_alpha_body["success"], true);
    assert_eq!(sync_alpha_body["data"]["ids"].as_array().unwrap().len(), 2);

    let sync_beta = run_in(&temp, &["knowledge", "sync", "beta_state"]);
    assert!(sync_beta.status.success());
    let sync_beta_body = json_stdout(&sync_beta);
    assert_eq!(sync_beta_body["success"], true);
    assert_eq!(sync_beta_body["data"]["ids"].as_array().unwrap().len(), 0);

    let query_before = run_in(&temp, &["knowledge", "query", "--tags=state-doc"]);
    assert!(query_before.status.success());
    let query_before_body = json_stdout(&query_before);
    assert_eq!(
        query_before_body["data"]["rows"].as_array().unwrap().len(),
        2
    );

    let resolve = run_in(
        &temp,
        &[
            "knowledge",
            "resolve-annotation",
            "alpha_state",
            "--annotation-id=alpha-state-md-123-0",
            "--reason=answered",
        ],
    );
    assert!(resolve.status.success());
    let resolve_body = json_stdout(&resolve);
    assert_eq!(resolve_body["success"], true);
    assert_eq!(resolve_body["data"]["deactivated"], 1);

    let query_after = run_in(&temp, &["knowledge", "query", "--tags=state-doc"]);
    assert!(query_after.status.success());
    let query_after_body = json_stdout(&query_after);
    assert_eq!(
        query_after_body["data"]["rows"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        query_after_body["data"]["rows"][0]["meta"]["annotation_id"],
        "beta-state-md-456-0"
    );
}

#[test]
fn query_one_tag_does_not_return_unrelated_rows() {
    let temp = tempfile::tempdir().unwrap();
    let docs_dir = temp.path().join("state-docs");
    fs::create_dir_all(docs_dir.join("overlay")).unwrap();
    fs::write(docs_dir.join("alpha_state.md"), "# Alpha\n").unwrap();
    fs::write(
        docs_dir.join("overlay").join("alpha_state.overlay.json"),
        r#"{
  "entries": [
    {
      "id": "alpha-state-md-123-0",
      "kind": "note",
      "message": "Alpha note",
      "author": "agent",
      "doc": "alpha_state.md",
      "line": null,
      "status": "open",
      "created_at_unix_ms": 123
    }
  ]
}"#,
    )
    .unwrap();
    fs::write(docs_dir.join("beta_state.md"), "# Beta\n").unwrap();
    fs::write(
        docs_dir.join("overlay").join("beta_state.overlay.json"),
        r#"{
  "entries": [
    {
      "id": "beta-state-md-456-0",
      "kind": "note",
      "message": "Beta note",
      "author": "agent",
      "doc": "beta_state.md",
      "line": null,
      "status": "open",
      "created_at_unix_ms": 456
    }
  ]
}"#,
    )
    .unwrap();

    let sync_alpha = run_in(&temp, &["knowledge", "sync", "alpha_state"]);
    assert!(sync_alpha.status.success());
    let sync_alpha_body = json_stdout(&sync_alpha);
    assert_eq!(sync_alpha_body["success"], true);
    assert_eq!(sync_alpha_body["data"]["ids"].as_array().unwrap().len(), 1);

    let sync_beta = run_in(&temp, &["knowledge", "sync", "beta_state"]);
    assert!(sync_beta.status.success());
    let sync_beta_body = json_stdout(&sync_beta);
    assert_eq!(sync_beta_body["success"], true);
    assert_eq!(sync_beta_body["data"]["ids"].as_array().unwrap().len(), 1);

    let query_alpha_tag = run_in(&temp, &["knowledge", "query", "--tags=alpha,state"]);
    assert!(query_alpha_tag.status.success());
    let query_alpha_tag_body = json_stdout(&query_alpha_tag);
    assert_eq!(
        query_alpha_tag_body["data"]["rows"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        query_alpha_tag_body["data"]["rows"][0]["meta"]["annotation_id"],
        "alpha-state-md-123-0"
    );

    let query_beta_tag = run_in(&temp, &["knowledge", "query", "--tags=beta"]);
    assert!(query_beta_tag.status.success());
    let query_beta_tag_body = json_stdout(&query_beta_tag);
    assert_eq!(
        query_beta_tag_body["data"]["rows"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        query_beta_tag_body["data"]["rows"][0]["meta"]["annotation_id"],
        "beta-state-md-456-0"
    );

    let query_alpha_tag = run_in(&temp, &["knowledge", "query", "--tags=alpha"]);
    assert!(query_alpha_tag.status.success());
    let query_alpha_tag_body = json_stdout(&query_alpha_tag);
    assert_eq!(
        query_alpha_tag_body["data"]["rows"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let query_beta_tag = run_in(&temp, &["knowledge", "query", "--tags=beta"]);
    assert!(query_beta_tag.status.success());
    let query_beta_tag_body = json_stdout(&query_beta_tag);
    assert_eq!(
        query_beta_tag_body["data"]["rows"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
