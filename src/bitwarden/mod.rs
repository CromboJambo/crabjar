// crabjar/src/bitwarden/mod.rs
// Bitwarden CLI integration for secure credential management

#[allow(dead_code)]
pub mod cli;
#[allow(dead_code)]
pub mod store;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum BitwardenError {
    #[error("bitwarden CLI not found: {0}")]
    CliNotFound(String),

    #[error("bitwarden CLI error: {0}")]
    CliError(String),

    #[error("bitwarden session error: {0}")]
    SessionError(String),

    #[error("bitwarden item not found: {0}")]
    NotFound(String),

    #[error("bitwarden JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("internal error: {0}")]
    Internal(String),
}

pub type BitwardenResult<T> = Result<T, BitwardenError>;

/// Bitwarden item representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitwardenItem {
    pub id: String,
    pub name: String,
    pub uri: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub notes: Option<String>,
    pub folder: Option<String>,
    pub collection: Option<String>,
    pub modified: String,
    pub deleted: bool,
}

/// Bitwarden login session state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct BitwardenSession {
    pub email: String,
    pub server: Option<String>,
    pub logged_in: bool,
    pub last_login: Option<String>,
}

impl BitwardenSession {
    #[allow(dead_code)]
    pub fn new(email: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            server: None,
            logged_in: false,
            last_login: None,
        }
    }
}
