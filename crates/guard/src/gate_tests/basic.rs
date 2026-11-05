//! Basic trust layer tests for ExecutionGate.

use crate::gate::ExecutionGate;
use crate::gate_context::GateContext;
use crate::gate_result::GateResult;
use crate::guard_db::GuardDb;
use crate::trust::TrustScore;
use tempfile::tempdir;

#[test]
fn test_gate_proceeds_for_trusted_action() {
    let dir = tempdir().unwrap();
    let db = GuardDb::open(dir.path().join("guard.db")).unwrap();

    let conn = db.conn();
    conn.execute(
        "INSERT INTO action_requests (id, source_event_id, action_type, payload, trust_layer, confidence, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            "test-action-1",
            "evt-1",
            "echo",
            "hello",
            4,
            0.95,
            "trust-approved",
        ],
    )
    .unwrap();
    drop(conn);

    let gate = ExecutionGate::new(&db, false, dir.path());

    let ctx = GateContext {
        action_type: "echo",
        command: "echo",
        args: vec!["hello".to_string()],
        trust_layer: 4,
        confidence: TrustScore::new(0.95),
        source_event_id: Some("evt-1"),
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
    assert_eq!(result, GateResult::Proceed);
}

#[test]
fn test_gate_pending_for_working_layer() {
    let dir = tempdir().unwrap();
    let db = GuardDb::open(dir.path().join("guard.db")).unwrap();

    let conn = db.conn();
    conn.execute(
        "INSERT INTO action_requests (id, source_event_id, action_type, payload, trust_layer, confidence, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            "test-action-2",
            "evt-2",
            "echo",
            "hello",
            2,
            0.65,
            "trust-approved",
        ],
    )
    .unwrap();
    drop(conn);

    let gate = ExecutionGate::new(&db, false, dir.path());

    let ctx = GateContext {
        action_type: "echo",
        command: "echo",
        args: vec!["hello".to_string()],
        trust_layer: 2,
        confidence: TrustScore::new(0.65),
        source_event_id: Some("evt-2"),
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

#[test]
fn test_gate_pending_for_low_trust() {
    let dir = tempdir().unwrap();
    let db = GuardDb::open(dir.path().join("guard.db")).unwrap();

    let conn = db.conn();
    conn.execute(
        "INSERT INTO action_requests (id, source_event_id, action_type, payload, trust_layer, confidence, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            "test-action-4",
            "evt-4",
            "echo",
            "hello",
            0,
            0.65,
            "trust-approved",
        ],
    )
    .unwrap();
    drop(conn);

    let gate = ExecutionGate::new(&db, false, dir.path());

    let ctx = GateContext {
        action_type: "echo",
        command: "echo",
        args: vec!["hello".to_string()],
        trust_layer: 0,
        confidence: TrustScore::new(0.65),
        source_event_id: Some("evt-4"),
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
