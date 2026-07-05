//! Full E2E integration tests — deep regression gate (~5min).
//!
//! These cover the complete execution pipeline with real DB-backed subsystems.
//! Gate behind `#[cfg(feature = "e2e-full")]` so they only run on merge/nightly.

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

// ──────────────────────────────────────────────
// 1. Exec pipeline with real guard DB (tempfile-backed)
// ──────────────────────────────────────────────

/// Full exec pipeline: config → gate check → concierge enforcement → result.
#[test]
fn full_exec_pipeline_dry_run() {
    let temp = tempfile::tempdir().unwrap();

    std::fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "full-exec"
tool_execution_enabled = true
"#,
    )
    .unwrap();

    // Dry-run should return gate_result: "dry_run" without executing
    let output = run_in(
        &temp,
        &["exec", "--command=echo", "--reason=e2e-test", "--dry-run"],
    );
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert_eq!(body["exec"]["gate_result"], "dry_run");
}

/// Exec pipeline with tool_execution_enabled=false → denied.
#[test]
fn full_exec_pipeline_denied() {
    let temp = tempfile::tempdir().unwrap();

    std::fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "full-exec-denied"
tool_execution_enabled = false
"#,
    )
    .unwrap();

    let output = run_in(
        &temp,
        &["exec", "--command=echo", "--reason=e2e-test"],
    );
    assert!(!output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], false);
    assert_eq!(body["exec"]["gate_result"], "denied");
}

/// Exec pipeline with tool_execution_enabled=true → proceeds (dry-run).
#[test]
fn full_exec_pipeline_proceeds() {
    let temp = tempfile::tempdir().unwrap();

    std::fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "full-exec-proceed"
tool_execution_enabled = true
"#,
    )
    .unwrap();

    let output = run_in(
        &temp,
        &["exec", "--command=true", "--reason=e2e-test", "--dry-run"],
    );
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert_eq!(body["exec"]["gate_result"], "dry_run");
}

// ──────────────────────────────────────────────
// 2. Domain allowlist enforcement (deny + allow paths)
// ──────────────────────────────────────────────

/// Direct library test: domain allowlist blocks unknown domains.
#[test]
fn full_domain_allowlist_blocks_unknown() {
    let allowlist = crabjar_guard::DomainAllowlist::new();

    // Known domains should pass
    assert!(allowlist.check("github.com").is_ok());
    assert!(allowlist.check("crates.io").is_ok());
    assert!(allowlist.check("localhost").is_ok());

    // Unknown domain should be blocked
    let result = allowlist.check("evil-malware.xyz");
    assert!(result.is_err());
    match result.unwrap_err() {
        crabjar_guard::DomainCheckError::NotAllowed(domain) => {
            assert_eq!(domain, "evil-malware.xyz");
        }
        _ => panic!("expected NotAllowed error"),
    }
}

/// Direct library test: trust layer enforcement.
#[test]
fn full_domain_allowlist_trust_layers() {
    let entries = vec![
        crabjar_guard::DomainEntry::new(
            "trusted.example.com",
            crabjar_guard::DomainTrustLevel::Trusted,
            "test",
        ),
        crabjar_guard::DomainEntry::new(
            "monitored.example.com",
            crabjar_guard::DomainTrustLevel::Monitored,
            "test",
        ),
        crabjar_guard::DomainEntry::new(
            "restricted.example.com",
            crabjar_guard::DomainTrustLevel::Restricted,
            "test",
        ),
    ];
    let allowlist = crabjar_guard::DomainAllowlist::with_entries(entries);

    // Layer 3 (high) can access all domains
    assert!(allowlist.check_for_trust_layer("trusted.example.com", 3).is_ok());
    assert!(allowlist.check_for_trust_layer("monitored.example.com", 3).is_ok());
    assert!(allowlist.check_for_trust_layer("restricted.example.com", 3).is_ok());

    // Layer 2 (medium) can access trusted + monitored, but not restricted
    assert!(allowlist.check_for_trust_layer("trusted.example.com", 2).is_ok());
    assert!(allowlist.check_for_trust_layer("monitored.example.com", 2).is_ok());
    assert!(allowlist.check_for_trust_layer("restricted.example.com", 2).is_err());

    // Layer 1 (low) can access trusted only
    assert!(allowlist.check_for_trust_layer("trusted.example.com", 1).is_ok());
    assert!(allowlist.check_for_trust_layer("monitored.example.com", 1).is_err());
    assert!(allowlist.check_for_trust_layer("restricted.example.com", 1).is_err());
}

/// Direct library test: wildcard domain matching.
#[test]
fn full_domain_allowlist_wildcards() {
    let allowlist = crabjar_guard::DomainAllowlist::new();

    // Wildcard *.githubusercontent.com should match subdomains
    assert!(allowlist.check("avatars.githubusercontent.com").is_ok());
    assert!(allowlist.check("raw.githubusercontent.com").is_ok());

    // But not unrelated domains
    assert!(allowlist.check("notgithub.com").is_err());
}

// ──────────────────────────────────────────────
// 3. Scope isolation checks (cross-scope blocking)
// ──────────────────────────────────────────────

/// Direct library test: different user/project scopes are mutually inaccessible.
#[test]
fn full_scope_isolation_different_users() {
    let scope_a = crabjar_guard::Scope::user_project("alice", "project-a");
    let scope_b = crabjar_guard::Scope::user_project("bob", "project-b");

    assert!(!scope_a.can_access(&scope_b));
    assert!(!scope_b.can_access(&scope_a));
}

/// Direct library test: same user, different projects are mutually inaccessible.
#[test]
fn full_scope_isolation_same_user_different_projects() {
    let scope_x = crabjar_guard::Scope::user_project("alice", "project-x");
    let scope_y = crabjar_guard::Scope::user_project("alice", "project-y");

    assert!(!scope_x.can_access(&scope_y));
    assert!(!scope_y.can_access(&scope_x));
}

/// Direct library test: same user + project scopes are accessible.
#[test]
fn full_scope_isolation_same_user_and_project() {
    let scope_1 = crabjar_guard::Scope::user_project("alice", "project-a");
    let scope_2 = crabjar_guard::Scope::user_project("alice", "project-a");

    assert!(scope_1.can_access(&scope_2));
    assert!(scope_2.can_access(&scope_1));
}

/// Direct library test: CrossScopeAuth auto-construction.
#[test]
fn full_cross_scope_auth_auto() {
    let scope_a = crabjar_guard::Scope::user_project("alice", "project-a");
    let scope_b = crabjar_guard::Scope::user_project("alice", "project-b");

    // Same user, different projects → should auto-construct CrossScopeAuth
    let auth = crabjar_guard::CrossScopeAuth::auto_for_scopes(&scope_a, &scope_b);
    assert!(auth.is_some());
    if let Some(ref a) = auth {
        assert_eq!(a.actor_scope, scope_a);
        assert_eq!(a.target_scope, scope_b);
    }

    // Same scope → should return None (no-op)
    let no_auth = crabjar_guard::CrossScopeAuth::auto_for_scopes(&scope_a, &scope_a);
    assert!(no_auth.is_none());
}

// ──────────────────────────────────────────────
// 4. Telemetry flight recorder write/read cycle
// ──────────────────────────────────────────────

/// Direct library test: flight recorder init → execute_command → query_records.
#[tokio::test]
async fn full_telemetry_flight_recorder_cycle() {
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("flight.db");

    // Open connection and create flight recorder
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let mut recorder = crabjar_telemetry::flight_recorder::FlightRecorder::new(
        &conn,
        "e2e-test-session",
    );

    // Init schema
    recorder.init().expect("flight recorder init failed");

    // Execute a command (spawns echo subprocess)
    let cmd_id = recorder
        .execute_command("echo", &["hello".to_string()], temp.path().to_str().unwrap(), "e2e-test")
        .await
        .expect("execute_command failed");

    assert!(!cmd_id.is_empty());

    // Query records back
    let records = recorder.query_records(10).expect("query_records failed");
    assert!(records.len() >= 1);

    let record = &records[0];
    assert_eq!(record.command, "echo");
}

// ──────────────────────────────────────────────
// 5. Agent loop tick with persistence (tempfile-backed WorkItemStore)
// ──────────────────────────────────────────────

/// Direct library test: WorkItemStore create → save → load → update → list_ids.
#[tokio::test]
async fn full_agent_loop_persistence() {
    use crabjar_host_core::{Status, TaskStatus};
    use crabjar_host_agent::work_item_store::{WorkItem, WorkItemStore};
    use uuid::Uuid;

    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("work_items.db");

    let store = WorkItemStore::open(db_path).expect("WorkItemStore open failed");

    // Create a work item using the builder API from host-core
    let mut wi = WorkItem::new("test-task");
    wi.add_task("Step one");
    wi.add_task("Step two");
    wi.update_task(0, TaskStatus::Completed, Some("done".into()));
    wi.set_confidence(0.5);

    // Save to store (async)
    store.save(&wi).await.expect("save failed");

    // Load it back
    let loaded = store.load(wi.id).await.expect("load failed");
    assert_eq!(loaded.objective, "test-task");
    assert_eq!(loaded.plan.len(), 2);
    assert!((loaded.confidence - 0.5).abs() < f32::EPSILON);

    // Update status
    wi.set_status(Status::Executing { current_task: Some(1) });
    wi.set_confidence(0.95);
    store.save(&wi).await.expect("update save failed");

    let updated = store.load(wi.id).await.expect("reload failed");
    assert!(matches!(updated.status, Status::Executing { .. }));
    assert!((updated.confidence - 0.95).abs() < f32::EPSILON);

    // List all item IDs
    let ids = store.list_ids().await.expect("list failed");
    assert_eq!(ids.len(), 1);
}

// ──────────────────────────────────────────────
// 6. Tool discovery across all 4 layers
// ──────────────────────────────────────────────

/// CLI test: `crabjar tool discover` discovers tools from project root.
#[test]
fn full_tool_discovery() {
    let temp = tempfile::tempdir().unwrap();

    std::fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "full-tool-discover"
"#,
    )
    .unwrap();

    let output = run_in(&temp, &["tool", "discover"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
}

/// CLI test: `crabjar tool list` with type filter.
#[test]
fn full_tool_list_with_filter() {
    let temp = tempfile::tempdir().unwrap();

    std::fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "full-tool-list"
"#,
    )
    .unwrap();

    let output = run_in(&temp, &["tool", "list", "--type=command"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
}

// ──────────────────────────────────────────────
// 7. Guard subcommands (queue, approve, reject, resolution)
// ──────────────────────────────────────────────

/// CLI test: `crabjar guard queue` lists entries with status filter.
#[test]
fn full_guard_queue_subcommand() {
    let temp = tempfile::tempdir().unwrap();

    std::fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "full-guard-queue"
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

/// CLI test: `crabjar guard provenance` verifies source event ID.
#[test]
fn full_guard_provenance_subcommand() {
    let temp = tempfile::tempdir().unwrap();

    std::fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "full-guard-prov"
tool_execution_enabled = true
"#,
    )
    .unwrap();

    let output = run_in(
        &temp,
        &["guard", "provenance", "--source-event-id=e2e-test-123"],
    );
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert!(body["guard"]["provenance"]["exists"].is_boolean());
}

/// CLI test: `crabjar guard resolution` shows trust resolution chain.
#[test]
fn full_guard_resolution_subcommand() {
    let temp = tempfile::tempdir().unwrap();

    std::fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "full-guard-res"
tool_execution_enabled = true
"#,
    )
    .unwrap();

    let output = run_in(&temp, &["guard", "resolution"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
}

/// CLI test: `crabjar guard grant` grants PID trust access.
#[test]
fn full_guard_grant_subcommand() {
    let temp = tempfile::tempdir().unwrap();

    std::fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "full-guard-grant"
tool_execution_enabled = true
"#,
    )
    .unwrap();

    // Grant trust for PID 1 (current process) with layer 3
    let output = run_in(&temp, &["guard", "grant", "--pid=1", "--trust-layer=3"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
}

/// CLI test: `crabjar guard revoke` revokes PID trust access.
#[test]
fn full_guard_revoke_subcommand() {
    let temp = tempfile::tempdir().unwrap();

    std::fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "full-guard-rev"
tool_execution_enabled = true
"#,
    )
    .unwrap();

    // Revoke trust for PID 1
    let output = run_in(&temp, &["guard", "revoke", "--pid=1"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
}

/// CLI test: `crabjar guard approve` approves a pending action.
#[test]
fn full_guard_approve_subcommand() {
    let temp = tempfile::tempdir().unwrap();

    std::fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "full-guard-approve"
tool_execution_enabled = true
"#,
    )
    .unwrap();

    // Approve a non-existent action ID — should succeed (no-op) or return structured response
    let output = run_in(
        &temp,
        &[
            "guard",
            "approve",
            "--action-id=nonexistent-e2e-test-id",
        ],
    );

    // The command may succeed with a message about the action not being found
    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
}

/// CLI test: `crabjar guard reject` rejects a pending action.
#[test]
fn full_guard_reject_subcommand() {
    let temp = tempfile::tempdir().unwrap();

    std::fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "full-guard-reject"
tool_execution_enabled = true
"#,
    )
    .unwrap();

    // Reject a non-existent action ID — should succeed (no-op) or return structured response
    let output = run_in(
        &temp,
        &[
            "guard",
            "reject",
            "--action-id=nonexistent-e2e-test-id",
            "--reason=test-rejection",
        ],
    );

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
}

// ──────────────────────────────────────────────
// 8. Guard DB init + schema verification
// ──────────────────────────────────────────────

/// Direct library test: GuardDb open → verify schema tables exist.
#[test]
fn full_guard_db_schema() {
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("guard.db");

    let guard_db = crabjar_guard::GuardDb::open(&db_path).expect("GuardDb open failed");

    // Verify schema tables exist
    let conn = guard_db.conn();
    let table_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table'",
            [],
            |r| r.get(0),
        )
        .expect("schema query failed");

    // Should have at least the core tables: action_requests, trust_resolutions, pending_queue, interrupted_log, etc.
    assert!(table_count >= 4, "expected >= 4 tables, got {}", table_count);
}

/// Direct library test: GuardDb persist + retrieve pending queue entry.
#[test]
fn full_guard_db_pending_queue() {
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("guard.db");

    let guard_db = crabjar_guard::GuardDb::open(&db_path).expect("GuardDb open failed");

    // Create a pending queue entry (matching actual struct fields)
    let entry = crabjar_guard::PendingQueueEntry {
        id: "e2e-test-entry".to_string(),
        gate_result_id: "gr-e2e-1".to_string(),
        action_type: "exec".to_string(),
        command: "echo test".to_string(),
        args: vec!["hello".to_string()],
        trust_layer: 3,
        confidence: 0.9,
        source_event_id: Some("e2e-source-123".to_string()),
        queued_at: chrono::Utc::now().timestamp_millis(),
        reason: "e2e-test".to_string(),
    };

    guard_db
        .persist_pending_queue_entry(&entry)
        .expect("persist pending entry failed");

    // Retrieve it back using read_pending_queue (no filter param)
    let entries = guard_db.read_pending_queue().expect("read pending queue failed");

    assert!(entries.len() >= 1);
    assert_eq!(entries[0].id, "e2e-test-entry");
}

// ──────────────────────────────────────────────
// 9. Knowledge store full lifecycle
// ──────────────────────────────────────────────

/// CLI test: knowledge insert → verify → events → deactivate → query confirms removal.
#[test]
fn full_knowledge_lifecycle() {
    let temp = tempfile::tempdir().unwrap();

    // Insert an entry
    let insert_output = run_in(
        &temp,
        &[
            "knowledge",
            "insert",
            "--content=Full lifecycle test entry",
            "--kind=context",
            "--tags=lifecycle,e2e",
        ],
    );
    assert!(insert_output.status.success());

    let insert_body = json_stdout(&insert_output);
    assert_eq!(insert_body["success"], true);
    let id: i64 = insert_body["data"]["id"].as_i64().unwrap();

    // Verify integrity
    let verify_output = run_in(&temp, &["knowledge", "verify"]);
    assert!(verify_output.status.success());
    let verify_body = json_stdout(&verify_output);
    assert_eq!(verify_body["success"], true);
    assert_eq!(verify_body["bad_ids"], Value::Array(vec![]));

    // List events
    let events_output = run_in(
        &temp,
        &["knowledge", "events", "--limit=10"],
    );
    assert!(events_output.status.success());
    let events_body = json_stdout(&events_output);
    assert_eq!(events_body["success"], true);
    assert!(!events_body["events"].as_array().unwrap().is_empty());

    // Deactivate the entry
    let deactivate_output = run_in(
        &temp,
        &[
            "knowledge",
            "deactivate",
            &id.to_string(),
            "--reason=lifecycle-test",
        ],
    );
    assert!(deactivate_output.status.success());

    let deactivate_body = json_stdout(&deactivate_output);
    assert_eq!(deactivate_body["success"], true);
    assert_eq!(deactivate_body["id"], id);

    // Query should return no results for the deactivated entry's tags
    let query_output = run_in(
        &temp,
        &["knowledge", "query", "--tags=lifecycle,e2e"],
    );
    assert!(query_output.status.success());
    let query_body = json_stdout(&query_output);
    assert_eq!(query_body["success"], true);
    // Rows should be empty since the entry was deactivated
    assert_eq!(query_body["data"]["rows"].as_array().unwrap().len(), 0);
}

// ──────────────────────────────────────────────
// 10. Workspace config edge cases
// ──────────────────────────────────────────────

/// CLI test: malformed TOML → returns null workspace (soft-fail).
#[test]
fn full_workspace_malformed_config() {
    let temp = tempfile::tempdir().unwrap();

    std::fs::write(
        temp.path().join(".crabjar_config.toml"),
        "this is not valid toml {{{",
    )
    .unwrap();

    let output = run_in(&temp, &["workspace", "status"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert_eq!(body["workspace"], Value::Null);
}

/// CLI test: missing config → returns null workspace.
#[test]
fn full_workspace_missing_config() {
    let temp = tempfile::tempdir().unwrap();

    let output = run_in(&temp, &["workspace", "status"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert_eq!(body["workspace"], Value::Null);
}

fn main() {}
