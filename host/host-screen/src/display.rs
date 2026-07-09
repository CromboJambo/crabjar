//! # host-display
//!
//! Display protocol routing for Teams preview window.
//! Handles SPICE/VNC protocol selection and WebSocket relay for display protocols.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Display protocol for remote display
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DisplayProtocol {
    /// SPICE protocol
    Spice,
    /// VNC protocol
    Vnc,
}

/// Display connection info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConnection {
    /// WebSocket URL
    pub ws_url: String,
    /// Display protocol
    pub protocol: DisplayProtocol,
    /// VM name (from manifest)
    pub vm_name: String,
}

/// Display manager
#[derive(Debug, Clone, Default)]
pub struct DisplayManager {
    connections: std::collections::HashMap<String, DisplayConnection>,
}

impl DisplayManager {
    /// Create a new DisplayManager
    pub fn new() -> Self {
        Self {
            connections: std::collections::HashMap::new(),
        }
    }

    /// Add a display connection
    pub fn add_connection(&mut self, name: &str, conn: DisplayConnection) {
        self.connections.insert(name.to_string(), conn);
    }

    /// Get a display connection by name
    pub fn get_connection(&self, name: &str) -> Option<&DisplayConnection> {
        self.connections.get(name)
    }

    /// List all connections
    pub fn list_connections(&self) -> Vec<&str> {
        self.connections.keys().map(|s| s.as_str()).collect()
    }

    /// Start a display relay
    pub async fn start_relay(&self, name: &str) -> Result<()> {
        if let Some(conn) = self.connections.get(name) {
            tracing::info!(
                vm = %conn.vm_name,
                protocol = ?conn.protocol,
                "starting display relay"
            );
            // WebSocket relay to vm-bridge is handled by the axum-mux proxy layer.
        } else {
            tracing::warn!(name, "display connection not found");
        }
        Ok(())
    }

    /// Stop a display relay
    pub async fn stop_relay(&self, name: &str) -> Result<()> {
        tracing::info!(name, "stopping display relay");
        Ok(())
    }
}

/// Generate a preview thumbnail (320x180)
pub async fn generate_preview_thumbnail(session: &str) -> Result<std::path::PathBuf> {
    // Preview generation requires image processing dependencies.
    // Returns a placeholder path for now.
    tracing::info!(session, "preview thumbnail generation not yet implemented");
    Err(anyhow::anyhow!("thumbnail generation not yet implemented"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_connection_serialization() {
        let conn = DisplayConnection {
            ws_url: "ws://localhost:8080/ws".to_string(),
            protocol: DisplayProtocol::Spice,
            vm_name: "test-vm".to_string(),
        };
        let json = serde_json::to_string(&conn).unwrap();
        assert!(json.contains("spice"));
    }
}
