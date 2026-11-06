//! Host-side binding utilities for Wasm plugin components.
//!
//! Provides host import implementations that plugin components call into CrabJar,
//! including state management, event publishing, and logging capabilities.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

/// Type alias for the event sink closure.
type EventSink = Arc<dyn Fn(String, Vec<u8>) + Send + Sync>;

/// Log level enum matching the WIT spec's log interface.
#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

impl From<u8> for LogLevel {
    fn from(value: u8) -> Self {
        match value {
            0 => LogLevel::Trace,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            _ => LogLevel::Error,
        }
    }
}

/// Host-side state store implementation.
pub struct StateStore {
    data: HashMap<String, Vec<u8>>,
    schema_version: u32,
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl StateStore {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            schema_version: 0,
        }
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        debug!(plugin_state_key = %key, "state.get");
        self.data.get(key).cloned()
    }

    pub fn put(&mut self, key: String, value: Vec<u8>) {
        debug!(plugin_state_key = %key, value_size = value.len(), "state.put");
        self.data.insert(key, value);
    }

    pub fn delete(&mut self, key: &str) {
        debug!(plugin_state_key = %key, "state.delete");
        self.data.remove(key);
    }

    pub fn list_keys(&self, prefix: &str) -> Vec<String> {
        debug!(plugin_state_prefix = %prefix, "state.list_keys");
        self.data
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect()
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn set_schema_version(&mut self, version: u32) {
        debug!(plugin_state_schema_version = version, "state.set_schema_version");
        self.schema_version = version;
    }
}

/// Host-side event publisher implementation.
pub struct EventPublisher {
    sink: Option<EventSink>,
}

impl Default for EventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl EventPublisher {
    pub fn new() -> Self {
        Self { sink: None }
    }

    pub fn with_sink<F>(sink: F) -> Self
    where
        F: Fn(String, Vec<u8>) + Send + Sync + 'static,
    {
        Self {
            sink: Some(Arc::new(sink)),
        }
    }

    pub fn publish(&self, kind: String, payload: Vec<u8>) {
        info!(plugin_event_kind = %kind, payload_size = payload.len(), "events.publish");
        if let Some(sink) = &self.sink {
            sink(kind, payload);
        }
    }
}

/// Host-side logger implementation.
pub struct PluginLogger;

impl Default for PluginLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginLogger {
    pub fn new() -> Self {
        Self
    }

    pub fn emit(&self, level: LogLevel, message: String) {
        match level {
            LogLevel::Trace => debug!(plugin = true, %message),
            LogLevel::Debug => debug!(plugin = true, %message),
            LogLevel::Info => info!(plugin = true, %message),
            LogLevel::Warn => warn!(plugin = true, %message),
            LogLevel::Error => error!(plugin = true, %message),
        }
    }
}

/// Complete host environment for a plugin component.
pub struct PluginEnvironment {
    pub state: Arc<Mutex<StateStore>>,
    pub events: Arc<EventPublisher>,
    pub logger: Arc<PluginLogger>,
}

impl Default for PluginEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginEnvironment {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(StateStore::new())),
            events: Arc::new(EventPublisher::new()),
            logger: Arc::new(PluginLogger),
        }
    }

    pub fn with_event_sink<F>(mut self, sink: F) -> Self
    where
        F: Fn(String, Vec<u8>) + Send + Sync + 'static,
    {
        self.events = Arc::new(EventPublisher::with_sink(sink));
        self
    }
}