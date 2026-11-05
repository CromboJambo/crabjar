//! Live review bridge (ADR-005, follow-on to item 5).
//!
//! The first real producer for the `TerminalEvent` relay. Wires the two
//! halves that ADR-005 built separately:
//!
//! 1. `terminal_relay::start` — the thin transport (this crate, outside
//!    the glass).
//! 2. `crabjar_terminal::HerdrBackend::run_command` — the structured
//!    execution path that mints typed `Receipt`s (inside the glass).
//!
//! The bridge sits at the seam: it owns a `SessionStream`, appends each
//! receipt, and publishes every new event to the relay via
//! `TerminalRelayState::publish_event`. It mints no ids and stamps no
//! times — the stream does both — so the relay stays stateless.
//!
//! `run` is the `--review` mode: it stays alive after the commands
//! finish, so a reviewer can join the session over WebSocket and inspect
//! the running herdr pane. Ctrl-C tears the session down.
//!
//! Known limitation (documented, not hidden): the relay has no backlog.
//! A reviewer that joins *after* an event was published does not receive
//! it — fan-out is live-only. `run` therefore waits before the first
//! command so a human can connect first. Replaying history is a separate
//! feature (a `SessionRecord` is already the faithful form).

use anyhow::{Context, Result};
use crabjar_terminal::{HerdrBackend, SessionStream, TerminalBackend};
use std::time::Duration;

use crate::terminal_relay;

/// Run the live review bridge.
///
/// Starts the relay on `port` (0 = ephemeral), spawns a herdr workspace
/// named `session`, runs `commands` through the structured round-trip,
/// and publishes every stream event to the relay as it lands. Blocks
/// until Ctrl-C.
pub async fn run(port: u16, session: &str, commands: &[&str]) -> Result<()> {
    let (addr, state) = terminal_relay::start("127.0.0.1".to_string(), port)
        .await
        .context("start terminal relay")?;

    let backend = HerdrBackend::new();
    if !backend.is_available_async().await {
        anyhow::bail!("herdr binary or server not reachable — start a herdr server first");
    }

    let workdir = std::env::temp_dir().join(format!(
        "crabjar-review-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&workdir).context("create scratch dir")?;
    backend
        .spawn(session, &workdir)
        .await
        .context("spawn herdr workspace")?;

    let ws_url = format!("ws://{addr}/ws");
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "review": session,
            "relay": addr.to_string(),
            "ws": ws_url,
            "join": { "type": "join", "session": session },
            "commands": commands,
            "note": "relay stays alive until Ctrl-C; events published before you joined are not replayed",
        }))?
    );

    let mut stream = SessionStream::new();
    let mut next_to_publish = 0usize;

    // Give a reviewer time to connect before the first event is
    // published (the relay has no backlog — see module docs). Configurable
    // via REVIEW_GRACE_SECS for slow-to-start reviewers.
    let grace_secs: u64 = std::env::var("REVIEW_GRACE_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    tracing::info!(grace_secs, "waiting for reviewers to join");
    tokio::time::sleep(Duration::from_secs(grace_secs)).await;

    for (i, command) in commands.iter().enumerate() {
        let receipt = backend
            .run_command(session, command, 30_000)
            .await
            .with_context(|| format!("run_command #{i} failed: {command}"))?;
        stream.push_receipt(&receipt);
        // Publish only the events appended by this receipt: the stream
        // is append-only, so everything from `next_to_publish` on is new.
        while next_to_publish < stream.events().len() {
            let event = &stream.events()[next_to_publish];
            let delivered = state.publish_event(session, event).await;
            if !delivered {
                tracing::warn!(
                    id = event.id(),
                    "event dropped — no client joined yet (relay has no backlog)"
                );
            }
            next_to_publish += 1;
        }
        tracing::info!(
            command = %command,
            exit = ?receipt.exit_code,
            "receipt published"
        );
        // Let the fan-out settle between commands so a live reviewer
        // sees distinct iterations, not a burst.
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    tracing::info!(
        events = next_to_publish,
        "commands complete — relay stays up until Ctrl-C"
    );
    tokio::signal::ctrl_c().await?;

    let _ = backend.kill_session(session).await;
    // Give the pane's shell a beat before removing the scratch dir.
    tokio::time::sleep(Duration::from_millis(250)).await;
    let _ = std::fs::remove_dir(&workdir);
    Ok(())
}
