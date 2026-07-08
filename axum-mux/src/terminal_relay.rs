/// Shared terminal protocol for multi-client session synchronization over WebSocket relay.
///
/// This module implements a bidirectional terminal relay that allows multiple WebSocket
/// clients to connect to the same terminal session (pane). Input from any client goes
/// to the single PTY master, and output is fanned out to all connected clients.
///
/// ## Protocol Overview
///
/// The protocol uses JSON control messages over text frames and raw terminal I/O bytes
/// over binary frames:
///
/// ### Control Messages (Text Frames)
/// - `{"type": "ping"}` - Keepalive ping from client or server
/// - `{"type": "pong", "ts": 1234567890}` - Pong response with timestamp
/// - `{"type": "join", "session": "my-session", "pane_id": "terminal_1"}` - Join a session
/// - `{"type": "leave"}` - Leave the current session
/// - `{"type": "error", "message": "..."}` - Error notification from server
///
/// ### Terminal I/O (Binary Frames)
/// - Client → Server: Raw terminal input bytes (e.g., keystrokes, commands)
/// - Server → Client: Raw terminal output bytes (screen content, command results)
///
/// ## Architecture
///
/// ```text
/// ┌──────────┐     WebSocket      ┌─────────────────────┐     crabjar-terminal     ┌──────────┐
/// │          │    ──────────────►  │                     │    ──────────────────►   │          │
/// │ Client 1 │ ◄─────────────────  │  Terminal Relay     │  ◄────────────────────  │ Wezterm/ │
/// │          │    WebSocket        │  (this module)      │                         │ Zellij   │
/// └──────────┘                     │                     │                         │ Backend  │
///                                  │                     │                         │          │
/// ┌──────────┐     WebSocket       │                     │                         └──────────┘
/// │          │    ──────────────►  │                     │
/// │ Client 2 │ ◄─────────────────  │                     │
/// │          │    WebSocket        │                     │
/// └──────────┘                     └─────────────────────┘
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
use tracing::{debug, error, info, warn};

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
    /// Error notification from server to client.
    Error { message: String },
}

/// Terminal I/O message sent over WebSocket binary frames.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum IoMessage {
    /// Terminal output bytes (server → client).
    Output { data: Vec<u8> },
    /// Terminal input bytes (client → server).
    Input { data: Vec<u8> },
}

/// State shared across all WebSocket connections for a relay instance.
#[derive(Clone)]
pub struct TerminalRelayState {
    /// Map of session_name → broadcast channel sender for terminal output fan-out.
    sessions: Arc<Mutex<HashMap<String, SessionData>>>,
}

/// Data associated with a single terminal session in the relay.
struct SessionData {
    /// Broadcast sender for fanning out terminal output to all connected clients.
    broadcast_tx: broadcast::Sender<Vec<u8>>,
    
    /// Number of currently connected clients.
    client_count: usize,
}

impl TerminalRelayState {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Get or create a broadcast channel for the given session.
    async fn get_or_create_session(&self, session_name: &str) -> Result<broadcast::Receiver<Vec<u8>>> {
        let mut sessions = self.sessions.lock().await;
        
        if let Some(session_data) = sessions.get_mut(session_name) {
            // Session already exists, increment client count
            session_data.client_count += 1;
            return Ok(session_data.broadcast_tx.subscribe());
        }
        
        // Create new session with broadcast channel
        let (broadcast_tx, _) = broadcast::channel(1024);
        
        sessions.insert(session_name.to_string(), SessionData {
            broadcast_tx: broadcast_tx.clone(),
            client_count: 1,
        });
        
        Ok(broadcast_tx.subscribe())
    }
    
    /// Remove a session when all clients have disconnected.
    async fn remove_session(&self, session_name: &str) {
        let mut sessions = self.sessions.lock().await;
        if let Some(session_data) = sessions.get_mut(session_name) {
            session_data.client_count -= 1;
            if session_data.client_count == 0 {
                sessions.remove(session_name);
            }
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
    info!(%addr, "terminal relay server listening");

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
    info!("terminal relay: new client connected");

    let (mut ws_sink, mut ws_stream) = socket.split();

    // Send initial pong to confirm connection
    if let Err(e) = ws_sink.send(Message::Text(serde_json::to_string(&ControlMessage::Pong { ts: now_ms() }).unwrap())).await {
        warn!(error = %e, "failed to send initial pong");
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
                            info!(session = %session, ?pane_id, "client joining terminal session");

                            // Leave current session if any.
                            _broadcast_rx = None;

                            // Join new session: get or create a broadcast channel for this session.
                            match state.get_or_create_session(&session).await {
                                Ok(rx) => {
                                    _broadcast_rx = Some(rx);
                                    current_session = Some(session);
                                }
                                Err(e) => {
                                    warn!(error = %e, "failed to join session");
                                    let _ = ws_sink.send(Message::Text(serde_json::to_string(&ControlMessage::Error { 
                                        message: format!("Failed to join session: {}", e) 
                                    }).unwrap())).await;
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
                } else {
                    warn!("invalid control message: {}", text);
                }
            }
            Ok(Message::Binary(data)) => {
                // Forward binary data to the PTY master.
                // In a full implementation, this would write to the wezterm/zellij pane.
                debug!(bytes = data.len(), "received terminal input");
                
                // TODO: Wire this up to the actual terminal session's send_text method
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
