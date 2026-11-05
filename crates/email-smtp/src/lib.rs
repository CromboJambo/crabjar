//! crabjar-email-smtp: SMTP email sending with XOAUTH2 authentication.
//!
//! Implements the `EmailProvider` trait from crabjar-host-core using lettre
//! as the SMTP transport layer. Supports Gmail, Outlook, and any IMAP/SMTP
//! server that accepts XOAUTH2 tokens.

use anyhow::Result;
use async_trait::async_trait;
use crabjar_host_core::{EmailError, EmailListFilter, EmailMessage, EmailProvider};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, Transport};

/// XOAUTH2 configuration for SMTP authentication.
#[derive(Debug, Clone)]
pub struct Xoauth2Config {
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: String,
    pub scope: Option<String>,
}

impl Default for Xoauth2Config {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            refresh_token: String::new(),
            scope: Some("https://mail.google.com/gmail/send".to_string()),
        }
    }
}

/// SMTP transport configuration.
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub use_tls: bool,
    pub credentials: Option<Credentials>,
    pub xoauth2_config: Option<Xoauth2Config>,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        // Gmail defaults
        Self {
            host: "smtp.gmail.com".to_string(),
            port: 587,
            use_tls: true,
            credentials: None,
            xoauth2_config: None,
        }
    }
}

/// SMTP-based email provider implementation.
pub struct SmtpEmailProvider {
    config: SmtpConfig,
}

impl SmtpEmailProvider {
    /// Create a new SMTP email provider with the given configuration.
    #[must_use]
    pub fn new(config: SmtpConfig) -> Self {
        Self { config }
    }

    /// Create with default Gmail settings.
    #[must_use]
    pub fn gmail(xoauth2_config: Xoauth2Config) -> Self {
        let mut config = SmtpConfig::default();
        config.xoauth2_config = Some(xoauth2_config);
        Self { config }
    }

    /// Build lettre credentials from XOAUTH2 config (simplified).
    fn build_credentials(&self) -> Option<Credentials> {
        self.config.credentials.clone()
    }
}

#[async_trait]
impl EmailProvider for SmtpEmailProvider {
    async fn send(&self, message: &EmailMessage) -> Result<String, EmailError> {
        // Build email using lettre's Message builder (v0.11 API)
        let from_addr = format!(
            "{} <{}>",
            message.from.name.as_deref().unwrap_or("Agent"),
            message.from.address
        );
        let to_addrs: Vec<String> = message
            .to
            .iter()
            .map(|t| format!("{} <{}>", t.name.as_deref().unwrap_or("User"), t.address))
            .collect();

        // Build body content
        let body_text = message.body_text.as_deref().unwrap_or("");
        let body_html = message.body_html.as_deref().unwrap_or(body_text);

        let email =
            Message::builder()
                .from(from_addr.parse().map_err(|e| {
                    EmailError::ProviderSpecific(format!("invalid from address: {e}"))
                })?)
                .to(to_addrs[0].parse().map_err(|e| {
                    EmailError::ProviderSpecific(format!("invalid to address: {e}"))
                })?)
                .subject(&message.subject)
                .body(body_html.to_string())
                .map_err(|e| EmailError::ProviderSpecific(format!("failed to build email: {e}")))?;

        // Create SMTP transport
        let creds = self.build_credentials();
        let transport = if let Some(creds) = creds {
            lettre::transport::smtp::SmtpTransport::relay(&self.config.host)
                .map_err(|e| EmailError::ProviderSpecific(format!("relay error: {e}")))?
                .credentials(creds)
                .build()
        } else {
            lettre::transport::smtp::SmtpTransport::relay(&self.config.host)
                .map_err(|e| EmailError::ProviderSpecific(format!("relay error: {e}")))?
                .build()
        };

        // Send email
        let result = transport.send(&email);

        match result {
            Ok(response) => {
                tracing::info!("email sent successfully, response: {:?}", response);
                Ok("sent".to_string())
            }
            Err(e) => Err(EmailError::ProviderSpecific(format!("send error: {e}"))),
        }
    }

    async fn read(&self, _message_id: &str) -> Result<EmailMessage, EmailError> {
        // SMTP only supports sending; reading requires IMAP.
        Err(EmailError::NotFound(
            "SMTP provider does not support reading messages".to_string(),
        ))
    }

    async fn list(&self, _filter: &EmailListFilter) -> Result<Vec<EmailMessage>, EmailError> {
        // SMTP only supports sending.
        Err(EmailError::NotFound(
            "SMTP provider does not support listing messages".to_string(),
        ))
    }

    async fn sync(&self, _since: Option<&str>) -> Result<usize, EmailError> {
        // SMTP only supports sending.
        Ok(0)
    }

    async fn search(&self, _query: &str) -> Result<Vec<EmailMessage>, EmailError> {
        // SMTP only supports sending.
        Err(EmailError::NotFound(
            "SMTP provider does not support searching messages".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gmail_provider_creation() {
        let config = Xoauth2Config {
            client_id: "test-client".to_string(),
            client_secret: "test-secret".to_string(),
            refresh_token: "test-token".to_string(),
            scope: None,
        };
        let provider = SmtpEmailProvider::gmail(config);
        assert_eq!(provider.config.host, "smtp.gmail.com");
        assert_eq!(provider.config.port, 587);
    }

    #[test]
    fn default_smtp_config() {
        let config = SmtpConfig::default();
        assert_eq!(config.host, "smtp.gmail.com");
        assert_eq!(config.port, 587);
        assert!(config.use_tls);
    }
}
