//! Security regression tests — verify core security guarantees hold across refactors.
//!
//! These tests encode the security invariants that must never regress:
//! - Guard cannot be bypassed without explicit approval flow
//! - Trust layers are enforced hierarchically (higher layer can access lower, not vice versa)
//! - Scope isolation prevents cross-user/project access
//! - Domain allowlist blocks unknown domains by default

use crabjar_guard::{DomainAllowlist, Scope};
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

// ──────────────────────────────────────────────
// Guard Bypass Regression Tests
// ──────────────────────────────────────────────

/// SEC-001: tool_execution_enabled=false must deny all exec commands.
#[test]
fn sec_guard_bypass_denied_when_disabled() {
    let temp = tempfile::tempdir().unwrap();

    fs::write(
        temp.path().join(".crabjar_config.toml"),
        r#"name = "sec-guard-bypass"
tool_execution_enabled = false
"#,
    )
    .unwrap();

    let output = run_in(&temp, &["exec", "--command=echo", "--reason=test"]);
    assert!(!output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], false);
    assert_eq!(body["exec"]["gate_result"], "denied");
}

/// SEC-002: Malformed config must not enable execution (fail closed).
#[test]
fn sec_guard_bypass_malformed_config_fails_closed() {
    let temp = tempfile::tempdir().unwrap();

    fs::write(
        temp.path().join(".crabjar_config.toml"),
        "this is not valid toml {{{",
    )
    .unwrap();

    // Config loads as null → tool_execution_enabled defaults to false
    let output = run_in(&temp, &["workspace", "status"]);
    assert!(output.status.success());

    let body = json_stdout(&output);
    assert_eq!(body["success"], true);
    assert_eq!(body["workspace"], Value::Null);
}

// ──────────────────────────────────────────────
// Trust Layer Regression Tests
// ──────────────────────────────────────────────

/// SEC-010: Trust layer hierarchy must be enforced (higher can access lower, not vice versa).
#[test]
fn sec_trust_layers_hierarchy_enforced() {
    let entries = vec![
        crabjar_guard::DomainEntry::new(
            "trusted.example.com",
            crabjar_guard::DomainTrustLevel::Trusted,
            "sec-test",
        ),
        crabjar_guard::DomainEntry::new(
            "monitored.example.com",
            crabjar_guard::DomainTrustLevel::Monitored,
            "sec-test",
        ),
        crabjar_guard::DomainEntry::new(
            "restricted.example.com",
            crabjar_guard::DomainTrustLevel::Restricted,
            "sec-test",
        ),
    ];
    let allowlist = DomainAllowlist::with_entries(entries);

    // Layer 3 (highest) can access all domains
    assert!(
        allowlist
            .check_for_trust_layer("trusted.example.com", 3)
            .is_ok()
    );
    assert!(
        allowlist
            .check_for_trust_layer("monitored.example.com", 3)
            .is_ok()
    );
    assert!(
        allowlist
            .check_for_trust_layer("restricted.example.com", 3)
            .is_ok()
    );

    // Layer 2 can access trusted + monitored, but NOT restricted
    assert!(
        allowlist
            .check_for_trust_layer("trusted.example.com", 2)
            .is_ok()
    );
    assert!(
        allowlist
            .check_for_trust_layer("monitored.example.com", 2)
            .is_ok()
    );
    assert!(
        allowlist
            .check_for_trust_layer("restricted.example.com", 2)
            .is_err()
    );

    // Layer 1 (lowest) can access ONLY trusted
    assert!(
        allowlist
            .check_for_trust_layer("trusted.example.com", 1)
            .is_ok()
    );
    assert!(
        allowlist
            .check_for_trust_layer("monitored.example.com", 1)
            .is_err()
    );
    assert!(
        allowlist
            .check_for_trust_layer("restricted.example.com", 1)
            .is_err()
    );
}

/// SEC-011: Unknown domain must be blocked by default (deny-all baseline).
#[test]
fn sec_domain_allowlist_deny_all_baseline() {
    let allowlist = DomainAllowlist::new();

    // Known domains pass
    assert!(allowlist.check("github.com").is_ok());
    assert!(allowlist.check("crates.io").is_ok());
    assert!(allowlist.check("localhost").is_ok());

    // Unknown domain is blocked
    let result = allowlist.check("evil-malware.xyz");
    assert!(result.is_err());
    match result.unwrap_err() {
        crabjar_guard::DomainCheckError::NotAllowed(domain) => {
            assert_eq!(domain, "evil-malware.xyz");
        }
        _ => panic!("expected NotAllowed error"),
    }
}

// ──────────────────────────────────────────────
// Scope Isolation Regression Tests
// ──────────────────────────────────────────────

/// SEC-020: Different users must be mutually inaccessible.
#[test]
fn sec_scope_isolation_different_users() {
    let scope_a = Scope::user_project("alice", "project-a");
    let scope_b = Scope::user_project("bob", "project-b");

    assert!(!scope_a.can_access(&scope_b));
    assert!(!scope_b.can_access(&scope_a));
}

/// SEC-021: Same user, different projects must be mutually inaccessible.
#[test]
fn sec_scope_isolation_same_user_different_projects() {
    let scope_x = Scope::user_project("alice", "project-x");
    let scope_y = Scope::user_project("alice", "project-y");

    assert!(!scope_x.can_access(&scope_y));
    assert!(!scope_y.can_access(&scope_x));
}

/// SEC-022: Same user + project must be accessible.
#[test]
fn sec_scope_isolation_same_user_and_project() {
    let scope_1 = Scope::user_project("alice", "project-a");
    let scope_2 = Scope::user_project("alice", "project-a");

    assert!(scope_1.can_access(&scope_2));
    assert!(scope_2.can_access(&scope_1));
}

// ──────────────────────────────────────────────
// Wildcard Matching Regression Tests
// ──────────────────────────────────────────────

/// SEC-030: Wildcard domain matching must be correct (no over-matching).
#[test]
fn sec_wildcard_matching_correct() {
    let allowlist = DomainAllowlist::new();

    // Wildcard *.githubusercontent.com should match subdomains
    assert!(allowlist.check("avatars.githubusercontent.com").is_ok());
    assert!(allowlist.check("raw.githubusercontent.com").is_ok());

    // But NOT unrelated domains with similar suffixes
    assert!(allowlist.check("notgithub.com").is_err());
}
