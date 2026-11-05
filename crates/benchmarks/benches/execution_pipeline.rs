//! Execution pipeline benchmarks.
//!
//! Measures latency across the guard → concierge → telemetry path.
//! Run with: cargo bench -p crabjar-benchmarks

use std::time::{Duration, Instant};

use crabjar_guard::{ContextBudget, ExecutionGate, GateContext, GuardDb, TrustScore};

/// Run a benchmark and report results.
fn benchmark(name: &str, iterations: usize, f: impl Fn() -> Duration) {
    let mut total = Duration::ZERO;
    let mut min = Duration::MAX;
    let mut max = Duration::ZERO;

    for _ in 0..iterations {
        let elapsed = f();
        total += elapsed;
        if elapsed < min {
            min = elapsed;
        }
        if elapsed > max {
            max = elapsed;
        }
    }

    let avg = total / iterations as u32;
    println!(
        "{:40} avg={:>10}ns min={:>10}ns max={:>10}ns",
        name,
        avg.as_nanos(),
        min.as_nanos(),
        max.as_nanos()
    );
}

fn make_context() -> GateContext<'static> {
    GateContext::new(
        "run_command",
        "echo hello",
        vec![],
        4,
        TrustScore::new(0.95),
    )
    .with_source_event("bench-event-1")
}

fn main() {
    let iterations = 1000;

    println!("Crabjar Execution Pipeline Benchmarks");
    println!("=====================================");
    println!("Iterations: {}\n", iterations);

    // Benchmark 1: Full gate check (happy path)
    {
        let db = GuardDb::open(":memory:").expect("Failed to create guard DB");
        let gate = ExecutionGate::new(&db, false, "/tmp/benchmark");
        benchmark("gate_check_happy_path", iterations, || {
            let ctx = make_context();
            let start = Instant::now();
            let _result = gate.check(ctx);
            start.elapsed()
        });
    }

    // Benchmark 2: Gate check with domain allowlist
    {
        let db = GuardDb::open(":memory:").expect("Failed to create guard DB");
        let gate = ExecutionGate::new(&db, false, "/tmp/benchmark");
        benchmark("gate_check_with_domains", iterations, || {
            let mut ctx = make_context();
            ctx.domains = vec!["example.com".to_string()];
            let start = Instant::now();
            let _result = gate.check(ctx);
            start.elapsed()
        });
    }

    // Benchmark 3: Gate check with context budget
    {
        let db = GuardDb::open(":memory:").expect("Failed to create guard DB");
        let gate = ExecutionGate::new(&db, false, "/tmp/benchmark");
        benchmark("gate_check_with_budget", iterations, || {
            let ctx = make_context().with_context_budget(ContextBudget::new(10000), 50);
            let start = Instant::now();
            let _result = gate.check(ctx);
            start.elapsed()
        });
    }

    // Benchmark 4: Command risk assessment (high-risk command)
    {
        let db = GuardDb::open(":memory:").expect("Failed to create guard DB");
        let gate = ExecutionGate::new(&db, false, "/tmp/benchmark");
        benchmark("gate_check_high_risk_cmd", iterations, || {
            let ctx = GateContext::new("delete", "rm -rf /", vec![], 4, TrustScore::new(0.95))
                .with_source_event("bench-event-4");
            let start = Instant::now();
            let _result = gate.check(ctx);
            start.elapsed()
        });
    }

    // Benchmark 5: Gate check with low confidence (interrupted)
    {
        let db = GuardDb::open(":memory:").expect("Failed to create guard DB");
        let gate = ExecutionGate::new(&db, false, "/tmp/benchmark");
        benchmark("gate_check_low_confidence", iterations, || {
            let ctx =
                GateContext::new("run_command", "echo hello", vec![], 4, TrustScore::new(0.1))
                    .with_source_event("bench-event-5");
            let start = Instant::now();
            let _result = gate.check(ctx);
            start.elapsed()
        });
    }

    // Benchmark 6: Multiple trust layers
    for (layer_name, layer) in [
        ("untrusted", 0u32),
        ("low_trust", 1u32),
        ("working", 2u32),
        ("high_trust", 3u32),
        ("fully_trusted", 4u32),
    ] {
        let db = GuardDb::open(":memory:").expect("Failed to create guard DB");
        let gate = ExecutionGate::new(&db, false, "/tmp/benchmark");
        benchmark(
            &format!("gate_check_layer_{}", layer_name),
            iterations,
            || {
                let ctx = GateContext::new(
                    "run_command",
                    "echo hello",
                    vec![],
                    layer,
                    TrustScore::new(0.95),
                )
                .with_source_event("bench-event-layer");
                let start = Instant::now();
                let _result = gate.check(ctx);
                start.elapsed()
            },
        );
    }

    println!("\nBenchmarks complete.");
}
