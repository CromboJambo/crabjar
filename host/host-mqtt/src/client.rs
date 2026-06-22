/// MQTT broker client with connection management, publishing, and subscription.
///
/// Handles the MQTT lifecycle: connect, publish, subscribe, reconnect, LWT.
/// Mirrors the Electron app's mqtt.js client behavior.

use crate::config::MqttConfig;
use rumqttc::{
    AsyncClient, Event, EventLoop, Incoming, MqttOptions, QoS,
    Transport,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use crate::MqttEvent;

/// The MQTT client for Teams status publishing and command reception.
pub struct MqttClient {
    config: MqttConfig,
    /// MqttOptions stored for cloning in start().
    mqtt_options: MqttOptions,
    /// AsyncClient for publish/subscribe operations.
    client: Arc<Mutex<AsyncClient>>,
    /// Channel to forward MQTT events to subscribers.
    event_tx: Arc<tokio::sync::broadcast::Sender<MqttEvent>>,
}

impl MqttClient {
    /// Create a new MQTT client from config.
    ///
    /// Does NOT start the client or spawn any tasks. Call `start()` to begin.
    pub fn new(config: MqttConfig) -> Self {
        let mut mqttoptions = MqttOptions::new(
            &config.client_id,
            Self::broker_host(&config.broker_url),
            Self::broker_port(&config.broker_url),
        );

        // Configure TLS if using mqtts://
        if config.broker_url.starts_with("mqtts://") {
            mqttoptions.set_transport(Transport::tls_with_default_config());
        }

        // Keep-alive interval (5s like mqtt.js default)
        mqttoptions.set_keep_alive(std::time::Duration::from_secs(5));

        // Last Will and Testament for connection state
        let lwt_topic = format!("{}/connected", config.topic_prefix);
        let lwt_payload = "false";
        mqttoptions.set_last_will(
            rumqttc::LastWill::new(&lwt_topic, lwt_payload, QoS::AtLeastOnce, false),
        );

        // Credentials if provided
        if !config.username.is_empty() {
            mqttoptions.set_credentials(&config.username, &config.password);
        }

        // Clone MqttOptions so we can store it for start() and still pass
        // the original to AsyncClient::new (which consumes it).
        let mqtt_options_clone = mqttoptions.clone();
        let (client, _event_loop) = AsyncClient::new(mqttoptions, 64);

        Self {
            config,
            mqtt_options: mqtt_options_clone,
            client: Arc::new(Mutex::new(client)),
            event_tx: Arc::new(tokio::sync::broadcast::channel(64).0),
        }
    }

    /// Start the MQTT client. Spawns the event loop polling task and publishes LWT.
    pub async fn start(&self) -> Result<(), MqttError> {
        let (client, event_loop) = AsyncClient::new(self.mqtt_options.clone(), 64);

        // Store the client
        {
            let mut c = self.client.lock().await;
            *c = client;
        }

        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            let mut el: EventLoop = event_loop;
            loop {
                match el.poll().await {
                    Ok(Event::Outgoing(_)) => {}
                    Ok(Event::Incoming(Incoming::ConnAck(_ack))) => {
                        info!("MQTT connected");
                        let _ = event_tx.send(MqttEvent::Connected);
                    }
                    Ok(Event::Incoming(Incoming::SubAck(_mid))) => {
                        debug!("MQTT subscription acknowledged");
                    }
                    Ok(Event::Incoming(Incoming::Publish(publish))) => {
                        let topic = publish.topic.clone();
                        let payload = String::from_utf8_lossy(&publish.payload).to_string();
                        debug!(topic, payload, "MQTT received");
                        let _ = event_tx.send(MqttEvent::CommandReceived {
                            action: format!("mqtt://{}", topic),
                            request_id: None,
                        });
                    }
                    Ok(Event::Incoming(Incoming::PingReq)) => {
                        debug!("MQTT ping request");
                    }
                    Ok(Event::Incoming(Incoming::PingResp)) => {
                        debug!("MQTT ping response");
                    }
                    Ok(Event::Incoming(Incoming::PubAck(_mid))) => {
                        debug!("MQTT publish acknowledged");
                    }
                    Ok(Event::Incoming(Incoming::PubRec(_mid))) => {
                        debug!("MQTT pubrec received");
                    }
                    Ok(Event::Incoming(Incoming::PubRel(_mid))) => {
                        debug!("MQTT pubrel received");
                    }
                    Ok(Event::Incoming(Incoming::PubComp(_mid))) => {
                        debug!("MQTT pubcomp received");
                    }
                    Ok(Event::Incoming(Incoming::Disconnect)) => {
                        debug!("MQTT disconnected");
                        let _ = event_tx.send(MqttEvent::Disconnected);
                    }
                    Ok(Event::Incoming(_)) => {}
                    Err(e) => {
                        error!(?e, "MQTT event loop error");
                    }
                }
            }
        });

        // Publish LWT "true" to indicate connected
        let connected_topic = format!("{}/connected", self.config.topic_prefix);
        self.publish(&connected_topic, "true", QoS::AtLeastOnce, true)
            .await?;

        info!(
            broker = %self.config.broker_url,
            client_id = %self.config.client_id,
            "MQTT client started"
        );

        Ok(())
    }

    /// Stop the MQTT client and publish LWT "false".
    pub async fn stop(&self) -> Result<(), MqttError> {
        let connected_topic = format!("{}/connected", self.config.topic_prefix);
        let _ = self
            .publish(&connected_topic, "false", QoS::AtLeastOnce, true)
            .await;

        let client = self.client.lock().await;
        client.disconnect().await.map_err(|e| {
            MqttError::Connection(format!("disconnect failed: {}", e))
        })?;

        info!("MQTT client stopped");
        Ok(())
    }

    /// Publish a message to a topic.
    pub async fn publish(
        &self,
        topic: &str,
        payload: &str,
        qos: QoS,
        retain: bool,
    ) -> Result<(), MqttError> {
        let client = self.client.lock().await;
        client
            .publish(topic, qos, retain, payload.as_bytes().to_vec())
            .await
            .map_err(|e| MqttError::Publish(format!("{}", e)))
    }

    /// Subscribe to a topic for command reception.
    pub async fn subscribe(&self, topic: &str, qos: QoS) -> Result<(), MqttError> {
        let client = self.client.lock().await;
        client
            .subscribe(topic, qos)
            .await
            .map_err(|e| MqttError::Subscribe(format!("{}", e)))
    }

    /// Unsubscribe from a topic.
    pub async fn unsubscribe(&self, topic: &str) -> Result<(), MqttError> {
        let client = self.client.lock().await;
        client
            .unsubscribe(topic)
            .await
            .map_err(|e| MqttError::Subscribe(format!("unsubscribe failed: {}", e)))
    }

    /// Get the event receiver for MQTT events.
    pub fn event_rx(&self) -> tokio::sync::broadcast::Receiver<MqttEvent> {
        self.event_tx.subscribe()
    }

    /// Get the topic prefix.
    pub fn topic_prefix(&self) -> &str {
        &self.config.topic_prefix
    }

    /// Get the status topic.
    pub fn status_topic(&self) -> String {
        format!("{}/{}", self.config.topic_prefix, self.config.status_topic)
    }

    /// Get the command topic.
    pub fn command_topic(&self) -> Option<String> {
        if self.config.command_topic.is_empty() {
            None
        } else {
            Some(format!("{}/{}", self.config.topic_prefix, self.config.command_topic))
        }
    }

    /// Check if MQTT is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Broker host from URL.
    fn broker_host(url: &str) -> String {
        url.strip_prefix("mqtt://")
            .or_else(|| url.strip_prefix("mqtts://"))
            .unwrap_or(url)
            .split(':')
            .next()
            .unwrap_or("localhost")
            .to_string()
    }

    /// Broker port from URL.
    fn broker_port(url: &str) -> u16 {
        let host = url.strip_prefix("mqtt://").or_else(|| url.strip_prefix("mqtts://")).unwrap_or(url);
        host.split(':').nth(1).and_then(|p| p.parse().ok()).unwrap_or(1883)
    }
}

/// MQTT errors.
#[derive(thiserror::Error, Debug)]
pub enum MqttError {
    #[error("connection failed: {0}")]
    Connection(String),
    #[error("publish failed: {0}")]
    Publish(String),
    #[error("subscribe failed: {0}")]
    Subscribe(String),
    #[error("broker unreachable: {0}")]
    BrokerUnreachable(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MqttConfig;

    #[test]
    fn test_broker_host_parsing() {
        assert_eq!(MqttClient::broker_host("mqtt://localhost:1883"), "localhost");
        assert_eq!(MqttClient::broker_host("mqtt://192.168.1.100"), "192.168.1.100");
        assert_eq!(MqttClient::broker_host("mqtts://broker.example.com:8883"), "broker.example.com");
        assert_eq!(MqttClient::broker_host("localhost"), "localhost");
    }

    #[test]
    fn test_broker_port_parsing() {
        assert_eq!(MqttClient::broker_port("mqtt://localhost:1883"), 1883);
        assert_eq!(MqttClient::broker_port("mqtt://192.168.1.100:1883"), 1883);
        assert_eq!(MqttClient::broker_port("mqtts://broker:8883"), 8883);
        assert_eq!(MqttClient::broker_port("localhost"), 1883);
    }

    #[test]
    fn test_new_client_defaults() {
        let config = MqttConfig::default();
        let client = MqttClient::new(config);
        assert!(!client.is_enabled());
    }

    #[test]
    fn test_enabled_config() {
        let config = MqttConfig {
            enabled: true,
            broker_url: "mqtt://localhost:1883".into(),
            ..MqttConfig::default()
        };
        let client = MqttClient::new(config);
        assert!(client.is_enabled());
        assert_eq!(client.status_topic(), "teams/status");
        assert!(client.command_topic().is_none());
    }

    #[test]
    fn test_command_topic_with_config() {
        let config = MqttConfig {
            enabled: true,
            broker_url: "mqtt://localhost:1883".into(),
            command_topic: "command".into(),
            ..MqttConfig::default()
        };
        let client = MqttClient::new(config);
        assert_eq!(
            client.command_topic(),
            Some("teams/command".to_string())
        );
    }
}
