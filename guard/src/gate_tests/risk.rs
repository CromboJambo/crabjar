//! Dry run and command risk tests for ExecutionGate.

use crate::gate::ExecutionGate;
use crate::gate_context::GateContext;
use crate::gate_result::GateResult;
use crate::guard_db::GuardDb;
use crate::trust::TrustScore;
use tempfile::tempdir;

#[test]
fn test_gate_dry_run() {
    let dir = tempdir().unwrap();
    let db = GuardDb::open(dir.path().join("guard.db")).unwrap();
    let gate = ExecutionGate::new(&db, true, dir.path());

    let ctx = GateContext {
        action_type: "rm",
        command: "rm",
        args: vec!["-rf".to_string(), "/".to_string()],
        trust_layer: 0,
        confidence: TrustScore::new(0.65),
        source_event_id: Some("evt-dry-run"),
        can_interrupt: false,
        pid: None,
        scope: None,
        target_scope: None,
        cross_scope_auth: None,
        domains: vec![],
        context_budget: None,
        context_fragment_tokens: None,
    };

    let result = gate.check(ctx).unwrap();
    assert_eq!(result, GateResult::DryRun);
}

#[test]
fn test_high_risk_command_blocked() {
    let dir = tempdir().unwrap();
    let db = GuardDb::open(dir.path().join("guard.db")).unwrap();

    let conn = db.conn();
    conn.execute(
        "INSERT INTO action_requests (id, source_event_id, action_type, payload, trust_layer, confidence, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            "test-action-5",
            "evt-5",
            "rm",
            "-rf /tmp/test",
            3,
            0.9,
            "trust-approved",
        ],
    )
    .unwrap();
    drop(conn);

    let gate = ExecutionGate::new(&db, false, dir.path());

    let ctx = GateContext {
        action_type: "delete",
        command: "rm",
        args: vec!["-rf".to_string(), "/tmp/test".to_string()],
        trust_layer: 4,
        confidence: TrustScore::new(0.95),
        source_event_id: Some("evt-5"),
        can_interrupt: true,
        pid: None,
        scope: None,
        target_scope: None,
        cross_scope_auth: None,
        domains: vec![],
        context_budget: None,
        context_fragment_tokens: None,
    };

    let result = gate.check(ctx).unwrap();
    assert!(matches!(result, GateResult::Interrupted { .. }));
}

#[test]
fn test_medium_risk_command_pending() {
    let dir = tempdir().unwrap();
    let db = GuardDb::open(dir.path().join("guard.db")).unwrap();

    let conn = db.conn();
    conn.execute(
        "INSERT INTO action_requests (id, source_event_id, action_type, payload, trust_layer, confidence, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            "test-action-6",
            "evt-6",
            "git",
            "commit -m test",
            3,
            0.9,
            "trust-approved",
        ],
    )
    .unwrap();
    drop(conn);

    let gate = ExecutionGate::new(&db, false, dir.path());

    let ctx = GateContext {
        action_type: "git_commit",
        command: "git",
        args: vec!["commit".to_string(), "-m".to_string(), "test".to_string()],
        trust_layer: 3,
        confidence: TrustScore::new(0.9),
        source_event_id: Some("evt-6"),
        can_interrupt: true,
        pid: None,
        scope: None,
        target_scope: None,
        cross_scope_auth: None,
        domains: vec![],
        context_budget: None,
        context_fragment_tokens: None,
    };

    let result = gate.check(ctx).unwrap();
    assert_eq!(result, GateResult::Pending);
}
