//! Inference server discovery and management.
//!
//! Discovers and manages multiple GPT/OpenAI-compatible inference servers
//! for multi-agent routing. Supports llama.cpp, Unsloth Studio, LM Studio,
//! Ollama, and any other OpenAI-compatible API endpoint.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Known inference server endpoints to probe during discovery.
const KNOWN_ENDPOINTS: &[(&str, u16)] = &[
    ("localhost", 8080),   // llama.cpp default
    ("127.0.0.1", 8080),
    ("localhost", 8081),   // Alternative
    ("127.0.0.1", 8081),
    ("localhost", 5000),   // Unsloth Studio default
    ("127.0.0.1", 5000),
    ("localhost", 11434),  // Ollama default
    ("127.0.0.1", 11434),
];

/// Inference server capabilities detected via API probing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Supports OpenAI-compatible chat completions endpoint.
    pub openai_chat: bool,
    /// Supports tool calling (function calling).
    pub tool_calling: bool,
    /// Maximum context window size (tokens).
    pub max_context_tokens: Option<u32>,
    /// Available models (if discoverable).
    pub available_models: Vec<String>,
    /// Server identifier/name.
    pub name: String,
}

/// Discovered inference server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceServer {
    /// Unique identifier for this server instance.
    pub id: String,
    /// Base URL (e.g., "http://localhost:8080").
    pub url: String,
    /// API key if required (optional).
    #[serde(default)]
    pub api_key: Option<String>,
    /// Detected capabilities.
    #[serde(default)]
    pub capabilities: ServerCapabilities,
    /// Last health check timestamp (Unix epoch seconds).
    #[serde(default)]
    pub last_checked: u64,
    /// Whether the server is currently healthy/responsive.
    #[serde(default = "default_true")]
    pub healthy: bool,
    /// User-defined tags for routing (e.g., ["fast", "coding"]).
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_true() -> bool {
    true
}

/// Server registry that manages discovered inference servers.
pub struct ServerRegistry {
    servers: RwLock<HashMap<String, InferenceServer>>,
}

impl ServerRegistry {
    /// Create a new empty server registry.
    pub fn new() -> Self {
        Self {
            servers: RwLock::new(HashMap::new()),
        }
    }

    /// Discover inference servers on the local network.
    ///
    /// Probes known endpoints and attempts to detect capabilities.
    pub async fn discover(&self) -> Vec<InferenceServer> {
        let mut discovered = Vec::new();

        for (host, port) in KNOWN_ENDPOINTS {
            let url = format!("http://{}:{}", host, port);
            match Self::probe_server(&url).await {
                Ok(Some(server)) => {
                    // Check if already registered
                    let id = Self::server_id(&server.url);
                    {
                        let servers = self.servers.read().await;
                        if servers.contains_key(&id) {
                            continue; // Already known
                        }
                    }

                    discovered.push(server.clone());
                    self.register(server).await;
                }
                Ok(None) | Err(_) => {
                    // Server not available or no capabilities detected
                    tracing::debug!("No inference server at {}", url);
                }
            }
        }

        discovered
    }

    /// Register a known inference server.
    pub async fn register(&self, server: InferenceServer) {
        let id = Self::server_id(&server.url);
        let mut servers = self.servers.write().await;
        servers.insert(id, server);
    }

    /// Get all registered servers.
    pub async fn list(&self) -> Vec<InferenceServer> {
        let servers = self.servers.read().await;
        servers.values().cloned().collect()
    }

    /// Select the best server for a given request based on tags and health.
    pub async fn select_server(&self, required_tags: &[&str]) -> Option<InferenceServer> {
        let servers = self.servers.read().await;

        // Filter by required tags
        let candidates: Vec<_> = servers.values().filter(|s| {
            if !s.healthy {
                return false;
            }
            for tag in required_tags {
                if !s.tags.iter().any(|t| t == *tag) {
                    return false;
                }
            }
            true
        }).cloned().collect();

        // Return first healthy, tagged server (could be smarter with load balancing)
        candidates.into_iter().next()
    }

    /// Update health status for a server.
    pub async fn update_health(&self, url: &str, healthy: bool) {
        let id = Self::server_id(url);
        let mut servers = self.servers.write().await;
        if let Some(server) = servers.get_mut(&id) {
            server.healthy = healthy;
            server.last_checked = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }
    }

    fn server_id(url: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        format!("srv-{:x}", hasher.finish())
    }

    /// Probe a server URL to detect if it's running and its capabilities.
    async fn probe_server(url: &str) -> Result<Option<InferenceServer>, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();

        // Try OpenAI-compatible chat endpoint first
        let chat_url = format!("{}/v1/chat/completions", url);
        let has_openai_chat = match client.get(&chat_url).send().await {
            Ok(resp) => resp.status().is_client_error() || resp.status().is_server_error(),
            Err(_) => false,
        };

        if !has_openai_chat {
            return Ok(None);
        }

        // Try to get available models
        let models_url = format!("{}/v1/models", url);
        let mut available_models = Vec::new();
        match client.get(&models_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.json::<serde_json::Value>().await {
                    if let Some(model_list) = body["data"].as_array() {
                        for model in model_list {
                            if let Some(name) = model["id"].as_str() {
                                available_models.push(name.to_string());
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Try to detect max context from a test completion (best effort)
        let max_context = None; // Would require actual model info

        let server = InferenceServer {
            id: Self::server_id(url),
            url: url.to_string(),
            api_key: None,
            capabilities: ServerCapabilities {
                openai_chat: true,
                tool_calling: false, // Hard to detect without testing
                max_context_tokens: max_context,
                available_models,
                name: format!("server-{}", url.split(':').last().unwrap_or("unknown")),
            },
            last_checked: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            healthy: true,
            tags: vec![],
        };

        Ok(Some(server))
    }
}