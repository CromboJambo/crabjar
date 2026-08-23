//! ADR-002 spike: drive a local Herdr server through HerdrBackend.
//!
//! Exercises the seam end-to-end against a running herdr server:
//! availability → spawn workspace → send text → read output → split pane
//! → agent status → close workspace. Prints a structured JSON summary.
//!
//! Run: `cargo run -p crabjar-terminal --example herdr-spike`

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use crabjar_terminal::{HerdrBackend, TerminalBackend, TerminalSession};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let started = std::time::Instant::now();
    let mut steps: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    let mut failures: Vec<String> = Vec::new();

    // Keep a handle for split/agent-status; the session owns a clone that
    // shares the same session map.
    let backend = HerdrBackend::new();

    // 1. Availability (binary + live server)
    let available = backend.is_available_async().await;
    steps.insert(
        "is_available".into(),
        serde_json::json!({ "available": available }),
    );
    if !available {
        failures.push("herdr binary or server not reachable — start a herdr server first".into());
        return finish(&steps, &failures, started);
    }

    // 2. Spawn a workspace in a scratch dir
    let workdir = tempfile::tempdir().context("tempdir")?;
    let session_name = format!("spike-{}", std::process::id());

    let mut session = TerminalSession::new(
        Box::new(backend.clone()),
        &session_name,
        PathBuf::from(workdir.path()),
    );
    session.spawn().await.context("spawn herdr workspace")?;
    let pane_id = session.snapshot().await.ok().and_then(|s| s.pane_id);
    steps.insert(
        "spawn".into(),
        serde_json::json!({
            "session": session_name,
            "workspace_dir": workdir.path().to_string_lossy(),
            "pane_id": pane_id,
        }),
    );

    // 3. Send text and read it back
    let marker = format!("spike-marker-{}", std::process::id());
    session
        .send(&format!("echo {marker}\n"))
        .await
        .context("send_text")?;
    tokio::time::sleep(Duration::from_millis(750)).await;

    let output = session.read(12).await.context("read_output")?;
    let echoed = output.contains(&marker);
    steps.insert(
        "send_read".into(),
        serde_json::json!({
            "marker": marker,
            "echoed": echoed,
            "output_tail": output.lines().take(4).collect::<Vec<_>>(),
        }),
    );
    if !echoed {
        failures.push(format!("marker {marker} not found in read output"));
    }

    // 4. Split the pane (horizontal → down)
    match backend.split_pane_horizontal(&session_name, None).await {
        Ok(new_pane) => {
            steps.insert(
                "split_horizontal".into(),
                serde_json::json!({ "new_pane_id": new_pane }),
            );
        }
        Err(e) => {
            failures.push(format!("split_pane_horizontal: {e}"));
            steps.insert(
                "split_horizontal".into(),
                serde_json::json!({ "error": e.to_string() }),
            );
        }
    }

    // 5. Agent status (plain shell → expect None)
    match backend.agent_status(&session_name).await {
        Ok(status) => {
            steps.insert(
                "agent_status".into(),
                serde_json::json!({ "status": status, "note": "plain shell: None expected" }),
            );
        }
        Err(e) => failures.push(format!("agent_status: {e}")),
    }

    // 6. Tear down (workspace close kills all panes, including the split)
    session.stop().await.context("stop session")?;
    steps.insert("stop".into(), serde_json::json!(true));

    finish(&steps, &failures, started)
}

fn finish(
    steps: &BTreeMap<String, serde_json::Value>,
    failures: &[String],
    started: std::time::Instant,
) -> anyhow::Result<()> {
    let success = failures.is_empty();
    let report = serde_json::json!({
        "success": success,
        "spike": "herdr-backend",
        "adr": "ADR-002",
        "steps": steps,
        "failures": failures,
        "elapsed_ms": started.elapsed().as_millis(),
        "doubt": {
            "assumptions": [
                "local herdr server is running and protocol-compatible",
                "pane read --source visible returns the visible viewport text",
                "workspace close kills all panes in the workspace"
            ],
            "blind_spots": [
                "no reconnection logic if the server dies mid-session",
                "split panes are not tracked per-session (root pane only)",
                "agent_status not yet wired into TerminalSession"
            ],
            "last_validation": "manual CLI run against herdr 0.8.2, local socket",
            "stale_after": "herdr server upgrade or protocol bump"
        }
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
