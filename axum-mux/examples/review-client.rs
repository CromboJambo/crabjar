//! Reviewer client for the live review bridge.
//!
//! Connects to a running `vm-bridge --review` relay, joins the session,
//! and prints every `TerminalEvent` as it arrives — the unintrusive
//! review path (ADR-005 follow-on). This is the seed of the temporal
//! reviewer: it renders the stream as it scrolls, hands off the keyboard.
//!
//! Run:
//!   cargo run -p vm-bridge --example review-client -- <ws-url> <session> [max-events]
//!
//! Exits after `max-events` terminal events (default: unlimited) or on
//! stream end.

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

type Ws =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Read the next terminal event (skipping control frames), with a timeout.
async fn read_event(ws: &mut Ws, timeout_s: u64) -> Result<serde_json::Value> {
    loop {
        let msg = tokio::time::timeout(std::time::Duration::from_secs(timeout_s), ws.next())
            .await
            .context("timed out waiting for event")?
            .context("stream ended")?
            .context("websocket read error")?;
        match msg {
            WsMessage::Close(_) => anyhow::bail!("server closed the connection"),
            WsMessage::Pong(_) => continue,
            WsMessage::Text(text) => {
                let v: serde_json::Value = serde_json::from_str(&text)?;
                // Control frames (pong/join-ack) are not terminal events.
                let t = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
                if matches!(t, "pong" | "error") {
                    continue;
                }
                return Ok(v);
            }
            _ => continue,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let url = args
        .get(1)
        .context("usage: review-client <ws-url> <session> [max-events]")?;
    let session = args
        .get(2)
        .context("usage: review-client <ws-url> <session> [max-events]")?;
    let max_events: Option<usize> = args.get(3).and_then(|s| s.parse().ok());

    let (mut ws, _) = connect_async(url).await.context("connect failed")?;
    // The server sends an initial pong on connect — consume it raw
    // (read_event skips pongs, so it would block here).
    let first = tokio::time::timeout(std::time::Duration::from_secs(5), ws.next())
        .await
        .context("timed out waiting for connect pong")?
        .context("stream ended")?;
    match first {
        Ok(WsMessage::Pong(_)) | Ok(WsMessage::Text(_)) => {}
        other => anyhow::bail!("unexpected first frame: {other:?}"),
    }

    let join = serde_json::json!({ "type": "join", "session": session }).to_string();
    ws.send(WsMessage::Text(join)).await.context("send join")?;
    // Settle: the server processes the join asynchronously.
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    let mut seen = 0usize;
    loop {
        let v = read_event(&mut ws, 60).await?;
        seen += 1;
        let t = v.get("type").and_then(|t| t.as_str()).unwrap_or("?");
        let id = v.get("id").and_then(|i| i.as_u64()).unwrap_or(u64::MAX);
        let detail = match t {
            "command" => v
                .get("text")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            "output" => {
                let data = v.get("data").and_then(|s| s.as_str()).unwrap_or("");
                let exit = v
                    .get("exit_code")
                    .map(|e| e.to_string())
                    .unwrap_or_default();
                let first = data.lines().next().unwrap_or("");
                format!("{first} (exit {exit})")
            }
            "prompt" => v
                .get("cwd")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string(),
            "raw" => v
                .get("data")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            other => other.to_string(),
        };
        println!("{seen:>2} {t:<8} #{id:<3} {detail}");
        if let Some(max) = max_events {
            if seen >= max {
                break;
            }
        }
    }

    let report = serde_json::json!({
        "success": true,
        "reviewer": "review-client",
        "session": session,
        "events_seen": seen,
        "doubt": {
            "assumptions": ["the bridge is still running and publishing"],
            "blind_spots": ["events published before join are not replayed (no backlog)"],
            "last_validation": "live run against vm-bridge --review",
            "stale_after": "relay protocol change"
        }
    });
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}
