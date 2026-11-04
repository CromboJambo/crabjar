//! Confidence and provenance tests for ExecutionGate.

use crate::gate::ExecutionGate;
use crate::gate_context::GateContext;
use crate::gate_result::GateResult;
use crate::guard_db::GuardDb;
use crate::trust::TrustScore;
use tempfile::tempdir;

#[test]
fn test_gate_interrupts_below_confidence_floor() {
    let dir = tempdir().unwrap();
    let db = GuardDb::open(dir.path().join("guard.db")).unwrap();
    let gate = ExecutionGate::new(&db, false, dir.path());

    let ctx = GateContext {
        action_type: "echo",
        command: "echo",
        args: vec!["hello".to_string()],
        trust_layer: 3,
        confidence: TrustScore::new(0.1),
        source_event_id: Some("evt-7"),
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
fn test_gate_denies_missing_provenance() {
    let dir = tempdir().unwrap();
    let db = GuardDb::open(dir.path().join("guard.db")).unwrap();
    let gate = ExecutionGate::new(&db, false, dir.path());

    let ctx = GateContext {
        action_type: "echo",
        command: "echo",
        args: vec!["hello".to_string()],
        trust_layer: 3,
        confidence: TrustScore::new(0.9),
        source_event_id: Some("nonexistent-provenance"),
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
fn test_gate_proceeds_with_valid_provenance() {
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
