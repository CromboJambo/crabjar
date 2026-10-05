// CrabJar SSH Bastion - Connection Handler
// Wraps russh's callback-style Handler trait in event-driven architecture
// following warpgate's pattern: send events through channels to session loop

use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{info, warn};

use super::targets::TargetRouter;

/// Events from the SSH server connection, sent to the session event loop
pub enum ServerHandlerEvent {
    Authenticated(String), // username
    ChannelOpenSession(russh::ChannelId),
    SubsystemRequest(russh::ChannelId, String),
    PtyRequest(russh::ChannelId, russh::Pty),
    ShellRequest(russh::ChannelId),
    ExecRequest(russh::ChannelId, Vec<u8>),
    Data(russh::ChannelId, Vec<u8>),
    ChannelClose(russh::ChannelId),
    Error(String),
}

/// Handler that wraps russh's callback-style API into event-driven pattern
pub struct BastionHandler {
    sender: mpsc::Sender<ServerHandlerEvent>,
    authenticated_user: Option<String>,
}

impl BastionHandler {
    pub fn new(sender: mpsc::Sender<ServerHandlerEvent>) -> Self {
        Self {
            sender,
            authenticated_user: None,
        }
    }
}

#[async_trait::async_trait]
impl russh::server::Handler for BastionHandler {
    type Error = anyhow::Error;

    async fn auth_password(
        &mut self,
        user: &str,
        _password: &str,
    ) -> Result<russh::Auth, Self::Error> {
        info!("Password auth attempt for user: {}", user);
        // For now, accept any password (SSH key auth at target handles security)
        self.authenticated_user = Some(user.to_string());
        let _ = self
            .sender
            .send(ServerHandlerEvent::Authenticated(user.to_string()))
            .await;
        Ok(russh::Auth::Accept)
    }

    async fn channel_open_session(
        &mut self,
        channel: russh::ChannelId,
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        info!("Channel open session: {}", channel);
        let _ = self
            .sender
            .send(ServerHandlerEvent::ChannelOpenSession(channel))
            .await;
        Ok(())
    }

    async fn subsystem_request(
        &mut self,
        channel: russh::ChannelId,
        name: &str,
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        info!("Subsystem request on {}: {}", channel, name);
        let _ = self
            .sender
            .send(ServerHandlerEvent::SubsystemRequest(
                channel,
                name.to_string(),
            ))
            .await;
        Ok(())
    }

    async fn pty_request(
        &mut self,
        channel: russh::ChannelId,
        term: &str,
        cols: u32,
        rows: u32,
        width: u32,
        height: u32,
        modes: Vec<u8>,
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        info!("PTY request on {}: {} ({}x{})", channel, term, cols, rows);
        let pty = russh::Pty {
            term: term.to_string(),
            cols,
            rows,
            width,
            height,
            modes,
        };
        let _ = self
            .sender
            .send(ServerHandlerEvent::PtyRequest(channel, pty))
            .await;
        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel: russh::ChannelId,
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        info!("Shell request on {}", channel);
        let _ = self
            .sender
            .send(ServerHandlerEvent::ShellRequest(channel))
            .await;
        Ok(())
    }

    async fn exec_request(
        &mut self,
        channel: russh::ChannelId,
        command: Vec<u8>,
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        info!("Exec request on {}: {:?}", channel, String::from_utf8_lossy(&command));
        let _ = self
            .sender
            .send(ServerHandlerEvent::ExecRequest(channel, command))
            .await;
        Ok(())
    }

    async fn data(
        &mut self,
        channel: russh::ChannelId,
        data: Vec<u8>,
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        let _ = self
            .sender
            .send(ServerHandlerEvent::Data(channel, data))
            .await;
        Ok(())
    }

    async fn channel_close(
        &mut self,
        channel: russh::ChannelId,
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        info!("Channel close: {}", channel);
        let _ = self
            .sender
            .send(ServerHandlerEvent::ChannelClose(channel))
            .await;
        Ok(())
    }

    async fn Error(&mut self, error: anyhow::Error) -> Result<(), Self::Error> {
        warn!("Handler error: {}", error);
        let _ = self
            .sender
            .send(ServerHandlerEvent::Error(error.to_string()))
            .await;
        Ok(())
    }
}

/// Handle an incoming SSH connection with event-driven architecture
pub async fn handle_connection(
    stream: TcpStream,
    router: Arc<TargetRouter>,
) -> anyhow::Result<()> {
    // Create channels for event communication
    let (tx, mut rx) = mpsc::channel::<ServerHandlerEvent>(100);

    // Spawn handler task that processes russh callbacks
    let handler = BastionHandler::new(tx);

    // TODO: Actually spawn the russh server session and wire up events
    // For now, just demonstrate the architecture pattern

    info!("Connection handler started (event-driven pattern)");

    Ok(())
}
