//! JEV Decision Evolution System
//!
//! Learns from user feedback and telemetry to improve decision accuracy over time.
//! Tracks approvals/rejections of JEV decisions and adjusts thresholds accordingly.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

/// Evolution configuration loaded from .crabjar_config.toml
#[derive(Debug, Clone, Deserialize)]
pub struct EvolutionConfig {
    pub enabled: bool,
    pub evolution_threshold: f64,
    pub feedback_window: usize,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            evolution_threshold: 0.85,
            feedback_window: 100,
        }
    }
}

/// Decision outcome tracked for evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOutcome {
    pub decision_id: String,
    pub criteria_file: String,
    pub option: String,
    pub confidence: f64,
    pub user_approved: bool,
    pub timestamp: u64,
}

/// Evolution state persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionState {
    /// Recent decision outcomes for learning
    pub recent_outcomes: Vec<DecisionOutcome>,
    /// Per-criteria threshold adjustments
    pub threshold_adjustments: HashMap<String, f64>,
    /// Decision counts per criteria file
    pub decision_counts: HashMap<String, usize>,
}

impl Default for EvolutionState {
    fn default() -> Self {
        Self {
            recent_outcomes: Vec::new(),
            threshold_adjustments: HashMap::new(),
            decision_counts: HashMap::new(),
        }
    }
}

/// JEV Evolution Manager
pub struct EvolutionManager {
    config: EvolutionConfig,
    state: EvolutionState,
    state_path: String,
}

impl EvolutionManager {
    /// Create a new evolution manager from .crabjar_config.toml
    pub fn new(project_root: &str) -> Result<Self> {
        let config_path = Path::new(project_root).join(".crabjar_config.toml");

        let config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let parsed: toml::Value = toml::from_str(&content)?;

            // Extract decision configuration
            if let Some(decision) = parsed.get("decision") {
                EvolutionConfig {
                    enabled: decision
                        .get("enabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true),
                    evolution_threshold: decision
                        .get("evolution_threshold")
                        .and_then(|v| {
                            if let Ok(f) = v.to_string().parse::<f64>() {
                                Some(f)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(0.85),
                    feedback_window: decision
                        .get("feedback_window")
                        .and_then(|v| v.as_integer())
                        .map(|v| v as usize)
                        .unwrap_or(100),
                }
            } else {
                EvolutionConfig::default()
            }
        } else {
            EvolutionConfig::default()
        };

        let state_path = Path::new(project_root)
            .join(".crabjar")
            .join("jev-evolution.json")
            .to_string_lossy()
            .to_string();

        // Load existing evolution state if it exists
        let state = if Path::new(&state_path).exists() {
            let content = fs::read_to_string(&state_path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            EvolutionState::default()
        };

        Ok(Self {
            config,
            state,
            state_path,
        })
    }

    /// Record a decision outcome for learning
    pub fn record_outcome(&mut self, outcome: DecisionOutcome) -> Result<()> {
        self.state.recent_outcomes.push(outcome);

        // Trim to feedback window size
        if self.state.recent_outcomes.len() > self.config.feedback_window {
            self.state.recent_outcomes.remove(0);
        }

        self.save_state()?;
        Ok(())
    }

    /// Get the evolved threshold for a criteria file
    pub fn get_threshold(&self, criteria_file: &str) -> f64 {
        if let Some(adjustment) = self.state.threshold_adjustments.get(criteria_file) {
            *adjustment
        } else {
            self.config.evolution_threshold
        }
    }

    /// Evolve thresholds based on recent outcomes
    pub fn evolve(&mut self) -> Result<()> {
        // Group outcomes by criteria file
        let mut outcomes_by_criteria: HashMap<String, Vec<&DecisionOutcome>> = HashMap::new();

        for outcome in &self.state.recent_outcomes {
            outcomes_by_criteria
                .entry(outcome.criteria_file.clone())
                .or_default()
                .push(outcome);
        }

        // Adjust thresholds based on approval rates
        for (criteria, outcomes) in outcomes_by_criteria {
            if outcomes.len() < 5 {
                continue; // Need minimum sample size
            }

            let approved = outcomes.iter().filter(|o| o.user_approved).count();
            let approval_rate = approved as f64 / outcomes.len() as f64;

            // If approval rate is low, increase threshold (be more conservative)
            // If approval rate is high, decrease threshold (be more aggressive)
            let current_threshold = self.get_threshold(&criteria);
            let adjustment = if approval_rate < 0.7 {
                // Too many rejections - increase threshold
                (current_threshold + 0.05).min(0.95)
            } else if approval_rate > 0.95 {
                // Almost all approved - decrease threshold
                (current_threshold - 0.02).max(0.6)
            } else {
                current_threshold
            };

            self.state
                .threshold_adjustments
                .insert(criteria, adjustment);
        }

        self.save_state()?;
        Ok(())
    }

    /// Get decision statistics for a criteria file
    pub fn get_stats(&self, criteria_file: &str) -> Option<DecisionStats> {
        let outcomes: Vec<&DecisionOutcome> = self
            .state
            .recent_outcomes
            .iter()
            .filter(|o| o.criteria_file == criteria_file)
            .collect();

        if outcomes.is_empty() {
            return None;
        }

        let approved = outcomes.iter().filter(|o| o.user_approved).count();

        Some(DecisionStats {
            total: outcomes.len(),
            approved,
            rejected: outcomes.len() - approved,
            approval_rate: approved as f64 / outcomes.len() as f64,
            threshold: self.get_threshold(criteria_file),
        })
    }

    /// Save evolution state to disk
    fn save_state(&self) -> Result<()> {
        let dir = Path::new(&self.state_path)
            .parent()
            .ok_or_else(|| anyhow!("Invalid state path"))?;

        fs::create_dir_all(dir)?;

        let content = serde_json::to_string_pretty(&self.state)?;
        fs::write(&self.state_path, content)?;

        Ok(())
    }

    /// Check if evolution is enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

/// Decision statistics for monitoring and reporting
#[derive(Debug, Clone)]
pub struct DecisionStats {
    pub total: usize,
    pub approved: usize,
    pub rejected: usize,
    pub approval_rate: f64,
    pub threshold: f64,
}
