//! Authorization-order invariant tests.
//!
//! Pins the monotonic-authorization contract (pattern adapted from
//! CodeWhale's `AUTHORIZATION_ORDER.md`): once a safety layer blocks or
//! defers an action, no later layer may loosen that decision. An approval
//! granted by one layer is not a universal bypass.
//!
//! Gate layer order (see `ExecutionGate::check`):
//!   1. policy engine pre-check
//!   2. dry-run
//!   3. provenance
//!   4. confidence floor
//!   5. interruptibility
//!   6. trust layer (auto-execute / requires-review)
//!   7. PID trust (revocation)
//!   8. scope isolation (CrossScopeAuth may clear ONLY this layer)
//!   9. command risk
//!  10. domain allowlist
//!  11. context budget
//!
//! Invariant under test: an earlier layer's approval (e.g. trust layer 4
//! auto-execute) cannot be overridden by a later layer's hold, and the
//! sole legitimate loosening mechanism (CrossScopeAuth) is scoped to the
//! scope-isolation layer — it cannot clear provenance, revocation, or
//! command-risk holds.

use std::path::Path;

use crate::gate::ExecutionGate;
use crate::gate_context::GateContext;
use crate::gate_result::GateResult;
use crate::guard_db::GuardDb;
use crate::scope::{CrossScopeAuth, Scope};
use crate::trust::TrustScore;
use tempfile::tempdir;

/// Open a guard DB with a single provenance row so the provenance check passes.
fn db_with_provenance(dir: &Path, event_id: &str) -> GuardDb {
    let db = GuardDb::open(dir.join("guard.db")).unwrap();
    let conn = db.conn();
    conn.execute(
        "INSERT INTO action_requests (id, source_event_id, action_type, payload, trust_layer, confidence, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            format!("act-{}", event_id),
            event_id,
            "echo",
            "hello",
            4,
            0.95,
            "trust-approved",
        ],
    )
    .unwrap();
    drop(conn);
    db
}

/// A maximally-trusted context: layer 4 (annealed, auto-execute) at 0.95 confidence.
/// This is the strongest possible earlier-layer approval the gate can grant.
fn trusted_ctx<'a>(
    source: Option<&'a str>,
    command: &'a str,
    args: Vec<String>,
) -> GateContext<'a> {
    let ctx = GateContext::new("echo", command, args, 4, TrustScore::new(0.95));
    match source {
        Some(id) => ctx.with_source_event(id),
        None => ctx,
    }
}

fn fresh_cross_scope_auth() -> (Scope, Scope, CrossScopeAuth) {
    let actor = Scope::user_project("alice", "project-a");
    let target = Scope::user_project("bob", "project-b");
    let auth = CrossScopeAuth::new(target.clone(), actor.clone(), "migration", "admin-policy");
    (actor, target, auth)
}

// ---------------------------------------------------------------------------
// Trust-layer approval cannot be loosened by later layers
// ---------------------------------------------------------------------------

#[test]
fn trust_layer_cannot_clear_missing_provenance() {
    let dir = tempdir().unwrap();
    let db = GuardDb::open(dir.path().join("guard.db")).unwrap();
    let gate = ExecutionGate::new(&db, false, dir.path());

    // Layer 4 + 0.95 confidence, but no provenance row.
    let result = gate
        .check(trusted_ctx(None, "echo", vec!["hello".into()]))
        .unwrap();
    assert!(
        matches!(result, GateResult::Interrupted { .. }),
        "missing provenance must hold regardless of trust layer: {:?}",
        result
    );
}

#[test]
fn trust_layer_cannot_clear_low_confidence() {
    let dir = tempdir().unwrap();
    let db = db_with_provenance(dir.path(), "evt-floor");
    let gate = ExecutionGate::new(&db, false, dir.path());

    let ctx = GateContext::new(
        "echo",
        "echo",
        vec!["hello".into()],
        4,
        TrustScore::new(0.1),
    )
    .with_source_event("evt-floor");
    let result = gate.check(ctx).unwrap();
    assert!(
        matches!(result, GateResult::Interrupted { .. }),
        "confidence below floor must hold regardless of trust layer: {:?}",
        result
    );
}

#[test]
fn trust_layer_cannot_clear_high_risk_command() {
    let dir = tempdir().unwrap();
    let db = db_with_provenance(dir.path(), "evt-rm");
    let gate = ExecutionGate::new(&db, false, dir.path());

    // Annealed trust at 0.95 cannot authorize `rm`.
    let result = gate
        .check(trusted_ctx(
            Some("evt-rm"),
            "rm",
            vec!["-rf".into(), "/tmp/test".into()],
        ))
        .unwrap();
    assert!(
        matches!(result, GateResult::Interrupted { .. }),
        "high-risk command must hold regardless of trust layer: {:?}",
        result
    );
}

#[test]
fn trust_layer_cannot_clear_medium_risk_review() {
    let dir = tempdir().unwrap();
    let db = db_with_provenance(dir.path(), "evt-git");
    let gate = ExecutionGate::new(&db, false, dir.path());

    // Annealed trust auto-executes, but `git` still requires review.
    let result = gate
        .check(trusted_ctx(
            Some("evt-git"),
            "git",
            vec!["commit".into(), "-m".into(), "test".into()],
        ))
        .unwrap();
    assert!(
        matches!(result, GateResult::Pending),
        "medium-risk command must defer to review regardless of trust layer: {:?}",
        result
    );
}

#[test]
fn trust_layer_cannot_clear_scope_isolation() {
    let dir = tempdir().unwrap();
    let db = db_with_provenance(dir.path(), "evt-scope");
    let gate = ExecutionGate::new(&db, false, dir.path());

    let (actor, target, _no_auth) = fresh_cross_scope_auth();
    let ctx = trusted_ctx(Some("evt-scope"), "echo", vec!["hello".into()])
        .with_scope(actor)
        .with_target_scope(target);
    let result = gate.check(ctx).unwrap();
    assert!(
        matches!(result, GateResult::Interrupted { .. }),
        "cross-scope access without auth must hold regardless of trust layer: {:?}",
        result
    );
}

#[test]
fn pid_revocation_cannot_be_loosened_by_trust_layer() {
    let dir = tempdir().unwrap();
    let db = db_with_provenance(dir.path(), "evt-pid");

    // PID trust that has fully decayed (2h idle > 1h decay interval).
    let conn = db.conn();
    conn.execute(
        "INSERT INTO pid_trust (pid, trust_layer, use_count, last_use, auto_grant, decay_interval, decay_rate)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            4242,
            0,
            0,
            chrono::Utc::now().timestamp() - 7200,
            false,
            3600,
            0.02,
        ],
    )
    .unwrap();
    drop(conn);

    let gate = ExecutionGate::new(&db, false, dir.path());
    let ctx = trusted_ctx(Some("evt-pid"), "echo", vec!["hello".into()]).with_pid(4242);
    let result = gate.check(ctx).unwrap();
    assert!(
        matches!(result, GateResult::Revoked { .. }),
        "decayed PID trust must revoke even when the action's trust layer auto-executes: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// CrossScopeAuth is scoped: it clears ONLY the scope-isolation layer
// ---------------------------------------------------------------------------

#[test]
fn cross_scope_auth_cannot_clear_high_risk_command() {
    let dir = tempdir().unwrap();
    let db = db_with_provenance(dir.path(), "evt-auth-rm");
    let gate = ExecutionGate::new(&db, false, dir.path());

    let (actor, target, auth) = fresh_cross_scope_auth();
    let ctx = trusted_ctx(
        Some("evt-auth-rm"),
        "rm",
        vec!["-rf".into(), "/tmp/test".into()],
    )
    .with_scope(actor)
    .with_target_scope(target)
    .with_cross_scope_auth(auth);
    let result = gate.check(ctx).unwrap();
    assert!(
        matches!(result, GateResult::Interrupted { .. }),
        "CrossScopeAuth must not clear the command-risk layer: {:?}",
        result
    );
}

#[test]
fn cross_scope_auth_cannot_clear_missing_provenance() {
    let dir = tempdir().unwrap();
    let db = GuardDb::open(dir.path().join("guard.db")).unwrap();
    let gate = ExecutionGate::new(&db, false, dir.path());

    let (actor, target, auth) = fresh_cross_scope_auth();
    let ctx = trusted_ctx(None, "echo", vec!["hello".into()])
        .with_scope(actor)
        .with_target_scope(target)
        .with_cross_scope_auth(auth);
    let result = gate.check(ctx).unwrap();
    assert!(
        matches!(result, GateResult::Interrupted { .. }),
        "CrossScopeAuth must not clear the provenance layer: {:?}",
        result
    );
}

#[test]
fn expired_cross_scope_auth_holds_at_scope_layer() {
    let dir = tempdir().unwrap();
    let db = db_with_provenance(dir.path(), "evt-auth-expired");
    let gate = ExecutionGate::new(&db, false, dir.path());

    let (actor, target, auth) = fresh_cross_scope_auth();
    let expired = CrossScopeAuth {
        authorized_at: chrono::Utc::now().timestamp() - 3700, // past the 1h TTL
        ..auth
    };
    let ctx = trusted_ctx(Some("evt-auth-expired"), "echo", vec!["hello".into()])
        .with_scope(actor)
        .with_target_scope(target)
        .with_cross_scope_auth(expired);
    let result = gate.check(ctx).unwrap();
    assert!(
        matches!(result, GateResult::Interrupted { .. }),
        "expired CrossScopeAuth must fall back to the scope hold: {:?}",
        result
    );
}

#[test]
fn cross_scope_auth_clears_only_the_scope_layer() {
    let dir = tempdir().unwrap();
    let db = db_with_provenance(dir.path(), "evt-auth-ok");
    let gate = ExecutionGate::new(&db, false, dir.path());

    // Positive control: valid auth + low-risk command + provenance proceeds.
    // Proves the auth clears its own layer and the invariant above is not
    // just "everything is blocked".
    let (actor, target, auth) = fresh_cross_scope_auth();
    let ctx = trusted_ctx(Some("evt-auth-ok"), "echo", vec!["hello".into()])
        .with_scope(actor)
        .with_target_scope(target)
        .with_cross_scope_auth(auth);
    let result = gate.check(ctx).unwrap();
    assert!(
        result.is_proceed(),
        "valid CrossScopeAuth + low-risk command should proceed: {:?}",
        result
    );
}
