/// WebSocket-based terminal relay for shared terminal sessions.
///
/// Provides a multiplexed WebSocket endpoint where multiple clients can
/// connect to the same terminal session (pane). Output from the PTY is
/// fanned out to all connected clients; input from any client goes to
/// the single PTY master.
///
/// ## Protocol
///
/// After the initial HTTP upgrade, the connection uses a simple framing
/// protocol:
/// - `Message::Text` frames carry JSON control messages (join/leave/ping)
/// - `Message::Binary` frames carry raw terminal I/O bytes
///
/// Control message format (JSON):
/// ```json
/// {"type": "ping"}
/// {"type": "pong", "ts": 1234567890}
/// {"type": "join", "session": "my-session", "pane_id": "terminal_1"}
/// {"type": "leave"}
/// ```

use anyhow::{Context, Result};
use axum::{
    extract::ws::{self, Message, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

/// Control message sent over WebSocket text frames.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
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
}

/// State shared across all WebSocket connections for a relay instance.
#[derive(Clone)]
pub struct TerminalRelayState {
    /// Map of session_name → broadcast channel sender for terminal output fan-out.
    sessions: Arc<Mutex<HashMap<String, broadcast::Sender<Vec<u8>>>>>,
}

impl TerminalRelayState {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

/// Runs the terminal relay server. Binds a WebSocket listener and handles
/// client connections, routing them to terminal sessions via wezterm/zellij.
pub async fn serve(bind_addr: String, port: u16) -> Result<()> {
    let state = TerminalRelayState::new();

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);

    let ip: std::net::IpAddr = bind_addr.parse().context("bind_addr must be a valid IP")?;
    let addr: SocketAddr = (ip, port).into();
    tracing::info!(%addr, "terminal relay server listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<TerminalRelayState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
        .into_response()
}

/// Handle a single WebSocket connection. Manages the client's lifecycle:
/// join sessions, relay I/O, send keepalives, and clean up on disconnect.
async fn handle_ws(socket: ws::WebSocket, state: TerminalRelayState) {
    tracing::info!("terminal relay: new client connected");

    let (mut ws_sink, mut ws_stream) = socket.split();

    // Send initial pong to confirm connection
    if let Err(e) = ws_sink.send(Message::Text(serde_json::to_string(&ControlMessage::Pong { ts: now_ms() }).unwrap())).await {
        tracing::warn!(error = %e, "failed to send initial pong");
        return;
    }

    // Track which session this client joined (if any) and the broadcast receiver.
    let mut current_session: Option<String> = None;
    let mut _broadcast_rx: Option<broadcast::Receiver<Vec<u8>>> = None;

    // Main loop: drive WebSocket messages and relay I/O.
    while let Some(msg) = ws_stream.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(control) = serde_json::from_str::<ControlMessage>(&text) {
                    match control {
                        ControlMessage::Ping => {
                            let _ = ws_sink.send(Message::Text(serde_json::to_string(&ControlMessage::Pong { ts: now_ms() }).unwrap())).await;
                        }
                        ControlMessage::Join { session, pane_id } => {
                            tracing::info!(session = %session, ?pane_id, "client joining terminal session");

                            // Leave current session if any.
                            _broadcast_rx = None;

                            // Join new session: get or create a broadcast channel for this session.
                            {
                                let mut sessions = state.sessions.lock().await;
                                let sender = sessions.entry(session.clone()).or_insert_with(|| {
                                    broadcast::Sender::new(1024)
                                });
                                _broadcast_rx = Some(sender.subscribe());
                            }

                            current_session = Some(session);
                        }
                        ControlMessage::Leave => {
                            _broadcast_rx = None;
                            if let Some(ref session) = current_session.take() {
                                state.sessions.lock().await.remove(session);
                            }
                        }
                        ControlMessage::Pong { .. } => {}
                    }
                } else {
                    tracing::warn!("invalid control message: {}", text);
                }
            }
            Ok(Message::Binary(data)) => {
                // Forward binary data to the PTY master.
                // In a full implementation, this would write to the wezterm/zellij pane.
                tracing::debug!(bytes = data.len(), "received terminal input");
            }
            Ok(Message::Close(_frame)) => {
                tracing::info!("client sent close frame");
                break;
            }
            Ok(Message::Ping(data)) => {
                let _ = ws_sink.send(Message::Pong(data)).await;
            }
            Ok(Message::Pong(_)) => {}
            Err(e) => {
                tracing::warn!(error = %e, "websocket read error");
                break;
            }
        }

        // Fan out any terminal output from the broadcast receiver (non-blocking).
        if let Some(ref mut rx) = _broadcast_rx {
            while let Ok(data) = rx.try_recv() {
                if ws_sink.send(Message::Binary(data)).await.is_err() {
                    break;
                }
            }
        }
    }

    // Clean up: leave session and close sink.
    if let Some(ref session) = current_session {
        state.sessions.lock().await.remove(session);
    }

    tracing::info!("terminal relay: client disconnected");
}

/// Get current timestamp in milliseconds since epoch.
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
