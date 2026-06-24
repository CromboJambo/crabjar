use chrono::Utc;
/// Health reporting for the host runtime.
///
/// Aggregates health status from all subsystems and plugins.
use std::collections::HashMap;

/// Health status of a component.
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
    Unknown,
}

/// Component health report.
#[derive(Debug, Clone)]
pub struct HealthReport {
    pub component: String,
    pub status: HealthStatus,
    pub details: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Aggregated health reporter.
#[derive(Default)]
pub struct HealthReporter {
    reports: HashMap<String, HealthReport>,
}

impl HealthReporter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a health report.
    pub fn report(&mut self, component: &str, status: HealthStatus, details: serde_json::Value) {
        self.reports.insert(
            component.to_string(),
            HealthReport {
                component: component.to_string(),
                status,
                details,
                timestamp: Utc::now(),
            },
        );
    }

    /// Get the overall health status.
    pub fn overall_health(&self) -> HealthStatus {
        let statuses: Vec<_> = self.reports.values().map(|r| &r.status).collect();

        if statuses.is_empty() {
            return HealthStatus::Unknown;
        }

        // If any component is unhealthy, overall is unhealthy
        if statuses
            .iter()
            .any(|s| matches!(s, HealthStatus::Unhealthy { .. }))
        {
            return HealthStatus::Unhealthy {
                reason: "One or more components unhealthy".into(),
            };
        }

        // If any is degraded, overall is degraded
        if statuses
            .iter()
            .any(|s| matches!(s, HealthStatus::Degraded { .. }))
        {
            return HealthStatus::Degraded {
                reason: "One or more components degraded".into(),
            };
        }

        HealthStatus::Healthy
    }

    /// Get all reports as JSON.
    pub fn snapshot(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        for (name, report) in &self.reports {
            map.insert(name.clone(), serde_json::json!({
                "status": match &report.status {
                    HealthStatus::Healthy => serde_json::json!("healthy"),
                    HealthStatus::Degraded { reason } => serde_json::json!(format!("degraded: {reason}")),
                    HealthStatus::Unhealthy { reason } => serde_json::json!(format!("unhealthy: {reason}")),
                    HealthStatus::Unknown => serde_json::json!("unknown"),
                },
                "details": report.details,
                "ts": report.timestamp.to_rfc3339(),
            }));
        }
        serde_json::Value::Object(map)
    }
}
