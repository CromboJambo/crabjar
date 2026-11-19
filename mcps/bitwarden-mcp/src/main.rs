//! Bitwarden Secrets Manager MCP Server

mod types;
use types::*;
//! Direct SDK integration with the bitwarden crate — no subprocess overhead.
//!
//! Auth: reads bws config (~/.config/bws/config) for server URLs and uses
//! the most recent state file from ~/.config/bws/state/ for token auth.
//! Override with BW_API_URL, BW_IDENTITY_URL, BW_STATE_FILE, BW_ORG_ID env vars.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use bitwarden::secrets_manager::projects::ProjectsListRequest;
use bitwarden::secrets_manager::secrets::{
    SecretCreateRequest, SecretGetRequest, SecretIdentifiersByProjectRequest,
    SecretIdentifiersRequest, SecretPutRequest, SecretsDeleteRequest,
};
use bitwarden::secrets_manager::{AccessTokenLoginRequest, ClientSettings, DeviceType};
use bitwarden::Client;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use serde_json::json;

/// Configuration loaded from bws config file + env overrides.

/// Lazily-initialized Bitwarden client.
struct BwClient {
    inner: tokio::sync::OnceCell<Client>,
    config: Arc<BwConfig>,
}

impl std::fmt::Debug for BwClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BwClient")
            .field("api_url", &self.config.api_url)
            .finish()
    }
}

impl BwClient {
    fn new(config: Arc<BwConfig>) -> Self {
        Self {
            inner: tokio::sync::OnceCell::new(),
            config,
        }
    }

    async fn get(&self) -> Result<&Client> {
        self.inner
            .get_or_try_init(|| async {
                let access_token = self.config.access_token.clone().ok_or_else(|| {
                    anyhow::anyhow!(
                        "No Bitwarden access token found. Set BW_ACCESS_TOKEN or BWS_ACCESS_TOKEN env var. \
                         Create a service account in your Bitwarden organization to get one."
                    )
                })?;

                let settings = ClientSettings {
                    identity_url: self.config.identity_url.clone(),
                    api_url: self.config.api_url.clone(),
                    user_agent: "bitwarden-mcp/0.1".to_string(),
                    device_type: DeviceType::SDK,
                    device_identifier: None,
                    bitwarden_package_type: None,
                    bitwarden_client_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                };

                let client = Client::new(Some(settings));

                let token = AccessTokenLoginRequest {
                    access_token,
                    state_file: None,
                };

                client
                    .auth()
                    .login_access_token(&token)
                    .await
                    .context("Failed to authenticate with Bitwarden")?;

                Ok(client)
            })
            .await
    }

    fn resolve_org_id(&self, provided: Option<&str>) -> Result<String> {
        let org = provided
            .or(self.config.org_id.as_deref())
            .context(
                "No organization ID provided. Pass org_id to the tool or set BW_ORG_ID env var.",
            )?;
        Ok(org.to_string())
    }

    fn parse_org_uuid(&self, org_id: &str) -> Result<uuid::Uuid> {
        uuid::Uuid::parse_str(org_id).map_err(|e| anyhow::anyhow!("Invalid org UUID '{}': {}", org_id, e))
    }
}

#[derive(Debug, Clone)]
pub struct BitwardenMcpServer {
    client: Arc<BwClient>,
}

impl BitwardenMcpServer {
    fn new() -> Result<Self> {
        let config = BwConfig::load()?;

        // Fail-closed: refuse to start without a token. No tools exposed, no probing surface.
        if config.access_token.is_none() {
            anyhow::bail!(
                "No Bitwarden access token configured. Set BWS_ACCESS_TOKEN or BW_ACCESS_TOKEN \
                 environment variable with a service account token from your organization."
            );
        }

        Ok(Self {
            client: Arc::new(BwClient::new(Arc::new(config))),
        })
    }
}

#[tool_router(server_handler)]
impl BitwardenMcpServer {
    #[tool(description = "List all secrets in a Bitwarden organization. Optionally filter by project ID. Returns secret metadata (id, key, project_ids) — not values.")]
    async fn list_secrets(&self, params: Parameters<ListSecretsRequest>) -> Result<String, String> {
        let req = &params.0;
        let client = self.client.get().await.map_err(|e| e.to_string())?;

        let org_id = self
            .client
            .resolve_org_id(req.org_id.as_deref())
            .map_err(|e| e.to_string())?;
        let org_uuid = self.client.parse_org_uuid(&org_id).map_err(|e| e.to_string())?;

        // If project filter provided, list by project; otherwise list all in org
        let response = if let Some(project_id) = &req.project_id {
            let project_uuid = uuid::Uuid::parse_str(project_id)
                .map_err(|e| format!("Invalid project UUID: {}", e))?;
            client
                .secrets()
                .list_by_project(&SecretIdentifiersByProjectRequest {
                    project_id: project_uuid,
                })
                .await
                .map_err(|e| e.to_string())?
        } else {
            client
                .secrets()
                .list(&SecretIdentifiersRequest {
                    organization_id: org_uuid,
                })
                .await
                .map_err(|e| e.to_string())?
        };

        let secrets: Vec<serde_json::Value> = response
            .data
            .iter()
            .map(|s| {
                json!({
                    "id": s.id,
                    "key": s.key,
                    "organization_id": s.organization_id,
                    "project_ids": s.project_ids,
                })
            })
            .collect();

        serde_json::to_string_pretty(&json!({
            "success": true,
            "count": secrets.len(),
            "secrets": secrets,
        }))
        .map_err(|e| e.to_string())
    }

    #[tool(description = "Get a specific secret by ID. Returns the full secret including its decrypted value.")]
    async fn get_secret(&self, params: Parameters<GetSecretRequest>) -> Result<String, String> {
        let req = &params.0;
        let client = self.client.get().await.map_err(|e| e.to_string())?;

        // If given a key, find the secret ID first by listing
        let secret_id = if let Some(id) = &req.id {
            uuid::Uuid::parse_str(id).map_err(|e| format!("Invalid secret UUID: {}", e))?
        } else if let Some(key) = &req.key {
            let org_id = self.client.resolve_org_id(req.org_id.as_deref()).map_err(|e| e.to_string())?;
            let org_uuid = self.client.parse_org_uuid(&org_id).map_err(|e| e.to_string())?;

            let response = client
                .secrets()
                .list(&SecretIdentifiersRequest {
                    organization_id: org_uuid,
                })
                .await
                .map_err(|e| e.to_string())?;

            let secret = response
                .data
                .iter()
                .find(|s| s.key == *key)
                .ok_or_else(|| format!("Secret with key '{}' not found", key))?;

            secret.id
        } else {
            return Err("Either 'id' or 'key' must be provided".to_string());
        };

        let get_req = SecretGetRequest { id: secret_id };

        match client.secrets().get(&get_req).await {
            Ok(secret) => serde_json::to_string_pretty(&json!({
                "success": true,
                "id": secret.id,
                "key": secret.key,
                "value": secret.value,
                "note": secret.note,
                "project_id": secret.project_id,
                "organization_id": secret.organization_id,
                "creation_date": secret.creation_date.to_rfc3339(),
                "revision_date": secret.revision_date.to_rfc3339(),
            }))
            .map_err(|e| e.to_string()),
            Err(e) => Err(format!("Failed to get secret: {}", e)),
        }
    }

    #[tool(description = "Create a new secret in a Bitwarden project. Requires project_id, key, and value. Note is required (use empty string if not needed).")]
    async fn create_secret(&self, params: Parameters<CreateSecretRequest>) -> Result<String, String> {
        let req = &params.0;
        let client = self.client.get().await.map_err(|e| e.to_string())?;

        let org_id = self
            .client
            .resolve_org_id(req.org_id.as_deref())
            .map_err(|e| e.to_string())?;
        let org_uuid = self.client.parse_org_uuid(&org_id).map_err(|e| e.to_string())?;

        let project_uuids: Vec<uuid::Uuid> = req
            .project_ids
            .iter()
            .map(|p| uuid::Uuid::parse_str(p))
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Invalid project UUID: {}", e))?;

        let create_req = SecretCreateRequest {
            organization_id: org_uuid,
            key: req.key.clone(),
            value: req.value.clone(),
            note: req.note.clone().unwrap_or_default(),
            project_ids: Some(project_uuids),
        };

        match client.secrets().create(&create_req).await {
            Ok(secret) => serde_json::to_string_pretty(&json!({
                "success": true,
                "id": secret.id,
                "key": secret.key,
                "project_id": secret.project_id,
                "note": secret.note,
                "creation_date": secret.creation_date.to_rfc3339(),
            }))
            .map_err(|e| e.to_string()),
            Err(e) => Err(format!("Failed to create secret: {}", e)),
        }
    }

    #[tool(description = "Update an existing secret's key, value, and/or note. Requires the secret ID.")]
    async fn update_secret(&self, params: Parameters<UpdateSecretRequest>) -> Result<String, String> {
        let req = &params.0;
        let client = self.client.get().await.map_err(|e| e.to_string())?;

        let org_id = self
            .client
            .resolve_org_id(req.org_id.as_deref())
            .map_err(|e| e.to_string())?;
        let org_uuid = self.client.parse_org_uuid(&org_id).map_err(|e| e.to_string())?;

        let secret_uuid = uuid::Uuid::parse_str(&req.id)
            .map_err(|e| format!("Invalid secret UUID: {}", e))?;

        let project_uuids: Option<Vec<uuid::Uuid>> = match &req.project_ids {
            Some(ids) => Some(
                ids.iter()
                    .map(|p| uuid::Uuid::parse_str(p))
                    .collect::<Result<_, _>>()
                    .map_err(|e| format!("Invalid project UUID: {}", e))?,
            ),
            None => None,
        };

        let update_req = SecretPutRequest {
            id: secret_uuid,
            organization_id: org_uuid,
            key: req.key.clone(),
            value: req.value.clone(),
            note: req.note.clone().unwrap_or_default(),
            project_ids: project_uuids,
        };

        match client.secrets().update(&update_req).await {
            Ok(secret) => serde_json::to_string_pretty(&json!({
                "success": true,
                "id": secret.id,
                "key": secret.key,
                "revision_date": secret.revision_date.to_rfc3339(),
            }))
            .map_err(|e| e.to_string()),
            Err(e) => Err(format!("Failed to update secret: {}", e)),
        }
    }

    #[tool(description = "Delete one or more secrets by ID. This is irreversible.")]
    async fn delete_secret(&self, params: Parameters<DeleteSecretRequest>) -> Result<String, String> {
        let req = &params.0;

        let secret_uuids: Vec<uuid::Uuid> = req
            .ids
            .iter()
            .map(|id| uuid::Uuid::parse_str(id))
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Invalid secret UUID: {}", e))?;

        let client = self.client.get().await.map_err(|e| e.to_string())?;

        let delete_req = SecretsDeleteRequest {
            ids: secret_uuids,
        };

        match client.secrets().delete(delete_req).await {
            Ok(response) => serde_json::to_string_pretty(&json!({
                "success": true,
                "deleted_count": response.data.len(),
                "deleted": response.data.iter().map(|d| d.id.to_string()).collect::<Vec<_>>(),
            }))
            .map_err(|e| e.to_string()),
            Err(e) => Err(format!("Failed to delete secret(s): {}", e)),
        }
    }

    #[tool(description = "List all projects in a Bitwarden organization. Useful for discovering project IDs before creating secrets.")]
    async fn list_projects(&self, params: Parameters<OrgRequest>) -> Result<String, String> {
        let req = &params.0;
        let client = self.client.get().await.map_err(|e| e.to_string())?;

        let org_id = self
            .client
            .resolve_org_id(req.org_id.as_deref())
            .map_err(|e| e.to_string())?;
        let org_uuid = self.client.parse_org_uuid(&org_id).map_err(|e| e.to_string())?;

        match client
            .projects()
            .list(&ProjectsListRequest {
                organization_id: org_uuid,
            })
            .await
        {
            Ok(response) => {
                let projects: Vec<serde_json::Value> = response
                    .data
                    .iter()
                    .map(|p| {
                        json!({
                            "id": p.id,
                            "name": p.name,
                            "organization_id": p.organization_id,
                            "creation_date": p.creation_date.to_rfc3339(),
                            "revision_date": p.revision_date.to_rfc3339(),
                        })
                    })
                    .collect();

                serde_json::to_string_pretty(&json!({
                    "success": true,
                    "count": projects.len(),
                    "projects": projects,
                }))
                .map_err(|e| e.to_string())
            }
            Err(e) => Err(format!("Failed to list projects: {}", e)),
        }
    }

    #[tool(description = "Check connectivity and authentication status with the Bitwarden server. Returns config info without exposing secrets.")]
    async fn status(&self, _params: Parameters<EmptyRequest>) -> Result<String, String> {
        let client = self.client.get().await.map_err(|e| e.to_string())?;

        // Check auth by seeing if we have an org associated with the token
        let org_id = client.get_access_token_organization();

        serde_json::to_string_pretty(&json!({
            "success": true,
            "authenticated": org_id.is_some(),
            "organization_id": org_id.map(|o| o.to_string()),
            "api_url": self.client.config.api_url,
            "identity_url": self.client.config.identity_url,
            "token_configured": self.client.config.access_token.is_some(),
        }))
        .map_err(|e| e.to_string())
    }
}

// Request types — use rmcp's re-exported schemars to avoid version conflicts

#[tokio::main]
async fn main() -> Result<()> {
    let server = BitwardenMcpServer::new()?;
    let token_status = if server.client.config.access_token.is_some() {
        "token set"
    } else {
        "NO TOKEN (set BW_ACCESS_TOKEN)"
    };
    eprintln!(
        "bitwarden-mcp starting (api={}, identity={}, auth={})",
        server.client.config.api_url,
        server.client.config.identity_url,
        token_status
    );
    let running = rmcp::serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;
    running.waiting().await?;
    Ok(())
}
