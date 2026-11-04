//! Context budget tests for ExecutionGate.

use crate::gate::ExecutionGate;
use crate::gate_context::GateContext;
use crate::gate_result::GateResult;
use crate::guard_db::GuardDb;
use crate::trust::TrustScore;
use tempfile::tempdir;

#[test]
fn test_context_budget_proceeds_when_fits() {
    let dir = tempdir().unwrap();
    let db = GuardDb::open(dir.path().join("guard.db")).unwrap();

    let conn = db.conn();
    conn.execute(
        "INSERT INTO action_requests (id, source_event_id, action_type, payload, trust_layer, confidence, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            "test-action-budget-1",
            "evt-budget-1",
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
    let budget = crate::context_budget::ContextBudget::new(1000);

    let ctx = GateContext::new("echo", "echo", vec!["hello".to_string()], 4, TrustScore::new(0.95))
        .with_source_event("evt-budget-1")
        .with_context_budget(budget, 500);

    let result = gate.check(ctx).unwrap();
    assert!(result.is_proceed());
}

#[test]
fn test_context_budget_rejects_when_exhausted() {
    let dir = tempdir().unwrap();
    let db = GuardDb::open(dir.path().join("guard.db")).unwrap();

    let conn = db.conn();
    conn.execute(
        "INSERT INTO action_requests (id, source_event_id, action_type, payload, trust_layer, confidence, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            "test-action-budget-2",
            "evt-budget-2",
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
    let budget = crate::context_budget::ContextBudget::new(1000);

    let ctx = GateContext::new("echo", "echo", vec!["hello".to_string()], 4, TrustScore::new(0.95))
        .with_source_event("evt-budget-2")
        .with_context_budget(budget, 2000);

    let result = gate.check(ctx).unwrap();
    assert!(result.is_context_exhausted());
    if let GateResult::ContextExhausted {
        used,
        budget: b,
        remaining,
    } = &result
    {
        assert_eq!(*used, 0);
        assert_eq!(*b, 1000);
        assert_eq!(*remaining, 1000);
    } else {
        panic!("expected ContextExhausted");
    }
}

#[test]
fn test_context_budget_skipped_when_none() {
    let dir = tempdir().unwrap();
    let db = GuardDb::open(dir.path().join("guard.db")).unwrap();

    let conn = db.conn();
    conn.execute(
        "INSERT INTO action_requests (id, source_event_id, action_type, payload, trust_layer, confidence, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            "test-action-budget-3",
            "evt-budget-3",
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

    // No context_budget set — should proceed regardless of fragment size
    let ctx = GateContext::new("echo", "echo", vec!["hello".to_string()], 4, TrustScore::new(0.95))
        .with_source_event("evt-budget-3");

    let result = gate.check(ctx).unwrap();
    assert!(result.is_proceed());
}

#[test]
fn test_context_budget_rejects_oversized_fragment() {
    let dir = tempdir().unwrap();
    let db = GuardDb::open(dir.path().join("guard.db")).unwrap();

    let conn = db.conn();
    conn.execute(
        "INSERT INTO action_requests (id, source_event_id, action_type, payload, trust_layer, confidence, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            "test-action-oversized",
            "evt-oversized",
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
    let budget = crate::context_budget::ContextBudget::new(128_000);

    // Fragment of 15K exceeds the 10K hard cap
    let ctx = GateContext::new("echo", "echo", vec!["hello".to_string()], 4, TrustScore::new(0.95))
        .with_source_event("evt-oversized")
        .with_context_budget(budget, 15_000);

    let result = gate.check(ctx).unwrap();
    assert!(matches!(result, GateResult::OversizedFragment { .. }));
    if let GateResult::OversizedFragment { actual, max } = result {
        assert_eq!(actual, 15_000);
        assert_eq!(max, 10_000);
    } else {
        panic!("expected OversizedFragment");
    }
}

#[test]
fn test_context_budget_allows_at_hard_cap() {
    let dir = tempdir().unwrap();
    let db = GuardDb::open(dir.path().join("guard.db")).unwrap();

    let conn = db.conn();
    conn.execute(
        "INSERT INTO action_requests (id, source_event_id, action_type, payload, trust_layer, confidence, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            "test-action-at-cap",
            "evt-at-cap",
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
    let budget = crate::context_budget::ContextBudget::new(128_000);

    // Exactly at the 10K cap — should proceed
    let ctx = GateContext::new("echo", "echo", vec!["hello".to_string()], 4, TrustScore::new(0.95))
        .with_source_event("evt-at-cap")
        .with_context_budget(budget, 10_000);

    let result = gate.check(ctx).unwrap();
    assert!(result.is_proceed());
}
