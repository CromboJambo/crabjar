/// Risk configuration — high/medium risk command lists, confidence floor.

use chrono;

use super::command_risk::{HIGH_RISK_COMMANDS, MEDIUM_RISK_COMMANDS};

/// Risk configuration for the execution gate.
#[derive(Debug, Clone)]
pub struct RiskConfig {
    pub high_risk: Vec<String>,
    pub medium_risk: Vec<String>,
    pub confidence_floor: f64,
    pub set_at: i64,
    pub reason: String,
    pub source: String,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            high_risk: HIGH_RISK_COMMANDS.iter().map(|s| s.to_string()).collect(),
            medium_risk: MEDIUM_RISK_COMMANDS.iter().map(|s| s.to_string()).collect(),
            confidence_floor: 0.6,
            set_at: chrono::Utc::now().timestamp(),
            reason: "default risk thresholds".to_string(),
            source: "mirror-guard".to_string(),
        }
    }
}

impl RiskConfig {
    pub fn with_high_risk(mut self, commands: Vec<String>) -> Self {
        self.high_risk = commands;
        self.set_at = chrono::Utc::now().timestamp();
        self
    }

    pub fn with_medium_risk(mut self, commands: Vec<String>) -> Self {
        self.medium_risk = commands;
        self.set_at = chrono::Utc::now().timestamp();
        self
    }

    pub fn with_confidence_floor(mut self, floor: f64) -> Self {
        self.confidence_floor = floor.clamp(0.0, 1.0);
        self.set_at = chrono::Utc::now().timestamp();
        self
    }
}
