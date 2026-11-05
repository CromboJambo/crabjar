//! Home Assistant REST API client and tools for crabjar execution pipeline.
//!
//! Provides four Hermes-compatible tools:
//! - `ha_get_state(entity_id)` — get entity state and attributes
//! - `ha_call_service(domain, service, data)` — call an HA service
//! - `ha_list_entities(domain)` — list entities by domain
//! - `ha_list_services()` — list available services

use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::env;

/// Home Assistant REST API client.
pub struct HaClient {
    base_url: String,
    token: String,
}

impl HaClient {
    /// Create a new HA client from environment variables or defaults.
    pub fn new() -> Result<Self> {
        let base_url =
            env::var("CRABJAR_HA_URL").unwrap_or_else(|_| "http://localhost:8123".to_string());
        let token = env::var("CRABJAR_HA_TOKEN")
            .ok()
            .ok_or_else(|| anyhow!("CRABJAR_HA_TOKEN environment variable not set"))?;

        Ok(Self { base_url, token })
    }

    /// Create a new HA client with explicit URL and token (for testing).
    pub fn with_config(base_url: &str, token: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            token: token.to_string(),
        }
    }

    async fn request(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);

        let mut req = match method {
            "GET" => reqwest::Client::new().get(&url),
            "POST" => reqwest::Client::new().post(&url),
            _ => return Err(anyhow!("Unsupported HTTP method: {}", method)),
        };

        req = req.header("Authorization", format!("Bearer {}", self.token));

        if let Some(body) = body {
            req = req.json(body);
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("HA API error {}: {}", status, text));
        }

        let body: Value = response.json().await?;
        Ok(body)
    }

    /// Get state of an entity.
    pub async fn get_state(&self, entity_id: &str) -> Result<Value> {
        let path = format!("/api/states/{}", urlencoding::encode(entity_id));
        self.request("GET", &path, None).await
    }

    /// Call a service.
    pub async fn call_service(
        &self,
        domain: &str,
        service: &str,
        data: Option<&Value>,
    ) -> Result<()> {
        let path = format!("/api/services/{}/{}", domain, service);
        let _body = json!({});

        if let Some(data) = data {
            // Merge data into the request body
            let mut merged = serde_json::Map::new();
            merged.insert("entity_id".to_string(), Value::Null);

            if let Value::Object(obj) = data {
                for (k, v) in obj {
                    merged.insert(k.clone(), v.clone());
                }
            }

            self.request("POST", &path, Some(&Value::Object(merged)))
                .await?;
        } else {
            self.request("POST", &path, None).await?;
        }

        Ok(())
    }

    /// List entities in a domain.
    pub async fn list_entities(&self, domain: Option<&str>) -> Result<Vec<String>> {
        let path = "/api/states";
        let all_states: Value = self.request("GET", path, None).await?;

        if !all_states.is_array() {
            return Err(anyhow!("Expected array of states"));
        }

        let mut entities = Vec::new();
        for state in all_states.as_array().unwrap() {
            if let Some(entity_id) = state.get("entity_id").and_then(Value::as_str) {
                if let Some(domain_filter) = domain {
                    if entity_id.starts_with(&format!("{}.", domain_filter)) {
                        entities.push(entity_id.to_string());
                    }
                } else {
                    entities.push(entity_id.to_string());
                }
            }
        }

        Ok(entities)
    }

    /// List available services.
    pub async fn list_services(&self) -> Result<Value> {
        let path = "/api/services";
        self.request("GET", path, None).await
    }
}

/// Tool: ha_get_state(entity_id)
pub async fn tool_ha_get_state(args: &Value) -> Result<Value> {
    let entity_id = args
        .get("entity_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Missing required parameter: entity_id"))?;

    let client = HaClient::new()?;
    let state = client.get_state(entity_id).await?;

    Ok(json!({
        "success": true,
        "entity_id": entity_id,
        "state": state.get("state").cloned().unwrap_or(Value::Null),
        "attributes": state.get("attributes").cloned().unwrap_or(json!({})),
        "last_updated": state.get("last_updated").cloned().unwrap_or(Value::Null),
        "last_changed": state.get("last_changed").cloned().unwrap_or(Value::Null)
    }))
}

/// Tool: ha_call_service(domain, service, data)
pub async fn tool_ha_call_service(args: &Value) -> Result<Value> {
    let domain = args
        .get("domain")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Missing required parameter: domain"))?;

    let service = args
        .get("service")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Missing required parameter: service"))?;

    let data = args.get("data");

    let client = HaClient::new()?;
    client.call_service(domain, service, data).await?;

    Ok(json!({
        "success": true,
        "domain": domain,
        "service": service
    }))
}

/// Tool: ha_list_entities(domain)
pub async fn tool_ha_list_entities(args: &Value) -> Result<Value> {
    let domain = args.get("domain").and_then(Value::as_str);

    let client = HaClient::new()?;
    let entities = client.list_entities(domain).await?;

    Ok(json!({
        "success": true,
        "entities": entities
    }))
}

/// Tool: ha_list_services()
pub async fn tool_ha_list_services(_args: &Value) -> Result<Value> {
    let client = HaClient::new()?;
    let services = client.list_services().await?;

    Ok(json!({
        "success": true,
        "services": services
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ha_client_new() {
        unsafe {
            std::env::set_var("CRABJAR_HA_URL", "http://test:8123");
            std::env::set_var("CRABJAR_HA_TOKEN", "test_token");
        }

        let client = HaClient::new().unwrap();
        assert_eq!(client.base_url, "http://test:8123");
        assert_eq!(client.token, "test_token");
    }

    #[test]
    fn test_ha_client_with_config() {
        let client = HaClient::with_config("http://custom:9000", "custom_token");
        assert_eq!(client.base_url, "http://custom:9000");
        assert_eq!(client.token, "custom_token");
    }

    #[tokio::test]
    async fn test_tool_ha_get_state_missing_entity_id() {
        let args = json!({});
        let result = tool_ha_get_state(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("entity_id"));
    }

    #[tokio::test]
    async fn test_tool_ha_call_service_missing_domain() {
        let args = json!({"service": "turn_on"});
        let result = tool_ha_call_service(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("domain"));
    }
}
