// CrabJar SSH Bastion Host
// Transparent SSH proxy to home-lab fleet using russh 0.63
// Architecture: per-connection state with real-time bidirectional forwarding
// Routes by port: each target gets its own listener port
// Supports multiple users (hermes, crombo) with pass-through auth

use russh::server::Server as _;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{info, warn};

/// Load persistent host key for the bastion (avoids host-key mismatch on restart)
fn load_bastion_key() -> russh::keys::PrivateKey {
    let key_path = PathBuf::from("/home/crombo/.ssh/id_bastion");
    match russh::keys::load_secret_key(&key_path, None) {
        Ok(key) => {
            info!("Loaded bastion host key from {}", key_path.display());
            key
        }
        Err(e) => {
            warn!(
                "Failed to load bastion key from {}: {} (using random fallback)",
                key_path.display(),
                e
            );
            let mut rng = rand::rng();
            russh::keys::PrivateKey::random(&mut rng, russh::keys::Algorithm::Ed25519).unwrap()
        }
    }
}

/// Target machine configuration
#[derive(Clone)]
struct TargetConfig {
    host: String,
    port: u16,
    user: String,
}

/// Per-client state: tracks outbound SSH connection to target machine
struct ClientState {
    channel_id: russh::ChannelId,
    server_handle: russh::server::Handle,
    user: String,
}

#[derive(Clone)]
struct Bastion {
    clients: Arc<Mutex<HashMap<usize, ClientState>>>,
    key_path: PathBuf,
    target: TargetConfig,
    id: usize,
    auth_user: String,
}

impl Bastion {
    fn new(key_path: PathBuf, target: TargetConfig) -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            key_path,
            target,
            id: 0,
            auth_user: String::new(),
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
        if self.auth_user.is_empty() {
            self.auth_user = user.to_string();
        }
        info!("Public key auth for: {} -> {}", user, self.target.host);
        Ok(russh::server::Auth::Accept)
    }

    async fn auth_password(
        &mut self,
        user: &str,
        _password: &str,
    ) -> Result<russh::server::Auth, Self::Error> {
        if self.auth_user.is_empty() {
            self.auth_user = user.to_string();
        }
        info!("Password auth for: {} -> {}", user, self.target.host);
        Ok(russh::server::Auth::Accept)
    }

    async fn auth_none(&mut self, user: &str) -> Result<russh::server::Auth, Self::Error> {
        if self.auth_user.is_empty() {
            self.auth_user = user.to_string();
        }
        info!("None auth for: {} (accepting to allow pass-through)", user);
        Ok(russh::server::Auth::Accept)
    }

    async fn channel_open_session(
        &mut self,
        channel: russh::Channel<russh::server::Msg>,
        reply: russh::server::ChannelOpenHandle,
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        let client_id = self.id;

        info!(
            "Channel open session: {} -> {}",
            channel.id(),
            self.target.host
        );

        // Store client state — wait for shell/exec request before connecting to target
        {
            let mut clients_lock = self.clients.lock().await;
            clients_lock.insert(
                client_id,
                ClientState {
                    channel_id: channel.id(),
                    server_handle: session.handle(),
                    user: String::new(), // Will be set on shell/exec
                },
            );
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
        let host = self.target.host.clone();
        let port = self.target.port;

        // Use the user from auth (passed through) or fallback to config default
        let user = if !self.auth_user.is_empty() {
            self.auth_user.clone()
        } else {
            self.target.user.clone()
        };

        info!(
            "Client requested interactive shell on channel {} -> {} as {}",
            channel, host, user
        );

        match connect_to_target(&host, port, &user, &self.key_path).await {
            Ok((mut target_session, mut outbound_channel)) => {
                info!("Outbound connection established to {}", host);

                // Request PTY for interactive shell
                if let Err(e) = outbound_channel
                    .request_pty(false, "xterm-256color", 80, 24, 0, 0, &[])
                    .await
                {
                    warn!("Failed to request PTY: {}", e);
                }

                // Spawn interactive shell
                if let Err(e) = outbound_channel.exec(true, "/bin/bash").await {
                    warn!("Failed to spawn shell on target: {}", e);
                } else {
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
        let host = self.target.host.clone();
        let port = self.target.port;

        // Get the user from client state (passed through from auth)
        let user = {
            let clients_lock = self.clients.lock().await;
            match clients_lock.get(&self.id) {
                Some(state) if !state.user.is_empty() => state.user.clone(),
                _ => self.target.user.clone(), // Fallback to config default
            }
        };

        let cmd_str = String::from_utf8_lossy(cmd).to_string();

        info!(
            "Client requested exec on channel {} -> {} as {}: {}",
            channel, host, user, cmd_str
        );

        match connect_to_target(&host, port, &user, &self.key_path).await {
            Ok((target_session, mut outbound_channel)) => {
                info!("Outbound connection established to {} for exec", host);

                if let Err(e) = outbound_channel.exec(true, cmd_str.as_bytes()).await {
                    warn!("Failed to exec on target: {}", e);
                } else {
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
) -> anyhow::Result<(
    russh::client::Handle<OutboundHandler>,
    russh::Channel<russh::client::Msg>,
)> {
    info!(
        "Connecting to {}@{}:{} with key {}",
        user,
        host,
        port,
        key_path.display()
    );

    let key = russh::keys::load_secret_key(key_path, None)?;

    let config = russh::client::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
        ..Default::default()
    };
    let config = Arc::new(config);

    let mut session =
        russh::client::connect(config, (host.to_string(), port), OutboundHandler {}).await?;
    info!("Connected to {}", host);

    let auth_result = session
        .authenticate_publickey(
            user.to_string(),
            russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), None),
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

/// Spawn a bastion listener for a specific target on a given port
async fn spawn_bastion(
    listen_port: u16,
    key_path: PathBuf,
    target: TargetConfig,
) -> anyhow::Result<()> {
    let config = russh::server::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
        auth_rejection_time: std::time::Duration::from_secs(3),
        keys: vec![load_bastion_key()],
        ..Default::default()
    };
    let config = Arc::new(config);

    let bastion_target = target.clone();
    info!(
        "Bastion for {} listening on 0.0.0.0:{}",
        target.host, listen_port
    );
    let mut bastion = Bastion::new(key_path, target);
    let socket = TcpListener::bind(("0.0.0.0", listen_port)).await?;

    tokio::spawn(async move {
        if let Err(e) = bastion.run_on_socket(config, &socket).await {
            warn!("Server error for {}: {}", bastion_target.host, e);
        }
    });

    Ok(())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    info!("CrabJar SSH Bastion starting");

    let key_path = PathBuf::from("/home/crombo/.ssh/id_lab");

    // Define all targets with their bastion ports
    // User is crombo for now; later hermes/crabjar users will have their own routing
    let targets: Vec<(u16, TargetConfig)> = vec![
        (
            2201,
            TargetConfig {
                host: "192.168.50.181".to_string(),
                port: 22,
                user: "crombo".to_string(),
            },
        ), // jambo
        (
            2202,
            TargetConfig {
                host: "192.168.50.163".to_string(),
                port: 22,
                user: "crombo".to_string(),
            },
        ), // ftw3
        (
            2203,
            TargetConfig {
                host: "192.168.50.229".to_string(),
                port: 22,
                user: "crombo".to_string(),
            },
        ), // artix-box
    ];

    for (port, target) in targets {
        match spawn_bastion(port, key_path.clone(), target).await {
            Ok(_) => {}
            Err(e) => warn!("Failed to start bastion on port {}: {}", port, e),
        }
    }

    info!("All bastion listeners started");

    // Keep running forever
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    }
}
