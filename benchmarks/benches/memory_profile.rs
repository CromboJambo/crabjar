//! Memory profiling for execution pipeline.
//!
//! Measures RSS (Resident Set Size) growth across gate operations
//! to identify memory leaks and allocation patterns.
//! Run with: cargo bench -p crabjar-benchmarks --bench memory_profile

use std::fs;
use std::process;

use crabjar_guard::{ContextBudget, ExecutionGate, GateContext, GuardDb, TrustScore};

/// Read current process RSS from /proc/self/status (Linux).
fn get_rss_kb() -> u64 {
    let status = fs::read_to_string("/proc/self/status").expect("Failed to read /proc/self/status");
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse::<u64>().expect("Failed to parse VmRSS");
            }
        }
    }
    panic!("VmRSS not found in /proc/self/status");
}

fn make_context() -> GateContext<'static> {
    GateContext::new(
        "run_command",
        "echo hello",
        vec![],
        4,
        TrustScore::new(0.95),
    )
    .with_source_event("mem-event-1")
}

fn main() {
    let iterations = 10000;

    println!("Crabjar Execution Pipeline Memory Profile");
    println!("=========================================");
    println!("Iterations: {}\n", iterations);

    // Baseline memory usage
    let baseline_rss = get_rss_kb();
    println!("Baseline RSS: {} KB", baseline_rss);

    // Run gate checks and measure memory growth
    {
        let db = GuardDb::open(":memory:").expect("Failed to create guard DB");
        let gate = ExecutionGate::new(&db, false, "/tmp/benchmark-mem");

        println!("\nRunning {} gate checks...", iterations);
        for i in 0..iterations {
            let ctx = make_context();
            let _result = gate.check(ctx);

            // Sample memory every 1000 operations
            if (i + 1) % 1000 == 0 {
                let current_rss = get_rss_kb();
                let growth = current_rss - baseline_rss;
                println!(
                    "  After {} ops: RSS={} KB, growth={} KB",
                    i + 1,
                    current_rss,
                    growth
                );
            }
        }

        // Final measurement
        let final_rss = get_rss_kb();
        let total_growth = final_rss - baseline_rss;
        println!("\nFinal RSS: {} KB", final_rss);
        println!("Total growth: {} KB", total_growth);
        println!(
            "Growth per operation: {:.3} KB/ops",
            total_growth as f64 / iterations as f64
        );

        if total_growth > 0 {
            // Check if memory is growing linearly (leak) or stabilizing
            let growth_per_batch = total_growth / 10; // rough estimate per 1000 ops
            println!("Estimated growth per 1000 ops: {} KB", growth_per_batch);
        } else {
            println!("Memory stable — no leak detected");
        }
    }

    // Test with domain allowlist (heavier policy evaluation)
    println!("\n--- Domain Allowlist Memory Profile ---");
    let baseline_rss2 = get_rss_kb();
    println!("Baseline RSS: {} KB", baseline_rss2);

    {
        let db = GuardDb::open(":memory:").expect("Failed to create guard DB");
        let gate = ExecutionGate::new(&db, false, "/tmp/benchmark-mem-domains");

        for i in 0..iterations {
            let mut ctx = make_context();
            ctx.domains = vec![format!("example{}.com", i % 10)];
            let _result = gate.check(ctx);

            if (i + 1) % 1000 == 0 {
                let current_rss = get_rss_kb();
                let growth = current_rss - baseline_rss2;
                println!(
                    "  After {} ops: RSS={} KB, growth={} KB",
                    i + 1,
                    current_rss,
                    growth
                );
            }
        }

        let final_rss2 = get_rss_kb();
        let total_growth2 = final_rss2 - baseline_rss2;
        println!("\nFinal RSS: {} KB", final_rss2);
        println!("Total growth: {} KB", total_growth2);
        println!(
            "Growth per operation: {:.3} KB/ops",
            total_growth2 as f64 / iterations as f64
        );
    }

    // Test with context budget (heavier state tracking)
    println!("\n--- Context Budget Memory Profile ---");
    let baseline_rss3 = get_rss_kb();
    println!("Baseline RSS: {} KB", baseline_rss3);

    {
        let db = GuardDb::open(":memory:").expect("Failed to create guard DB");
        let gate = ExecutionGate::new(&db, false, "/tmp/benchmark-mem-budget");

        for i in 0..iterations {
            let ctx = make_context().with_context_budget(ContextBudget::new(10000), 50);
            let _result = gate.check(ctx);

            if (i + 1) % 1000 == 0 {
                let current_rss = get_rss_kb();
                let growth = current_rss - baseline_rss3;
                println!(
                    "  After {} ops: RSS={} KB, growth={} KB",
                    i + 1,
                    current_rss,
                    growth
                );
            }
        }

        let final_rss3 = get_rss_kb();
        let total_growth3 = final_rss3 - baseline_rss3;
        println!("\nFinal RSS: {} KB", final_rss3);
        println!("Total growth: {} KB", total_growth3);
        println!(
            "Growth per operation: {:.3} KB/ops",
            total_growth3 as f64 / iterations as f64
        );
    }

    println!("\nMemory profile complete.");
}
