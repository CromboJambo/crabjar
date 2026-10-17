//! JEV Rust Rule Engine
//!
//! Deterministic keyword/pattern matching layer that evaluates observations
//! against structured rules before falling through to embedding similarity.
//! Provides fast-path decisions for clear-cut cases, reducing expensive API calls.
//!
//! Design principle: Just Enough Variables — minimal decision space per rule.

use serde::{Deserialize, Serialize};

/// A single evaluation rule with keyword matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Unique identifier for this rule
    pub id: String,
    /// Decision option ID to select if this rule matches
    pub decision_option: String,
    /// Required keywords (all must be present)
    #[serde(default)]
    pub requires_all: Vec<String>,
    /// Optional keywords (any may be present, boost confidence)
    #[serde(default)]
    pub requires_any: Vec<String>,
    /// Keywords that negate this rule (if any present, rule doesn't match)
    #[serde(default)]
    pub negated_by: Vec<String>,
    /// Base confidence when rule matches (0.0 - 1.0)
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    /// Confidence boost per optional keyword matched
    #[serde(default = "default_boost")]
    pub confidence_boost_per_keyword: f64,
}

fn default_confidence() -> f64 {
    0.85
}

fn default_boost() -> f64 {
    0.02
}

/// Rule evaluation result
#[derive(Debug, Clone)]
pub struct RuleMatch {
    pub rule_id: String,
    pub decision_option: String,
    pub confidence: f64,
    /// Number of required keywords matched
    pub required_matched: usize,
    /// Number of optional keywords matched
    pub optional_matched: usize,
}

/// Rust Rule Engine that evaluates observations against a rule set
pub struct RuleEngine {
    rules: Vec<Rule>,
}

impl RuleEngine {
    /// Create a new rule engine with the given rules
    pub fn new(rules: Vec<Rule>) -> Self {
        Self { rules }
    }

    /// Load rules from a criteria-like JSON structure
    /// Expects an object with a "rules" array field
    pub fn from_json(json_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let parsed: serde_json::Value = serde_json::from_str(json_str)?;

        let rules_value = parsed.get("rules").ok_or("Missing 'rules' array in JSON")?;

        let rules: Vec<Rule> = serde_json::from_value(rules_value.clone())?;
        Ok(Self { rules })
    }

    /// Evaluate an observation against all rules.
    /// Returns the highest-confidence matching rule, or None if no rules match.
    pub fn evaluate(&self, observation: &str) -> Option<RuleMatch> {
        let obs_lower = observation.to_lowercase();

        let mut best_match: Option<RuleMatch> = None;

        for rule in &self.rules {
            // Check negation keywords first (fast fail)
            if !rule.negated_by.is_empty() {
                let negated = rule
                    .negated_by
                    .iter()
                    .any(|kw| obs_lower.contains(&kw.to_lowercase()));
                if negated {
                    continue;
                }
            }

            // Check required keywords (all must match)
            let required_matched = rule
                .requires_all
                .iter()
                .filter(|kw| obs_lower.contains(&kw.to_lowercase()))
                .count();

            if required_matched < rule.requires_all.len() {
                continue; // Not all required keywords present
            }

            // Check optional keywords (boost confidence)
            let optional_matched = rule
                .requires_any
                .iter()
                .filter(|kw| obs_lower.contains(&kw.to_lowercase()))
                .count();

            // Calculate confidence with boost
            let mut confidence = rule.confidence;
            if !rule.requires_any.is_empty() {
                confidence += optional_matched as f64 * rule.confidence_boost_per_keyword;
            }
            confidence = confidence.min(1.0);

            // Track best match by confidence
            let match_result = RuleMatch {
                rule_id: rule.id.clone(),
                decision_option: rule.decision_option.clone(),
                confidence,
                required_matched,
                optional_matched,
            };

            if best_match.is_none()
                || match_result.confidence > best_match.as_ref().unwrap().confidence
            {
                best_match = Some(match_result);
            }
        }

        best_match
    }

    /// Get the number of rules in this engine
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_keyword_match() {
        let rules = vec![Rule {
            id: "nginx-fail".to_string(),
            decision_option: "restart_service".to_string(),
            requires_all: vec!["nginx".to_string(), "failed".to_string()],
            requires_any: vec![],
            negated_by: vec![],
            confidence: 0.9,
            confidence_boost_per_keyword: 0.0,
        }];

        let engine = RuleEngine::new(rules);
        let result = engine.evaluate("Service nginx failed to start");
        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.decision_option, "restart_service");
        assert!((match_result.confidence - 0.9).abs() < 0.001);
    }

    #[test]
    fn optional_keywords_boost_confidence() {
        let rules = vec![Rule {
            id: "disk-full".to_string(),
            decision_option: "investigate".to_string(),
            requires_all: vec!["disk".to_string()],
            requires_any: vec!["full".to_string(), "90%".to_string(), "space".to_string()],
            negated_by: vec![],
            confidence: 0.7,
            confidence_boost_per_keyword: 0.05,
        }];

        let engine = RuleEngine::new(rules);
        let result = engine.evaluate("Disk is full at 95% usage");
        assert!(result.is_some());
        let match_result = result.unwrap();
        // Base 0.7 + 1 optional keyword (full) * 0.05 = 0.75
        assert!((match_result.confidence - 0.75).abs() < 0.001);
    }

    #[test]
    fn negation_prevents_match() {
        let rules = vec![Rule {
            id: "service-fail".to_string(),
            decision_option: "restart_service".to_string(),
            requires_all: vec!["failed".to_string()],
            requires_any: vec![],
            negated_by: vec!["test".to_string()],
            confidence: 0.9,
            confidence_boost_per_keyword: 0.0,
        }];

        let engine = RuleEngine::new(rules);
        // Should match (no negation keyword)
        assert!(engine.evaluate("Service failed").is_some());
        // Should not match (contains "test")
        assert!(engine.evaluate("Test run failed as expected").is_none());
    }

    #[test]
    fn highest_confidence_wins() {
        let rules = vec![
            Rule {
                id: "generic-fail".to_string(),
                decision_option: "investigate".to_string(),
                requires_all: vec!["failed".to_string()],
                requires_any: vec![],
                negated_by: vec![],
                confidence: 0.6,
                confidence_boost_per_keyword: 0.0,
            },
            Rule {
                id: "nginx-fail".to_string(),
                decision_option: "restart_service".to_string(),
                requires_all: vec!["nginx".to_string(), "failed".to_string()],
                requires_any: vec![],
                negated_by: vec![],
                confidence: 0.9,
                confidence_boost_per_keyword: 0.0,
            },
        ];

        let engine = RuleEngine::new(rules);
        let result = engine.evaluate("nginx failed to start");
        assert!(result.is_some());
        assert_eq!(result.unwrap().rule_id, "nginx-fail");
    }

    #[test]
    fn no_match_returns_none() {
        let rules = vec![Rule {
            id: "specific".to_string(),
            decision_option: "do_something".to_string(),
            requires_all: vec!["very".to_string(), "specific".to_string()],
            requires_any: vec![],
            negated_by: vec![],
            confidence: 0.9,
            confidence_boost_per_keyword: 0.0,
        }];

        let engine = RuleEngine::new(rules);
        assert!(engine.evaluate("unrelated observation").is_none());
    }
}
