//! Celld mirror helper for pending guard actions.

/// Fire-and-forget mirror of a pending action to the celld approval-queue worker.
///
/// GuardDb remains the local authority — this is a durable copy for remote approval.
/// Failure here never blocks or fails the main command; reconciliation catches up later.
///
/// The celld endpoint is configured via CELLD_MIRROR_URL (default: http://127.0.0.1:8788).
pub fn mirror_pending_to_celld(action_id: &str, description: &str) {
    let base_url =
        std::env::var("CELLD_MIRROR_URL").unwrap_or_else(|_| "http://127.0.0.1:8788".to_string());
    let queue = std::env::var("CELLD_MIRROR_QUEUE").unwrap_or_else(|_| "crabjar-guard".to_string());

    let url = format!("{}/?queue={}&action={}", base_url, queue, action_id);
    // Escape for JSON body
    let escaped_desc = description.replace('"', "\\\"");
    let body = format!(r#"{{"description": "{}"}}"#, escaped_desc);

    let result = std::process::Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-d",
            &body,
            "--max-time",
            "3",
            &url,
        ])
        .output();

    match result {
        Ok(output) if output.status.success() => {
            eprintln!(
                "DEBUG mirror: pending action {} mirrored to celld ({})",
                action_id, queue
            );
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!(
                "DEBUG mirror: failed to mirror {}: {}",
                action_id,
                stderr.trim()
            );
        }
        Err(e) => {
            eprintln!("DEBUG mirror: curl error mirroring {}: {}", action_id, e);
        }
    }
}
