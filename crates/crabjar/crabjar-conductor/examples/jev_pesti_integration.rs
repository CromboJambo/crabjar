//! Demonstrates pesti structural tokenizer integration with JEV decision-making.
//! Shows how code structure analysis informs automated risk assessment and approval decisions.

use crabjar_conductor::decision::evaluate::evaluate_structural_conditions;
use crabjar_conductor::decision::structural::{SpanKind, build_target};

fn main() {
    println!("=== pesti + JEV Integration Demo ===\n");

    // Example 1: Error handling improvement (should be low risk)
    let _payment_before = r#"
        fn charge_card(amount: i32) -> bool {
            if amount < 0 { return false; }
            process_payment(amount);
            true
        }
    "#;

    let payment_after = r#"
        fn charge_card(amount: i32) -> Result<(), PaymentError> {
            if amount < 0 {
                return Err(PaymentError::InvalidAmount);
            }
            process_payment(amount)?;
            Ok(())
        }
    "#;

    println!("Scenario 1: Error handling improvement");
    println!("----------------------------------------");

    let target_after = build_target(
        SpanKind::FnDecl,
        [0, payment_after.len()],
        payment_after,
        Some("charge_card"),
    );

    println!("Structural analysis of improved version:");
    println!("  Error handling: {:?}", target_after.error_handling);

    // Evaluate against risk criteria (fast-path structural evaluation)
    let risk_criteria =
        "crates/crabjar/crabjar-structured-aux/specs/criteria/code-review-risk.json";
    if let Some(result) = evaluate_structural_conditions(&target_after, risk_criteria) {
        println!(
            "  JEV Decision: {} (confidence: {:.2})",
            result.option, result.confidence
        );
        println!("  Rationale: {}", result.rationale);
    } else {
        println!("  No structural condition matched - would use embedding evaluation");
    }

    // Example 2: Panic-prone code (should be higher risk)
    let risky_code = r#"
        fn load_config() -> Config {
            read_file("config.json").unwrap()
        }
    "#;

    println!("\nScenario 2: Panic-prone error handling");
    println!("----------------------------------------");

    let target_risky = build_target(
        SpanKind::FnDecl,
        [0, risky_code.len()],
        risky_code,
        Some("load_config"),
    );

    println!("Structural analysis of risky version:");
    println!("  Error handling: {:?}", target_risky.error_handling);

    if let Some(result) = evaluate_structural_conditions(&target_risky, risk_criteria) {
        println!(
            "  JEV Decision: {} (confidence: {:.2})",
            result.option, result.confidence
        );
        println!("  Rationale: {}", result.rationale);
    } else {
        println!("  No structural condition matched - would use embedding evaluation");
    }

    println!("\n=== Key Takeaways ===");
    println!("1. pesti's semantic analyzers detect code structure patterns");
    println!("2. These patterns feed into JEV's fast-path evaluation");
    println!("3. Decisions can be made without LLM embedding backend");
    println!("4. Falls back to embedding evaluation when needed");
}
