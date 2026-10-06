// CrabJar SSH Bastion Host
// Transparent SSH proxy to home-lab fleet using russh 0.63
// Architecture: per-connection state with real-time bidirectional forwarding
// Handles both interactive shells and command execution (exec)

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{info, warn};
use russh::server::Server as _;

/// Per-client state: tracks outbound SSH connection to target machine
struct ClientState {
    channel_id: russh::ChannelId,
    server_handle: russh::server::Handle,
}

#[derive(Clone)]
struct Bastion {
    clients: Arc<Mutex<HashMap<usize, ClientState>>>,
    key_path: PathBuf,
    id: usize,
}

impl Bastion {
    fn new(key_path: PathBuf) -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            key_path,
            id: 0,
        }
    }
}

impl russh::server::Server for Bastion {
    type Handler = Self;

    fn new_client(&mut self, _addr: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }

    fn handle_session_error(&mut self, error: anyhow::Error) {
        warn!("Session error: {}", error);
    }
}

impl russh::server::Handler for Bastion {
    type Error = anyhow::Error;

    async fn auth_publickey(
        &mut self,
        user: &str,
        _public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<russh::server::Auth, Self::Error> {
        info!("Public key auth for: {}", user);
        Ok(russh::server::Auth::Accept)
    }

    async fn auth_password(
        &mut self,
        user: &str,
        _password: &str,
    ) -> Result<russh::server::Auth, Self::Error> {
        info!("Password auth for: {}", user);
        Ok(russh::server::Auth::Accept)
    }

    async fn channel_open_session(
        &mut self,
        channel: russh::Channel<russh::server::Msg>,
        reply: russh::server::ChannelOpenHandle,
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        let client_id = self.id;

        info!("Channel open session: {}", channel.id());

        // Store client state — wait for shell/exec request before connecting to target
        {
            let mut clients_lock = self.clients.lock().await;
            clients_lock.insert(client_id, ClientState {
                channel_id: channel.id(),
                server_handle: session.handle(),
            });
        }

        reply.accept().await;
        Ok(())
    }

    /// Client wants an interactive shell
    async fn shell_request(
        &mut self,
        channel: russh::ChannelId,
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        let client_id = self.id;
        info!("Client requested interactive shell on channel {}", channel);

        // Establish outbound connection and spawn shell — do this inline
        let host = "192.168.50.181".to_string(); // jambo
        let user = "hermes".to_string();

        match connect_to_target(&host, 22, &user, &self.key_path).await {
            Ok((mut target_session, mut outbound_channel)) => {
                info!("Outbound connection established to {}", host);

                // Request PTY for interactive shell
                if let Err(e) = outbound_channel.request_pty(false, "xterm-256color", 80, 24, 0, 0, &[]).await {
                    warn!("Failed to request PTY: {}", e);
                }

                // Spawn interactive shell
                if let Err(e) = outbound_channel.exec(true, "/bin/bash").await {
                    warn!("Failed to spawn shell on target: {}", e);
                } else {
                    // Wait for data from target and forward to client
                    loop {
                        match outbound_channel.wait().await {
                            Some(russh::ChannelMsg::Data { ref data }) => {
                                if let Err(e) = session.data(channel, (*data).to_vec()) {
                                    warn!("Failed to send data to client: {:?}", e);
                                    break;
                                }
                            }
                            Some(russh::ChannelMsg::ExitStatus { exit_status }) => {
                                info!("Target exited with status {}", exit_status);
                                if let Err(e) = session.eof(channel) {
                                    warn!("Failed to send EOF: {:?}", e);
                                }
                                // Close the channel after sending exit status
                                if let Err(e) = session.close(channel) {
                                    warn!("Failed to close channel: {:?}", e);
                                }
                                break;
                            }
                            Some(russh::ChannelMsg::Eof) => {
                                info!("Target connection EOF");
                                if let Err(e) = session.eof(channel) {
                                    warn!("Failed to send EOF: {:?}", e);
                                }
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to connect to target {}: {}", host, e);
            }
        }

        Ok(())
    }

    /// Client wants to execute a command
    async fn exec_request(
        &mut self,
        channel: russh::ChannelId,
        cmd: &[u8],
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        let client_id = self.id;
        let cmd_str = String::from_utf8_lossy(cmd).to_string();
        info!("Client requested exec on channel {}: {}", channel, cmd_str);

        // Establish outbound connection and exec command — do this inline
        let host = "192.168.50.181".to_string(); // jambo
        let user = "hermes".to_string();

        match connect_to_target(&host, 22, &user, &self.key_path).await {
            Ok((target_session, mut outbound_channel)) => {
                info!("Outbound connection established to {} for exec", host);

                // Execute the command (no PTY needed)
                if let Err(e) = outbound_channel.exec(true, cmd_str.as_bytes()).await {
                    warn!("Failed to exec on target: {}", e);
                } else {
                    // Wait for data from target and forward to client
                    loop {
                        match outbound_channel.wait().await {
                            Some(russh::ChannelMsg::Data { ref data }) => {
                                info!("Received {} bytes from target", data.len());
                                if let Err(e) = session.data(channel, (*data).to_vec()) {
                                    warn!("Failed to send data to client: {:?}", e);
                                    break;
                                }
                            }
                            Some(russh::ChannelMsg::ExitStatus { exit_status }) => {
                                info!("Target exited with status {}", exit_status);
                                if let Err(e) = session.eof(channel) {
                                    warn!("Failed to send EOF: {:?}", e);
                                }
                                // Close the channel after sending exit status
                                if let Err(e) = session.close(channel) {
                                    warn!("Failed to close channel: {:?}", e);
                                }
                                break;
                            }
                            Some(russh::ChannelMsg::Eof) => {
                                info!("Target connection EOF");
                                if let Err(e) = session.eof(channel) {
                                    warn!("Failed to send EOF: {:?}", e);
                                }
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to connect to target {}: {}", host, e);
            }
        }

        Ok(())
    }

    async fn data(
        &mut self,
        channel: russh::ChannelId,
        data: &[u8],
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        info!("Client sent {} bytes on channel {}", data.len(), channel);
        // Data forwarding handled by outbound connection loop
        Ok(())
    }

    async fn channel_close(
        &mut self,
        channel: russh::ChannelId,
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        info!("Client channel closed: {}", channel);
        Ok(())
    }
}

/// Connect to target machine and return session + channel
async fn connect_to_target(
    host: &str,
    port: u16,
    user: &str,
    key_path: &PathBuf,
) -> anyhow::Result<(russh::client::Handle<OutboundHandler>, russh::Channel<russh::client::Msg>)> {
    info!("Connecting to {}@{}:{} with key {}", user, host, port, key_path.display());

    let key = russh::keys::load_secret_key(key_path, None)?;

    let config = russh::client::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
        ..Default::default()
    };
    let config = Arc::new(config);

    let mut session = russh::client::connect(config, (host.to_string(), port), OutboundHandler {}).await?;
    info!("Connected to {}", host);

    let auth_result = session
        .authenticate_publickey(
            user.to_string(),
            russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), None)
        )
        .await?;

    if !auth_result.success() {
        anyhow::bail!("Authentication failed for {}@{}", user, host);
    }

    info!("Authenticated as {} on {}", user, host);

    let channel = session.channel_open_session().await?;

    Ok((session, channel))
}

/// Outbound SSH client handler
struct OutboundHandler;

impl russh::client::Handler for OutboundHandler {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    info!("CrabJar SSH Bastion starting on 0.0.0.0:2222");

    // Generate ephemeral host key for the bastion
    let config = russh::server::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
        auth_rejection_time: std::time::Duration::from_secs(3),
        keys: vec![
            russh::keys::PrivateKey::random(&mut rand::rng(), russh::keys::Algorithm::Ed25519).unwrap(),
        ],
        ..Default::default()
    };
    let config = Arc::new(config);

    let key_path = PathBuf::from("/home/crombo/.ssh/id_lab");
    let mut bastion = Bastion::new(key_path);

    let socket = TcpListener::bind(("0.0.0.0", 2222)).await.unwrap();
    info!("Listening on 0.0.0.0:2222");

    let server = bastion.run_on_socket(config, &socket);
    if let Err(e) = server.await {
        warn!("Server error: {}", e);
    }
}
