//! Minimal celld bridge — proves crabjar (Rust) can interact with celld cells.
//!
//! This is an experiment, not production code. It demonstrates:
//! 1. Rust → HTTP → celld worker → Durable Object cell
//! 2. State persists across service restarts
//! 3. The session-lifecycle contract (abstract interface)
//!
//! Usage:
//!   cargo run --example celld-bridge-minimal
//!   # or directly:
//!   rustc -o /tmp/celld-bridge example/celld_bridge_minimal.rs && /tmp/celld-bridge

use std::env;
use std::process::Command;
use std::time::Duration;

fn main() {
    let base_url = env::args().nth(1).unwrap_or_else(|| "http://127.0.0.1:8787".into());
    let session_name = env::args().nth(2).unwrap_or_else(|| format!("rust-bridge-{}", std::process::id()));

    println!("celld bridge minimal test");
    println!("  target:   {}", base_url);
    println!("  session:  {}", session_name);
    println!();

    // Step 1: Write state (increment counter)
    let url1 = format!("{}/?name={}", base_url, session_name);
    println!("Step 1: GET {} (write/increment)", url1);
    let n1 = http_get_json_n(&url1).expect("first request failed");
    println!("  → n = {}", n1);

    // Step 2: Read state again
    println!("Step 2: GET {} (read/increment again)", url1);
    let n2 = http_get_json_n(&url1).expect("second request failed");
    println!("  → n = {}", n2);
    assert_eq!(n2, n1 + 1, "counter should have incremented");

    // Step 3: Restart the celld service (simulates node restart)
    println!();
    println!("Step 3: Restarting celld-dev-counter service...");
    let restart = Command::new("dinitctl")
        .arg("restart")
        .arg("celld-dev-counter")
        .output()
        .expect("failed to run dinitctl");
    if !restart.status.success() {
        eprintln!("  dinitctl restart failed: {}", String::from_utf8_lossy(&restart.stderr));
        std::process::exit(1);
    }
    println!("  Service restarted. Waiting for it to come back...");

    // Wait for service to be ready (use a separate probe name so we don't increment the test counter)
    let probe_url = format!("{}/?name=__health_probe__", base_url);
    for attempt in 1..=20 {
        std::thread::sleep(Duration::from_secs(1));
        if http_get_json_n(&probe_url).is_ok() {
            println!("  Service ready after {}s", attempt);
            break;
        }
        if attempt == 20 {
            eprintln!("  Service did not come back within 20s");
            std::process::exit(1);
        }
    }

    // Step 4: Read state after restart — should continue from n2
    println!();
    println!("Step 4: GET {} (after restart)", url1);
    let n3 = http_get_json_n(&url1).expect("post-restart request failed");
    println!("  → n = {}", n3);

    // Verify persistence
    if n3 == n2 + 1 {
        println!();
        println!("✅ SUCCESS: State persisted across service restart");
        println!("   Counter continued from {} to {} (no reset)", n2, n3);
        println!();
        println!("This proves the session-lifecycle contract:");
        println!("  - crabjar (Rust) can write to a celld cell via HTTP");
        println!("  - State survives process/node restart");
        println!("  - No celld-specific knowledge needed in crabjar — just HTTP + JSON");
    } else {
        eprintln!();
        eprintln!("❌ FAILURE: Expected n={}, got n={}", n2 + 1, n3);
        eprintln!("   State did NOT persist across restart.");
        std::process::exit(1);
    }
}

/// Minimal HTTP GET that extracts the "n" field from a JSON response.
/// Uses curl to avoid adding dependencies for this experiment.
fn http_get_json_n(url: &str) -> Result<i64, String> {
    let output = Command::new("curl")
        .args(["-s", "--connect-timeout", "3", "--max-time", "5", url])
        .output()
        .map_err(|e| format!("curl failed: {}", e))?;

    if !output.status.success() {
        return Err(format!("curl exit {}: {}", output.status.code().unwrap_or(-1), String::from_utf8_lossy(&output.stderr)));
    }

    let body = String::from_utf8_lossy(&output.stdout);
    // Parse {"n":123,...} — find "n": and extract the number after it
    let n_str = body
        .split("\"n\":")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .ok_or_else(|| format!("could not parse 'n' from: {}", body))?;

    n_str
        .trim()
        .parse::<i64>()
        .map_err(|e| format!("parse error: {}: {}", e, body))
}
