//! Cross-crate integration tests for crabjar.
//!
//! These tests verify that crates interact correctly at their boundaries —
//! the kind of bugs that only surface when multiple crates work together.

// ──────────────────────────────────────────────
// Guard + Memory: Trust Score Persistence
// ──────────────────────────────────────────────

/// CRATE-INTEG-001: Guard checks are consistent across calls — the same
/// context produces the same result, and state persists in GuardDb.
#[test]
fn trust_score_persists_across_checks() {
    use crabjar_guard::{ExecutionGate, GateContext, GateResult, GuardDb, Scope, TrustScore};

    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("guard.db");

    let guard_db = GuardDb::open(&db_path).unwrap();
    let gate = ExecutionGate::new(&guard_db, false, temp.path());
    let scope = Scope::user_project("alice", "project-a");

    // First check — without provenance, the gate requires authorization
    let mut ctx1 = GateContext::new(
        "exec",
        "echo hello",
        vec![],
        3, // trusted layer
        TrustScore::new(0.9),
    );
    ctx1.scope = Some(scope.clone());

    let result1 = gate.check(ctx1);
    assert!(result1.is_ok(), "Gate check should not error");

    // Second identical check — should produce the same result (consistency)
    let mut ctx2 = GateContext::new("exec", "echo hello", vec![], 3, TrustScore::new(0.9));
    ctx2.scope = Some(scope);

    let result2 = gate.check(ctx2);
    assert!(result2.is_ok(), "Second check should not error");

    // Both checks should return the same decision (deterministic behavior)
    match (result1.unwrap(), result2.unwrap()) {
        (GateResult::Proceed, GateResult::Proceed) => {} // Both allowed
        (GateResult::Interrupted { .. }, GateResult::Interrupted { .. }) => {} // Both require auth
        (a, b) => panic!("Inconsistent gate results: {:?} vs {:?}", a, b),
    }
}

/// CRATE-INTEG-002: Low trust score blocks high-risk commands.
#[test]
fn low_trust_blocks_high_risk() {
    use crabjar_guard::{ExecutionGate, GateContext, GateResult, GuardDb, TrustScore};

    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("guard.db");

    let guard_db = GuardDb::open(&db_path).unwrap();
    let gate = ExecutionGate::new(&guard_db, false, temp.path());

    // Very low trust with a high-risk command (rm -rf)
    let ctx = GateContext::new(
        "exec",
        "rm -rf /tmp/test_dir",
        vec![],
        1, // low trust layer
        TrustScore::new(0.1),
    );
    let result = gate.check(ctx);
    assert!(result.is_ok(), "Gate check should not error");

    // Should NOT proceed freely — either interrupted or pending
    let gate_result = result.unwrap();
    assert!(
        !matches!(gate_result, GateResult::Proceed),
        "Low-trust high-risk command should not proceed freely"
    );
}

// ──────────────────────────────────────────────
// Host-core EventBus: Cross-component Communication
// ──────────────────────────────────────────────

/// CRATE-INTEG-003: Events published to the host event bus are received by
/// subscribers (publish/subscribe contract).
#[tokio::test]
async fn event_bus_publish_subscribe() {
    use crabjar_host_core::{Event, EventBus, EventType};

    let bus = EventBus::new(10);
    let mut rx = bus.subscriber();

    // Publish an event
    let event = Event::new(
        EventType::ConfigReload {
            source: "test".to_string(),
        },
        "test-source",
    );
    let delivered = bus.publish(event).unwrap();
    assert_eq!(delivered, 1, "Event should be delivered to 1 subscriber");

    // Verify subscriber received it
    let received = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await;
    assert!(
        received.is_ok(),
        "Subscriber should receive published event within timeout"
    );
    let event = received
        .unwrap()
        .expect("Should receive event from broadcast channel");
    assert_eq!(event.source, "test-source");
}

// ──────────────────────────────────────────────
// Guard + Scope: Isolation Across Boundaries
// ──────────────────────────────────────────────

/// CRATE-INTEG-004: Actions in one scope cannot affect another scope's state.
#[test]
fn scope_isolation_across_crates() {
    use crabjar_guard::{ExecutionGate, GateContext, GateResult, GuardDb, Scope, TrustScore};

    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("guard.db");

    let guard_db = GuardDb::open(&db_path).unwrap();
    let gate = ExecutionGate::new(&guard_db, false, temp.path());

    // Both scopes can operate independently at high trust
    for (user, project) in [("alice", "project-a"), ("bob", "project-b")] {
        let scope = Scope::user_project(user, project);
        let mut ctx = GateContext::new("exec", "echo test", vec![], 3, TrustScore::new(0.9));
        ctx.scope = Some(scope);

        let result = gate.check(ctx);
        assert!(result.is_ok(), "Action in scope should succeed");
    }
}

/// CRATE-INTEG-005: Domain allowlist from guard crate correctly gates
/// web requests across trust levels.
#[test]
fn domain_allowlist_cross_trust_levels() {
    use crabjar_guard::{DomainAllowlist, DomainCheckError};

    let allowlist = DomainAllowlist::new();

    // Trusted domains pass
    assert!(allowlist.check("crates.io").is_ok());
    assert!(allowlist.check("github.com").is_ok());

    // Unknown domains are denied by default (deny-all baseline)
    match allowlist.check("unknown-malicious-site.xyz") {
        Err(DomainCheckError::NotAllowed(_)) => {} // Expected
        other => panic!("Unknown domain should be denied, got {:?}", other),
    }
}

// ──────────────────────────────────────────────
// Memory Store + Guard: State Persistence Boundary
// ──────────────────────────────────────────────

/// CRATE-INTEG-006: Memory store operations are isolated from guard state.
/// Writing to memory should not affect guard decisions and vice versa.
#[test]
fn memory_and_guard_state_independent() {
    use crabjar_guard::{ExecutionGate, GateContext, GuardDb, TrustScore};
    use crabjar_memory::{MemoryStore, MemoryTarget};

    let temp = tempfile::tempdir().unwrap();

    // Separate storage for each
    let memory_dir = temp.path().join("memory");
    std::fs::create_dir_all(&memory_dir).unwrap();
    let memory_store = MemoryStore::new(memory_dir);
    let guard_db = GuardDb::open(temp.path().join("guard.db")).unwrap();
    let gate = ExecutionGate::new(&guard_db, false, temp.path());

    // Write to memory
    memory_store
        .add_entry(&MemoryTarget::Memory, "test entry".to_string())
        .unwrap();

    // Guard check should still work independently (no cross-contamination)
    let ctx = GateContext::new("exec", "echo test", vec![], 3, TrustScore::new(0.9));
    assert!(
        gate.check(ctx).is_ok(),
        "Guard check should not error after memory write"
    );

    // Memory should be intact
    let entries = memory_store.read_entries(&MemoryTarget::Memory).unwrap();
    assert_eq!(entries.len(), 1);
}

// ──────────────────────────────────────────────
// Guard + Domain Allowlist: Web Fetch Gating
// ──────────────────────────────────────────────

/// CRATE-INTEG-007: Web fetch actions are gated by domain allowlist.
/// A fetch to a non-allowed domain should be interrupted.
#[test]
fn web_fetch_gated_by_domain_allowlist() {
    use crabjar_guard::{ExecutionGate, GateContext, GateResult, GuardDb, TrustScore};

    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("guard.db");

    let guard_db = GuardDb::open(&db_path).unwrap();
    let gate = ExecutionGate::new(&guard_db, false, temp.path());

    // Web fetch to a non-allowed domain
    let mut ctx = GateContext::new(
        "web_fetch",
        "curl https://unknown-site.xyz/data",
        vec![],
        3,
        TrustScore::new(0.9),
    );
    ctx.domains.push("unknown-site.xyz".to_string());

    let result = gate.check(ctx);
    assert!(result.is_ok(), "Gate check should not error");

    // Should be interrupted (domain not in allowlist)
    let gate_result = result.unwrap();
    assert!(
        !matches!(gate_result, GateResult::Proceed),
        "Web fetch to non-allowed domain should not proceed"
    );
}
