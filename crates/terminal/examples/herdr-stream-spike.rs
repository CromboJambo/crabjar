//! ADR-005 spike: structured herdr execution → typed `Receipt` → `TerminalEvent` stream.
//!
//! Proves the segmentation claim on ONE backend (herdr): command/output
//! boundaries come from the structured round-trip (`pane run` + sentinel +
//! `wait-output` + `pane read`), not from PTY scraping.
//!
//! Run: `cargo run -p crabjar-terminal --example herdr-stream-spike`

use std::collections::BTreeMap;

use anyhow::Context;
use crabjar_terminal::{HerdrBackend, SessionRecord, SessionStream, TerminalBackend};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let started = std::time::Instant::now();
    let mut steps: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    let mut failures: Vec<String> = Vec::new();

    let backend = HerdrBackend::new();
    if !backend.is_available_async().await {
        failures.push("herdr binary or server not reachable — start a herdr server first".into());
        return finish(&steps, &failures, started);
    }

    let workdir = std::env::temp_dir().join(format!(
        "crabjar-stream-spike-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&workdir).context("create scratch dir")?;
    let session_name = format!("stream-spike-{}", std::process::id());
    backend
        .spawn(&session_name, &workdir)
        .await
        .context("spawn herdr workspace")?;
    steps.insert(
        "spawn".into(),
        serde_json::json!({ "session": session_name, "cwd": workdir.to_string_lossy() }),
    );

    let mut stream = SessionStream::new();

    // 1. Success case: multi-line output, exit 0.
    let r1 = backend
        .run_command(&session_name, "echo line1; echo line2", 10_000)
        .await
        .context("run_command #1")?;
    steps.insert("receipt_1".into(), serde_json::to_value(&r1)?);
    if r1.exit_code != Some(0) {
        failures.push(format!("receipt #1: expected exit 0, got {:?}", r1.exit_code));
    }
    if r1.output.lines().count() != 2 || !r1.output.contains("line1") {
        failures.push(format!("receipt #1: output boundary wrong: {:?}", r1.output));
    }
    stream.push_receipt(&r1);

    // 2. Failure case: nonzero exit must survive the round-trip.
    //    Subshell so `exit 7` doesn't kill the pane's interactive shell;
    //    $? then carries the subshell's 7.
    let r2 = backend
        .run_command(&session_name, "(echo boom >&2; exit 7)", 10_000)
        .await
        .context("run_command #2")?;
    steps.insert("receipt_2".into(), serde_json::to_value(&r2)?);
    if r2.exit_code != Some(7) {
        failures.push(format!("receipt #2: expected exit 7, got {:?}", r2.exit_code));
    }
    if !r2.output.contains("boom") {
        failures.push(format!("receipt #2: stderr missing from output: {:?}", r2.output));
    }
    stream.push_receipt(&r2);

    // 3. cwd must be the workspace dir herdr reported.
    let r3 = backend
        .run_command(&session_name, "pwd", 10_000)
        .await
        .context("run_command #3")?;
    steps.insert("receipt_3".into(), serde_json::to_value(&r3)?);
    if r3.cwd.as_deref() != Some(workdir.to_str().unwrap()) {
        failures.push(format!("receipt #3: cwd mismatch: {:?}", r3.cwd));
    }
    stream.push_receipt(&r3);

    // 4. Blocks: one addressable cell per command.
    let blocks = stream.blocks();
    steps.insert("blocks".into(), serde_json::json!({ "count": blocks.len() }));
    if blocks.len() != 3 {
        failures.push(format!("expected 3 blocks, got {}", blocks.len()));
    }
    if blocks.iter().any(|b| b.command.is_none()) {
        failures.push("a block lost its command — segmentation failed".into());
    }

    // 5. Native JSONL round-trip (the faithful on-disk form).
    let record = SessionRecord {
        version: crabjar_terminal::STREAM_VERSION,
        session: session_name.clone(),
        backend: "herdr".into(),
        events: stream.events().to_vec(),
    };
    let jsonl = record.to_jsonl();
    let parsed = SessionRecord::from_jsonl(&jsonl).context("jsonl round-trip")?;
    if parsed.events != record.events {
        failures.push("JSONL round-trip changed the event stream".into());
    }
    steps.insert(
        "jsonl".into(),
        serde_json::json!({ "lines": jsonl.lines().count(), "events": parsed.events.len() }),
    );

    // Tear down. The pane's shell exits asynchronously; give it a beat so
    // the scratch dir isn't removed out from under a still-alive process.
    backend.kill_session(&session_name).await.context("kill session")?;
    steps.insert("stop".into(), serde_json::json!(true));
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    let _ = std::fs::remove_dir(&workdir);

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
        "spike": "herdr-stream",
        "adr": "ADR-005",
        "steps": steps,
        "failures": failures,
        "elapsed_ms": started.elapsed().as_millis(),
        "doubt": {
            "assumptions": [
                "local herdr server is running and protocol-compatible (0.8.2)",
                "pane run submits one line to the pane's shell",
                "pane read --source recent covers the last 2000 lines"
            ],
            "blind_spots": [
                "the wrapper line is parsed by the pane's shell — commands with unbalanced quotes or multi-line scripts will misbehave",
                "very long output may exceed the 2000-line read window",
                "concurrent run_command on one pane would interleave markers"
            ],
            "last_validation": "manual CLI run against herdr 0.8.2, local socket",
            "stale_after": "herdr server upgrade or protocol bump"
        }
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !success {
        std::process::exit(1);
    }
    Ok(())
}
