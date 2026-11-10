//! JEV Evolution Full Example
//!
//! Demonstrates threshold evolution with sufficient feedback data.

use crabjar_conductor::decision::evolution::{DecisionOutcome, EvolutionManager};
use std::time::SystemTime;

fn main() {
    println!("=== JEV Evolution Full Example ===\n");

    let mut evolver = EvolutionManager::new(".").expect("Failed to create evolution manager");

    println!("Default threshold: 0.85");
    println!();

    // Simulate 10 decisions with low approval rate (7 rejected, 3 approved)
    println!("Simulating 10 decisions with LOW approval rate...");

    for i in 1..=10 {
        let outcome = DecisionOutcome {
            decision_id: format!("test-{}", i),
            criteria_file: "code-review-risk.json".to_string(),
            option: "low".to_string(),
            confidence: 0.70,
            user_approved: i <= 3, // Only first 3 approved
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        evolver
            .record_outcome(outcome)
            .expect("Failed to record outcome");
    }

    let stats_before = evolver.get_stats("code-review-risk.json").unwrap();
    println!("Approval rate: 0.30 (3/10)");
    println!("Threshold before evolution: {:.2}", stats_before.threshold);

    // Evolve - should increase threshold due to low approval rate
    evolver.evolve().expect("Failed to evolve");

    let stats_after = evolver.get_stats("code-review-risk.json").unwrap();
    println!("Threshold after evolution: {:.2}", stats_after.threshold);

    if stats_after.threshold > stats_before.threshold {
        println!("\n✓ Threshold INCREASED (more conservative) due to low approval rate");
    }

    // Now simulate high approval rate scenario
    println!("\n--- High Approval Rate Scenario ---\n");

    for i in 11..=20 {
        let outcome = DecisionOutcome {
            decision_id: format!("test-{}", i),
            criteria_file: "code-review-risk.json".to_string(),
            option: "high".to_string(),
            confidence: 0.95,
            user_approved: true, // All approved
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        evolver
            .record_outcome(outcome)
            .expect("Failed to record outcome");
    }

    let stats_before2 = evolver.get_stats("code-review-risk.json").unwrap();
    println!("Threshold before evolution: {:.2}", stats_before2.threshold);

    evolver.evolve().expect("Failed to evolve");

    let stats_after2 = evolver.get_stats("code-review-risk.json").unwrap();
    println!("Threshold after evolution: {:.2}", stats_after2.threshold);

    if stats_after2.threshold < stats_before2.threshold {
        println!("\n✓ Threshold DECREASED (more aggressive) due to high approval rate");
    }

    println!("\nFinal threshold: {:.2}", stats_after2.threshold);
}
