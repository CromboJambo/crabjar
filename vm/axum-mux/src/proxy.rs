use anyhow::{Context, Result};
use axum::{
    Router,
    extract::{Query, State},
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UnixStream};

use crate::manifest::Vm;

#[derive(Clone)]
struct ProxyState {
    vm: Vm,
}

/// Health-check timeout for target connectivity probes.
const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(2);

/// Verifies the VM's display target is reachable before accepting a
/// websocket connection. Returns an error response if the target is
/// down, so the browser client gets immediate feedback instead of a
/// silent dead relay.
async fn check_target_health(vm: &Vm) -> Result<()> {
    let probe = async {
        if let Some(path) = vm.target.strip_prefix("unix:") {
            UnixStream::connect(path).await?;
        } else {
            TcpStream::connect(&vm.target).await?;
        }
        Ok::<(), anyhow::Error>(())
    };

    tokio::time::timeout(HEALTH_CHECK_TIMEOUT, probe)
        .await
        .map_err(|_| anyhow::anyhow!("health check timed out"))?
}

/// Extracts the auth token from either a `?token=` query parameter or
/// an `Authorization: Bearer <token>` header. Returns None if neither
/// is present.
fn extract_token(headers: &HeaderMap, query: &HashMap<String, String>) -> Option<String> {
    // Query param takes precedence (simpler for browser clients).
    if let Some(token) = query.get("token") {
        return Some(token.clone());
    }
    // Bearer token in Authorization header.
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|auth| auth.strip_prefix("Bearer "))
        .map(|bearer| bearer.trim().to_string())
}

/// Verifies the request's token matches the VM's configured token.
/// If no token is configured, any request is accepted (open access).
fn check_auth(vm: &Vm, headers: &HeaderMap, query: &HashMap<String, String>) -> Result<(), Response> {
    let required = match &vm.token {
        None => return Ok(()), // no auth required
        Some(t) => t,
    };

    let provided = extract_token(headers, query);
    match provided {
        Some(p) if p == *required => Ok(()),
        Some(_) => Err((StatusCode::UNAUTHORIZED, "invalid token").into_response()),
        None => Err((StatusCode::UNAUTHORIZED, "token required").into_response()),
    }
}

/// Runs the worker for a single VM: binds a websocket listener and, for
/// each incoming connection, dials the VM's display socket and relays
/// raw bytes in both directions. The browser-side SPICE/VNC client
/// (spice-html5 / noVNC) does all the actual protocol decoding — this
/// proxy is byte-transparent and performs no interpretation of the payload.
///
/// When `screen_state` is provided, also mounts screen-sharing routes
/// (thumbnails) for VNC targets.
pub async fn serve_with_screen(
    vm: Vm,
    bind_addr: String,
    screen_state: Option<crate::screen::ScreenState>,
) -> Result<()> {
    let listen_port = vm.listen_port;
    let name = vm.name.clone();
    let state = ProxyState { vm };

    let mut app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state.clone());

    if let Some(screen) = screen_state {
        app = app.merge(crate::screen::routes(screen));
    }

    let ip: std::net::IpAddr = bind_addr.parse().context("bind_addr must be a valid IP")?;
    let addr: SocketAddr = (ip, listen_port).into();
    tracing::info!(vm = %name, %addr, "worker listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<ProxyState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let vm = state.vm.clone();

    // Auth check (if token configured for this VM).
    if let Err(resp) = check_auth(&vm, &headers, &query) {
        tracing::warn!(vm = %vm.name, "auth failed");
        return resp;
    }

    // Health-check: reject the connection if the target is unreachable.
    // This gives the browser client immediate feedback (503) instead of
    // accepting a websocket that can't relay anything.
    if let Err(e) = check_target_health(&vm).await {
        tracing::warn!(vm = %vm.name, error = %e, "target health check failed");
        return (StatusCode::SERVICE_UNAVAILABLE, e.to_string()).into_response();
    }

    ws.on_upgrade(move |socket| handle_socket(socket, vm))
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
            if let Ok(Message::Binary(data)) = msg
                && let Err(e) = write_half.write_all(&data).await
            {
                tracing::warn!(error = %e, "drain write failed");
                break;
            }
        }
        anyhow::Ok(())
    };

    let _ = tokio::join!(drain_target, drain_ws);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};
    use std::collections::HashMap;

    fn vm_with_token(token: &str) -> Vm {
        Vm {
            name: "test".to_string(),
            protocol: crate::manifest::Protocol::Spice,
            target: "127.0.0.1:5930".to_string(),
            listen_port: 7001,
            token: Some(token.to_string()),
        }
    }

    fn vm_open() -> Vm {
        Vm {
            name: "test".to_string(),
            protocol: crate::manifest::Protocol::Spice,
            target: "127.0.0.1:5930".to_string(),
            listen_port: 7001,
            token: None,
        }
    }

    #[test]
    fn auth_open_vm_accepts_anything() {
        let vm = vm_open();
        assert!(check_auth(&vm, &HeaderMap::new(), &HashMap::new()).is_ok());
    }

    #[test]
    fn auth_missing_token_rejected() {
        let vm = vm_with_token("secret");
        let result = check_auth(&vm, &HeaderMap::new(), &HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn auth_wrong_token_rejected() {
        let vm = vm_with_token("secret");
        let mut query = HashMap::new();
        query.insert("token".to_string(), "wrong".to_string());
        let result = check_auth(&vm, &HeaderMap::new(), &query);
        assert!(result.is_err());
    }

    #[test]
    fn auth_correct_query_token_accepted() {
        let vm = vm_with_token("secret");
        let mut query = HashMap::new();
        query.insert("token".to_string(), "secret".to_string());
        assert!(check_auth(&vm, &HeaderMap::new(), &query).is_ok());
    }

    #[test]
    fn auth_correct_bearer_header_accepted() {
        let vm = vm_with_token("secret");
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_str("Bearer secret").unwrap(),
        );
        assert!(check_auth(&vm, &headers, &HashMap::new()).is_ok());
    }

    #[test]
    fn query_token_takes_precedence_over_header() {
        let vm = vm_with_token("query-wins");
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_str("Bearer header-loses").unwrap(),
        );
        let mut query = HashMap::new();
        query.insert("token".to_string(), "query-wins".to_string());
        assert!(check_auth(&vm, &headers, &query).is_ok());
    }
}
