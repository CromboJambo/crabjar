use anyhow::{Context, Result};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::Response,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UnixStream};

use crate::manifest::Vm;

#[derive(Clone)]
struct ProxyState {
    vm: Vm,
}

/// Runs the worker for a single VM: binds a websocket listener and, for
/// each incoming connection, dials the VM's display socket and relays
/// raw bytes in both directions. The browser-side SPICE/VNC client
/// (spice-html5 / noVNC) does all the actual protocol decoding — this
/// proxy is byte-transparent and performs no interpretation of the payload.
pub async fn serve(vm: Vm, bind_addr: String) -> Result<()> {
    let listen_port = vm.listen_port;
    let name = vm.name.clone();
    let state = ProxyState { vm };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);

    let ip: std::net::IpAddr = bind_addr.parse().context("bind_addr must be a valid IP")?;
    let addr: SocketAddr = (ip, listen_port).into();
    tracing::info!(vm = %name, %addr, "worker listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<ProxyState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state.vm))
}

async fn handle_socket(socket: WebSocket, vm: Vm) {
    let result = if let Some(path) = vm.target.strip_prefix("unix:") {
        match UnixStream::connect(path).await {
            Ok(stream) => relay(socket, stream).await,
            Err(e) => {
                tracing::error!(vm = %vm.name, error = %e, "failed to connect target");
                return;
            }
        }
    } else {
        match TcpStream::connect(&vm.target).await {
            Ok(stream) => relay(socket, stream).await,
            Err(e) => {
                tracing::error!(vm = %vm.name, error = %e, "failed to connect target");
                return;
            }
        }
    };

    if let Err(e) = result {
        tracing::warn!(vm = %vm.name, error = %e, "session ended");
    }
}

/// Relay bytes between a WebSocket and a bidirectional async stream.
/// Uses a single select loop to drive both directions concurrently.
/// When either side closes, the other is drained and the resulting
/// error (if any) is returned.
async fn relay<S>(socket: WebSocket, stream: S) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + 'static,
{
    let (mut read_half, mut write_half) = tokio::io::split(stream);
    let (mut ws_sink, mut ws_stream) = socket.split();

    let mut target_buf = [0u8; 8192];

    loop {
        tokio::select! {
            // Read from target stream, forward to websocket
            n = read_half.read(&mut target_buf) => {
                match n {
                    Ok(0) => {
                        tracing::info!("target stream closed");
                        break;
                    }
                    Ok(n) => {
                        if let Err(e) = ws_sink.send(Message::Binary(target_buf[..n].to_vec())).await {
                            tracing::warn!(error = %e, "failed to write to websocket");
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "target read error");
                        break;
                    }
                }
            }

            // Read from websocket, forward to target
            msg = ws_stream.next() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        write_half.write_all(&data).await?;
                    }
                    Some(Ok(Message::Close(_frame))) => {
                        tracing::info!("client sent close frame");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = ws_sink.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Text(text))) => {
                        tracing::warn!(text_len = text.len(), "received text frame — expected binary");
                    }
                    Some(Err(e)) => {
                        tracing::warn!(error = %e, "websocket read error");
                        break;
                    }
                    None => {
                        tracing::info!("websocket stream ended");
                        break;
                    }
                }
            }

            else => {
                break;
            }
        }
    }

    // Drain remaining data from the side that didn't close first
    let drain_target = async {
        let mut buf = [0u8; 8192];
        loop {
            match read_half.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if let Err(e) = ws_sink.send(Message::Binary(buf[..n].to_vec())).await {
                        tracing::warn!(error = %e, "drain write failed");
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        anyhow::Ok(())
    };

    let drain_ws = async {
        while let Some(msg) = ws_stream.next().await {
            if let Ok(Message::Binary(data)) = msg {
                if let Err(e) = write_half.write_all(&data).await {
                    tracing::warn!(error = %e, "drain write failed");
                    break;
                }
            }
        }
        anyhow::Ok(())
    };

    let _ = tokio::join!(drain_target, drain_ws);
    Ok(())
}
