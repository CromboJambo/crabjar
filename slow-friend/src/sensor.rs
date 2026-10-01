//! Tier 1: Deterministic Sensors — cheap, constant-time checks that run
//! continuously to detect anomalies or noteworthy events.

use serde::{Deserialize, Serialize};

/// Result of a single sensor check.
#[derive(Debug, Clone, PartialEq)]
pub enum SensorResult {
    /// No anomaly detected; continue sleeping.
    Quiet,
    /// Anomaly detected; payload contains event data for interpretation.
    Alert(SensorEvent),
}

/// Raw event data from a sensor, before interpretation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorEvent {
    pub source: String,
    pub event_type: String,
    pub timestamp: i64,
    pub payload: serde_json::Value,
}

/// A deterministic, cheap-to-run sensor that detects anomalies.
///
/// Sensors should be stateless or have minimal state. They run frequently
/// (every tick of the daemon loop) and must be fast — microseconds to low
/// milliseconds. Expensive checks belong in interpretation or decision layers.
pub trait Sensor: Send + Sync {
    /// Name of this sensor for logging and identification.
    fn name(&self) -> &str;

    /// Check for anomalies. Returns Quiet if nothing interesting, Alert with
    /// event data otherwise. Must be cheap to call repeatedly.
    fn check(&self) -> SensorResult;
}

/// Example: disk usage sensor (simplified).
pub struct DiskUsageSensor {
    pub threshold_pct: f64,
}

impl DiskUsageSensor {
    pub fn new(threshold_pct: f64) -> Self {
        Self { threshold_pct }
    }
}

impl Sensor for DiskUsageSensor {
    fn name(&self) -> &str {
        "disk_usage"
    }

    fn check(&self) -> SensorResult {
        // In a real implementation, this would query the filesystem.
        // For now, return Quiet (no anomaly).
        SensorResult::Quiet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensor_quiet_by_default() {
        let sensor = DiskUsageSensor::new(90.0);
        assert_eq!(sensor.check(), SensorResult::Quiet);
    }
}
