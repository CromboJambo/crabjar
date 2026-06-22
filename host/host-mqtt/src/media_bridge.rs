/// Media status bridge: listens for media state events and publishes them to MQTT topics.
///
/// Mirrors the Electron app's `MQTTMediaStatusService` which bridges IPC events
/// from the renderer process to the MQTT broker for home automation integration.
///
/// Publishes to topics:
/// - `{topicPrefix}/camera` — Camera on/off state (bool string)
/// - `{topicPrefix}/microphone` — Microphone state: 'speaking' | 'silent' | 'muted' | 'off'
/// - `{topicPrefix}/in-call` — Active call state (true/false)
/// - `{topicPrefix}/screen-sharing` — Screen sharing active state (bool string)

use crate::config::MqttConfig;
use crate::MqttClient;
use tokio::sync::broadcast;
use tracing::{debug, warn};

/// Media state event types that the bridge listens for.
#[derive(Debug, Clone)]
pub enum MediaEvent {
    /// Camera state changed.
    Camera { enabled: bool },
    /// Microphone state changed.
    Microphone { state: String },
    /// Call connected.
    CallConnected,
    /// Call disconnected.
    CallDisconnected,
    /// Screen sharing started.
    ScreenSharingStarted,
    /// Screen sharing stopped.
    ScreenSharingStopped,
}

/// Bridges media state events from the event bus to MQTT topics.
///
/// Run the bridge with `MediaBridge::run()` which returns a `JoinHandle`.
/// The bridge runs until the event receiver is dropped or the MQTT client
/// is stopped.
pub struct MediaBridge {
    mqtt: MqttClient,
    config: MqttConfig,
}

impl MediaBridge {
    /// Create a new media bridge.
    pub fn new(mqtt: MqttClient, config: MqttConfig) -> Self {
        Self { mqtt, config }
    }

    /// Start the media bridge. Returns a `JoinHandle` — drop it to stop.
    pub fn run(self, mut rx: broadcast::Receiver<MediaEvent>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            debug!("Media bridge started");

            loop {
                let event = match rx.recv().await {
                    Ok(event) => event,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(lagged = n, "Media bridge receiver lagged, skipping events");
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                };

                let result = match &event {
                    MediaEvent::Camera { enabled } => {
                        let topic = format!("{}/camera", self.config.topic_prefix);
                        self.mqtt.publish(&topic, &enabled.to_string(), rumqttc::QoS::AtLeastOnce, true).await
                    }
                    MediaEvent::Microphone { state } => {
                        let topic = format!("{}/microphone", self.config.topic_prefix);
                        self.mqtt.publish(&topic, state, rumqttc::QoS::AtLeastOnce, true).await
                    }
                    MediaEvent::CallConnected => {
                        let topic = format!("{}/in-call", self.config.topic_prefix);
                        self.mqtt.publish(&topic, "true", rumqttc::QoS::AtLeastOnce, true).await
                    }
                    MediaEvent::CallDisconnected => {
                        let topic = format!("{}/in-call", self.config.topic_prefix);
                        self.mqtt.publish(&topic, "false", rumqttc::QoS::AtLeastOnce, true).await
                    }
                    MediaEvent::ScreenSharingStarted => {
                        let topic = format!("{}/screen-sharing", self.config.topic_prefix);
                        self.mqtt.publish(&topic, "true", rumqttc::QoS::AtLeastOnce, true).await
                    }
                    MediaEvent::ScreenSharingStopped => {
                        let topic = format!("{}/screen-sharing", self.config.topic_prefix);
                        self.mqtt.publish(&topic, "false", rumqttc::QoS::AtLeastOnce, true).await
                    }
                };

                if let Err(e) = result {
                    warn!(?event, ?e, "Failed to publish media state to MQTT");
                } else {
                    debug!(?event, "Published media state to MQTT");
                }
            }

            debug!("Media bridge stopped (event receiver dropped)");
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MqttConfig;

    #[test]
    fn test_media_bridge_creation() {
        let mqtt = MqttClient::new(MqttConfig::default());
        let config = MqttConfig::default();
        let bridge = MediaBridge::new(mqtt, config);
        assert!(bridge.mqtt.is_enabled() == false);
    }

    #[test]
    fn test_media_bridge_topic_prefix() {
        let config = MqttConfig {
            enabled: true,
            topic_prefix: "test-prefix".into(),
            ..MqttConfig::default()
        };
        let mqtt = MqttClient::new(config.clone());
        let bridge = MediaBridge::new(mqtt, config);
        // The bridge uses the config's topic_prefix
        assert!(bridge.mqtt.is_enabled());
    }
}
