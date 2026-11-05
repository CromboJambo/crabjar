//! Home Assistant application-layer tools (P2-2 drop-in replacement blocker).
//!
//! Provides crabjar's `ha` CLI subcommands that interact with a Home Assistant
//! instance via its REST API. Configuration is read from environment variables:
//! - HA_URL: Home Assistant base URL (default: http://localhost:8123)
//! - HA_TOKEN: Long-lived access token
//!
//! Supported operations:
//! - Get entity state
//! - List entities (optionally filtered by domain)
//! - Call services on entities
//! - List available services

use serde::{Deserialize, Serialize};

const DEFAULT_HA_URL: &str = "http://localhost:8123";

#[derive(Debug, Deserialize)]
pub struct EntityState {
    pub entity_id: String,
    pub state: String,
    #[serde(default)]
    pub attributes: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct HaResult<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn ha_url() -> Result<String, String> {
    std::env::var("HA_URL")
        .map_err(|_| format!("HA_URL not set (default would be {})", DEFAULT_HA_URL))
}

fn ha_token() -> Result<String, String> {
    std::env::var("HA_TOKEN").map_err(|_| "HA_TOKEN not set".to_string())
}

/// Get the state of a single entity.
pub fn get_entity_state(entity_id: &str) -> Result<EntityState, String> {
    let url = ha_url()?;
    let token = ha_token()?;
    let endpoint = format!("{}/api/states/{}", url.trim_end_matches('/'), entity_id);

    let client = reqwest::blocking::Client::new();
    let response = client
        .get(&endpoint)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "HA API returned {} for entity {}: {}",
            response.status(),
            entity_id,
            response.text().unwrap_or_default()
        ));
    }

    let state: EntityState = response
        .json()
        .map_err(|e| format!("Failed to parse entity state JSON: {}", e))?;
    Ok(state)
}

/// List entities, optionally filtered by domain.
pub fn list_entities(domain: Option<&str>) -> Result<Vec<EntityState>, String> {
    let url = ha_url()?;
    let token = ha_token()?;
    let endpoint = format!("{}/api/states", url.trim_end_matches('/'));

    let client = reqwest::blocking::Client::new();
    let response = client
        .get(&endpoint)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "HA API returned {} for entity list: {}",
            response.status(),
            response.text().unwrap_or_default()
        ));
    }

    let entities: Vec<EntityState> = response
        .json()
        .map_err(|e| format!("Failed to parse entity list JSON: {}", e))?;

    match domain {
        Some(d) => Ok(entities
            .into_iter()
            .filter(|e| e.entity_id.starts_with(&format!("{}.", d)))
            .collect()),
        None => Ok(entities),
    }
}

/// Call a service on an entity.
pub fn call_service(
    domain: &str,
    service: &str,
    entity_id: Option<&str>,
    data: Option<&str>,
) -> Result<String, String> {
    let url = ha_url()?;
    let token = ha_token()?;
    let endpoint = format!(
        "{}/api/services/{}/{}",
        url.trim_end_matches('/'),
        domain,
        service
    );

    // Build service data payload
    let mut payload = serde_json::Map::new();
    if let Some(eid) = entity_id {
        payload.insert(
            "entity_id".to_string(),
            serde_json::Value::String(eid.to_string()),
        );
    }
    if let Some(d) = data {
        let extra: serde_json::Value = serde_json::from_str(d)
            .map_err(|e| format!("Failed to parse service data JSON: {}", e))?;
        for (k, v) in extra
            .as_object()
            .ok_or("Service data must be a JSON object")?
        {
            payload.insert(k.clone(), v.clone());
        }
    }

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", token))
        .json(&payload)
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "HA API returned {} for service call {}: {}: {}",
            response.status(),
            domain,
            service,
            response.text().unwrap_or_default()
        ));
    }

    Ok("Service call successful".to_string())
}

/// List available services (domains and their service names).
pub fn list_services() -> Result<serde_json::Value, String> {
    let url = ha_url()?;
    let token = ha_token()?;
    let endpoint = format!("{}/api/services", url.trim_end_matches('/'));

    let client = reqwest::blocking::Client::new();
    let response = client
        .get(&endpoint)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "HA API returned {} for service list: {}",
            response.status(),
            response.text().unwrap_or_default()
        ));
    }

    let services: serde_json::Value = response
        .json()
        .map_err(|e| format!("Failed to parse service list JSON: {}", e))?;
    Ok(services)
}
