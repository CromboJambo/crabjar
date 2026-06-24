//! Minimal project configuration loader.
//!
//! Loads `.crabjar_config.toml` from project directories.

use serde::Deserialize;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur when loading project configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config not found at {0}")]
    NotFound(PathBuf),
    #[error("failed to parse config: {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("config is missing required 'name' field")]
    MissingName,
}

impl From<io::Error> for ConfigError {
    fn from(e: io::Error) -> Self {
        ConfigError::NotFound(PathBuf::from(e.to_string()))
    }
}

/// A single tool definition from the config.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolDef {
    pub path: String,
    #[allow(dead_code)]
    #[serde(default)]
    pub commands: Vec<String>,
}

/// Project-level configuration loaded from `.crabjar_config.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectConfig {
    #[serde(alias = "workspace_name")]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_auto_register")]
    pub auto_register: bool,
    #[serde(default)]
    pub tools: Vec<ToolDef>,
    #[serde(default)]
    pub tool_execution_enabled: bool,
    #[serde(default)]
    pub user_dinit_socket: Option<String>,
}

impl ProjectConfig {
    /// Returns the workspace name (alias for name).
    #[allow(dead_code)]
    pub fn workspace_name(&self) -> &str {
        &self.name
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            auto_register: default_auto_register(),
            tools: Vec::new(),
            tool_execution_enabled: false,
            user_dinit_socket: None,
        }
    }
}

fn default_auto_register() -> bool {
    false
}

impl ProjectConfig {
    /// Load configuration from `.crabjar_config.toml` in the given directory.
    pub fn load(dir: &Path) -> Result<Self, ConfigError> {
        let config_path = dir.join(".crabjar_config.toml");
        if !config_path.exists() {
            return Err(ConfigError::NotFound(config_path));
        }
        let content = std::fs::read_to_string(&config_path)?;
        let config: Self = toml::from_str(&content).map_err(ConfigError::TomlError)?;
        if config.name.is_empty() {
            return Err(ConfigError::MissingName);
        }
        Ok(config)
    }

    /// Returns all commands defined across all tool entries.
    #[allow(dead_code)]
    pub fn get_all_commands(&self) -> Vec<String> {
        self.tools.iter().flat_map(|t| t.commands.clone()).collect()
    }
}
