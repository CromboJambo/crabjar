/// MQTT configuration.
///
/// Mirrors the Teams-for-Linux config.json mqtt section:
/// <https://github.com/IsmaelMartinez/teams-for-linux/blob/main/docs-site/docs/mqtt-integration.md>
use serde::{Deserialize, Serialize};

/// MQTT broker configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MqttConfig {
    /// Enable/disable MQTT integration (both publishing and commands)
    #[serde(default)]
    pub enabled: bool,

    /// MQTT broker URL (e.g., `mqtt://192.168.1.100:1883` or `mqtts://broker:8883`)
    #[serde(default)]
    pub broker_url: String,

    /// MQTT username (optional)
    #[serde(default)]
    pub username: String,

    /// MQTT password (optional)
    #[serde(default)]
    pub password: String,

    /// Unique client identifier
    #[serde(default = "default_client_id")]
    pub client_id: String,

    /// Topic prefix for all messages
    #[serde(default = "default_topic_prefix")]
    pub topic_prefix: String,

    /// Topic name for status messages (outbound)
    #[serde(default = "default_status_topic")]
    pub status_topic: String,

    /// Topic name for receiving commands (inbound).
    /// Leave empty or omit to disable command reception (status publishing only).
    #[serde(default)]
    pub command_topic: String,

    /// Polling fallback interval for status detection (milliseconds)
    #[serde(default = "default_status_check_interval")]
    pub status_check_interval: u64,

    /// Home Assistant auto-discovery configuration
    #[serde(default)]
    pub home_assistant: HomeAssistantConfig,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            broker_url: String::new(),
            username: String::new(),
            password: String::new(),
            client_id: default_client_id(),
            topic_prefix: default_topic_prefix(),
            status_topic: default_status_topic(),
            command_topic: String::new(),
            status_check_interval: default_status_check_interval(),
            home_assistant: HomeAssistantConfig::default(),
        }
    }
}

/// Home Assistant auto-discovery configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HomeAssistantConfig {
    /// Publish HA discovery configs on connect
    #[serde(default)]
    pub enabled: bool,

    /// Discovery topic prefix (must match HA's `discovery_prefix` setting)
    #[serde(default = "default_ha_discovery_prefix")]
    pub discovery_prefix: String,

    /// Device name shown in HA
    #[serde(default = "default_ha_device_name")]
    pub device_name: String,
}

impl Default for HomeAssistantConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            discovery_prefix: default_ha_discovery_prefix(),
            device_name: default_ha_device_name(),
        }
    }
}

fn default_client_id() -> String {
    "crabjar-host".into()
}

fn default_topic_prefix() -> String {
    "teams".into()
}

fn default_status_topic() -> String {
    "status".into()
}

fn default_status_check_interval() -> u64 {
    10_000
}

fn default_ha_discovery_prefix() -> String {
    "homeassistant".into()
}

fn default_ha_device_name() -> String {
    "Teams for Linux".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MqttConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.client_id, "crabjar-host");
        assert_eq!(config.topic_prefix, "teams");
        assert_eq!(config.status_topic, "status");
        assert!(config.command_topic.is_empty());
        assert_eq!(config.status_check_interval, 10_000);
    }

    #[test]
    fn test_default_ha_config() {
        let ha = HomeAssistantConfig::default();
        assert!(!ha.enabled);
        assert_eq!(ha.discovery_prefix, "homeassistant");
        assert_eq!(ha.device_name, "Teams for Linux");
    }

    #[test]
    fn test_config_roundtrip() {
        let config = MqttConfig {
            enabled: true,
            broker_url: "mqtt://localhost:1883".into(),
            username: "user".into(),
            password: "pass".into(),
            client_id: "test-client".into(),
            topic_prefix: "crabjar".into(),
            status_topic: "presence".into(),
            command_topic: "commands".into(),
            status_check_interval: 5_000,
            home_assistant: HomeAssistantConfig {
                enabled: true,
                discovery_prefix: "ha".into(),
                device_name: "CrabJar Host".into(),
            },
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let loaded: MqttConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(loaded.enabled, config.enabled);
        assert_eq!(loaded.broker_url, config.broker_url);
        assert_eq!(loaded.username, config.username);
        assert_eq!(loaded.client_id, config.client_id);
        assert_eq!(loaded.topic_prefix, config.topic_prefix);
        assert_eq!(loaded.status_topic, config.status_topic);
        assert_eq!(loaded.command_topic, config.command_topic);
        assert_eq!(loaded.status_check_interval, config.status_check_interval);
        assert_eq!(loaded.home_assistant.enabled, config.home_assistant.enabled);
        assert_eq!(
            loaded.home_assistant.discovery_prefix,
            config.home_assistant.discovery_prefix
        );
        assert_eq!(
            loaded.home_assistant.device_name,
            config.home_assistant.device_name
        );
    }
}
