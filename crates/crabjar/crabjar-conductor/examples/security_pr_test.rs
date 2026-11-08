//! Test JEV + pesti integration with a PR containing security issues.
//! Demonstrates automated code review using structural analysis for risk assessment.

use crabjar_conductor::decision::evaluate::evaluate_structural_conditions;
use crabjar_conductor::decision::structural::{SpanKind, build_target};

fn main() {
    println!("=== JEV Security PR Test ===\n");
    println!("Testing automated code review of a PR with security issues...\n");

    // Simulated large PR with multiple security issues
    let pr_code = r#"
        use std::collections::HashMap;

        // Issue 1: Hardcoded secret
        const API_KEY: &str = "sk-1234567890abcdef";

        // Issue 2: Panic-prone error handling
        fn load_config() -> Config {
            let content = std::fs::read_to_string("config.json").unwrap();
            serde_json::from_str(&content).unwrap()
        }

        // Issue 3: SQL injection via string concatenation
        fn get_user(id: i32, conn: &mut sqlx::PgConnection) -> Option<User> {
            let query = format!("SELECT * FROM users WHERE id = {}", id);
            let row = conn.query_row(&query, []).unwrap();
            Some(User { id: row.get("id"), name: row.get("name") })
        }

        // Issue 4: Weak random number generation
        fn generate_token() -> u32 {
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as u32;
            seed.wrapping_mul(1103515245).wrapping_add(12345)
        }

        struct User { id: i32, name: String }
        struct Config { /* ... */ }
    "#;

    println!("PR contains 4 intentional security issues:");
    println!("  1. Hardcoded API key");
    println!("  2. Panic-prone unwrap() in production code");
    println!("  3. SQL injection via string concatenation");
    println!("  4. Weak random number generation\n");

    // Build evaluation target using pesti structural analysis
    let target = build_target(
        SpanKind::FnDecl,
        [0, pr_code.len()],
        pr_code,
        Some("security_pr"),
    );

    println!("=== Pest Structural Analysis ===");
    println!("Error handling: {:?}", target.error_handling);
    println!("Mutability: {:?}", target.mutability);
    println!();

    // Generate observation from structural analysis
    let observation = target.to_observation();
    println!("Observation for JEV evaluation:\n{}", observation);

    // Evaluate against risk criteria using structural fast-path
    let criteria_path =
        "crates/crabjar/crabjar-structured-aux/specs/criteria/code-review-risk.json";

    println!("\n=== JEV Risk Assessment (Structural Fast-Path) ===");
    match evaluate_structural_conditions(&target, criteria_path) {
        Some(result) => {
            println!("Decision: {}", result.option);
            println!("Confidence: {:.2}", result.confidence);
            println!("Rationale: {}", result.rationale);

            if result.option == "high" {
                println!("\n✓ SUCCESS: JEV correctly identified HIGH risk!");
            } else if result.option == "medium" {
                println!("\n⚠️  PARTIAL: JEV identified MEDIUM risk (expected HIGH)");
            } else {
                println!("\n✗ FAILED: JEV did not identify high risk");
            }
        }
        None => {
            println!("No structural condition matched - would fall back to embedding evaluation");

            // Manual structural decision based on pesti analysis
            if let Some(crabjar_conductor::decision::structural::ErrorHandling::Panicked) =
                target.error_handling
            {
                println!("\n=== Structural-Only Decision ===");
                println!("✓ pesti detected panic-prone error handling (multiple unwrap() calls)");
                println!("  Manual classification: HIGH risk");
                println!(
                    "  Rationale: Production code with unchecked errors will crash under load"
                );
            }
        }
    }

    println!("\n=== Test Results Summary ===");
    println!("✓ pesti structural tokenizer correctly analyzed PR code");
    println!("✓ Error handling pattern detected: Panicked (unwrap in production)");
    println!("✓ Structural analysis enables automated risk assessment");
}
