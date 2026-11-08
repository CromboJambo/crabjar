//! JEV Evolution Example
//!
//! Demonstrates loading configuration from .crabjar_config.toml,
//! tracking decision outcomes, and evolving thresholds based on feedback.

use crabjar_conductor::decision::evolution::{DecisionOutcome, EvolutionManager};
use std::time::SystemTime;

fn main() {
    println!("=== JEV Evolution Example ===\n");

    // Create evolution manager from .crabjar_config.toml
    let mut evolver = EvolutionManager::new(".").expect("Failed to create evolution manager");

    println!("Evolution enabled: {}", evolver.is_enabled());
    println!("Default threshold: 0.85");
    println!();

    // Simulate some decision outcomes
    println!("Simulating decision outcomes...");

    // Scenario 1: High confidence structural decision, user approves
    let outcome1 = DecisionOutcome {
        decision_id: "test-1".to_string(),
        criteria_file: "code-review-risk.json".to_string(),
        option: "high".to_string(),
        confidence: 0.95,
        user_approved: true,
        timestamp: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    evolver.record_outcome(outcome1).expect("Failed to record outcome");
    println!("  Recorded: high risk decision (approved, confidence 0.95)");

    // Scenario 2: Another high confidence decision, user approves
    let outcome2 = DecisionOutcome {
        decision_id: "test-2".to_string(),
        criteria_file: "code-review-risk.json".to_string(),
        option: "high".to_string(),
        confidence: 0.90,
        user_approved: true,
        timestamp: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    evolver.record_outcome(outcome2).expect("Failed to record outcome");
    println!("  Recorded: high risk decision (approved, confidence 0.90)");

    // Scenario 3: Low confidence decision, user rejects
    let outcome3 = DecisionOutcome {
        decision_id: "test-3".to_string(),
        criteria_file: "code-review-risk.json".to_string(),
        option: "low".to_string(),
        confidence: 0.60,
        user_approved: false,
        timestamp: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    evolver.record_outcome(outcome3).expect("Failed to record outcome");
    println!("  Recorded: low risk decision (rejected, confidence 0.60)");

    // Get stats before evolution
    let stats_before = evolver.get_stats("code-review-risk.json");
    println!("\nStats before evolution:");
    if let Some(stats) = &stats_before {
        println!("  Total decisions: {}", stats.total);
        println!("  Approved: {}", stats.approved);
        println!("  Rejected: {}", stats.rejected);
        println!("  Approval rate: {:.2}", stats.approval_rate);
        println!("  Current threshold: {:.2}", stats.threshold);
    }

    // Evolve thresholds based on feedback
    println!("\nEvolving thresholds...");
    evolver.evolve().expect("Failed to evolve");

    // Get stats after evolution
    let stats_after = evolver.get_stats("code-review-risk.json");
    println!("Stats after evolution:");
    if let Some(stats) = &stats_after {
        println!("  Total decisions: {}", stats.total);
        println!("  Approved: {}", stats.approved);
        println!("  Rejected: {}", stats.rejected);
        println!("  Approval rate: {:.2}", stats.approval_rate);
        println!("  Evolved threshold: {:.2}", stats.threshold);
    }

    if let (Some(before), Some(after)) = (&stats_before, &stats_after) {
        if after.threshold != before.threshold {
            println!("\n✓ Threshold evolved from {:.2} to {:.2}", before.threshold, after.threshold);
        } else {
            println!("\n  No threshold change (insufficient data or balanced approval rate)");
        }
    }

    println!("\nEvolution state saved to .crabjar/jev-evolution.json");
}