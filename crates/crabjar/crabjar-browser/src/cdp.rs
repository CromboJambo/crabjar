use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tungstenite::Message as WsMessage;

type WsStream = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
type WsSink = futures_util::stream::SplitSink<WsStream, WsMessage>;
type WsReader = futures_util::stream::SplitStream<WsStream>;

type PendingMap = Arc<tokio::sync::Mutex<HashMap<u64, oneshot::Sender<anyhow::Result<Value>>>>>;

/// CDP (Chrome DevTools Protocol) event received from the browser.
#[derive(Debug, Clone)]
pub struct CdpEvent {
    pub method: String,
    pub params: Value,
}

/// Thin CDP client over WebSocket. Handles JSON-RPC request/response matching
/// and dispatches server-pushed events to a channel.
///
/// Uses stream splitting for concurrent read/write: the writer half (`sink`)
/// is used by `send()`, the reader half is consumed by a background task that
/// matches responses to pending requests and forwards events.
pub struct CdpClient {
    sink: tokio::sync::Mutex<WsSink>,
    pending: PendingMap,
    next_id: AtomicU64,
    event_tx: mpsc::UnboundedSender<CdpEvent>,
    session_id: Option<String>,
}

impl CdpClient {
    /// Connect to a browser-level WebSocket endpoint and create a page target.
    /// Returns the client, an event receiver channel, and the reader task handle.
    pub async fn connect_browser(
        ws_url: &str,
        initial_url: &str,
    ) -> Result<(Self, mpsc::UnboundedReceiver<CdpEvent>, JoinHandle<()>)> {
        tracing::debug!(ws_url, "connecting CDP client (browser-level)");
        let (stream, _) = tokio_tungstenite::connect_async(ws_url).await?;

        // Split the stream once into write and read halves
        let (mut sink, mut reader) = stream.split();

        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let pending: PendingMap = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

        // Send Target.createTarget and wait for the attachedToTarget event
        let message = json!({
            "id": 0,
            "method": "Target.createTarget",
            "params": { "url": initial_url }
        });
        sink.send(WsMessage::Text(message.to_string().into())).await?;

        // Read from the stream until we get attachedToTarget
        let session_id = {
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() {
                    return Err(anyhow::anyhow!("Timed out waiting for target attachment"));
                }

                tokio::select! {
                    msg = reader.next() => {
                        match msg {
                            Some(Ok(WsMessage::Text(text))) => {
                                if let Ok(value) = serde_json::from_str::<Value>(&text) {
                                    if let Some(method) = value.get("method").and_then(|m| m.as_str()) {
                                        if method == "Target.attachedToTarget" {
                                            if let Some(sid) = value
                                                .get("params")
                                                .and_then(|p| p.get("sessionId"))
                                                .and_then(|s| s.as_str())
                                            {
                                                break Ok::<String, anyhow::Error>(sid.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                            Some(Ok(WsMessage::Close(_))) => {
                                return Err(anyhow::anyhow!("CDP connection closed during init"));
                            }
                            Some(Err(e)) => return Err(anyhow::anyhow!("CDP read error: {}", e)),
                            None => return Err(anyhow::anyhow!("CDP stream ended during init")),
                            _ => {}
                        }
                    }
                    _ = tokio::time::sleep(remaining) => {
                        return Err(anyhow::anyhow!("Timed out waiting for target attachment"));
                    }
                }
            }?
        };

        tracing::debug!(session_id, "created page target");

        // Spawn the background reader task with the read half and shared pending map
        let reader_handle = Self::spawn_reader_task(
            reader,
            Arc::clone(&pending),
            event_tx.clone(),
        );

        let client = Self {
            sink: tokio::sync::Mutex::new(sink),
            pending,
            next_id: AtomicU64::new(1),
            event_tx,
            session_id: Some(session_id),
        };

        Ok((client, event_rx, reader_handle))
    }

    /// Send a CDP command and wait for the response.
    pub async fn send(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending.lock().await;
            pending.insert(id, tx);
        }

        let mut message = json!({ "id": id, "method": method, "params": params });
        if let Some(session_id) = &self.session_id {
            message["sessionId"] = json!(session_id);
        }

        {
            let mut sink = self.sink.lock().await;
            sink.send(WsMessage::Text(message.to_string().into())).await?;
        }

        // Wait for response with timeout
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            rx,
        )
        .await
        .map_err(|_| anyhow::anyhow!("CDP command '{}' timed out", method))?;

        match result {
            Ok(inner) => inner,
            Err(_) => Err(anyhow::anyhow!("CDP response channel closed for '{}'", method)),
        }
    }

    /// Spawn a background reader task that consumes the read half of the stream.
    fn spawn_reader_task(
        mut reader: WsReader,
        pending: PendingMap,
        event_tx: mpsc::UnboundedSender<CdpEvent>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = reader.next().await {
                match msg {
                    Ok(WsMessage::Text(text)) => {
                        if let Ok(value) = serde_json::from_str::<Value>(&text) {
                            // Response to a pending request
                            if let Some(id) = value.get("id").and_then(|v| v.as_u64()) {
                                if let Some(tx) = pending.lock().await.remove(&id) {
                                    if let Some(error) = value.get("error") {
                                        let _ = tx.send(Err(anyhow::anyhow!("CDP error: {}", error)));
                                    } else {
                                        let result = value.get("result").cloned().unwrap_or(Value::Null);
                                        let _ = tx.send(Ok(result));
                                    }
                                }
                            }
                            // Server-pushed event
                            if let Some(method) = value.get("method").and_then(|m| m.as_str()) {
                                let params = value.get("params").cloned().unwrap_or(Value::Null);
                                if !method.starts_with("Target.") && !method.starts_with("Log.") {
                                    let _ = event_tx.send(CdpEvent { method: method.to_string(), params });
                                }
                            }
                        }
                    }
                    Ok(WsMessage::Close(_)) => break,
                    Err(_) => break,
                    _ => {}
                }
            }
        })
    }

    /// Close the CDP connection.
    pub async fn close(&self) -> Result<()> {
        let mut sink = self.sink.lock().await;
        sink.close().await?;
        Ok(())
    }
}
