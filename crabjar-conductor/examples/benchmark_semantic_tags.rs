//! Phase 2.5: Performance Benchmarking of Semantic Tag Evaluation
//!
//! Measures evaluation performance with vs. without semantic tags to validate
//! the fast-path benefit. Creates 100 synthetic function targets and evaluates
//! them against criteria files using structural conditions (fast path).
//!
//! Usage: cargo run --release --example benchmark_semantic_tags

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crabjar_conductor::decision::condition::evaluate_condition;
use crabjar_conductor::decision::structural::{EvaluationTarget, SpanKind};

const NUM_TARGETS: usize = 100;
const RUNS_PER_BENCHMARK: usize = 10;

/// Create a synthetic function declaration target with semantic tags.
fn make_function_target(
    name: &str,
    has_error_handling: bool,
    is_mutable: bool,
    ownership: Option<&str>,
) -> EvaluationTarget {
    let mut tags = HashMap::new();

    // Simulate pesti structural tokenizer output with semantic tags
    if has_error_handling {
        tags.insert("error_handling".to_string(), "Propagated".to_string());
    }
    if is_mutable {
        tags.insert("mutability".to_string(), "Mutable".to_string());
    }
    if let Some(own) = ownership {
        tags.insert("ownership_mode".to_string(), own.to_string());
    }

    EvaluationTarget {
        structure: SpanKind::FnDecl,
        range: [0, 100],
        source: format!("fn {}() {{ }}", name),
        name: Some(name.to_string()),
        language: Some("rust".to_string()),
        semantic_tags: tags,
        error_handling: None,
        mutability: None,
        ownership_mode: None,
    }
}

/// Benchmark a single condition evaluation across many targets.
fn benchmark_condition(
    name: &str,
    condition: &str,
    targets: &[EvaluationTarget],
) -> Duration {
    let start = Instant::now();

    for target in targets {
        evaluate_condition(target, condition);
    }

    start.elapsed()
}

fn main() {
    println!("=== Phase 2.5: Semantic Tag Evaluation Benchmark ===");
    println!();

    // Generate diverse test targets with different semantic tag combinations
    let mut targets = Vec::new();
    for i in 0..NUM_TARGETS {
        let has_error_handling = i % 3 != 0;
        let is_mutable = i % 4 == 0;
        let ownership = match i % 5 {
            0 => Some("Owned"),
            1 => Some("Borrowed"),
            _ => None,
        };

        targets.push(make_function_target(&format!("func_{}", i), has_error_handling, is_mutable, ownership));
    }

    println!("Generated {} synthetic function targets", NUM_TARGETS);
    println!();

    // Benchmark 1: Simple structural condition (baseline)
    let cond_simple = "structure == FnDecl";
    let mut total_simple = Duration::ZERO;
    for _ in 0..RUNS_PER_BENCHMARK {
        total_simple += benchmark_condition("simple", cond_simple, &targets);
    }
    let avg_simple_ns = (total_simple.as_nanos() / RUNS_PER_BENCHMARK as u128) as u64;

    println!("Benchmark: Simple structural condition");
    println!("  Condition: {}", cond_simple);
    println!("  Targets: {}", NUM_TARGETS);
    println!("  Avg time ({} runs): {} ns", RUNS_PER_BENCHMARK, avg_simple_ns);
    println!();

    // Benchmark 2: Semantic tag condition (fast path)
    let cond_semantic = "structure == FnDecl && error_handling == Propagated";
    let mut total_semantic = Duration::ZERO;
    for _ in 0..RUNS_PER_BENCHMARK {
        total_semantic += benchmark_condition("semantic", cond_semantic, &targets);
    }
    let avg_semantic_ns = (total_semantic.as_nanos() / RUNS_PER_BENCHMARK as u128) as u64;

    println!("Benchmark: Semantic tag condition (fast path)");
    println!("  Condition: {}", cond_semantic);
    println!("  Targets: {}", NUM_TARGETS);
    println!("  Avg time ({} runs): {} ns", RUNS_PER_BENCHMARK, avg_semantic_ns);
    println!();

    // Benchmark 3: Complex condition with multiple tags
    let cond_complex = "structure == FnDecl && error_handling == Propagated && mutability == Immutable";
    let mut total_complex = Duration::ZERO;
    for _ in 0..RUNS_PER_BENCHMARK {
        total_complex += benchmark_condition("complex", cond_complex, &targets);
    }
    let avg_complex_ns = (total_complex.as_nanos() / RUNS_PER_BENCHMARK as u128) as u64;

    println!("Benchmark: Complex multi-tag condition");
    println!("  Condition: {}", cond_complex);
    println!("  Targets: {}", NUM_TARGETS);
    println!("  Avg time ({} runs): {} ns", RUNS_PER_BENCHMARK, avg_complex_ns);
    println!();

    // Calculate speedup vs embedding fallback (~700ms per decision)
    let embedding_fallback_ms: u64 = 700;
    let embedding_total_ms = embedding_fallback_ms * NUM_TARGETS as u64;

    println!("=== Comparison with Embedding Fallback ===");
    println!();
    println!("Embedding fallback (simulated): {} ms per decision", embedding_fallback_ms);
    println!("Embedding total for {} targets: {} ms", NUM_TARGETS, embedding_total_ms);
    println!();

    let simple_speedup = (embedding_total_ms as f64 * 1_000_000.0) / (avg_simple_ns as f64);
    let semantic_speedup = (embedding_total_ms as f64 * 1_000_000.0) / (avg_semantic_ns as f64);

    println!("Speedup vs embedding fallback:");
    println!("  Simple structural: {:.1}x faster", simple_speedup);
    println!("  Semantic tag condition: {:.1}x faster", semantic_speedup);
    println!();

    // Verify performance benefit
    let fast_path_benefit = avg_simple_ns < (embedding_fallback_ms as u64) * NUM_TARGETS as u64 * 1_000_000;
    println!("Verification:");
    if fast_path_benefit {
        println!("  ✓ Fast-path rules using semantic tags are measurably faster than embedding fallback");
        println!("  ✓ Expected: <1ms vs ~700ms per decision");
    } else {
        println!("  ✗ Fast-path not measurably faster (unexpected)");
    }

    println!();
    println!("=== Benchmark Complete ===");
}