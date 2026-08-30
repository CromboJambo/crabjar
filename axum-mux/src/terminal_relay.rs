//! Thin `TerminalEvent` transport for multi-client session synchronization
//! (ADR-005, item 5).
//!
//! This module is a *relay*, not a model: it forwards the typed terminal
//! event stream ([`crabjar_terminal::TerminalEvent`]) between a producer
//! (the session driver that owns the PTY) and any number of WebSocket
//! clients. It owns **no session state** — no id minting, no stream
//! buffering, no segmentation. The event model, block addressing, and
//! receipts all live in `crabjar-terminal` (inside the glass); this relay
//! is one of its wire representations (outside the glass).
//!
//! ## Protocol
//!
//! All frames are JSON text. Two disjoint vocabuaries share the `"type"`
//! tag — control messages (this module) and terminal events
//! (`crabjar-terminal`).
//!
//! ### Control messages
//! - `{"type": "ping"}` — keepalive from client or server
//! - `{"type": "pong", "ts": 1234567890}` — pong response
//! - `{"type": "join", "session": "my-session"}` — join a session
//! - `{"type": "leave"}` — leave the current session
//! - `{"type": "error", "message": "..."}` — server error notification
//!
//! ### Terminal events (fan-out)
//! Serialized `TerminalEvent`s: `{"type": "prompt", ...}`,
//! `{"type": "command", ...}`, `{"type": "output", ...}`,
//! `{"type": "raw", ...}`. These flow **server → client**, fanned out to
//! every client joined to the session.
//!
//! ### Terminal input (client → server)
//! Raw binary frames are terminal input bytes (keystrokes). They are
//! wrapped in a `Raw` event — the escape hatch — and fanned out to all
//! joined clients (including the sender). The relay stamps `at`; the
//! receiving session re-stamps `id` when it appends to its own stream, so
//! the relay never mints ids.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────┐     WebSocket      ┌─────────────────────┐   publish_event ┌──────────────┐
//! │          │    ──────────────►  │                     │ ◄──────────────  │ crabjar-     │
//! │ Client 1 │ ◄─────────────────  │  Terminal Relay     │                  │ terminal     │
//! │          │    WebSocket        │  (this module)      │                  │ (producer)   │
//! └──────────┘                     │                     │                  └──────────────┘
//! ┌──────────┐                     │                     │
//! │ Client 2 │ ◄─────────────────  │  no session state   │
//! │          │    WebSocket        │  (thin transport)   │
//! └──────────┘                     └─────────────────────┘
//! ```
//!
//! The SPICE/VNC display relay (`proxy.rs`) is a *different* concrete —
//! genuinely byte-transparent, no protocol decoding — and stays raw binary
//! (ADR-005 Decision 5). Do not force it into this typed stream.

use anyhow::{Context, Result};
use axum::{
    extract::ws::{self, Message, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
    routing::get,
    Router,
};
use crabjar_terminal::TerminalEvent;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, error, info, warn};

/// Broadcast channel capacity for per-session event fan-out.
const FANOUT_CAPACITY: usize = 1024;

/// Control message sent over WebSocket text frames.
///
/// The `"type"` tag is disjoint from `TerminalEvent`'s tag
/// (`prompt`/`command`/`output`/`raw`), so a text frame is either a control
/// message or a terminal event — never ambiguous.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ControlMessage {
    /// Keepalive ping from client or server.
    Ping,
    /// Pong response with timestamp (milliseconds since epoch).
    Pong { ts: u64 },
    /// Join a terminal session/pane.
    Join {
        session: String,
        #[serde(default)]
        pane_id: Option<String>,
    },
    /// Leave the current session.
    Leave,
    /// Error notification from server to client.
    Error { message: String },
}

/// State shared across all WebSocket connections for a relay instance.
///
/// Holds only the fan-out channels — no session state. The event stream
/// itself lives in `crabjar-terminal`; this is the wire.
#[derive(Clone)]
pub struct TerminalRelayState {
    /// Map of session_name → broadcast sender for event fan-out.
    sessions: Arc<Mutex<HashMap<String, SessionData>>>,
}

/// Data associated with a single terminal session in the relay.
struct SessionData {
    /// Broadcast sender for fanning out events to all connected clients.
    broadcast_tx: broadcast::Sender<String>,
    /// Number of currently connected clients.
    client_count: usize,
}

impl Default for TerminalRelayState {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalRelayState {
    /// Create an empty relay state.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Publish a `TerminalEvent` to all clients joined to `session_name`.
    ///
    /// This is the producer's entry point: the session driver (which owns
    /// the PTY and mints monotonic ids) serializes each event and hands it
    /// to the relay, which fans it out. The relay does not inspect, mutate,
    /// or store the event. Returns `true` if the session exists (i.e. at
    /// least one client is joined), `false` if the event is dropped
    /// because nobody is listening.
    pub async fn publish_event(&self, session_name: &str, event: &TerminalEvent) -> bool {
        let json = match serde_json::to_string(event) {
            Ok(j) => j,
            Err(e) => {
                error!(error = %e, session = %session_name, "failed to serialize event");
                return false;
            }
        };
        let mut sessions = self.sessions.lock().await;
        match sessions.get_mut(session_name) {
            Some(data) => {
                // Lagged receivers are dropped by the broadcast channel; a
                // full channel (no receivers) is a no-op. Neither is an
                // error — the relay is best-effort fan-out.
                let _ = data.broadcast_tx.send(json);
                true
            }
            None => {
                debug!(session = %session_name, "no session — event dropped");
                false
            }
        }
    }

    /// Whether a session currently has joined clients.
    ///
    /// Used by the test suite to verify join bookkeeping; a session driver
    /// can use it before publishing to avoid dropped events.
    #[cfg_attr(not(test), allow(dead_code))]
    pub async fn has_session(&self, session_name: &str) -> bool {
        self.sessions.lock().await.contains_key(session_name)
    }

    /// Get or create a broadcast channel for the given session.
    async fn get_or_create_session(
        &self,
        session_name: &str,
    ) -> Result<broadcast::Receiver<String>> {
        let mut sessions = self.sessions.lock().await;

        if let Some(session_data) = sessions.get_mut(session_name) {
            session_data.client_count += 1;
            return Ok(session_data.broadcast_tx.subscribe());
        }

        let (broadcast_tx, _) = broadcast::channel(FANOUT_CAPACITY);
        sessions.insert(
            session_name.to_string(),
            SessionData {
                broadcast_tx: broadcast_tx.clone(),
                client_count: 1,
            },
        );

        Ok(broadcast_tx.subscribe())
    }

    /// Remove a session when all clients have disconnected.
    async fn remove_session(&self, session_name: &str) {
        let mut sessions = self.sessions.lock().await;
        if let Some(session_data) = sessions.get_mut(session_name) {
            session_data.client_count = session_data.client_count.saturating_sub(1);
            if session_data.client_count == 0 {
                sessions.remove(session_name);
            }
        }
    }
}

/// Bind the listener for the relay and return the bound address plus the
/// shared state (the producer's handle).
async fn bind(
    bind_addr: String,
    port: u16,
) -> Result<(SocketAddr, tokio::net::TcpListener, TerminalRelayState)> {
    let state = TerminalRelayState::new();
    let ip: IpAddr = bind_addr.parse().context("bind_addr must be a valid IP")?;
    let addr: SocketAddr = (ip, port).into();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bound = listener
        .local_addr()
        .context("bound listener has no local address")?;
    Ok((bound, listener, state))
}

/// Runs the terminal relay server. Binds a WebSocket listener and handles
/// client connections, fanning out `TerminalEvent` frames to all clients
/// joined to a session. Blocks until the server exits.
pub async fn serve(bind_addr: String, port: u16) -> Result<()> {
    let (addr, listener, state) = bind(bind_addr, port).await?;
    info!(%addr, "terminal relay server listening");
    axum::serve(listener, router(state)).await?;
    Ok(())
}

/// Bind the relay and return the bound address plus the shared state.
///
/// The state is the producer's handle: call [`TerminalRelayState::publish_event`]
/// to fan out events to joined clients. The server runs in the background
/// (the caller owns the returned state; drop it to stop publishing).
///
/// `serve` is the binary's entry point (see `main.rs`); `start` is the
/// embedded entry point for a session driver that needs the producer
/// handle (ADR-005 item 5). Both are exercised by the test suite.
#[cfg_attr(not(test), allow(dead_code))]
pub async fn start(bind_addr: String, port: u16) -> Result<(SocketAddr, TerminalRelayState)> {
    let (bound, listener, state) = bind(bind_addr, port).await?;
    // Spawn the server; the caller keeps the state handle for publishing.
    let server_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router(server_state)).await {
            error!(error = %e, "terminal relay server exited");
        }
    });
    Ok((bound, state))
}

/// Build the axum router for the relay.
fn router(state: TerminalRelayState) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<TerminalRelayState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
        .into_response()
}

/// Handle a single WebSocket connection. Manages the client's lifecycle:
/// join sessions, relay I/O, send keepalives, and clean up on disconnect.
async fn handle_ws(socket: ws::WebSocket, state: TerminalRelayState) {
    info!("terminal relay: new client connected");

    let (mut ws_sink, mut ws_stream) = socket.split();

    // Send initial pong to confirm connection.
    if let Err(e) = ws_sink
        .send(Message::Text(
            serde_json::to_string(&ControlMessage::Pong { ts: now_ms() }).unwrap_or_default(),
        ))
        .await
    {
        warn!(error = %e, "failed to send initial pong");
        return;
    }

    // Track which session this client joined (if any) and the broadcast receiver.
    let mut current_session: Option<String> = None;
    let mut _broadcast_rx: Option<broadcast::Receiver<String>> = None;

    // Main loop: drive WebSocket messages and relay I/O, racing the
    // broadcast receiver so idle clients still receive published events.
    loop {
        tokio::select! {
            msg = ws_stream.next() => {
                let Some(msg) = msg else {
                    break; // stream ended
                };
                match msg {
                    Ok(Message::Text(text)) => {
                        // A text frame is either a control message or a
                        // terminal event. Control first (the client's
                        // vocabulary); if that fails, treat it as an
                        // inbound event (the producer's vocabulary) and
                        // fan it out.
                        if let Ok(control) = serde_json::from_str::<ControlMessage>(&text) {
                            match control {
                                ControlMessage::Ping => {
                                    let _ = ws_sink
                                        .send(Message::Text(
                                            serde_json::to_string(&ControlMessage::Pong {
                                                ts: now_ms(),
                                            })
                                            .unwrap_or_default(),
                                        ))
                                        .await;
                                }
                                ControlMessage::Join { session, pane_id } => {
                                    info!(session = %session, ?pane_id, "client joining terminal session");

                                    // Leave current session if any.
                                    _broadcast_rx = None;

                                    match state.get_or_create_session(&session).await {
                                        Ok(rx) => {
                                            _broadcast_rx = Some(rx);
                                            current_session = Some(session);
                                        }
                                        Err(e) => {
                                            warn!(error = %e, "failed to join session");
                                            let _ = ws_sink
                                                .send(Message::Text(
                                                    serde_json::to_string(&ControlMessage::Error {
                                                        message: format!("Failed to join session: {e}"),
                                                    })
                                                    .unwrap_or_default(),
                                                ))
                                                .await;
                                        }
                                    }
                                }
                                ControlMessage::Leave => {
                                    if let Some(ref session) = current_session.clone() {
                                        state.remove_session(session).await;
                                    }
                                    _broadcast_rx = None;
                                    current_session = None;
                                }
                                ControlMessage::Pong { .. } => {}
                                ControlMessage::Error { message } => {
                                    error!(message = %message, "received error from server");
                                }
                            }
                        } else if let Ok(event) = serde_json::from_str::<TerminalEvent>(&text) {
                            // Inbound terminal event (producer pushing
                            // through a client connection). Fan out to all
                            // joined clients — including the sender (the
                            // broadcast reaches only the *other*
                            // subscribers, so the sender gets a direct
                            // send).
                            if let Some(ref session) = current_session {
                                let _ = ws_sink.send(Message::Text(text.clone())).await;
                                let _ = state.publish_event(session, &event).await;
                            } else {
                                warn!("received terminal event before join — dropped");
                            }
                        } else {
                            warn!("invalid control message: {text}");
                        }
                    }
                    Ok(Message::Binary(data)) => {
                        // Terminal input bytes (keystrokes). Wrap in a Raw
                        // event — the escape hatch — and fan out to all
                        // joined clients. The relay stamps `at`; the
                        // receiving session re-stamps `id` when it appends
                        // to its own stream.
                        let data_str = String::from_utf8_lossy(&data).to_string();
                        let raw_event = TerminalEvent::Raw {
                            id: 0, // placeholder — the receiving session mints the real id
                            data: data_str,
                            at: chrono::Utc::now(),
                        };
                        if let Some(ref session) = current_session {
                            debug!(bytes = data.len(), session = %session, "fanning out terminal input as Raw event");
                            // Direct send to the sender (the broadcast
                            // reaches only the *other* subscribers), then
                            // fan out to the rest.
                            if let Ok(json) = serde_json::to_string(&raw_event) {
                                let _ = ws_sink.send(Message::Text(json)).await;
                            }
                            let _ = state.publish_event(session, &raw_event).await;
                        } else {
                            warn!(bytes = data.len(), "received terminal input before join — dropped");
                        }
                    }
                    Ok(Message::Close(_frame)) => {
                        info!("client sent close frame");
                        break;
                    }
                    Ok(Message::Ping(data)) => {
                        let _ = ws_sink.send(Message::Pong(data)).await;
                    }
                    Ok(Message::Pong(_)) => {}
                    Err(e) => {
                        warn!(error = %e, "websocket read error");
                        break;
                    }
                }
            }
            // Fan out published events to this client even while it is
            // idle (no inbound WS traffic).
            json = async {
                match _broadcast_rx.as_mut() {
                    Some(rx) => rx.recv().await,
                    None => std::future::pending().await,
                }
            } => {
                // Channel closed (all senders dropped) or lagged —
                // best-effort fan-out; keep relaying.
                if let Ok(json) = json {
                    if ws_sink.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
        }
    }

    // Clean up: leave session and close sink.
    if let Some(ref session) = current_session {
        state.remove_session(session).await;
    }

    info!("terminal relay: client disconnected");
}

/// Get current timestamp in milliseconds since epoch.
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
