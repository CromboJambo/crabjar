//! Session management — processes events from the SSH handler and routes connections.

use crate::handler::{HandleWrapper, ServerHandlerEvent};
use crate::targets::TargetRouter;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::*;

/// Main session event loop — processes events from the SSH handler and routes connections.
pub async fn run_session_loop(
    mut event_rx: UnboundedReceiver<ServerHandlerEvent>,
    targets: Arc<TargetRouter>,
) {
    let mut authenticated_user = None;
    let mut active_channel = None;

    while let Some(event) = event_rx.recv().await {
        match event {
            ServerHandlerEvent::Authenticated(handle) => {
                info!("Client authenticated");
                // Store handle for sending data back to client
            }
            ServerHandlerEvent::ChannelOpenSession(channel_id, reply) => {
                info!(channel = channel_id, "Channel opened");
                active_channel = Some(channel_id);
                // Auto-accept for now (routing happens on shell/exec request)
                reply.accept();
            }
            ServerHandlerEvent::ShellRequest(channel_id, tx) => {
                if let Some(user) = &authenticated_user {
                    info!(user, channel = channel_id, "Routing to target machine");

                    // For now, route to first available target (simplified)
                    let targets_list = targets.get_targets_for_user(user);
                    if !targets_list.is_empty() {
                        let target = &targets_list[0];
                        info!(target = %target.name, "Routing shell request");

                        // Would establish connection to target here
                        tx.send(true).unwrap();
                    } else {
                        tx.send(false).unwrap();
                    }
                } else {
                    tx.send(false).unwrap();
                }
            }
            ServerHandlerEvent::ExecRequest(channel_id, data, tx) => {
                if let Some(user) = &authenticated_user {
                    info!(user, channel = channel_id, cmd = %String::from_utf8_lossy(&data), "Routing exec request");

                    // For now, route to first available target (simplified)
                    let targets_list = targets.get_targets_for_user(user);
                    if !targets_list.is_empty() {
                        let target = &targets_list[0];
                        info!(target = %target.name, "Routing exec request");

                        // Would establish connection to target here
                        tx.send(true).unwrap();
                    } else {
                        tx.send(false).unwrap();
                    }
                } else {
                    tx.send(false).unwrap();
                }
            }
            ServerHandlerEvent::Data(channel_id, data, tx) => {
                info!(channel = channel_id, bytes = data.len(), "Received data");
                // Would forward data to target machine here
                tx.send(()).unwrap();
            }
            ServerHandlerEvent::AuthPublicKey(user, key, tx) => {
                info!(user, "Public key auth request");
                authenticated_user = Some(user.clone());
                tx.send(russh::server::Auth::Ok).unwrap();
            }
            ServerHandlerEvent::AuthPassword(user, password, tx) => {
                info!(user, "Password auth request");
                authenticated_user = Some(user.clone());
                tx.send(russh::server::Auth::Ok).unwrap();
            }
            ServerHandlerEvent::Disconnect => {
                info!("Client disconnected");
                break;
            }
        }
    }

    info!("Session loop ended");
}