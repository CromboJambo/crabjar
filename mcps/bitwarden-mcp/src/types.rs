//! Type definitions for bitwarden-mcp.

use std::path::PathBuf;
use anyhow::Result;

/// Configuration loaded from bws config file + env overrides.
#[derive(Debug, Clone)]
struct BwConfig {
    api_url: String,
    identity_url: String,
    access_token: Option<String>,
    org_id: Option<String>,
}

impl BwConfig {
    fn load() -> Result<Self> {
        let mut api_url = std::env::var("BW_API_URL")
            .unwrap_or_else(|_| "https://api.bitwarden.com".to_string());
        let mut identity_url = std::env::var("BW_IDENTITY_URL")
            .unwrap_or_else(|_| "https://identity.bitwarden.com".to_string());

        // Try loading bws config file for server URLs
        let bws_config_path = config_dir().join("bws").join("config");
        if let Ok(config_str) = std::fs::read_to_string(&bws_config_path) {
            if let Ok(parsed) = toml::from_str::<toml::Value>(&config_str) {
                // profiles.default or first profile
                let profile = parsed.get("profiles").and_then(|p| p.get("default")).or_else(|| {
                    parsed.get("profiles").and_then(|p| {
                        p.as_table()?.values().next()
                    })
                });

                if let Some(profile) = profile {
                    if api_url == "https://api.bitwarden.com" {
                        if let Some(base) = profile.get("server_base").and_then(|v| v.as_str()) {
                            api_url = format!("{}/api", base.trim_end_matches('/'));
                        } else if let Some(server_api) =
                            profile.get("server_api").and_then(|v| v.as_str())
                        {
                            api_url = server_api.to_string();
                        }
                    }
                    if identity_url == "https://identity.bitwarden.com" {
                        if let Some(base) = profile.get("server_base").and_then(|v| v.as_str()) {
                            identity_url = format!("{}/identity", base.trim_end_matches('/'));
                        } else if let Some(server_identity) =
                            profile.get("server_identity").and_then(|v| v.as_str())
                        {
                            identity_url = server_identity.to_string();
                        }
                    }
                }
            }
        }

        // Access token: BW_ACCESS_TOKEN or BWS_ACCESS_TOKEN env var
        let access_token = std::env::var("BW_ACCESS_TOKEN")
            .or_else(|_| std::env::var("BWS_ACCESS_TOKEN"))
            .ok();

        // Org ID from env (optional — can be passed per-call)
        let org_id = std::env::var("BW_ORG_ID").ok();

        Ok(Self {
            api_url,
            identity_url,
            access_token,
            org_id,
        })
    }
}

fn config_dir() -> PathBuf {
    // Check XDG_CONFIG_HOME first, but fall back to ~/.config if bws isn't there
    let home_config = std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(".config"))
        .unwrap_or_else(|_| PathBuf::from("/tmp"));

    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let xdg_path = PathBuf::from(xdg);
        if xdg_path.join("bws").exists() {
            return xdg_path;
        }
    }
    home_config
}

// Request types — use rmcp's re-exported schemars to avoid version conflicts

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EmptyRequest {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct OrgRequest {
    /// Organization ID (UUID). Falls back to BW_ORG_ID env var if not provided.
    #[schemars(description = "Organization ID (UUID). Falls back to BW_ORG_ID env var if not provided.")]
    pub org_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListSecretsRequest {
    /// Organization ID (UUID). Falls back to BW_ORG_ID env var if not provided.
    #[schemars(description = "Organization ID (UUID). Falls back to BW_ORG_ID env var if not provided.")]
    pub org_id: Option<String>,
    /// Optional project ID to filter secrets by.
    #[schemars(description = "Optional project ID to filter secrets by.")]
    pub project_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetSecretRequest {
    /// Secret ID (UUID). Either this or 'key' must be provided.
    #[schemars(description = "Secret ID (UUID). Either this or 'key' must be provided.")]
    pub id: Option<String>,
    /// Secret key name (e.g., "DATABASE_URL"). Either this or 'id' must be provided.
    #[schemars(description = "Secret key name (e.g., 'DATABASE_URL'). Either this or 'id' must be provided.")]
    pub key: Option<String>,
    /// Organization ID (UUID), required when looking up by key. Falls back to BW_ORG_ID env var.
    #[schemars(description = "Organization ID (UUID), required when looking up by key. Falls back to BW_ORG_ID env var.")]
    pub org_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateSecretRequest {
    /// Project IDs (UUIDs) to attach the secret to.
    #[schemars(description = "Project IDs (UUIDs) to attach the secret to.")]
    pub project_ids: Vec<String>,
    /// Secret key name (e.g., "DATABASE_URL"). Max 500 chars.
    #[schemars(description = "Secret key name (e.g., 'DATABASE_URL'). Max 500 chars.")]
    pub key: String,
    /// Secret value. Max 25000 chars.
    #[schemars(description = "Secret value. Max 25000 chars.")]
    pub value: String,
    /// Optional note/description. Max 7000 chars. Use empty string if not needed.
    #[schemars(description = "Optional note/description. Max 7000 chars.")]
    pub note: Option<String>,
    /// Organization ID (UUID). Falls back to BW_ORG_ID env var if not provided.
    #[schemars(description = "Organization ID (UUID). Falls back to BW_ORG_ID env var if not provided.")]
    pub org_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateSecretRequest {
    /// Secret ID (UUID) to update.
    #[schemars(description = "Secret ID (UUID) to update.")]
    pub id: String,
    /// New secret key name. Max 500 chars.
    #[schemars(description = "New secret key name. Max 500 chars.")]
    pub key: String,
    /// New secret value. Max 25000 chars.
    #[schemars(description = "New secret value. Max 25000 chars.")]
    pub value: String,
    /// New note/description. Max 7000 chars.
    #[schemars(description = "New note/description. Max 7000 chars.")]
    pub note: Option<String>,
    /// Project IDs to reassign (optional).
    #[schemars(description = "Project IDs to reassign (optional).")]
    pub project_ids: Option<Vec<String>>,
    /// Organization ID (UUID). Falls back to BW_ORG_ID env var if not provided.
    #[schemars(description = "Organization ID (UUID). Falls back to BW_ORG_ID env var if not provided.")]
    pub org_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DeleteSecretRequest {
    /// Secret IDs (UUIDs) to delete.
    #[schemars(description = "Secret IDs (UUIDs) to delete.")]
    pub ids: Vec<String>,
}

