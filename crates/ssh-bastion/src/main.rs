// CrabJar SSH Bastion Host
// Transparent SSH proxy to home-lab fleet using russh
// Architecture follows warpgate's proven event-driven pattern

use std::collections::HashMap;
use std::sync::Arc;

use russh::keys::*;
use russh::server::{Msg, Server as _, Session};
use russh::*;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Clone)]
struct Bastion {
    clients: Arc<Mutex<HashMap<usize, (ChannelId, russh::server::Handle)>>>,
    id: usize,
}

impl Bastion {
    async fn post(&mut self, data: Vec<u8>) {
        let mut clients = self.clients.lock().await;
        for (id, (channel, s)) in clients.iter_mut() {
            if *id != self.id {
                let _ = s.data(*channel, data.clone()).await;
            }
        }
    }
}

impl server::Server for Bastion {
    type Handler = Self;

    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }

    fn handle_session_error(
        &mut self,
        _error: <Self::Handler as russh::server::Handler>::Error,
    ) {
        warn!("Session error");
    }
}

impl server::Handler for Bastion {
    type Error = russh::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: server::ChannelOpenHandle,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!("Channel open session: {}", channel.id());
        {
            let mut clients = self.clients.lock().await;
            clients.insert(self.id, (channel.id(), session.handle()));
        }
        reply.accept().await;
        Ok(())
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        _key: &ssh_key::PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        info!("Public key auth for: {}", user);
        // For now accept any key - real impl would validate against target machine
        Ok(server::Auth::Accept)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        // Echo back for now - real impl would route to target machine
        let data = format!("Got data: {}\r\n", String::from_utf8_lossy(data)).into_bytes();
        self.post(data.clone()).await;
        session.data(channel, data)?;
        Ok(())
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

    // Generate host key (ed25519) - same pattern as russh echoserver example
    let config = russh::server::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
        auth_rejection_time: std::time::Duration::from_secs(3),
        keys: vec![
            PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).unwrap(),
        ],
        ..Default::default()
    };
    let config = Arc::new(config);

    let mut bastion = Bastion {
        clients: Arc::new(Mutex::new(HashMap::new())),
        id: 0,
    };

    let socket = TcpListener::bind(("0.0.0.0", 2222)).await.unwrap();
    info!("Listening on 0.0.0.0:2222");

    let server = bastion.run_on_socket(config, &socket);
    server.await.unwrap();
}
