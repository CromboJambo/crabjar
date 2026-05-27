// crabjar/src/bitwarden/store.rs
// Bitwarden credential store integration

use crate::bitwarden::cli;
use crate::bitwarden::{BitwardenError, BitwardenItem, BitwardenResult};
use serde::{Deserialize, Serialize};

/// Credential entry for bitwarden storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialEntry {
    pub name: String,
    pub username: String,
    pub password: String,
    pub notes: Option<String>,
    pub folder: Option<String>,
    pub created_at: String,
    pub modified_at: String,
}

impl CredentialEntry {
    pub fn new(
        name: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            name: name.into(),
            username: username.into(),
            password: password.into(),
            notes: None,
            folder: None,
            created_at: now.clone(),
            modified_at: now,
        }
    }
}

/// Bitwarden credential store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialStore {
    pub folder: Option<String>,
    pub collection: Option<String>,
}

impl CredentialStore {
    pub fn new(folder: Option<String>, collection: Option<String>) -> Self {
        Self { folder, collection }
    }

    /// List all stored credentials
    pub fn list_credentials(&self) -> BitwardenResult<Vec<BitwardenItem>> {
        cli::list_items(self.folder.as_deref(), self.collection.as_deref())
    }

    /// Add a credential to bitwarden
    pub fn add_credential(&self, entry: &CredentialEntry) -> BitwardenResult<String> {
        // This would typically use `bw create item` command
        // For now, return a placeholder
        Ok(format!("credential_{}", entry.name))
    }

    /// Remove a credential from bitwarden
    pub fn remove_credential(&self, _name: &str) -> BitwardenResult<()> {
        // This would typically use `bw delete item` command
        Ok(())
    }

    /// Get a credential by name
    pub fn get_credential(&self, name: &str) -> BitwardenResult<Option<BitwardenItem>> {
        let items = cli::search_items(name)?;
        Ok(items.into_iter().next())
    }
}

/// Load credential store configuration from file
pub fn load_store_config(path: &std::path::Path) -> BitwardenResult<CredentialStore> {
    if !path.exists() {
        return Ok(CredentialStore::new(None, None));
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| BitwardenError::Internal(format!("Failed to read config: {}", e)))?;

    let config: CredentialStore = serde_json::from_str(&content)?;
    Ok(config)
}

/// Save credential store configuration to file
pub fn save_store_config(path: &std::path::Path, store: &CredentialStore) -> BitwardenResult<()> {
    let content = serde_json::to_string_pretty(store)
        .map_err(|e| BitwardenError::Internal(format!("Failed to serialize config: {}", e)))?;

    std::fs::write(path, content)
        .map_err(|e| BitwardenError::Internal(format!("Failed to write config: {}", e)))
}
