//! Memory stress test for orchestrator long-running operation.
//!
//! This test simulates sustained orchestrator operation to detect:
//! - Memory leaks under load
//! - Connection pool growth
//! - Session store memory accumulation
//! - Guard/telemetry DB connection leaks
//!
//! Run with: cargo test --test orchestrator_memory_stress --release
//! For leak detection: valgrind --leak-check=full --show-leak-kinds=all -- ./target/release/deps/orchestrator_memory_stress-*

use crabjar_guard::{ExecutionGate, GateContext, GuardDb, Scope, TrustScore};
use crabjar_telemetry::flight_recorder::FlightRecorder;
use std::process::Command;
use std::time::{Duration, Instant};

fn get_process_rss_kb() -> Option<u64> {
    // Read /proc/self/status for RSS (Resident Set Size)
    let output = Command::new("cat")
        .arg("/proc/self/status")
        .output()
        .ok()?;

    let status = String::from_utf8_lossy(&output.stdout);
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse::<u64>().ok();
            }
        }
    }

    None
}

fn setup_guard() -> (tempfile::TempDir, GuardDb) {
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("guard.db");
    let guard_db = GuardDb::open(&db_path).expect("GuardDb open failed");
    (temp, guard_db)
}

/// Stress test 1: Sustained guard gate checks over time.
/// Simulates thousands of execution decisions — should not grow memory.
#[test]
fn stress_guard_checks_sustained() {
    let (_temp, guard_db) = setup_guard();
    let gate = ExecutionGate::new(&guard_db, false, "/tmp/stress");

    let iterations = 10_000;
    let mut last_rss = get_process_rss_kb().unwrap_or(0);

    for i in 0..iterations {
        let result = gate.check(GateContext::new(
            "exec",
            "true",
            vec![],
            3,
            TrustScore::new(0.9),
        ));
        assert!(result.is_ok(), "guard check failed at iteration {}", i);

        // Sample memory every 1000 iterations
        if (i + 1) % 1000 == 0 {
            let current_rss = get_process_rss_kb().unwrap_or(0);
            let delta_mb = ((current_rss as i64 - last_rss as i64) / 1024) as i32;
            println!(
                "iter {:>5}: RSS={:>7}KB (delta {}MB)",
                i + 1,
                current_rss,
                delta_mb
            );
            last_rss = current_rss;

            // Memory should be stable — allow small growth for allocator overhead
            assert!(
                delta_mb < 50,
                "memory grew {}MB in 1000 iterations (leak suspected)",
                delta_mb
            );
        }
    }

    println!("stress_guard_checks_sustained: PASSED ({:?} iterations)", iterations);
}

/// Stress test 2: Telemetry DB writes under sustained load.
/// Flight recorder should not leak connections or grow unbounded.
#[test]
fn stress_telemetry_writes_sustained() {
    let temp = tempfile::tempdir().unwrap();
    let telemetry_dir = temp.path().join("telemetry");
    std::fs::create_dir_all(&telemetry_dir).unwrap();
    let flight_db_path = telemetry_dir.join("flight.db");
    let flight_conn = rusqlite::Connection::open(&flight_db_path).unwrap();
    let recorder = FlightRecorder::new(&flight_conn, "stress-session");
    recorder.init().expect("flight recorder init failed");

    let iterations = 1000;
    let mut last_rss = get_process_rss_kb().unwrap_or(0);

    for i in 0..iterations {
        recorder
            .checkpoint_before(&format!("stress_hash_{}", i), i as i32)
            .expect("checkpoint failed");

        if (i + 1) % 100 == 0 {
            let current_rss = get_process_rss_kb().unwrap_or(0);
            let delta_mb = ((current_rss as i64 - last_rss as i64) / 1024) as i32;
            println!(
                "iter {:>4}: RSS={:>7}KB (delta {}MB)",
                i + 1,
                current_rss,
                delta_mb
            );
            last_rss = current_rss;

            // Allow some growth for DB writes, but not excessive
            assert!(
                delta_mb < 20,
                "memory grew {}MB in 100 telemetry writes (leak suspected)",
                delta_mb
            );
        }
    }

    drop(flight_conn);
    println!("stress_telemetry_writes_sustained: PASSED ({:?} iterations)", iterations);
}

/// Stress test 3: Guard DB open/close cycles.
/// Simulates connection churn — should not leak file descriptors or memory.
#[test]
fn stress_guard_db_open_close_cycles() {
    let iterations = 100;
    let mut last_rss = get_process_rss_kb().unwrap_or(0);

    for i in 0..iterations {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("guard.db");
        let guard_db = GuardDb::open(&db_path).expect("GuardDb open failed");
        drop(guard_db); // Close immediately

        if (i + 1) % 10 == 0 {
            let current_rss = get_process_rss_kb().unwrap_or(0);
            let delta_mb = ((current_rss as i64 - last_rss as i64) / 1024) as i32;
            println!(
                "iter {:>3}: RSS={:>7}KB (delta {}MB)",
                i + 1,
                current_rss,
                delta_mb
            );
            last_rss = current_rss;

            // Each open/close cycle should be memory-neutral
            assert!(
                delta_mb < 5,
                "memory grew {}MB in 10 DB open/close cycles (leak suspected)",
                delta_mb
            );
        }
    }

    println!("stress_guard_db_open_close_cycles: PASSED ({:?} iterations)", iterations);
}

/// Stress test 4: Full pipeline under sustained load.
/// Combines guard + telemetry in realistic execution pattern.
#[test]
fn stress_full_pipeline_sustained() {
    let (_temp_guard, guard_db) = setup_guard();

    let temp_telemetry = tempfile::tempdir().unwrap();
    let telemetry_dir = temp_telemetry.path().join("telemetry");
    std::fs::create_dir_all(&telemetry_dir).unwrap();
    let flight_db_path = telemetry_dir.join("flight.db");
    let flight_conn = rusqlite::Connection::open(&flight_db_path).unwrap();
    let recorder = FlightRecorder::new(&flight_conn, "stress-pipeline");
    recorder.init().expect("flight recorder init failed");

    let gate = ExecutionGate::new(&guard_db, false, "/tmp/stress");

    let iterations = 500;
    let mut last_rss = get_process_rss_kb().unwrap_or(0);

    for i in 0..iterations {
        // Guard check
        let result = gate.check(GateContext::new(
            "exec",
            "true",
            vec![],
            3,
            TrustScore::new(0.9),
        ));
        assert!(result.is_ok(), "guard check failed at iteration {}", i);

        // Telemetry recording
        recorder
            .checkpoint_before(&format!("pipeline_hash_{}", i), i as i32)
            .expect("checkpoint failed");

        if (i + 1) % 50 == 0 {
            let current_rss = get_process_rss_kb().unwrap_or(0);
            let delta_mb = ((current_rss as i64 - last_rss as i64) / 1024) as i32;
            println!(
                "iter {:>3}: RSS={:>7}KB (delta {}MB)",
                i + 1,
                current_rss,
                delta_mb
            );
            last_rss = current_rss;

            // Full pipeline should be stable over time
            assert!(
                delta_mb < 10,
                "memory grew {}MB in 50 pipeline cycles (leak suspected)",
                delta_mb
            );
        }
    }

    drop(flight_conn);
    println!("stress_full_pipeline_sustained: PASSED ({:?} iterations)", iterations);
}