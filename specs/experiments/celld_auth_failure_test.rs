//! Authority failure test — does crabjar's trust model survive infrastructure changes?
//!
//! Scenario:
//!   1. Agent stages a proposed action (status: pending)
//!   2. Node crashes (service stopped)
//!   3. Node recovers
//!   4. Verify: action is STILL pending, NOT auto-approved, NOT lost
//!
//! This tests the critical boundary: recovery/migration must not become an authorization event.
//!
//! Usage: rustc -o /tmp/auth-test specs/experiments/celld_auth_failure_test.rs && /tmp/auth-test

use std::process::Command;
use std::thread;
use std::time::Duration;

const BASE_URL: &str = "http://127.0.0.1:8788";
const QUEUE: &str = "auth-test-queue";
const ACTION_ID: &str = "critical-file-deletion";

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Authority Failure Test                                  ║");
    println!("║  Does recovery implicitly authorize pending actions?     ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    let url = format!("{}/?queue={}&action={}", BASE_URL, QUEUE, ACTION_ID);

    // Step 1: Stage a proposed action
    println!("Step 1: Agent stages proposed action");
    println!("       Action: '{}' — 'rm -rf /var/lib/important-data'", ACTION_ID);
    let body = r#"{"description": "rm -rf /var/lib/important-data"}"#;
    let stage_out = run_curl(&[
        "-s", "-X", "POST", "--connect-timeout", "3", "--max-time", "5",
        "-H", "Content-Type: application/json",
        "-d", body, &url,
    ]);
    match stage_out {
        Ok(out) if out.contains("\"status\":\"pending\"") => {
            println!("       ✅ Staged: status=pending");
        }
        Ok(out) => panic!("Unexpected staging response: {}", out),
        Err(e) => panic!("Failed to stage action: {}", e),
    }

    // Step 2: Verify it's pending
    println!();
    println!("Step 2: Verify action is pending (pre-crash)");
    let pre = run_curl(&["-s", "--connect-timeout", "3", "--max-time", "5", &url])
        .expect("failed to read action before crash");
    assert!(pre.contains("\"status\":\"pending\""), "action should be pending: {}", pre);
    assert!(pre.contains("\"decided_at\":null"), "action should not be decided yet: {}", pre);
    println!("       ✅ Confirmed: status=pending, decided_at=null");

    // Step 3: Simulate node crash — stop the service
    println!();
    println!("Step 3: SIMULATING NODE CRASH (stopping celld-dev-approval)");
    let kill = Command::new("dinitctl").arg("stop").arg("celld-dev-approval").output();
    match kill {
        Ok(o) if o.status.success() => println!("       ✅ Service stopped"),
        Ok(o) => eprintln!("       ⚠️ dinitctl stop: {}", String::from_utf8_lossy(&o.stderr)),
        Err(e) => panic!("Failed to stop service: {}", e),
    }
    thread::sleep(Duration::from_secs(2));

    // Verify it's actually down
    let down = run_curl(&["-s", "--connect-timeout", "2", "--max-time", "3", &url]);
    assert!(down.is_err(), "service should be unreachable after crash");
    println!("       ✅ Service confirmed unreachable");

    // Step 4: Recover the node
    println!();
    println!("Step 4: Recovering node (restarting celld-dev-approval)");
    let start = Command::new("dinitctl").arg("start").arg("celld-dev-approval").output();
    match start {
        Ok(o) if o.status.success() => println!("       ✅ Service restarted"),
        Ok(o) => eprintln!("       ⚠️ dinitctl start: {}", String::from_utf8_lossy(&o.stderr)),
        Err(e) => panic!("Failed to start service: {}", e),
    }

    // Wait for recovery
    println!("       Waiting for service to recover...");
    let mut recovered = false;
    for attempt in 1..=20 {
        thread::sleep(Duration::from_secs(1));
        if run_curl(&["-s", "--connect-timeout", "2", "--max-time", "3", &url]).is_ok() {
            println!("       ✅ Service recovered after {}s", attempt);
            recovered = true;
            break;
        }
    }
    assert!(recovered, "service did not recover within 20s");

    // Step 5: THE CRITICAL CHECK
    println!();
    println!("Step 5: CRITICAL CHECK — authorization state after recovery");
    let post = run_curl(&["-s", "--connect-timeout", "3", "--max-time", "5", &url])
        .expect("action should still exist after recovery");
    println!("       Post-recovery response: {}", post);

    if post.contains("\"status\":\"pending\"") && post.contains("\"decided_at\":null") {
        println!();
        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║  ✅ AUTHORITY FAILURE TEST PASSED                       ║");
        println!("║                                                  ║");
        println!("║  Action survived crash as PENDING.              ║");
        println!("║  Recovery did NOT implicitly authorize it.      ║");
        println!("║                                                  ║");
        println!("║  Trust model survives infrastructure changes.   ║");
        println!("╚══════════════════════════════════════════════════════════╝");
    } else if post.contains("\"status\":\"approved\"") {
        println!();
        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║  ❌ FAILED: Action was AUTO-APPROVED by recovery!       ║");
        println!("║     This is an authorization bypass.                   ║");
        println!("╚══════════════════════════════════════════════════════════╝");
        std::process::exit(1);
    } else {
        println!();
        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║  ⚠️  UNEXPECTED STATE after recovery                    ║");
        println!("╚══════════════════════════════════════════════════════════╝");
        std::process::exit(1);
    }

    // Bonus: verify authorization still works post-recovery
    println!();
    println!("Bonus: Verify authorization flow intact post-recovery");
    let approve_body = r#"{"status": "approved", "decided_by": "human-operator"}"#;
    let approve_out = run_curl(&[
        "-s", "-X", "PUT", "--connect-timeout", "3", "--max-time", "5",
        "-H", "Content-Type: application/json",
        "-d", approve_body, &url,
    ]);
    match approve_out {
        Ok(out) if out.contains("\"status\":\"approved\"") => {
            println!("       ✅ Action approved post-recovery by human-operator");
            println!("       ✅ Authorization flow fully intact after crash recovery");
        }
        Ok(out) => eprintln!("       ⚠️ Unexpected approval response: {}", out),
        Err(e) => eprintln!("       ⚠️ Post-recovery approval failed: {}", e),
    }
}

/// Run curl with the given args, return stdout on success or Err with details.
fn run_curl(args: &[&str]) -> Result<String, String> {
    let output = Command::new("curl").args(args).output()
        .map_err(|e| format!("curl spawn failed: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "curl exit {}: {}",
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if stdout.is_empty() {
        return Err("curl returned empty response".into());
    }

    Ok(stdout)
}
