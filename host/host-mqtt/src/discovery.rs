/// Home Assistant auto-discovery for CrabJar MQTT integration.
///
/// Publishes Home Assistant discovery configs for:
/// - Presence sensor (`available` / `busy` / `dnd` / `away`)
/// - Microphone sensor (`speaking` / `silent` / `muted` / `off`)
/// - Binary sensors (`in-call`, `screen-sharing`, `camera`)
/// - Buttons (`toggle-mute`, `toggle-video`, `toggle-hand-raise`)
///
/// Mirrors the Teams-for-Linux `homeAssistantDiscovery.js` pattern:
/// sensors for state, buttons for one-shot actions.
use serde_json::json;
use tracing::info;

use crate::config::HomeAssistantConfig;
use crate::MqttClient;

/// Home Assistant auto-discovery payload generator.
pub struct HaDiscovery {
    config: HomeAssistantConfig,
}

impl HaDiscovery {
    /// Create a new discovery config generator.
    pub fn new(config: HomeAssistantConfig) -> Self {
        Self { config }
    }

    /// Publish all Home Assistant discovery configs via the MQTT client.
    pub async fn publish_all(&self, client: &MqttClient) -> Result<(), crate::MqttError> {
        if !self.config.enabled {
            return Ok(());
        }

        let prefix = &self.config.discovery_prefix;
        let device_name = &self.config.device_name;
        let topic_prefix = client.topic_prefix();
        let device_id = format!("crabjar-{topic_prefix}");

        // Helper: build common device metadata
        let device = json!({
            "identifiers": [device_id.clone()],
            "name": device_name,
            "manufacturer": "CrabJar",
            "model": "Host Integration",
        });

        // Helper: build availability config
        let availability = json!({
            "topic": format!("{topic_prefix}/connected"),
            "payload_available": "true",
            "payload_not_available": "false",
        });

        // === Presence sensor ===
        let topic = format!("{prefix}/sensor/{device_id}/presence/config");
        let payload = json!({
            "name": "Presence",
            "unique_id": format!("{device_id}_presence"),
            "state_topic": format!("{topic_prefix}/status"),
            "value_template": "{{ value_json.status }}",
            "icon": "mdi:microsoft-teams",
            "availability": availability.clone(),
            "device": device.clone(),
        });
        client.publish(&topic, &payload.to_string(), rumqttc::QoS::AtLeastOnce, true).await?;
        info!("Published HA presence sensor config");

        // === Microphone sensor ===
        let topic = format!("{prefix}/sensor/{device_id}/microphone/config");
        let payload = json!({
            "name": "Microphone",
            "unique_id": format!("{device_id}_microphone"),
            "state_topic": format!("{topic_prefix}/microphone"),
            "icon": "mdi:microphone",
            "availability": availability.clone(),
            "device": device.clone(),
        });
        client.publish(&topic, &payload.to_string(), rumqttc::QoS::AtLeastOnce, true).await?;
        info!("Published HA microphone sensor config");

        // === In-call binary sensor ===
        let topic = format!("{prefix}/binary_sensor/{device_id}/in_call/config");
        let payload = json!({
            "name": "In Call",
            "unique_id": format!("{device_id}_in_call"),
            "state_topic": format!("{topic_prefix}/in-call"),
            "payload_on": "true",
            "payload_off": "false",
            "icon": "mdi:phone-in-talk",
            "availability": availability.clone(),
            "device": device.clone(),
        });
        client.publish(&topic, &payload.to_string(), rumqttc::QoS::AtLeastOnce, true).await?;
        info!("Published HA in-call binary sensor config");

        // === Screen-sharing binary sensor ===
        let topic = format!("{prefix}/binary_sensor/{device_id}/screen_sharing/config");
        let payload = json!({
            "name": "Screen Sharing",
            "unique_id": format!("{device_id}_screen_sharing"),
            "state_topic": format!("{topic_prefix}/screen-sharing"),
            "payload_on": "true",
            "payload_off": "false",
            "icon": "mdi:monitor-share",
            "availability": availability.clone(),
            "device": device.clone(),
        });
        client.publish(&topic, &payload.to_string(), rumqttc::QoS::AtLeastOnce, true).await?;
        info!("Published HA screen-sharing binary sensor config");

        // === Camera binary sensor ===
        let topic = format!("{prefix}/binary_sensor/{device_id}/camera/config");
        let payload = json!({
            "name": "Camera",
            "unique_id": format!("{device_id}_camera"),
            "state_topic": format!("{topic_prefix}/camera"),
            "payload_on": "true",
            "payload_off": "false",
            "icon": "mdi:camera",
            "availability": availability.clone(),
            "device": device.clone(),
        });
        client.publish(&topic, &payload.to_string(), rumqttc::QoS::AtLeastOnce, true).await?;
        info!("Published HA camera binary sensor config");

        // === Buttons (one-shot actions, not switches) ===
        // Mirrors the Electron app's button pattern: payload_press with action.
        let button_actions = [
            ("toggle_mute", "Toggle Mute", "toggle-mute", "mdi:microphone"),
            ("toggle_video", "Toggle Video", "toggle-video", "mdi:video"),
            ("toggle_hand_raise", "Toggle Hand Raise", "toggle-hand-raise", "mdi:hand-back-left"),
        ];

        for (object_id, name, action, icon) in button_actions {
            let topic = format!("{prefix}/button/{device_id}/{object_id}/config");
            let payload = json!({
                "name": name,
                "unique_id": format!("{device_id}_{object_id}"),
                "command_topic": format!("{topic_prefix}/command"),
                "payload_press": serde_json::json!({"action": action}).to_string(),
                "icon": format!("mdi:{icon}"),
                "availability": availability.clone(),
                "device": device.clone(),
            });
            client.publish(&topic, &payload.to_string(), rumqttc::QoS::AtLeastOnce, true).await?;
            info!("Published HA {name} button config");
        }

        // === Device registry entry ===
        let topic = format!("{prefix}/device_registry/{device_id}");
        let payload = json!({
            "identifiers": [device_id],
            "name": device_name,
            "manufacturer": "CrabJar",
            "model": "Host Integration",
            "sw_version": env!("CARGO_PKG_VERSION"),
        });
        client.publish(&topic, &payload.to_string(), rumqttc::QoS::AtLeastOnce, true).await?;
        info!("Published HA device registry entry");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MqttConfig;

    #[test]
    fn test_ha_discovery_creation() {
        let ha_config = HomeAssistantConfig {
            enabled: true,
            discovery_prefix: "homeassistant".into(),
            device_name: "Test Device".into(),
        };
        let discovery = HaDiscovery::new(ha_config);
        assert!(discovery.config.enabled);
    }

    #[test]
    fn test_default_ha_discovery() {
        let ha_config = HomeAssistantConfig::default();
        let discovery = HaDiscovery::new(ha_config);
        assert!(!discovery.config.enabled);
        assert_eq!(discovery.config.discovery_prefix, "homeassistant");
    }
}
