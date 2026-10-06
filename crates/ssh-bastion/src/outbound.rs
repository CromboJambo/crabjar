//! Outbound SSH client connection to target machines.
//! Uses russh's client API to establish connections and forward data.

use std::path::Path;
use std::sync::Arc;
use anyhow::Result;
use russh::keys::*;
use russh::*;
use tracing::{info, warn};

/// Simple outbound SSH client handler
pub struct OutboundHandler {
    /// Callback channel for forwarding data back to the bastion session
    pub tx: tokio::sync::mpsc::Sender<OutboundMsg>,
}

/// Messages from the outbound connection back to the bastion
pub enum OutboundMsg {
    Data(Vec<u8>),
    ExitStatus(u32),
    Closed,
}

#[async_trait::async_trait]
impl client::Handler for OutboundHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        // Accept any server key (known_hosts validation is future work)
        Ok(true)
    }
}

/// Wrapper for outbound SSH session to target machine
pub struct OutboundSession {
    handle: client::Handle<OutboundHandler>,
    channel: Channel<client::Msg>,
}

impl OutboundSession {
    /// Connect to target and open a shell channel
    pub async fn connect(
        host: &str,
        port: u16,
        user: &str,
        key_path: &Path,
    ) -> Result<(Self, tokio::sync::mpsc::Receiver<OutboundMsg>)> {
        info!("Connecting to {}@{}:{} with key {}", user, host, port, key_path.display());

        // Load SSH private key
        let key_pair = load_secret_key(key_path, None)?;

        // Create client config
        let config = client::Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
            ..<_>::default()
        };
        let config = Arc::new(config);

        // Create event channel for data forwarding
        let (tx, rx) = tokio::sync::mpsc::channel::<OutboundMsg>(100);
        let handler = OutboundHandler { tx };

        // Connect to target
        let mut session = client::connect(config, (host.to_string(), port), handler).await?;
        info!("Connected to {}", host);

        // Authenticate with SSH key
        let auth_res = session
            .authenticate_publickey(
                user.to_string(),
                PrivateKeyWithHashAlg::new(
                    Arc::new(key_pair),
                    session.best_supported_rsa_hash().await?.flatten(),
                ),
            )
            .await?;

        if !auth_res.success() {
            anyhow::bail!("Authentication failed for {}@{}", user, host);
        }

        info!("Authenticated as {} on {}", user, host);

        // Open channel and request shell
        let channel = session.channel_open_session().await?;
        
        // Request PTY for interactive shell
        if let Err(e) = channel.request_pty(
            false,
            "xterm-256color",
            80,   // cols
            24,   // rows
            0,    // width pixels
            0,    // height pixels
            &[],  // modes
        ).await {
            warn!("Failed to request PTY: {}", e);
        }

        // Request shell on target
        channel.shell().await?;

        let handle = session.handle();

        Ok((Self { handle, channel }, rx))
    }

    /// Send data from client to target
    pub async fn send_data(&self, data: &[u8]) -> Result<()> {
        self.handle.data(self.channel.id(), data).await?;
        Ok(())
    }

    /// Close the connection
    pub async fn close(&self) -> Result<()> {
        self.handle.close(self.channel.id()).await?;
        Ok(())
    }
}
