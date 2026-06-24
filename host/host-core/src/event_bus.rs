/// Event bus for the CrabJar host runtime.
///
/// All subsystems communicate through this async pub/sub channel.
/// Events flow from system hooks, plugins, and the agent loop.
use std::fmt;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Types of events that can flow through the bus.
#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    /// System tray state changed (shown/hidden, icon updated)
    TrayChanged { action: String },
    /// Desktop notification delivered
    Notification { title: String, body: String },
    /// Clipboard content changed
    ClipboardChanged { mime_type: String },
    /// WebView lifecycle event
    WebView { event: String, url: Option<String> },
    /// Agent loop stage transition
    Agent { stage: String, work_item_id: String },
    /// Plugin lifecycle
    Plugin { event: String, plugin_id: String },
    /// Configuration reload
    ConfigReload { source: String },
    /// Application-specific (plugin-defined)
    App {
        app_id: String,
        event: String,
        data: serde_json::Value,
    },
    /// Heartbeat / timer tick
    Tick { interval_ms: u64 },
    /// User input (from Ratatui or GUI)
    UserInput { input: String },
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventType::TrayChanged { action } => write!(f, "tray:{action}"),
            EventType::Notification { title, .. } => write!(f, "notify:{title}"),
            EventType::ClipboardChanged { mime_type } => write!(f, "clipboard:{mime_type}"),
            EventType::WebView { event, .. } => write!(f, "webview:{event}"),
            EventType::Agent { stage, .. } => write!(f, "agent:{stage}"),
            EventType::Plugin { event, .. } => write!(f, "plugin:{event}"),
            EventType::ConfigReload { source } => write!(f, "config:{source}"),
            EventType::App { app_id, event, .. } => write!(f, "app:{app_id}:{event}"),
            EventType::Tick { interval_ms } => write!(f, "tick:{interval_ms}ms"),
            EventType::UserInput { .. } => write!(f, "input:<redacted>"),
        }
    }
}

/// A single event with metadata.
#[derive(Debug, Clone)]
pub struct Event {
    pub id: Uuid,
    pub kind: EventType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source: String,
}

impl Event {
    pub fn new(kind: EventType, source: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            timestamp: chrono::Utc::now(),
            source: source.into(),
        }
    }
}

/// Broadcast-based event bus.
///
/// Uses tokio::broadcast for fan-out to subscribers.
/// Publishers can optionally filter by EventType.
pub struct EventBus {
    sender: broadcast::Sender<Event>,
    max_capacity: usize,
}

impl EventBus {
    pub fn new(max_capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(max_capacity);
        Self {
            sender,
            max_capacity,
        }
    }

    pub fn subscriber(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// Publish an event to all subscribers.
    /// Returns the number of receivers that got the event.
    pub fn publish(&self, event: Event) -> Result<usize, Box<broadcast::error::SendError<Event>>> {
        tracing::debug!(event = %event.kind, source = %event.source, "event published");
        self.sender.send(event).map_err(Box::new)
    }

    /// Publish a typed event shorthand.
    pub fn publish_typed<T>(
        &self,
        kind: EventType,
        source: T,
    ) -> Result<usize, Box<broadcast::error::SendError<Event>>>
    where
        T: Into<String>,
    {
        self.publish(Event::new(kind, source))
    }

    pub fn max_capacity(&self) -> usize {
        self.max_capacity
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_publish_and_subscribe() {
        let bus = EventBus::new(16);
        let mut sub = bus.subscriber();

        let event = Event::new(EventType::Tick { interval_ms: 1000 }, "test");
        bus.publish(event.clone()).unwrap();

        let received = sub.recv().await.unwrap();
        assert_eq!(received.kind, EventType::Tick { interval_ms: 1000 });
        assert_eq!(received.source, "test");
    }

    #[test]
    fn test_event_display() {
        let event = Event::new(
            EventType::TrayChanged {
                action: "shown".into(),
            },
            "test",
        );
        assert!(format!("{}", event.kind).starts_with("tray:"));
    }
}
