// CrabJar SSH Bastion Host
// Transparent SSH proxy to home-lab fleet using russh
// Architecture: per-connection state with real-time bidirectional forwarding

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

mod config;
mod targets;

/// Per-client state: tracks outbound SSH connection to target machine
struct ClientState {
    channel_id: russh::ChannelId,
    /// Handle for sending data to the client
    client_handle: russh::server::Handle,
}

#[derive(Clone)]
struct Bastion {
    clients: Arc<Mutex<HashMap<usize, ClientState>>>,
    router: Arc<targets::TargetRouter>,
    key_path: PathBuf,
    id: usize,
}

impl Bastion {
    fn new(router: Arc<targets::TargetRouter>, key_path: PathBuf) -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            router,
            key_path,
            id: 0,
        }
    }
}

#[async_trait::async_trait]
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

#[async_trait::async_trait]
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
        info!("Channel open session: {}", channel.id());

        // Get authenticated user from client context (simplified for now)
        let user = "hermes".to_string();

        // Determine target based on user
        let targets_list = self.router.get_targets_for_user(&user);
        if targets_list.is_empty() {
            warn!("No target machines for user {}", user);
            reply.reject(russh::ChannelOpenFailure::AdminProhibited).await;
            return Ok(());
        }

        let target = &targets_list[0];
        info!("Routing {} to {} ({})", user, target.name, target.ip);

        // Store client state for data forwarding
        {
            let mut clients = self.clients.lock().await;
            clients.insert(self.id, ClientState {
                channel_id: channel.id(),
                client_handle: session.handle(),
            });
        }

        reply.accept().await;

        // Establish outbound SSH connection to target in background
        let key_path = self.key_path.clone();
        let host = target.ip.clone();
        let ssh_user = target.ssh_user.clone();
        let clients = self.clients.clone();
        let client_id = self.id;

        tokio::spawn(async move {
            match establish_outbound(&host, 22, &ssh_user, &key_path).await {
                Ok(outbound) => {
                    info!("Outbound connection established to {}", host);

                    // Forward data from target back to client
                    let mut channel = outbound.channel;
                    loop {
                        match channel.wait().await {
                            Some(russh::ChannelMsg::Data { ref data }) => {
                                if let Some(client) = clients.lock().await.get(&client_id) {
                                    if let Err(e) = client.client_handle.data(client.channel_id, data).await {
                                        warn!("Failed to send data to client: {}", e);
                                        break;
                                    }
                                }
                            }
                            Some(russh::ChannelMsg::ExitStatus { exit_status }) => {
                                info!("Target exited with status {}", exit_status);
                                if let Some(client) = clients.lock().await.get(&client_id) {
                                    if let Err(e) = client.client_handle.exit_status(client.channel_id, exit_status).await {
                                        warn!("Failed to send exit status: {}", e);
                                    }
                                }
                                break;
                            }
                            Some(russh::ChannelMsg::Eof) => {
                                info!("Target connection EOF");
                                if let Some(client) = clients.lock().await.get(&client_id) {
                                    if let Err(e) = client.client_handle.eof(client.channel_id).await {
                                        warn!("Failed to send EOF: {}", e);
                                    }
                                }
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to connect to target {}: {}", host, e);
                }
            }

            // Clean up client state
            clients.lock().await.remove(&client_id);
        });

        Ok(())
    }

    async fn data(
        &mut self,
        channel: russh::ChannelId,
        data: &[u8],
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        // Forward client data to target via outbound connection
        let clients = self.clients.clone();
        let client_id = self.id;

        tokio::spawn(async move {
            if let Some(client) = clients.lock().await.get(&client_id) {
                // TODO: Send data to target's outbound channel
                info!("Client sent {} bytes", data.len());
            }
        });

        Ok(())
    }

    async fn channel_close(
        &mut self,
        channel: russh::ChannelId,
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        info!("Client channel closed: {}", channel);

        // Clean up client state
        self.clients.lock().await.remove(&self.id);

        Ok(())
    }
}

/// Outbound SSH session to target machine
struct OutboundSession {
    handle: russh::client::Handle<OutboundHandler>,
    channel: russh::Channel<russh::client::Msg>,
}

/// Establish outbound SSH connection to target machine
async fn establish_outbound(
    host: &str,
    port: u16,
    user: &str,
    key_path: &PathBuf,
) -> anyhow::Result<OutboundSession> {
    info!("Connecting to {}@{}:{} with key {}", user, host, port, key_path.display());

    // Load SSH private key
    let key = russh::keys::PrivateKey::read_file(key_path, None)?;

    // Create client config
    let config = russh::client::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
        ..Default::default()
    };
    let config = Arc::new(config);

    // Connect to target
    let mut session = russh::client::connect(config, (host.to_string(), port), OutboundHandler {}).await?;
    info!("Connected to {}", host);

    // Authenticate with SSH key
    let auth_result = session
        .authenticate_publickey(user.to_string(), Arc::new(key))
        .await?;

    if !auth_result.success() {
        anyhow::bail!("Authentication failed for {}@{}", user, host);
    }

    info!("Authenticated as {} on {}", user, host);

    // Open channel and request shell
    let channel = session.channel_open_session().await?;

    // Request PTY for interactive shell
    if let Err(e) = channel.request_pty(false, "xterm-256color", 80, 24, 0, 0, &[]).await {
        warn!("Failed to request PTY: {}", e);
    }

    // Request shell on target
    if let Err(e) = channel.shell().await {
        warn!("Failed to request shell: {}", e);
    }

    let handle = session.handle();

    Ok(OutboundSession { handle, channel })
}

/// Outbound SSH client handler
struct OutboundHandler;

#[async_trait::async_trait]
impl russh::client::Handler for OutboundHandler {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        // Accept any server key (known_hosts validation is future work)
        Ok(true)
    }
}

impl Drop for Bastion {
    fn drop(&mut self) {
        let id = self.id;
        let clients = self.clients.clone();
        tokio::spawn(async move {
            let mut clients = clients.lock().await;
            clients.remove(&id);
        });
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    info!("CrabJar SSH Bastion starting on 0.0.0.0:2222");

    // Load target configuration from lab.toml
    let lab_config = config::load("../home-lab/lab.toml").await;

    let router = match lab_config {
        Ok(config) => {
            let machines = config.machines.into_iter().map(|m| targets::Machine {
                name: m.name,
                ip: m.lan_ip,
                ssh_user: m.ssh_user,
            }).collect();
            Arc::new(targets::TargetRouter::new(machines))
        }
        Err(e) => {
            warn!("Failed to load lab.toml: {}. Using default targets.", e);
            let machines = vec![targets::Machine {
                name: "jambo".to_string(),
                ip: "192.168.50.181".to_string(),
                ssh_user: "hermes".to_string(),
            }];
            Arc::new(targets::TargetRouter::new(machines))
        }
    };

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
    let mut bastion = Bastion::new(router, key_path);

    let socket = tokio::net::TcpListener::bind(("0.0.0.0", 2222)).await.unwrap();
    info!("Listening on 0.0.0.0:2222");

    let server = bastion.run_on_socket(config, &socket);
    if let Err(e) = server.await {
        warn!("Server error: {}", e);
    }
}
