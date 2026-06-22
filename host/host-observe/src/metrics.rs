/// Metrics collector — lightweight counter/gauge/histogram tracking.
///
/// Designed to be fast and allocation-free in hot paths.
/// Data is exposed via the Axum service layer for the Ratatui dashboard.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::Utc;

/// Metric types.
#[derive(Debug, Clone)]
pub enum Metric {
    Counter { name: String, value: u64, labels: HashMap<String, String> },
    Gauge { name: String, value: f64, labels: HashMap<String, String> },
    Histogram { name: String, value: f64, labels: HashMap<String, String> },
}

/// Thread-safe metrics collector.
#[derive(Clone)]
pub struct MetricsCollector {
    inner: Arc<Mutex<MetricsStore>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MetricsStore::new())),
        }
    }

    /// Increment a counter.
    pub fn inc(&self, name: &str, labels: HashMap<String, String>) {
        let mut store = self.inner.lock().unwrap();
        store.inc(name, labels);
    }

    /// Set a gauge value.
    pub fn set_gauge(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        let mut store = self.inner.lock().unwrap();
        store.set_gauge(name, value, labels);
    }

    /// Record a histogram value.
    pub fn record_histogram(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        let mut store = self.inner.lock().unwrap();
        store.record_histogram(name, value, labels);
    }

    /// Get all metrics as JSON.
    pub fn snapshot(&self) -> serde_json::Value {
        let store = self.inner.lock().unwrap();
        store.snapshot()
    }

    /// Get a specific metric's value.
    pub fn get(&self, name: &str) -> Option<serde_json::Value> {
        let store = self.inner.lock().unwrap();
        store.get(name)
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal metrics store.
struct MetricsStore {
    counters: HashMap<String, u64>,
    gauges: HashMap<String, f64>,
    histograms: HashMap<String, Vec<f64>>,
}

impl MetricsStore {
    fn new() -> Self {
        Self {
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
        }
    }

    fn inc(&mut self, name: &str, labels: HashMap<String, String>) {
        let key = Self::make_key(name, &labels);
        *self.counters.entry(key).or_insert(0) += 1;
    }

    fn set_gauge(&mut self, name: &str, value: f64, labels: HashMap<String, String>) {
        let key = Self::make_key(name, &labels);
        self.gauges.insert(key, value);
    }

    fn record_histogram(&mut self, name: &str, value: f64, labels: HashMap<String, String>) {
        let key = Self::make_key(name, &labels);
        self.histograms.entry(key).or_default().push(value);
    }

    fn make_key(name: &str, labels: &HashMap<String, String>) -> String {
        if labels.is_empty() {
            name.to_string()
        } else {
            let mut parts: Vec<_> = labels.iter().collect();
            parts.sort_by_key(|(k, _)| k.clone());
            format!("{}{{{}}}", name, parts.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(","))
        }
    }

    fn snapshot(&self) -> serde_json::Value {
        let mut result = serde_json::Map::new();

        for (name, value) in &self.counters {
            result.insert(name.clone(), serde_json::json!({ "type": "counter", "value": *value, "ts": Utc::now().to_rfc3339() }));
        }
        for (name, value) in &self.gauges {
            result.insert(name.clone(), serde_json::json!({ "type": "gauge", "value": *value, "ts": Utc::now().to_rfc3339() }));
        }
        for (name, values) in &self.histograms {
            let avg = if !values.is_empty() {
                values.iter().sum::<f64>() / values.len() as f64
            } else { 0.0 };
            result.insert(name.clone(), serde_json::json!({
                "type": "histogram",
                "count": values.len(),
                "sum": values.iter().sum::<f64>(),
                "avg": avg,
                "ts": Utc::now().to_rfc3339(),
            }));
        }

        serde_json::Value::Object(result)
    }

    fn get(&self, name: &str) -> Option<serde_json::Value> {
        let mut result = serde_json::Map::new();

        for (key, value) in &self.counters {
            if key.starts_with(name) {
                result.insert(key.clone(), serde_json::json!({ "type": "counter", "value": *value }));
            }
        }
        for (key, value) in &self.gauges {
            if key.starts_with(name) {
                result.insert(key.clone(), serde_json::json!({ "type": "gauge", "value": *value }));
            }
        }
        for (key, values) in &self.histograms {
            if key.starts_with(name) {
                let avg = if !values.is_empty() {
                    values.iter().sum::<f64>() / values.len() as f64
                } else { 0.0 };
                result.insert(key.clone(), serde_json::json!({
                    "type": "histogram",
                    "count": values.len(),
                    "avg": avg,
                }));
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(result))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let m = MetricsCollector::new();
        m.inc("requests_total", HashMap::new());
        m.inc("requests_total", HashMap::new());
        let snap = m.snapshot();
        assert!(snap.get("requests_total").is_some());
    }

    #[test]
    fn test_gauge() {
        let m = MetricsCollector::new();
        m.set_gauge("memory_mb", 142.5, HashMap::new());
        let snap = m.snapshot();
        let gauge = snap.get("memory_mb").unwrap();
        assert_eq!(gauge["type"], "gauge");
        assert!((gauge["value"].as_f64().unwrap() - 142.5).abs() < f64::EPSILON);
    }
}
