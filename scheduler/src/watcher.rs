//! Watcher — monitor conditions and trigger actions.
//! Observation does NOT equal permission: watchers report, scheduler decides.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
pub struct WatchEvent {
    pub watcher_id: String,
    pub condition: String,
    pub triggered_at: DateTime<Utc>,
    pub data: serde_json::Value,
}

type WatcherFn = Arc<dyn Fn() -> bool + Send + Sync>;

pub struct Watcher {
    id: String,
    name: String,
    check: WatcherFn,
    last_triggered: Option<DateTime<Utc>>,
}

impl Watcher {
    pub fn new<F>(id: &str, name: &str, check: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            check: Arc::new(check),
            last_triggered: None,
        }
    }

    pub fn evaluate(&mut self) -> Option<WatchEvent> {
        if (self.check)() {
            let now = Utc::now();
            let event = WatchEvent {
                watcher_id: self.id.clone(),
                condition: self.name.clone(),
                triggered_at: now,
                data: serde_json::json!({}),
            };
            self.last_triggered = Some(now);
            Some(event)
        } else {
            None
        }
    }
}

pub struct WatcherRegistry {
    watchers: HashMap<String, Watcher>,
}

impl WatcherRegistry {
    pub fn new() -> Self {
        Self {
            watchers: HashMap::new(),
        }
    }

    pub fn register<F>(&mut self, watcher: Watcher) where F: Fn() -> bool + Send + Sync + 'static {
        let id = watcher.id.clone();
        self.watchers.insert(id, watcher);
    }

    pub fn evaluate_all(&mut self) -> Vec<WatchEvent> {
        let mut events = Vec::new();
        for (_, watcher) in self.watchers.iter_mut() {
            if let Some(event) = watcher.evaluate() {
                events.push(event);
            }
        }
        events
    }

    pub fn count(&self) -> usize {
        self.watchers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watcher_registry() {
        let mut registry = WatcherRegistry::new();
        assert_eq!(registry.count(), 0);
    }
}
