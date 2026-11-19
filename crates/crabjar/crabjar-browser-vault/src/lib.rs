//! Browser vault integration for credential management during browser automation.
//!
//! Wraps the Bitwarden CLI (`bw`) to provide secure credential access for login walls.
//! Supports self-hosted Vaultwarden instances via `BW_URL` environment variable.
//!
//! # Usage
//!
//! ```no_run
//! use crabjar_browser_vault::{BrowserVaultClient, VaultConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = VaultConfig::from_env();
//!     let client = BrowserVaultClient::new(config);
//!
//!     // List all credentials
//!     let items = client.list_items(None, None).await.unwrap();
//!     println!("Found {} credentials", items.len());
//!
//!     // Get a specific credential by URL match
//!     let item = client.get_by_url("https://github.com").await.unwrap();
//!     if let Some(username) = &item.username {
//!         println!("GitHub login: {}", username);
//!     }
//! }
//! ```

mod error;

pub use error::{VaultError, VaultResult};

use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Default timeout for Bitwarden CLI operations in seconds.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Configuration for the browser vault client.
#[derive(Debug, Clone)]
pub struct VaultConfig {
    /// Path to the Bitwarden CLI binary.
    pub cli_path: String,
    /// Base URL of the Bitwarden/Vaultwarden server (for self-hosted).
    pub server_url: Option<String>,
    /// Timeout for CLI operations in seconds.
    pub timeout_secs: u64,
}

impl VaultConfig {
    /// Load configuration from environment variables.
    ///
    /// Env vars (in priority order):
    /// - `BW_CLI_PATH` — path to the Bitwarden CLI (default: "bw")
    /// - `BW_URL` or `VAULTWARDEN_URL` — self-hosted server URL
    /// - `BW_TIMEOUT` — timeout in seconds (default: 30)
    pub fn from_env() -> Self {
        let cli_path = env::var("BW_CLI_PATH").unwrap_or_else(|_| "bw".to_string());
        let server_url = env::var("BW_URL")
            .or_else(|_| env::var("VAULTWARDEN_URL"))
            .ok();
        let timeout_secs = env::var("BW_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        Self {
            cli_path,
            server_url,
            timeout_secs,
        }
    }

    /// Create a default configuration (uses "bw" from PATH).
    pub fn default() -> Self {
        Self {
            cli_path: "bw".to_string(),
            server_url: None,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self::default()
    }
}

/// A credential item from the vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultItem {
    /// Unique identifier for this item.
    pub id: String,
    /// Display name of the item.
    pub name: String,
    /// Associated URI (if any).
    #[serde(default)]
    pub uri: Option<String>,
    /// Username for login items.
    #[serde(default)]
    pub username: Option<String>,
    /// Password for login items.
    #[serde(default)]
    pub password: Option<String>,
    /// Optional notes.
    #[serde(default)]
    pub notes: Option<String>,
    /// Folder name (if organized into folders).
    #[serde(default)]
    pub folder_name: Option<String>,
}

impl VaultItem {
    /// Check if this item matches a given URL by domain comparison.
    pub fn matches_url(&self, target_url: &str) -> bool {
        let Some(item_uri) = &self.uri else {
            return false;
        };

        // Extract hostnames and compare (supports subdomain matching)
        let target_host = extract_hostname(target_url);
        let item_host = extract_hostname(item_uri);

        match (target_host, item_host) {
            (Some(t), Some(i)) => t == i || t.ends_with(&format!(".{}", i)),
            _ => false,
        }
    }
}

/// Extract the hostname from a URL string.
fn extract_hostname(url: &str) -> Option<String> {
    // Handle URLs with or without scheme
    let cleaned = if url.starts_with("http://") || url.starts_with("https://") {
        url.trim_start_matches("https://")
            .trim_start_matches("http://")
    } else {
        url
    };

    cleaned
        .split('/')
        .next()
        .map(|h| h.to_string())
        .filter(|h| !h.is_empty())
}

/// Client for interacting with the Bitwarden vault via CLI.
pub struct BrowserVaultClient {
    config: VaultConfig,
}

impl BrowserVaultClient {
    /// Create a new browser vault client with the given configuration.
    pub fn new(config: VaultConfig) -> Self {
        Self { config }
    }

    /// Check if the Bitwarden CLI is available and authenticated.
    ///
    /// Returns `Ok(true)` if the CLI is available and unlocked,
    /// `Ok(false)` if locked, or an error if the CLI is not found.
    pub async fn is_available(&self) -> VaultResult<bool> {
        let output = self.run_cli(&["status"]).await?;

        if !output.status.success() {
            return Ok(false);
        }

        let status: serde_json::Value =
            serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
        let is_unlocked = status
            .get("unlocked")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(is_unlocked)
    }

    /// List all vault items, optionally filtered by folder or collection.
    pub async fn list_items(
        &self,
        folder: Option<&str>,
        collection: Option<&str>,
    ) -> VaultResult<Vec<VaultItem>> {
        let mut args = vec!["list", "items"];

        if let Some(f) = folder {
            args.push("--collectionid");
            args.push(f);
        }

        if let Some(c) = collection {
            args.push("--foldername");
            args.push(c);
        }

        let output = self.run_cli(&args).await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let items: Vec<VaultItem> = serde_json::from_str(&stdout)?;

        info!(count = items.len(), "Listed vault items");
        Ok(items)
    }

    /// Get a specific vault item by ID.
    pub async fn get_item(&self, id: &str) -> VaultResult<VaultItem> {
        let output = self.run_cli(&["get", "item", id]).await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let item: VaultItem = serde_json::from_str(&stdout)?;

        Ok(item)
    }

    /// Search vault items by query string.
    pub async fn search_items(&self, query: &str) -> VaultResult<Vec<VaultItem>> {
        let output = self.run_cli(&["list", "items", "--search", query]).await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let items: Vec<VaultItem> = serde_json::from_str(&stdout)?;

        info!(count = items.len(), query, "Searched vault items");
        Ok(items)
    }

    /// Find a credential that matches the given URL.
    ///
    /// Searches for items whose URI matches the target URL by hostname comparison.
    /// Returns the first matching item found.
    pub async fn get_by_url(&self, url: &str) -> VaultResult<VaultItem> {
        let all_items = self.list_items(None, None).await?;

        // First try exact hostname match
        for item in &all_items {
            if let Some(uri) = &item.uri {
                if extract_hostname(uri) == extract_hostname(url) {
                    info!(name = %item.name, url, "Found credential by URL match");
                    return Ok(item.clone());
                }
            }
        }

        // Then try subdomain matching
        for item in &all_items {
            if item.matches_url(url) {
                info!(name = %item.name, url, "Found credential by subdomain match");
                return Ok(item.clone());
            }
        }

        Err(VaultError::NotFound(format!(
            "No credential found for URL: {}",
            url
        )))
    }

    /// Find a credential by name (case-insensitive).
    pub async fn get_by_name(&self, name: &str) -> VaultResult<VaultItem> {
        let all_items = self.list_items(None, None).await?;

        for item in &all_items {
            if item.name.to_lowercase() == name.to_lowercase() {
                info!(name = %item.name, "Found credential by name");
                return Ok(item.clone());
            }
        }

        Err(VaultError::NotFound(format!(
            "No credential found with name: {}",
            name
        )))
    }

    /// Generate a new password using the Bitwarden CLI.
    pub async fn generate_password(
        &self,
        length: usize,
        uppercase: bool,
        lowercase: bool,
        numbers: bool,
        special: bool,
    ) -> VaultResult<String> {
        let length_str = length.to_string();
        let mut args = vec![
            "generate",
            "--length",
            &length_str,
            "--uppercase",
            if uppercase { "true" } else { "false" },
            "--lowercase",
            if lowercase { "true" } else { "false" },
            "--numbers",
            if numbers { "true" } else { "false" },
            "--special",
            if special { "true" } else { "false" },
        ];

        let output = self.run_cli(&args).await?;
        let password = String::from_utf8_lossy(&output.stdout).trim().to_string();

        Ok(password)
    }

    /// Run a Bitwarden CLI command and return the output.
    async fn run_cli(&self, args: &[&str]) -> VaultResult<std::process::Output> {
        debug!(cli = %self.config.cli_path, args = ?args, "Running Bitwarden CLI");

        let mut cmd = Command::new(&self.config.cli_path);
        cmd.args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Set environment for self-hosted servers
        if let Some(url) = &self.config.server_url {
            cmd.env("BW_URL", url);
        }

        let timeout = std::time::Duration::from_secs(self.config.timeout_secs);

        let output = tokio::time::timeout(timeout, cmd.output())
            .await
            .map_err(|_| VaultError::Timeout {
                seconds: self.config.timeout_secs,
            })?
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => VaultError::CliNotFound(format!(
                    "Bitwarden CLI not found at '{}'. Install with 'npm install -g @bitwarden/cli' or set BW_CLI_PATH.",
                    self.config.cli_path
                )),
                _ => VaultError::CliError {
                    code: -1,
                    stderr: format!("Failed to execute CLI: {}", e),
                },
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            // Check for common error patterns
            if stderr.contains("locked") || stderr.contains("unlock") {
                return Err(VaultError::Locked(stderr));
            }

            return Err(VaultError::CliError {
                code: output.status.code().unwrap_or(-1),
                stderr,
            });
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_hostname_with_scheme() {
        assert_eq!(
            extract_hostname("https://github.com"),
            Some("github.com".to_string())
        );
        assert_eq!(
            extract_hostname("http://example.com/path"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn test_extract_hostname_without_scheme() {
        assert_eq!(
            extract_hostname("github.com"),
            Some("github.com".to_string())
        );
        assert_eq!(
            extract_hostname("example.com/some/path"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn test_extract_hostname_empty() {
        assert_eq!(extract_hostname(""), None);
    }

    #[test]
    fn test_vault_item_matches_url_exact() {
        let item = VaultItem {
            id: "1".to_string(),
            name: "GitHub".to_string(),
            uri: Some("https://github.com".to_string()),
            username: None,
            password: None,
            notes: None,
            folder_name: None,
        };

        assert!(item.matches_url("https://github.com"));
        assert!(item.matches_url("https://github.com/user/repo"));
    }

    #[test]
    fn test_vault_item_matches_url_subdomain() {
        let item = VaultItem {
            id: "1".to_string(),
            name: "GitHub Enterprise".to_string(),
            uri: Some("https://enterprise.github.com".to_string()),
            username: None,
            password: None,
            notes: None,
            folder_name: None,
        };

        assert!(item.matches_url("https://enterprise.github.com"));
        // Subdomain of the item's domain should match
        assert!(item.matches_url("https://api.enterprise.github.com"));
    }

    #[test]
    fn test_vault_item_no_uri() {
        let item = VaultItem {
            id: "1".to_string(),
            name: "No URL".to_string(),
            uri: None,
            username: Some("user".to_string()),
            password: None,
            notes: None,
            folder_name: None,
        };

        assert!(!item.matches_url("https://example.com"));
    }

    #[test]
    fn test_vault_config_defaults() {
        let config = VaultConfig::default();
        assert_eq!(config.cli_path, "bw");
        assert_eq!(config.server_url, None);
        assert_eq!(config.timeout_secs, DEFAULT_TIMEOUT_SECS);
    }

    #[test]
    fn test_vault_config_from_env() {
        let config = VaultConfig::from_env();
        // Should not panic; values depend on environment
        assert!(!config.cli_path.is_empty());
        assert!(config.timeout_secs > 0);
    }
}
