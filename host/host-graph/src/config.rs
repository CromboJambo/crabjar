//! Configuration for the Graph API client.
use serde::{Deserialize, Serialize};

/// Graph API configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphApiConfig {
    /// Whether the Graph API client is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Base URL for the Graph API. Defaults to `https://graph.microsoft.com/v1.0`.
    #[serde(default = "default_base_url")]
    pub base_url: String,
}

impl GraphApiConfig {
    /// Create a new config with the given enabled flag.
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            base_url: default_base_url(),
        }
    }
}

impl Default for GraphApiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: default_base_url(),
        }
    }
}

fn default_base_url() -> String {
    "https://graph.microsoft.com/v1.0".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = GraphApiConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.base_url, "https://graph.microsoft.com/v1.0");
    }

    #[test]
    fn test_config_enabled() {
        let config = GraphApiConfig::new(true);
        assert!(config.enabled);
        assert_eq!(config.base_url, "https://graph.microsoft.com/v1.0");
    }

    #[test]
    fn test_config_serialization() {
        let config = GraphApiConfig {
            enabled: true,
            base_url: "https://graph.microsoft.com/beta".into(),
        };
        let toml = toml::to_string(&config).expect("serialize");
        let deserialized: GraphApiConfig = toml::from_str(&toml).expect("deserialize");
        assert!(deserialized.enabled);
        assert_eq!(deserialized.base_url, "https://graph.microsoft.com/beta");
    }

    #[test]
    fn test_config_deserialize_missing_enabled() {
        let toml = r#"base_url = "https://example.com""#;
        let config: GraphApiConfig = toml::from_str(toml).expect("deserialize");
        assert!(!config.enabled);
        assert_eq!(config.base_url, "https://example.com");
    }
}
