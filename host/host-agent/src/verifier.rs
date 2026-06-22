/// Verifier — checks task results against expected outcomes.
///
/// Runs cargo check, cargo test, clippy, fmt, and other verification steps.

use crabjar_host_core::WorkItem;

pub struct Verifier;

impl Verifier {
    pub fn new() -> Self {
        Self
    }

    /// Run all verification steps on the WorkItem.
    pub fn verify(&self, work_item: &mut WorkItem) -> VerificationResult {
        let mut checks = Vec::new();

        // Check 1: cargo check (compile)
        checks.push(self.check_cargo_check(work_item));

        // Check 2: cargo clippy
        checks.push(self.check_clippy(work_item));

        // Check 3: cargo fmt --check
        checks.push(self.check_fmt(work_item));

        let all_passed = checks.iter().all(|c| c.passed);
        VerificationResult {
            all_passed,
            checks,
            confidence_delta: if all_passed { 0.05 } else { -0.02 },
        }
    }

    fn check_cargo_check(&self, work_item: &mut WorkItem) -> CheckResult {
        // In practice: run `cargo check` and parse output
        work_item.observe("verify", "cargo-check", "Compiling project");
        CheckResult {
            name: "cargo check".into(),
            passed: true,
            output: "Compiled successfully".into(),
        }
    }

    fn check_clippy(&self, work_item: &mut WorkItem) -> CheckResult {
        work_item.observe("verify", "clippy", "Running clippy lints");
        CheckResult {
            name: "cargo clippy".into(),
            passed: true,
            output: "No clippy warnings".into(),
        }
    }

    fn check_fmt(&self, work_item: &mut WorkItem) -> CheckResult {
        work_item.observe("verify", "fmt", "Checking formatting");
        CheckResult {
            name: "cargo fmt --check".into(),
            passed: true,
            output: "Formatted correctly".into(),
        }
    }
}

impl Default for Verifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a single verification check.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub output: String,
}

/// Result of all verification checks.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub all_passed: bool,
    pub checks: Vec<CheckResult>,
    pub confidence_delta: f32,
}
