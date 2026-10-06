// CrabJar SSH Bastion Host
// Transparent SSH proxy to home-lab fleet using russh 0.63
// Architecture: per-connection state with real-time bidirectional forwarding

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{info, warn};
use russh::server::Server as _;

mod config;
mod targets;

/// Per-client state: tracks outbound SSH connection to target machine
struct ClientState {
    channel_id: russh::ChannelId,
    /// Handle for sending data back to the client
    server_handle: russh::server::Handle,
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

    fn auth_publickey(
        &mut self,
        user: &str,
        _public_key: &russh::keys::ssh_key::PublicKey,
    ) -> impl std::future::Future<Output = Result<russh::server::Auth, Self::Error>> + Send {
        info!("Public key auth for: {}", user);
        async { Ok(russh::server::Auth::Accept) }
    }

    fn auth_password(
        &mut self,
        user: &str,
        _password: &str,
    ) -> impl std::future::Future<Output = Result<russh::server::Auth, Self::Error>> + Send {
        info!("Password auth for: {}", user);
        async { Ok(russh::server::Auth::Accept) }
    }

    fn channel_open_session(
        &mut self,
        channel: russh::Channel<russh::server::Msg>,
        reply: russh::server::ChannelOpenHandle,
        session: &mut russh::server::Session,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        let client_id = self.id;
        let key_path = self.key_path.clone();
        let clients = self.clients.clone();

        async move {
            info!("Channel open session: {}", channel.id());

            // Get authenticated user from client context (simplified for now)
            let user = "hermes".to_string();

            // Determine target based on user
            let targets_list = Self::get_targets_for_user(&user);
            if targets_list.is_empty() {
                warn!("No target machines for user {}", user);
                reply.reject(russh::ChannelOpenFailure::ConnectFailed).await;
                return Ok(());
            }

            let target = &targets_list[0];
            info!("Routing {} to {} ({})", user, target.name, target.ip);

            // Store client state for data forwarding
            {
                let mut clients_lock = clients.lock().await;
                clients_lock.insert(client_id, ClientState {
                    channel_id: channel.id(),
                    server_handle: session.handle(),
                });
            }

            reply.accept().await;

            // Establish outbound SSH connection to target in background
            let host = target.ip.clone();
            let ssh_user = target.ssh_user.clone();
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
                                        // Copy the data (can't move from behind shared ref)
                                        let bytes: Vec<u8> = (*data).to_vec();
                                        if let Err(e) = client.server_handle.data(client.channel_id, bytes).await {
                                            warn!("Failed to send data to client: {:?}", e);
                                            break;
                                        }
                                    }
                                }
                                Some(russh::ChannelMsg::ExitStatus { exit_status }) => {
                                    info!("Target exited with status {}", exit_status);
                                    if let Some(client) = clients.lock().await.get(&client_id) {
                                        if let Err(e) = client.server_handle.eof(client.channel_id).await {
                                            warn!("Failed to send EOF: {:?}", e);
                                        }
                                    }
                                    break;
                                }
                                Some(russh::ChannelMsg::Eof) => {
                                    info!("Target connection EOF");
                                    if let Some(client) = clients.lock().await.get(&client_id) {
                                        if let Err(e) = client.server_handle.eof(client.channel_id).await {
                                            warn!("Failed to send EOF: {:?}", e);
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
    }

    fn data(
        &mut self,
        channel: russh::ChannelId,
        data: &[u8],
        session: &mut russh::server::Session,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        let clients = self.clients.clone();
        let client_id = self.id;

        async move {
            info!("Client sent {} bytes", data.len());
            Ok(())
        }
    }

    fn channel_close(
        &mut self,
        channel: russh::ChannelId,
        session: &mut russh::server::Session,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        let clients = self.clients.clone();
        let client_id = self.id;

        async move {
            info!("Client channel closed: {}", channel);
            clients.lock().await.remove(&client_id);
            Ok(())
        }
    }
}

impl Bastion {
    fn get_targets_for_user(user: &str) -> Vec<targets::Machine> {
        // Simplified: return all targets for now
        vec![targets::Machine {
            name: "jambo".to_string(),
            ip: "192.168.50.181".to_string(),
            ssh_user: user.to_string(),
        }]
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

    // Load SSH private key (russh 0.63 uses load_secret_key)
    let key = russh::keys::load_secret_key(key_path, None)?;

    // Create client config
    let config = russh::client::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
        ..Default::default()
    };
    let config = Arc::new(config);

    // Connect to target
    let mut session = russh::client::connect(config, (host.to_string(), port), OutboundHandler {}).await?;
    info!("Connected to {}", host);

    // Authenticate with SSH key using PrivateKeyWithHashAlg
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

    // Open channel and request shell
    let channel = session.channel_open_session().await?;

    // Request PTY for interactive shell
    if let Err(e) = channel.request_pty(false, "xterm-256color", 80, 24, 0, 0, &[]).await {
        warn!("Failed to request PTY: {}", e);
    }

    // Request shell on target (use exec with /bin/bash for interactive shell)
    if let Err(e) = channel.exec(true, "/bin/bash").await {
        warn!("Failed to request shell: {}", e);
    }

    Ok(OutboundSession { handle: session, channel })
}

/// Outbound SSH client handler
struct OutboundHandler;

impl russh::client::Handler for OutboundHandler {
    type Error = anyhow::Error;

    fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKeyOrCertificate,
    ) -> impl std::future::Future<Output = Result<bool, Self::Error>> + Send {
        async { Ok(true) }
    }
}

impl Drop for Bastion {
    fn drop(&mut self) {
        let id = self.id;
        let clients = self.clients.clone();
        tokio::spawn(async move {
            let mut clients_lock = clients.lock().await;
            clients_lock.remove(&id);
        });
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
    let mut bastion = Bastion::new(Arc::new(targets::TargetRouter::new(vec![])), key_path);

    let socket = TcpListener::bind(("0.0.0.0", 2222)).await.unwrap();
    info!("Listening on 0.0.0.0:2222");

    // Use run_on_socket from the Server trait (imported via `as _`)
    let server = bastion.run_on_socket(config, &socket);
    if let Err(e) = server.await {
        warn!("Server error: {}", e);
    }
}
