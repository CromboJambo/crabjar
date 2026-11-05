//! Decision gate and JEV evaluation methods for the conductor.

use crate::conductor::Conductor;
use crate::decision::{BackendConfig, CacheStats, DecisionGate, DecisionPolicy};
use anyhow::Result;
use tracing::{info, warn};

impl Conductor {
    /// Configure JEV decision gate for automated fleet decisions.
    pub fn with_decision_gate(
        &mut self,
        backend_config: BackendConfig,
        policy: Option<DecisionPolicy>,
    ) {
        let evaluator = crate::decision::evaluate::JevEvaluator::new(backend_config);
        self.decision_gate = Some(DecisionGate::new(evaluator, policy));
    }

    /// Load registered decision capabilities from manifest files.
    pub fn load_decision_capabilities(&mut self, decisions_dir: &str) -> Result<usize, String> {
        if let Some(gate) = &mut self.decision_gate {
            gate.load_decision_capabilities(decisions_dir)
        } else {
            Err("Decision gate not configured; call with_decision_gate() first".to_string())
        }
    }

    /// List all registered decision capabilities as JSON values.
    pub fn list_decision_capabilities(&self) -> serde_json::Value {
        self.decision_gate.as_ref().map_or_else(
            || serde_json::json!({"count": 0, "capabilities": []}),
            |gate| gate.registry().to_json(),
        )
    }

    /// Configure hybrid attention + JEV decision layer.
    pub fn with_hybrid_decision_layer(
        &mut self,
        backend_config: BackendConfig,
        policy: Option<crate::attention::HybridDecisionPolicy>,
    ) {
        let evaluator = crate::decision::evaluate::JevEvaluator::new(backend_config.clone());
        let gate = DecisionGate::new(evaluator, None::<crate::decision::DecisionPolicy>);

        if let Some(mirror_log_path) = &self.config.mirror_log_db_path {
            match crate::attention::HybridDecisionLayer::new(mirror_log_path, gate, policy) {
                Ok(layer) => {
                    self.hybrid_layer = Some(layer);
                    info!("Hybrid attention + JEV decision layer configured");
                }
                Err(e) => {
                    warn!(error = %e, "Failed to configure hybrid decision layer, falling back to JEV-only");
                    let evaluator2 = crate::decision::evaluate::JevEvaluator::new(backend_config);
                    self.decision_gate = Some(DecisionGate::new(
                        evaluator2,
                        None::<crate::decision::DecisionPolicy>,
                    ));
                }
            }
        } else {
            // No mirror-log path configured, use plain JEV gate
            self.decision_gate = Some(gate);
        }
    }

    /// Get JEV decision cache statistics (for monitoring and metrics).
    pub fn cache_stats(&self) -> Option<CacheStats> {
        self.decision_gate
            .as_ref()
            .map(crate::decision::DecisionGate::cache_stats)
    }

    /// Extract candidate rules from decision history.
    pub fn extract_rules(
        &self,
        criteria_file: &str,
        min_occurrences: i64,
    ) -> anyhow::Result<Vec<crate::decision::rule_extraction::CandidateRule>> {
        let rules = crate::decision::rule_extraction::extract_rules(
            &self.store,
            criteria_file,
            min_occurrences,
        )?;
        Ok(rules)
    }

    /// Promote a candidate rule to persistent storage.
    pub fn promote_rule(
        &self,
        pattern_id: &str,
        criteria_file: &str,
        recommended_option: &str,
    ) -> anyhow::Result<i64> {
        let rule = crate::decision::rule_extraction::CandidateRule {
            criteria_file: criteria_file.to_string(),
            pattern_id: pattern_id.to_string(),
            recommended_option: recommended_option.to_string(),
            avg_confidence: 0.0,
            occurrence_count: 0,
        };
        let rule_id = crate::decision::rule_extraction::promote_rule(&self.store, &rule)?;
        Ok(rule_id)
    }

    /// Evaluate a decision via JEV (for external callers like pesti-runner model routing).
    pub async fn evaluate_decision(
        &self,
        observation: &str,
        criteria_file: &str,
    ) -> anyhow::Result<crate::decision::evaluate::DecisionResult> {
        // If hybrid layer is configured, use the two-stage pipeline
        if let Some(hybrid) = &self.hybrid_layer {
            return hybrid
                .process_observation(
                    uuid::Uuid::new_v4().to_string().as_str(),
                    "conductor",
                    observation,
                    None,
                )
                .await
                .map(|decision| {
                    // Extract JEV result if available, otherwise create a default
                    decision.jev_result.unwrap_or_else(|| {
                        crate::decision::evaluate::DecisionResult {
                            option: "ignore".to_string(),
                            rationale: format!(
                                "Filtered by attention layer (score: {:.0})",
                                decision.attention_score
                            ),
                            confidence: 0.95,
                        }
                    })
                });
        }

        // Fall back to plain JEV evaluation
        let gate = self
            .decision_gate
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("JEV decision gate not configured"))?;

        gate.evaluate(observation, criteria_file, Some(&self.store))
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    }
}
