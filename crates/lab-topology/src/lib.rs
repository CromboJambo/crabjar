//! Crabjar Lab Topology
//!
//! Defines home-lab infrastructure as Rust data structures with target resolution
//! for SSH bastion and remote execution. Separates topology definitions (public)
//! from environment-specific configuration (private).

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
    pub key_path: String,
    #[serde(default)]
    pub port: u16,
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

    /// Get machine by name
    pub fn get_machine(&self, name: &str) -> Result<&MachineDef, TopologyError> {
        self.machines.get(name).ok_or(TopologyError::MachineNotFound(name.to_string()))
    }

    /// Get SSH config for a machine (falls back to defaults if not specified)
    pub fn get_ssh_config(&self, name: &str) -> Result<SshConfig, TopologyError> {
        let machine = self.get_machine(name)?;
        match &machine.ssh {
            Some(config) => Ok(config.clone()),
            None => Ok(SshConfig {
                user: "crombo".to_string(),
                key_path: "~/.ssh/id_lab".to_string(),
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
                    key_path: "~/.ssh/id_lab".to_string(),
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

/// Default topology for the Crombo home-lab
/// This is the public-safe version with named references instead of actual IPs.
/// Actual IPs are resolved at runtime via environment or private config.
pub fn default_topology() -> LabTopology {
    let mut machines = HashMap::new();

    // jambo - main desktop workstation
    machines.insert(
        "jambo".to_string(),
        MachineDef {
            name: "jambo".to_string(),
            label: "Main Desktop Workstation".to_string(),
            architecture: Architecture::X86_64,
            os: "Artix Linux".to_string(),
            init_system: InitSystem::Dinit,
            network: NetworkConfig {
                interfaces: vec![NetworkInterface {
                    name: "enp3s0".to_string(),
                    ip: "${JAMBO_LAN_IP}".to_string(), // Resolved at runtime
                    mac: Some("4c:c7:8f:5e:2a:1b".to_string()),
                }],
                tailscale_ip: Some("${JAMBO_TAILSCALE_IP}".to_string()),
            },
            ssh: Some(SshConfig {
                user: "crombo".to_string(),
                key_path: "~/.ssh/id_lab".to_string(),
                port: 22,
            }),
            services: vec![
                ServiceDef {
                    name: "ssh".to_string(),
                    ports: vec![ServicePort {
                        port: 22,
                        protocol: "tcp".to_string(),
                        description: Some("OpenSSH Server".to_string()),
                    }],
                    enabled: true,
                    description: None,
                },
                ServiceDef {
                    name: "vm-bridge".to_string(),
                    ports: vec![ServicePort {
                        port: 7001,
                        protocol: "tcp".to_string(),
                        description: Some("VM Bridge API".to_string()),
                    }],
                    enabled: true,
                    description: None,
                },
            ],
            notes: Some("Primary development workstation. SSH-only access (no display server).".to_string()),
        },
    );

    // ftw3 - GPU compute node
    machines.insert(
        "ftw3".to_string(),
        MachineDef {
            name: "ftw3".to_string(),
            label: "GPU Compute Node (RTX 3070 Ti)".to_string(),
            architecture: Architecture::X86_64,
            os: "Artix Linux".to_string(),
            init_system: InitSystem::Dinit,
            network: NetworkConfig {
                interfaces: vec![NetworkInterface {
                    name: "enp2s0".to_string(),
                    ip: "${FTW3_LAN_IP}".to_string(),
                    mac: Some("4c:c7:8f:5e:2a:1c".to_string()),
                }],
                tailscale_ip: Some("${FTW3_TAILSCALE_IP}".to_string()),
            },
            ssh: Some(SshConfig {
                user: "crombo".to_string(),
                key_path: "~/.ssh/id_lab".to_string(),
                port: 22,
            }),
            services: vec![ServiceDef {
                name: "ssh".to_string(),
                ports: vec![ServicePort {
                    port: 22,
                    protocol: "tcp".to_string(),
                    description: Some("OpenSSH Server".to_string()),
                }],
                enabled: true,
                description: None,
            }],
            notes: Some("RTX 3070 Ti GPU node for LLM inference and compute tasks.".to_string()),
        },
    );

    // sovnode - ARM index node
    machines.insert(
        "sovnode".to_string(),
        MachineDef {
            name: "sovnode".to_string(),
            label: "ARM Index Node (Pi5)".to_string(),
            architecture: Architecture::AArch64,
            os: "Arch Linux ARM".to_string(),
            init_system: InitSystem::Systemd,
            network: NetworkConfig {
                interfaces: vec![NetworkInterface {
                    name: "eth0".to_string(),
                    ip: "${SOVNODE_LAN_IP}".to_string(),
                    mac: Some("88:a2:9e:03:89:43".to_string()),
                }],
                tailscale_ip: Some("${SOVNODE_TAILSCALE_IP}".to_string()),
            },
            ssh: Some(SshConfig {
                user: "root".to_string(),
                key_path: "~/.ssh/id_lab".to_string(),
                port: 8822,
            }),
            services: vec![
                ServiceDef {
                    name: "ssh".to_string(),
                    ports: vec![ServicePort {
                        port: 8822,
                        protocol: "tcp".to_string(),
                        description: Some("OpenSSH Server".to_string()),
                    }],
                    enabled: true,
                    description: None,
                },
                ServiceDef {
                    name: "unbound".to_string(),
                    ports: vec![ServicePort {
                        port: 53,
                        protocol: "udp".to_string(),
                        description: Some("DNS Resolver".to_string()),
                    }],
                    enabled: true,
                    description: None,
                },
            ],
            notes: Some("Index node for lab. Runs unbound DNS and other infrastructure services.".to_string()),
        },
    );

    // lappy - laptop workstation
    machines.insert(
        "lappy".to_string(),
        MachineDef {
            name: "lappy".to_string(),
            label: "Laptop Workstation".to_string(),
            architecture: Architecture::X86_64,
            os: "Arch Linux".to_string(),
            init_system: InitSystem::Systemd,
            network: NetworkConfig {
                interfaces: vec![NetworkInterface {
                    name: "wlan0".to_string(),
                    ip: "${LAPPY_LAN_IP}".to_string(),
                    mac: None,
                }],
                tailscale_ip: Some("${LAPPY_TAILSCALE_IP}".to_string()),
            },
            ssh: Some(SshConfig {
                user: "crombo".to_string(),
                key_path: "~/.ssh/id_lab".to_string(),
                port: 22,
            }),
            services: vec![
                ServiceDef {
                    name: "ssh".to_string(),
                    ports: vec![ServicePort {
                        port: 22,
                        protocol: "tcp".to_string(),
                        description: Some("OpenSSH Server".to_string()),
                    }],
                    enabled: true,
                    description: None,
                },
            ],
            notes: Some("Mobile workstation. Primarily accessed via Tailscale.".to_string()),
        },
    );

    LabTopology {
        name: "crombo-home-lab".to_string(),
        version: 1,
        machines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_topology_has_machines() {
        let topo = default_topology();
        assert_eq!(topo.machines.len(), 2);
        assert!(topo.get_machine("jambo").is_ok());
        assert!(topo.get_machine("ftw3").is_ok());
    }

    #[test]
    fn test_ssh_config_resolution() {
        let topo = default_topology();
        let ssh = topo.get_ssh_config("jambo").unwrap();
        assert_eq!(ssh.user, "crombo");
        assert_eq!(ssh.key_path, "~/.ssh/id_lab");
    }

    #[test]
    fn test_ssh_targets_generation() {
        let topo = default_topology();
        let targets = topo.ssh_targets();
        assert_eq!(targets.len(), 2);
        // IPs are placeholders that would be resolved at runtime
        let (jambo_ip, _) = targets.get("jambo").unwrap();
        assert!(jambo_ip.contains("${JAMBO_TAILSCALE_IP}"));
    }
}