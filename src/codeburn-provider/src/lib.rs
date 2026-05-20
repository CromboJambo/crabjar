use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("provider not found: {0}")]
    NotFound(String),
    #[error("discovery error: {0}")]
    DiscoveryError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataFormat {
    Jsonl,
    Sqlite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRegistry {
    pub providers: Vec<SessionData>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self { providers: Vec::new() }
    }

    pub fn default() -> Self {
        Self::new()
    }

    pub fn discover(_path: &std::path::Path) -> Result<Self, ProviderError> {
        Ok(Self::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub provider_name: String,
    pub format: DataFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceEntry {
    pub source: String,
    pub provenance_id: String,
}
