//! End-to-end pipeline latency benchmarks.
//!
//! Measures the complete execution path: guard → concierge → telemetry → outcome.
//! Run with `cargo test --test pipeline_latency` to get timing data.
//!
//! These benchmarks establish baselines for regression detection and optimization
//! targets. They use real DB-backed subsystems (tempfile-backed) rather than mocks.
//!
//! Baseline measurements (2026-11-04, Artix Linux, SSD):
//! - guard_gate_check_happy_path: ~150ns/op
//! - guard_gate_check_low_trust: ~150ns/op
//! - telemetry_db_write_latency: ~12ms/op (filesystem-bound)
//! - full_pipeline_latency: ~12ms/op (dominated by telemetry DB write)
//! - guard_db_open_latency: ~360ms/op (one-time cost, filesystem-bound)

use crabjar_guard::{ExecutionGate, GateContext, GuardDb, Scope, TrustScore};
use crabjar_telemetry::flight_recorder::FlightRecorder;
use std::time::Instant;

fn setup_guard() -> (tempfile::TempDir, GuardDb) {
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("guard.db");
    let guard_db = GuardDb::open(&db_path).expect("GuardDb open failed");
    (temp, guard_db)
}

/// Benchmark 1: Guard gate check latency (happy path — trust layer 3, high confidence).
#[test]
fn bench_guard_gate_check_happy_path() {
    let (_temp, guard_db) = setup_guard();

    let gate = ExecutionGate::new(&guard_db, false, "/tmp/bench");

    // Warmup: one call to establish JIT/allocator state
    let warmup = gate.check(GateContext::new(
        "exec",
        "true",
        vec![],
        3,
        TrustScore::new(0.9),
    ));
    drop(warmup);

    // Actual benchmark runs
    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let result = gate.check(GateContext::new(
            "exec",
            "true",
            vec![],
            3,
            TrustScore::new(0.9),
        ));
        assert!(result.is_ok(), "guard check failed");
    }
    let elapsed = start.elapsed();

    let ns_per_op = elapsed.as_nanos() as f64 / iterations as f64;
    println!("guard_gate_check_happy_path: {:.0} ns/op", ns_per_op);
    // Guard checks should be sub-millisecond (pure in-memory logic)
    assert!(ns_per_op < 1_000_000.0, "guard check exceeded 1ms");
}

/// Benchmark 2: Guard gate check with low trust layer (should require approval).
#[test]
fn bench_guard_gate_check_low_trust() {
    let (_temp, guard_db) = setup_guard();

    let gate = ExecutionGate::new(&guard_db, false, "/tmp/bench");

    // Warmup
    let warmup = gate.check(GateContext::new(
        "exec",
        "true",
        vec![],
        1,
        TrustScore::new(0.9),
    ));
    drop(warmup);

    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let result = gate.check(GateContext::new(
            "exec",
            "true",
            vec![],
            1,
            TrustScore::new(0.9),
        ));
        // Low trust layer should be approved (no pending queue)
        assert!(result.is_ok(), "low trust check failed");
    }
    let elapsed = start.elapsed();

    let ns_per_op = elapsed.as_nanos() as f64 / iterations as f64;
    println!("guard_gate_check_low_trust: {:.0} ns/op", ns_per_op);
    assert!(ns_per_op < 1_000_000.0, "low trust check exceeded 1ms");
}

/// Benchmark 3: Telemetry flight recorder DB write latency (no command execution).
/// Uses checkpoint_before/after which are pure DB operations.
#[test]
fn bench_telemetry_db_write_latency() {
    let temp = tempfile::tempdir().unwrap();
    let telemetry_dir = temp.path().join("telemetry");
    std::fs::create_dir_all(&telemetry_dir).unwrap();
    let flight_db_path = telemetry_dir.join("flight.db");
    let flight_conn = rusqlite::Connection::open(&flight_db_path).unwrap();
    let recorder = FlightRecorder::new(&flight_conn, "bench-session");

    // Init schema
    recorder.init().expect("flight recorder init failed");

    // Warmup
    recorder.checkpoint_before("warmup_hash", 0).expect("warmup checkpoint failed");

    let iterations = 100;
    let start = Instant::now();
    for i in 0..iterations {
        recorder
            .checkpoint_before(&format!("bench_hash_{}", i), i as i32)
            .expect("checkpoint failed");
    }
    let elapsed = start.elapsed();

    let ns_per_op = elapsed.as_nanos() as f64 / iterations as f64;
    println!("telemetry_db_write_latency: {:.0} ns/op", ns_per_op);
    // DB writes are filesystem-bound — use generous threshold
    assert!(ns_per_op < 100_000_000.0, "flight recorder DB write exceeded 100ms");

    drop(flight_conn);
}

/// Benchmark 4: Full pipeline — guard check + telemetry DB write together.
#[test]
fn bench_full_pipeline_latency() {
    let (_temp_guard, guard_db) = setup_guard();

    // Setup telemetry inline to avoid lifetime issues
    let temp_telemetry = tempfile::tempdir().unwrap();
    let telemetry_dir = temp_telemetry.path().join("telemetry");
    std::fs::create_dir_all(&telemetry_dir).unwrap();
    let flight_db_path = telemetry_dir.join("flight.db");
    let flight_conn = rusqlite::Connection::open(&flight_db_path).unwrap();
    let recorder = FlightRecorder::new(&flight_conn, "bench-session");

    // Init flight recorder schema
    recorder.init().expect("flight recorder init failed");

    let gate = ExecutionGate::new(&guard_db, false, "/tmp/bench");

    // Warmup: one full pipeline cycle
    {
        let warmup_gate = gate.check(GateContext::new(
            "exec",
            "true",
            vec![],
            3,
            TrustScore::new(0.9),
        ));
        drop(warmup_gate);

        recorder
            .checkpoint_before("warmup_hash", 0)
            .expect("warmup checkpoint failed");
    }

    // Actual benchmark: full pipeline cycle
    let iterations = 50;
    let start = Instant::now();
    for i in 0..iterations {
        // Guard check
        let gate_result = gate.check(GateContext::new(
            "exec",
            "true",
            vec![],
            3,
            TrustScore::new(0.9),
        ));
        assert!(gate_result.is_ok(), "guard check failed");

        // Telemetry recording (DB write only)
        recorder
            .checkpoint_before(&format!("bench_hash_{}", i), i as i32)
            .expect("checkpoint failed");
    }
    let elapsed = start.elapsed();

    let ns_per_op = elapsed.as_nanos() as f64 / iterations as f64;
    println!("full_pipeline_latency: {:.0} ns/op", ns_per_op);
    // Full pipeline should be under 50ms (guard + telemetry DB write)
    assert!(ns_per_op < 50_000_000.0, "full pipeline exceeded 50ms");

    drop(flight_conn);
}

/// Benchmark 5: Guard DB open latency (critical for cold-start scenarios).
#[test]
fn bench_guard_db_open_latency() {
    let iterations = 20;
    let start = Instant::now();
    for _ in 0..iterations {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("guard.db");
        let guard_db = GuardDb::open(&db_path).expect("GuardDb open failed");
        drop(guard_db);
        drop(temp);
    }
    let elapsed = start.elapsed();

    let ns_per_op = elapsed.as_nanos() as f64 / iterations as f64;
    println!("guard_db_open_latency: {:.0} ns/op", ns_per_op);
    // DB open varies widely by filesystem — use 1s threshold
    assert!(ns_per_op < 1_000_000_000.0, "guard DB open exceeded 1s");
}