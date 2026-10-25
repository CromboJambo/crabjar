//! Abstract email provider interface — inside the glass.
//!
//! Concrete implementations (SMTP clients, IMAP readers, REST API wrappers)
//! live outside the glass as separate integration crates. This trait defines
//! the stable contract that host-core and its dependents use.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Email address with optional display name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAddress {
    pub address: String,
    #[serde(default)]
    pub name: Option<String>,
}

impl From<&str> for EmailAddress {
    fn from(addr: &str) -> Self {
        Self {
            address: addr.to_string(),
            name: None,
        }
    }
}

/// A mail message to be sent or received.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMessage {
    pub id: String,
    pub from: EmailAddress,
    pub to: Vec<EmailAddress>,
    #[serde(default)]
    pub cc: Vec<EmailAddress>,
    #[serde(default)]
    pub bcc: Vec<EmailAddress>,
    pub subject: String,
    /// Plain text body (primary).
    pub body_text: Option<String>,
    /// HTML body (optional alternative).
    pub body_html: Option<String>,
    /// MIME type of attachments or inline content.
    #[serde(default)]
    pub attachments: Vec<EmailAttachment>,
    /// ISO 8601 timestamp when sent.
    #[serde(default)]
    pub date_sent: Option<String>,
}

/// An email attachment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAttachment {
    pub filename: String,
    pub content_type: String,
    /// Base64-encoded content or local file path (provider-specific).
    pub data: Vec<u8>,
}

/// Filter criteria for listing messages.
#[derive(Debug, Clone, Default)]
pub struct EmailListFilter {
    pub limit: Option<usize>,
    pub since: Option<String>,
    pub label: Option<String>,
    pub unread_only: bool,
}

/// Abstract email provider — inside the glass.
///
/// Implementations live outside the glass (email-smtp, email-imap, etc.).
/// This trait is the stable interface that host-core and dependents use.
#[async_trait]
pub trait EmailProvider: Send + Sync {
    /// Send an email message.
    async fn send(&self, message: &EmailMessage) -> Result<String, EmailError>;

    /// Read a single message by ID.
    async fn read(&self, message_id: &str) -> Result<EmailMessage, EmailError>;

    /// List messages matching filter criteria.
    async fn list(&self, filter: &EmailListFilter) -> Result<Vec<EmailMessage>, EmailError>;

    /// Synchronize local state with remote provider.
    async fn sync(&self, since: Option<&str>) -> Result<usize, EmailError>;

    /// Search messages by query string.
    async fn search(&self, query: &str) -> Result<Vec<EmailMessage>, EmailError>;
}

/// Error type for email operations.
#[derive(Debug)]
pub enum EmailError {
    AuthFailed(String),
    Network(String),
    NotFound(String),
    RateLimited(String),
    ProviderSpecific(String),
}

impl std::fmt::Display for EmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailError::AuthFailed(msg) => write!(f, "email auth failed: {}", msg),
            EmailError::Network(msg) => write!(f, "email network error: {}", msg),
            EmailError::NotFound(msg) => write!(f, "email not found: {}", msg),
            EmailError::RateLimited(msg) => write!(f, "email rate limited: {}", msg),
            EmailError::ProviderSpecific(msg) => write!(f, "email provider error: {}", msg),
        }
    }
}

impl std::error::Error for EmailError {}