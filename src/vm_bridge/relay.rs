/// WebSocket relay integration for vm-bridge
///
/// Provides integration with vm-bridge's WebSocket relay for:
/// - Screen sharing (PipeWire, XDG-Portal)
/// - Shared terminal (wezterm, zellij)
/// - Display protocol routing

use anyhow::Result;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::WebSocket};
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::SinkExt;
use futures_util::stream::SplitSink;
use futures_util::stream::SplitStream;

/// WebSocket relay client for vm-bridge
pub struct RelayClient {
    ws: SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, Message>,
    rx: futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<TcpStream>>,
}

impl RelayClient {
    /// Create a new WebSocket relay client
    pub async fn new(uri: &str) -> Result<Self> {
        let (ws, _) = connect_async(uri).await?;
        let (sink, stream) = ws.split();
        
        Ok(Self {
            ws: sink,
            rx: stream,
        })
    }
    
    /// Send a message to the relay
    pub async fn send(&mut self, msg: Message) -> Result<()> {
        self.ws.send(msg).await?;
        Ok(())
    }
    
    /// Receive a message from the relay
    pub async fn recv(&mut self) -> Option<Result<Message>> {
        self.rx.next().await.map(|r| r.map_err(Int::from))
    }
}
