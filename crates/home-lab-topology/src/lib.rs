//! Crabjar Lab Topology
//!
//! Defines home-lab infrastructure as Rust data structures with target resolution
//! for SSH bastion and remote execution. Separates topology definitions (public)
//! from environment-specific configuration (private).
//!
//! Topology is loaded from an external TOML file at runtime, not hardcoded.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Error types for topology operations
#[derive(Error, Debug)]
pub enum TopologyError {
    #[error("machine '{0}' not found in topology")]
    MachineNotFound(String),
    #[error("service '{0}' not found on machine '{1}'")]
    ServiceNotFound(String, String),
    #[error("failed to parse topology config: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("missing required field: {0}")]
    MissingField(String),
    #[error("failed to read topology file '{0}': {1}")]
    FileReadError(String, String),
}

/// Machine architecture
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Architecture {
    #[serde(rename = "x86_64")]
    X86_64,
    #[serde(rename = "aarch64")]
    AArch64,
}

/// Init system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InitSystem {
    #[serde(rename = "dinit")]
    Dinit,
    #[serde(rename = "systemd")]
    Systemd,
    #[serde(rename = "runit")]
    Runit,
}

/// Network interface configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: String,
    #[serde(default)]
    pub mac: Option<String>,
}

/// Machine network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    #[serde(default)]
    pub interfaces: Vec<NetworkInterface>,
    #[serde(default)]
    pub tailscale_ip: Option<String>,
}

/// Service port definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePort {
    pub port: u16,
    pub protocol: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Service definition on a machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDef {
    pub name: String,
    pub ports: Vec<ServicePort>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub description: Option<String>,
}

/// SSH access configuration for a machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub user: String,
    #[serde(default)]
    pub key_path: Option<String>,
    #[serde(default)]
    pub password_auth: bool,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
}

fn default_ssh_port() -> u16 {
    22
}

/// A single machine in the lab topology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineDef {
    pub name: String,
    pub label: String,
    pub architecture: Architecture,
    pub os: String,
    pub init_system: InitSystem,
    pub network: NetworkConfig,
    #[serde(default)]
    pub ssh: Option<SshConfig>,
    #[serde(default)]
    pub services: Vec<ServiceDef>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Complete lab topology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabTopology {
    pub name: String,
    pub version: u32,
    pub machines: HashMap<String, MachineDef>,
}

impl LabTopology {
    /// Parse topology from TOML string
    pub fn from_toml(content: &str) -> Result<Self, TopologyError> {
        let topo: LabTopology = toml::from_str(content)?;
        Ok(topo)
    }

    /// Load topology from a file path
    pub fn from_file(path: &str) -> Result<Self, TopologyError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| TopologyError::FileReadError(path.to_string(), e.to_string()))?;
        Self::from_toml(&content)
    }

    /// Get machine by name
    pub fn get_machine(&self, name: &str) -> Result<&MachineDef, TopologyError> {
        self.machines
            .get(name)
            .ok_or(TopologyError::MachineNotFound(name.to_string()))
    }

    /// Get SSH config for a machine (falls back to defaults if not specified)
    pub fn get_ssh_config(&self, name: &str) -> Result<SshConfig, TopologyError> {
        let machine = self.get_machine(name)?;
        match &machine.ssh {
            Some(config) => Ok(config.clone()),
            None => Ok(SshConfig {
                user: "crombo".to_string(),
                key_path: Some("~/.ssh/id_lab".to_string()),
                password_auth: false,
                port: 22,
            }),
        }
    }

    /// Get all machines with their primary IPs for SSH bastion target registry
    pub fn ssh_targets(&self) -> HashMap<String, (String, SshConfig)> {
        let mut targets = HashMap::new();
        for (name, machine) in &self.machines {
            // Prefer Tailscale IP if available, otherwise first interface
            let ip = match &machine.network.tailscale_ip {
                Some(ts_ip) => ts_ip.clone(),
                None => machine
                    .network
                    .interfaces
                    .first()
                    .map(|iface| iface.ip.clone())
                    .unwrap_or_default(),
            };
            if !ip.is_empty() {
                let ssh_config = self.get_ssh_config(name).unwrap_or(SshConfig {
                    user: "crombo".to_string(),
                    key_path: Some("~/.ssh/id_lab".to_string()),
                    password_auth: false,
                    port: 22,
                });
                targets.insert(name.clone(), (ip, ssh_config));
            }
        }
        targets
    }

    /// List all machine names
    pub fn machine_names(&self) -> Vec<String> {
        self.machines.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_topology() {
        let toml_str = r#"
            name = "test-lab"
            version = 1
            [machines.test-machine]
            name = "test-machine"
            label = "Test"
            architecture = "x86_64"
            os = "Linux"
            init_system = "systemd"
            network = { tailscale_ip = "100.100.100.100" }
        "#;
        let topo = LabTopology::from_toml(toml_str).unwrap();
        assert_eq!(topo.name, "test-lab");
        assert!(topo.get_machine("test-machine").is_ok());
    }

    #[test]
    fn test_ssh_config_fallback() {
        let toml_str = r#"
            name = "test"
            version = 1
            [machines.m1]
            name = "m1"
            label = "M1"
            architecture = "x86_64"
            os = "Linux"
            init_system = "systemd"
            network = { tailscale_ip = "1.2.3.4" }
        "#;
        let topo = LabTopology::from_toml(toml_str).unwrap();
        let ssh = topo.get_ssh_config("m1").unwrap();
        assert_eq!(ssh.user, "crombo");
        assert_eq!(ssh.port, 22);
    }
}
