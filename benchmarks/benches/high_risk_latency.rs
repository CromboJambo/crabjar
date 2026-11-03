//! High-risk command latency analysis.
//!
//! Isolates different command risk levels to measure their contribution
//! to gate check latency, specifically for high-risk commands.
//! Run with: cargo bench -p crabjar-benchmarks --bench high_risk_latency

use std::time::{Duration, Instant};

use crabjar_guard::{ExecutionGate, GateContext, GuardDb, TrustScore};

/// Run a benchmark and report results.
fn benchmark(name: &str, iterations: usize, f: impl Fn() -> Duration) {
    let mut total = Duration::ZERO;
    let mut min = Duration::MAX;
    let mut max = Duration::ZERO;
    let mut outliers = 0;

    for _ in 0..iterations {
        let elapsed = f();
        total += elapsed;
        if elapsed < min {
            min = elapsed;
        }
        if elapsed > max {
            max = elapsed;
        }
        // Count outliers (>50μs)
        if elapsed > Duration::from_nanos(50_000) {
            outliers += 1;
        }
    }

    let avg = total / iterations as u32;
    println!(
        "{:40} avg={:>10}ns min={:>10}ns max={:>10}ns outliers(>50μs)={}",
        name,
        avg.as_nanos(),
        min.as_nanos(),
        max.as_nanos(),
        outliers
    );
}

fn main() {
    let iterations = 10000;

    println!("High-Risk Command Latency Analysis");
    println!("==================================");
    println!("Iterations: {}\n", iterations);

    // Baseline: low-risk command
    {
        let db = GuardDb::open(":memory:").expect("Failed to create guard DB");
        let gate = ExecutionGate::new(&db, false, "/tmp/benchmark-hr");
        benchmark("low_risk_echo", iterations, || {
            let ctx = GateContext::new(
                "run_command",
                "echo hello",
                vec![],
                4,
                TrustScore::new(0.95),
            )
            .with_source_event("hr-event-1");
            let start = Instant::now();
            let _result = gate.check(ctx);
            start.elapsed()
        });
    }

    // High-risk: rm -rf / (single string)
    {
        let db = GuardDb::open(":memory:").expect("Failed to create guard DB");
        let gate = ExecutionGate::new(&db, false, "/tmp/benchmark-hr");
        benchmark("high_risk_rm_rf_string", iterations, || {
            let ctx = GateContext::new("delete", "rm -rf /", vec![], 4, TrustScore::new(0.95))
                .with_source_event("hr-event-2");
            let start = Instant::now();
            let _result = gate.check(ctx);
            start.elapsed()
        });
    }

    // High-risk: rm -rf / (as separate args)
    {
        let db = GuardDb::open(":memory:").expect("Failed to create guard DB");
        let gate = ExecutionGate::new(&db, false, "/tmp/benchmark-hr");
        benchmark("high_risk_rm_rf_args", iterations, || {
            let ctx = GateContext::new(
                "delete",
                "rm",
                vec!["-rf".to_string(), "/".to_string()],
                4,
                TrustScore::new(0.95),
            )
            .with_source_event("hr-event-3");
            let start = Instant::now();
            let _result = gate.check(ctx);
            start.elapsed()
        });
    }

    // High-risk: sudo apt install
    {
        let db = GuardDb::open(":memory:").expect("Failed to create guard DB");
        let gate = ExecutionGate::new(&db, false, "/tmp/benchmark-hr");
        benchmark("high_risk_sudo_apt_install", iterations, || {
            let ctx = GateContext::new(
                "install",
                "sudo apt install",
                vec!["nginx".to_string()],
                4,
                TrustScore::new(0.95),
            )
            .with_source_event("hr-event-4");
            let start = Instant::now();
            let _result = gate.check(ctx);
            start.elapsed()
        });
    }

    // Medium-risk: git clone
    {
        let db = GuardDb::open(":memory:").expect("Failed to create guard DB");
        let gate = ExecutionGate::new(&db, false, "/tmp/benchmark-hr");
        benchmark("medium_risk_git_clone", iterations, || {
            let ctx = GateContext::new(
                "clone",
                "git clone",
                vec!["https://github.com/example/repo.git".to_string()],
                4,
                TrustScore::new(0.95),
            )
            .with_source_event("hr-event-5");
            let start = Instant::now();
            let _result = gate.check(ctx);
            start.elapsed()
        });
    }

    // High-risk with many args (stress test string join)
    {
        let db = GuardDb::open(":memory:").expect("Failed to create guard DB");
        let gate = ExecutionGate::new(&db, false, "/tmp/benchmark-hr");
        benchmark("high_risk_rm_many_args", iterations, || {
            let args: Vec<String> = (0..100).map(|i| format!("file{}.txt", i)).collect();
            let ctx = GateContext::new("delete", "rm", args, 4, TrustScore::new(0.95))
                .with_source_event("hr-event-6");
            let start = Instant::now();
            let _result = gate.check(ctx);
            start.elapsed()
        });
    }

    println!("\nHigh-risk command latency analysis complete.");
}
