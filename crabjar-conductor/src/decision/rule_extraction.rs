//! Rule extraction pipeline — analyzes logged decisions to identify patterns
//! that can become cached rules for faster future evaluations.
//!
//! The goal is to reduce LLM invocations by promoting frequently-observed
//! decision patterns into deterministic rules that bypass embedding computation.

use crate::state_store::StateStore;
use tracing::{debug, info};

/// A candidate rule extracted from decision history patterns.
#[derive(Debug, Clone)]
pub struct CandidateRule {
    /// The criteria file this rule applies to
    pub criteria_file: String,
    /// Pattern identifier (derived from observation hash prefix)
    pub pattern_id: String,
    /// Recommended option based on historical majority
    pub recommended_option: String,
    /// Average confidence across matching decisions
    pub avg_confidence: f64,
    /// Number of times this pattern was observed
    pub occurrence_count: i64,
}

impl CandidateRule {
    /// Check if this rule meets the threshold to be promoted.
    pub fn is_promotable(&self, min_occurrences: i64, min_confidence: f64) -> bool {
        self.occurrence_count >= min_occurrences && self.avg_confidence >= min_confidence
    }
}

/// Analyze decision history and extract candidate rules for caching.
pub fn extract_rules(
    store: &StateStore,
    criteria_file: &str,
    min_occurrences: i64,
) -> anyhow::Result<Vec<CandidateRule>> {
    info!(
        "Extracting rules from decision history for {} (min occurrences: {})",
        criteria_file, min_occurrences
    );

    // Get frequency statistics from the state store
    let frequencies = store.get_decision_frequencies(criteria_file)?;

    let mut rules = Vec::new();
    for freq in frequencies {
        if freq.count >= min_occurrences {
            rules.push(CandidateRule {
                criteria_file: criteria_file.to_string(),
                pattern_id: format!("freq-{}", freq.option),
                recommended_option: freq.option,
                avg_confidence: freq.avg_confidence,
                occurrence_count: freq.count,
            });
        }
    }

    info!(
        "Extracted {} candidate rules from decision history",
        rules.len()
    );
    Ok(rules)
}

/// Promote a candidate rule to the persistent rule store.
pub fn promote_rule(
    store: &StateStore,
    rule: &CandidateRule,
) -> anyhow::Result<i64> {
    info!(
        "Promoting rule {} for criteria {} (confidence: {:.2}, occurrences: {})",
        rule.pattern_id, rule.criteria_file, rule.avg_confidence, rule.occurrence_count
    );

    let id = store.promote_rule(
        &rule.pattern_id,
        &rule.criteria_file,
        &rule.recommended_option,
        rule.avg_confidence,
        rule.occurrence_count,
    )?;

    info!("Rule promoted with ID {}", id);
    Ok(id)
}

/// Check if a promoted rule exists for the given pattern and criteria.
pub fn has_rule(
    store: &StateStore,
    pattern_id: &str,
    criteria_file: &str,
) -> anyhow::Result<bool> {
    Ok(store.has_rule(pattern_id, criteria_file)?)
}

/// Get the recommended option from a promoted rule.
pub fn get_rule_option(
    store: &StateStore,
    pattern_id: &str,
    criteria_file: &str,
) -> anyhow::Result<Option<String>> {
    Ok(store.get_rule_option(pattern_id, criteria_file)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candidate_rule_promotability() {
        let rule = CandidateRule {
            criteria_file: "test".to_string(),
            pattern_id: "abc123".to_string(),
            recommended_option: "option_a".to_string(),
            avg_confidence: 0.85,
            occurrence_count: 5,
        };

        assert!(rule.is_promotable(3, 0.8));
        assert!(!rule.is_promotable(10, 0.8));
        assert!(!rule.is_promotable(3, 0.95));
    }
}
