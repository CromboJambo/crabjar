//! HttpSessionMirror: concrete SessionMirror implementation using HTTP + JSON.
//!
//! Talks to the celld approval-queue worker (port 8788) as a durable store.
//! This is the "outside the glass" part — disposable, replaceable, no celld SDK needed.
//!
//! Worker API:
//!   POST /?queue=X&action=<id>  body: {description} → stage pending
//!   GET  /?queue=X&action=<id>              → {action: {...}}
//!   PUT  /?queue=X&action=<id>  body: {status, decided_by} → decide
//!
//! Build: `rustc --edition 2024 -L target/debug/deps ... specs/experiments/celld_session_mirror.rs -o /tmp/session-mirror`
//! Run: `/tmp/session-mirror`

use crabjar_guard::action::ActionStatus;
use crabjar_guard::mirror::{MirrorError, MirroredAction, SessionMirror};

const BASE_URL: &str = "http://127.0.0.1:8788";
const QUEUE: &str = "session-mirror-test";

/// HTTP implementation of SessionMirror that talks to the celld approval-queue worker.
pub struct HttpSessionMirror {
    base_url: String,
    queue: String,
}

impl HttpSessionMirror {
    pub fn new(base_url: &str, queue: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            queue: queue.to_string(),
        }
    }

    fn url(&self, action_id: &str) -> String {
        format!(
            "{}/?queue={}&action={}",
            self.base_url, self.queue, action_id
        )
    }

    fn curl(&self, args: &[&str], url: &str) -> Result<String, MirrorError> {
        let mut cmd = std::process::Command::new("curl");
        cmd.args(["-s", "--max-time", "5"]);
        cmd.args(args);
        cmd.arg(url);
        
        let output = cmd.output().map_err(|e| {
            MirrorError::Unreachable(format!("curl failed: {}", e))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MirrorError::Unreachable(format!(
                "HTTP error: {} (stderr: {})",
                output.status, stderr
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

impl SessionMirror for HttpSessionMirror {
    fn mirror_pending(&self, action: &MirroredAction) -> Result<(), MirrorError> {
        // Check if action already exists in the mirror.
        // If it has a terminal status (approved/rejected), don't overwrite —
        // GuardDb is authoritative and the decision is final.
        if let Some(existing) = self.get_action(&action.action_id)? {
            match existing.status.as_str() {
                "approved" | "rejected" => {
                    // Already decided — skip re-staging to avoid overwriting the decision.
                    return Ok(());
                }
                "pending" => {
                    // Already staged as pending — idempotent, nothing to do.
                    return Ok(());
                }
                _ => {} // Unknown status, fall through and re-stage
            }
        }

        let url = self.url(&action.action_id);
        let body = format!(r#"{{"description": "{}"}}"#, action.payload.replace('"', "\\\""));
        
        let args = vec![
            "-X", "POST",
            "-H", "Content-Type: application/json",
            "-d", &body,
        ];
        let response = self.curl(&args, &url)?;

        if !response.contains("\"status\":\"pending\"") {
            return Err(MirrorError::BadResponse(format!(
                "Expected pending status, got: {}", response
            )));
        }
        Ok(())
    }

    fn update_status(
        &self,
        action_id: &str,
        status: &ActionStatus,
        decided_by: Option<&str>,
    ) -> Result<(), MirrorError> {
        let worker_status = match status {
            ActionStatus::Executed | ActionStatus::TrustApproved => "approved",
            ActionStatus::Denied | ActionStatus::Interrupted => "rejected",
            ActionStatus::Pending => return Ok(()), // No-op
        };

        let url = self.url(action_id);
        let body = format!(
            r#"{{"status": "{}", "decided_by": "{}"}}"#,
            worker_status,
            decided_by.unwrap_or("crabjar-guard")
        );

        let args = vec![
            "-X", "PUT",
            "-H", "Content-Type: application/json",
            "-d", &body,
        ];
        let _response = self.curl(&args, &url)?;
        Ok(())
    }

    fn list_pending(&self) -> Result<Vec<MirroredAction>, MirrorError> {
        // The approval-queue worker doesn't have a "list all" endpoint.
        // This is a known limitation — we'd need to add one for full reconciliation.
        // For now, return empty (reconciliation will only catch GuardDb→celld drift).
        Ok(Vec::new())
    }

    fn get_action(&self, action_id: &str) -> Result<Option<MirroredAction>, MirrorError> {
        let url = self.url(action_id);
        let response = match self.curl(&[], &url) {
            Ok(r) => r,
            Err(MirrorError::Unreachable(_)) => return Ok(None),
            Err(e) => return Err(e),
        };

        if response.contains("\"error\"") || response.is_empty() {
            return Ok(None);
        }

        let parsed: serde_json::Value = serde_json::from_str(&response)
            .map_err(|e| MirrorError::Serde(format!("Failed to parse: {}", e)))?;

        // The worker nests the action under an "action" key
        let action_val = &parsed["action"];
        if action_val.is_null() {
            return Ok(None);
        }

        let status = action_val["status"].as_str().unwrap_or("unknown").to_string();
        let decided_at_ts = action_val["decided_at"].as_str().and_then(|s| {
            // Parse ISO timestamp to unix seconds (rough)
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.timestamp())
                .ok()
        });

        Ok(Some(MirroredAction {
            action_id: action_id.to_string(),
            status,
            action_type: "execute".to_string(), // Worker doesn't store type; default
            payload: action_val["description"].as_str().unwrap_or("").to_string(),
            requested_at: 0, // Worker stores staged_at as ISO string; not converting for now
            decided_at: decided_at_ts,
            decided_by: action_val["decided_by"].as_str().map(|s| s.to_string()),
            envelope: None,
        }))
    }

    fn is_available(&self) -> bool {
        let output = std::process::Command::new("curl")
            .args([
                "-s", "--max-time", "3", "-o", "/dev/null", "-w", "%{http_code}",
                &self.base_url,
            ])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let code = String::from_utf8_lossy(&o.stdout);
                code.starts_with('2') || code.starts_with('4')
            }
            _ => false,
        }
    }
}

fn main() {
    println!("=== Session Mirror Integration Test ===\n");

    let mirror = HttpSessionMirror::new(BASE_URL, QUEUE);

    // 1. Check availability
    println!("1. Checking mirror availability...");
    if !mirror.is_available() {
        eprintln!("   ❌ Mirror not available at {}", BASE_URL);
        eprintln!("   Is the approval-queue worker running? (dinitctl status celld-dev-approval)");
        std::process::exit(1);
    }
    println!("   ✅ Mirror available at {}\n", BASE_URL);

    // 2. Mirror a pending action
    println!("2. Mirroring pending action...");
    let action = MirroredAction {
        action_id: "mirror-test-001".to_string(),
        status: "pending".to_string(),
        action_type: "execute".to_string(),
        payload: "echo 'this is a session mirror test'".to_string(),
        requested_at: chrono::Utc::now().timestamp(),
        decided_at: None,
        decided_by: None,
        envelope: None,
    };

    match mirror.mirror_pending(&action) {
        Ok(()) => println!("   ✅ Action mirrored as pending"),
        Err(e) => {
            eprintln!("   ❌ Failed to mirror action: {}", e);
            std::process::exit(1);
        }
    }

    // 3. Verify it's there
    println!("\n3. Verifying action in mirror...");
    match mirror.get_action("mirror-test-001") {
        Ok(Some(a)) => {
            println!("   ✅ Found: status={}, payload={}", a.status, a.payload);
            assert_eq!(a.status, "pending", "Expected pending status");
        }
        Ok(None) => {
            eprintln!("   ❌ Action not found in mirror");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("   ❌ Failed to get action: {}", e);
            std::process::exit(1);
        }
    }

    // 4. Update status (approve)
    println!("\n4. Updating status to executed...");
    match mirror.update_status("mirror-test-001", &ActionStatus::Executed, Some("integration-test")) {
        Ok(()) => println!("   ✅ Status updated"),
        Err(e) => {
            eprintln!("   ❌ Failed to update status: {}", e);
            std::process::exit(1);
        }
    }

    // 5. Verify the update
    println!("\n5. Verifying status update...");
    match mirror.get_action("mirror-test-001") {
        Ok(Some(a)) => {
            println!("   ✅ Final state: status={}, decided_by={:?}", a.status, a.decided_by);
            assert_eq!(a.status, "approved", "Expected approved status after execution");
            assert_eq!(a.decided_by, Some("integration-test".to_string()));
        }
        Ok(None) => {
            eprintln!("   ❌ Action not found after update");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("   ❌ Failed to get action: {}", e);
            std::process::exit(1);
        }
    }

    // 6. Test idempotency: re-mirror of a decided action must NOT overwrite it
    println!("\n6. Testing re-mirror of decided action (should be no-op)...");
    let result = mirror.mirror_pending(&action);
    match result {
        Ok(()) => println!("   ✅ Re-mirror returned Ok (skipped, no overwrite)"),
        Err(e) => {
            eprintln!("   ❌ Re-mirror failed unexpectedly: {}", e);
            std::process::exit(1);
        }
    }

    // Verify the action is STILL approved (not reset to pending)
    match mirror.get_action("mirror-test-001") {
        Ok(Some(a)) => {
            assert_eq!(
                a.status, "approved",
                "OVERWRITE BUG: action was reset to {} after re-mirror",
                a.status
            );
            println!("   ✅ Action still approved after re-mirror attempt (no overwrite)");
        }
        Ok(None) => {
            eprintln!("   ❌ Action disappeared after re-mirror!");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("   ❌ Failed to verify: {}", e);
            std::process::exit(1);
        }
    }

    println!("\n=== All tests passed ===");
}
