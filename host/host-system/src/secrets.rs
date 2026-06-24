/// Secrets backend abstraction.
///
/// Wraps the keyring crate for OS-level secret storage.
/// Provides a simple key-value store for credentials.
use crabjar_host_core::event_bus::EventBus;
use std::sync::Arc;

pub struct SecretsBackend {
    service_name: String,
    _event_bus: Arc<EventBus>,
}

impl SecretsBackend {
    pub fn new(event_bus: Arc<EventBus>, service_name: &str) -> Self {
        Self {
            service_name: service_name.into(),
            _event_bus: event_bus,
        }
    }

    /// Save a secret.
    pub fn save(&self, key: &str, value: &str) -> Result<(), SecretsError> {
        let entry = keyring::Entry::new(&self.service_name, key)
            .map_err(|e| SecretsError::EntryFailed(e.to_string()))?;
        entry.set_password(value)
            .map_err(|e| SecretsError::SaveFailed(e.to_string()))?;
        tracing::debug!(key, "secret saved");
        Ok(())
    }

    /// Retrieve a secret.
    pub fn get(&self, key: &str) -> Result<String, SecretsError> {
        let entry = keyring::Entry::new(&self.service_name, key)
            .map_err(|e| SecretsError::EntryFailed(e.to_string()))?;
        let password = entry.get_password()
            .map_err(|e| SecretsError::GetFailed(e.to_string()))?;
        tracing::debug!(key, "secret retrieved");
        Ok(password)
    }

    /// Delete a secret.
    pub fn delete(&self, key: &str) -> Result<(), SecretsError> {
        let entry = keyring::Entry::new(&self.service_name, key)
            .map_err(|e| SecretsError::EntryFailed(e.to_string()))?;
        entry.delete_credential()
            .map_err(|e| SecretsError::DeleteFailed(e.to_string()))?;
        tracing::debug!(key, "secret deleted");
        Ok(())
    }

    /// Check if a secret exists.
    pub fn exists(&self, key: &str) -> Result<bool, SecretsError> {
        let entry = keyring::Entry::new(&self.service_name, key)
            .map_err(|e| SecretsError::EntryFailed(e.to_string()))?;
        match entry.get_password() {
            Ok(_) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(SecretsError::GetFailed(e.to_string())),
        }
    }
}

/// Secrets errors.
#[derive(thiserror::Error, Debug)]
pub enum SecretsError {
    #[error("failed to create entry: {0}")]
    EntryFailed(String),
    #[error("failed to save secret: {0}")]
    SaveFailed(String),
    #[error("failed to retrieve secret: {0}")]
    GetFailed(String),
    #[error("failed to delete secret: {0}")]
    DeleteFailed(String),
}
