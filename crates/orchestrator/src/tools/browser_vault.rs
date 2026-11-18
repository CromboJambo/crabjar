//! Browser vault tool — provides credential access during browser automation.
//!
//! Wraps `crabjar-browser-vault` to expose Bitwarden/Vaultwarden credentials
//! for handling login walls. Arguments are passed as a JSON string in args[0].

use crabjar_browser_vault::{BrowserVaultClient, VaultConfig};
use serde_json::json;
use std::sync::Arc;
use std::sync::OnceLock;
use tracing::info;

/// Cached vault client (created once per process).
static VAULT_CLIENT: OnceLock<Arc<BrowserVaultClient>> = OnceLock::new();

fn get_client() -> &'static Arc<BrowserVaultClient> {
    VAULT_CLIENT.get_or_init(|| {
        let config = VaultConfig::from_env();
        info!(cli = %config.cli_path, "Initializing browser vault client");
        Arc::new(BrowserVaultClient::new(config))
    })
}

/// Handle a browser_vault tool call.
///
/// Args:
///   - args[0]: JSON string with the request parameters
///     {"action": "list|get|search|generate", ...}
pub async fn handle(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Err("browser_vault requires a JSON argument".to_string());
    }

    let request: serde_json::Value = serde_json::from_str(&args[0])
        .map_err(|e| format!("Failed to parse browser_vault arguments as JSON: {}", e))?;

    let action = request
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required field: 'action'".to_string())?;

    let client = get_client();

    match action {
        "list" => handle_list(client, &request).await,
        "get" => handle_get(client, &request).await,
        "search" => handle_search(client, &request).await,
        "generate" => handle_generate(client, &request).await,
        _ => Err(format!(
            "Unknown action: '{}'. Valid actions: list, get, search, generate",
            action
        )),
    }
}

async fn handle_list(
    client: &Arc<BrowserVaultClient>,
    request: &serde_json::Value,
) -> Result<String, String> {
    let folder = request.get("folder").and_then(|v| v.as_str());
    let collection = request.get("collection").and_then(|v| v.as_str());

    let items = client
        .list_items(folder, collection)
        .await
        .map_err(|e| format!("Failed to list vault items: {}", e))?;

    // Return sanitized items (no passwords in list view)
    let results: Vec<serde_json::Value> = items
        .iter()
        .map(|item| {
            json!({
                "id": item.id,
                "name": item.name,
                "uri": item.uri,
                "username": item.username.as_ref().map(|u| u.to_string()),
                "folder": item.folder_name,
            })
        })
        .collect();

    let response = json!({
        "count": results.len(),
        "items": results
    });

    serde_json::to_string_pretty(&response)
        .map_err(|e| format!("Failed to serialize response: {}", e))
}

async fn handle_get(
    client: &Arc<BrowserVaultClient>,
    request: &serde_json::Value,
) -> Result<String, String> {
    let url = request.get("url").and_then(|v| v.as_str());
    let name = request.get("name").and_then(|v| v.as_str());
    let id = request.get("id").and_then(|v| v.as_str());

    let item = if let Some(id) = id {
        client
            .get_item(id)
            .await
            .map_err(|e| format!("Failed to get vault item by ID: {}", e))?
    } else if let Some(url) = url {
        client
            .get_by_url(url)
            .await
            .map_err(|e| format!("Failed to find credential for URL '{}': {}", url, e))?
    } else if let Some(name) = name {
        client
            .get_by_name(name)
            .await
            .map_err(|e| format!("Failed to find credential by name '{}': {}", name, e))?
    } else {
        return Err(
            "One of 'url', 'name', or 'id' is required for the 'get' action".to_string(),
        );
    };

    let response = json!({
        "id": item.id,
        "name": item.name,
        "uri": item.uri,
        "username": item.username,
        "password": item.password,
        "notes": item.notes,
        "folder": item.folder_name
    });

    serde_json::to_string_pretty(&response)
        .map_err(|e| format!("Failed to serialize response: {}", e))
}

async fn handle_search(
    client: &Arc<BrowserVaultClient>,
    request: &serde_json::Value,
) -> Result<String, String> {
    let query = request
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required field: 'query' for search action".to_string())?;

    let items = client
        .search_items(query)
        .await
        .map_err(|e| format!("Failed to search vault: {}", e))?;

    let results: Vec<serde_json::Value> = items
        .iter()
        .map(|item| {
            json!({
                "id": item.id,
                "name": item.name,
                "uri": item.uri,
                "username": item.username.as_ref().map(|u| u.to_string()),
                "folder": item.folder_name,
            })
        })
        .collect();

    let response = json!({
        "count": results.len(),
        "query": query,
        "items": results
    });

    serde_json::to_string_pretty(&response)
        .map_err(|e| format!("Failed to serialize response: {}", e))
}

async fn handle_generate(
    client: &Arc<BrowserVaultClient>,
    request: &serde_json::Value,
) -> Result<String, String> {
    let length = request
        .get("length")
        .and_then(|v| v.as_u64())
        .unwrap_or(24) as usize;
    let special = request.get("special").and_then(|v| v.as_bool()).unwrap_or(true);

    let password = client
        .generate_password(length, true, true, true, special)
        .await
        .map_err(|e| format!("Failed to generate password: {}", e))?;

    let response = json!({
        "password": password,
        "length": length
    });

    serde_json::to_string_pretty(&response)
        .map_err(|e| format!("Failed to serialize response: {}", e))
}
