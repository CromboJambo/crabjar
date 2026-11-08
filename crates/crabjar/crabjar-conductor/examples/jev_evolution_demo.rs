//! JEV Evolution Demo
//!
//! Clear demonstration of threshold evolution in both directions.

use crabjar_conductor::decision::evolution::{DecisionOutcome, EvolutionManager};
use std::time::SystemTime;

fn simulate_decisions(
    evolver: &mut EvolutionManager,
    count: usize,
    approval_rate: f64,
    criteria: &str,
) {
    let approve_count = (count as f64 * approval_rate) as usize;

    for i in 0..count {
        let outcome = DecisionOutcome {
            decision_id: format!("demo-{}-{}", criteria, i),
            criteria_file: criteria.to_string(),
            option: "test".to_string(),
            confidence: 0.80,
            user_approved: i < approve_count,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        evolver.record_outcome(outcome).expect("Failed to record");
    }
}

fn main() {
    println!("=== JEV Evolution Demo ===\n");

    let mut evolver = EvolutionManager::new(".").expect("Failed to create evolution manager");

    // Scenario 1: Low approval rate → threshold should increase
    println!("Scenario 1: LOW approval rate (30%)");
    simulate_decisions(&mut evolver, 20, 0.30, "low-approval-test.json");

    let before = evolver.get_stats("low-approval-test.json").unwrap();
    println!("  Approval rate: {:.0}%", before.approval_rate * 100.0);
    println!("  Threshold before: {:.2}", before.threshold);

    evolver.evolve().expect("Failed to evolve");

    let after = evolver.get_stats("low-approval-test.json").unwrap();
    println!("  Threshold after: {:.2}", after.threshold);

    if after.threshold > before.threshold {
        println!("  ✓ INCREASED (more conservative)\n");
    }

    // Scenario 2: High approval rate → threshold should decrease
    println!("Scenario 2: HIGH approval rate (100%)");
    simulate_decisions(&mut evolver, 20, 1.00, "high-approval-test.json");

    let before = evolver.get_stats("high-approval-test.json").unwrap();
    println!("  Approval rate: {:.0}%", before.approval_rate * 100.0);
    println!("  Threshold before: {:.2}", before.threshold);

    evolver.evolve().expect("Failed to evolve");

    let after = evolver.get_stats("high-approval-test.json").unwrap();
    println!("  Threshold after: {:.2}", after.threshold);

    if after.threshold < before.threshold {
        println!("  ✓ DECREASED (more aggressive)\n");
    }

    println!("Evolution state saved to .crabjar/jev-evolution.json");
}
