//! Configuration loading from lab.toml

use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabConfig {
    pub name: String,
    pub machines: Vec<Machine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Machine {
    pub name: String,
    pub label: Option<String>,
    pub lan_ip: String,
    pub tailscale_ip: Option<String>,
    pub ssh_user: String,
    #[serde(default)]
    pub ssh_port: u16,
    pub role: String,
}

pub async fn load(path: &str) -> anyhow::Result<LabConfig> {
    let content = fs::read_to_string(path)?;
    let config: LabConfig = toml::from_str(&content)?;
    Ok(config)
}