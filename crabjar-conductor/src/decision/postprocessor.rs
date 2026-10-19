//! Phase 3: Post-Processor & Reflection Integration
//!
//! Validates JEV decisions against structural rules and collects feedback
//! for criteria improvement over time.

use crate::decision::structural::{EvaluationTarget, SpanKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of post-processing a JEV decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostProcessResult {
    /// Whether the decision passed validation.
    pub valid: bool,
    /// Issues found during validation (empty if valid).
    pub issues: Vec<String>,
    /// Suggested confidence adjustment based on validation.
    pub confidence_adjustment: f64,
}

/// Validate a JEV decision against structural rules.
pub fn validate_decision(
    target: &EvaluationTarget,
    option_id: &str,
    criteria_path: &str,
) -> PostProcessResult {
    let mut issues = Vec::new();
    let mut confidence_adjustment = 0.0;

    // Rule 1: Function declarations must have error handling analysis
    if target.structure == SpanKind::FnDecl {
        if target.error_handling.is_none() && target.semantic_tags.get("error_handling").is_none() {
            issues.push("Function declaration lacks error handling analysis".to_string());
            confidence_adjustment -= 0.1;
        }
    }

    // Rule 2: Struct declarations should have ownership analysis
    if target.structure == SpanKind::StructDecl {
        if target.ownership_mode.is_none() && target.semantic_tags.get("ownership_mode").is_none() {
            issues.push("Struct declaration lacks ownership analysis".to_string());
            confidence_adjustment -= 0.05;
        }
    }

    // Rule 3: High-confidence decisions should have supporting tags
    if !target.semantic_tags.is_empty() || target.error_handling.is_some() {
        // Decision has structural support — no adjustment needed
    } else {
        issues.push("Decision lacks structural tag support".to_string());
        confidence_adjustment -= 0.15;
    }

    PostProcessResult {
        valid: issues.is_empty(),
        issues,
        confidence_adjustment,
    }
}

/// Collect feedback on a JEV decision for reflection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionFeedback {
    pub target_id: String,
    pub option_id: String,
    pub was_correct: bool,
    pub notes: Option<String>,
    pub timestamp: u64,
}

/// Store feedback for later reflection.
pub fn record_feedback(feedback: &DecisionFeedback) -> Result<(), String> {
    // In production, this would persist to a database or file.
    // For now, just validate the input.
    if feedback.target_id.is_empty() {
        return Err("target_id is required".to_string());
    }
    if feedback.option_id.is_empty() {
        return Err("option_id is required".to_string());
    }
    Ok(())
}

/// Analyze accumulated feedback to suggest criteria improvements.
pub fn reflect_on_feedback(feedbacks: &[DecisionFeedback]) -> Vec<String> {
    let mut suggestions = Vec::new();

    if feedbacks.is_empty() {
        return suggestions;
    }

    // Calculate accuracy by option
    let mut option_stats: HashMap<String, (u32, u32)> = HashMap::new();
    for fb in feedbacks {
        let entry = option_stats.entry(fb.option_id.clone()).or_insert((0, 0));
        entry.0 += 1; // total
        if fb.was_correct {
            entry.1 += 1; // correct
        }
    }

    for (option_id, (total, correct)) in option_stats {
        let accuracy = correct as f64 / total as f64;
        if accuracy < 0.7 && total >= 5 {
            suggestions.push(format!(
                "Option '{}' has low accuracy ({:.1}% over {} decisions). Consider refining criteria.",
                option_id,
                accuracy * 100.0,
                total
            ));
        }
    }

    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_function_with_error_handling() {
        let target = EvaluationTarget {
            structure: SpanKind::FnDecl,
            range: [0, 100],
            source: "fn foo() -> Result<()> {} ".to_string(),
            name: Some("foo".to_string()),
            semantic_tags: HashMap::new(),
            ownership_mode: None,
            error_handling: Some(crate::decision::structural::ErrorHandling::Propagated),
            mutability: None,
        };

        let result = validate_decision(&target, "well-structured-function", "test.json");
        assert!(result.valid);
        assert_eq!(result.issues.len(), 0);
    }

    #[test]
    fn test_validate_function_without_error_handling() {
        let target = EvaluationTarget {
            structure: SpanKind::FnDecl,
            range: [0, 100],
            source: "fn foo() {} ".to_string(),
            name: Some("foo".to_string()),
            semantic_tags: HashMap::new(),
            ownership_mode: None,
            error_handling: None,
            mutability: None,
        };

        let result = validate_decision(&target, "simple-function", "test.json");
        assert!(!result.valid);
        assert!(result.issues.iter().any(|i| i.contains("error handling")));
    }

    #[test]
    fn test_record_feedback() {
        let feedback = DecisionFeedback {
            target_id: "target-123".to_string(),
            option_id: "well-structured-function".to_string(),
            was_correct: true,
            notes: None,
            timestamp: 1234567890,
        };

        assert!(record_feedback(&feedback).is_ok());
    }

    #[test]
    fn test_reflect_on_low_accuracy() {
        let mut feedbacks = Vec::new();
        for i in 0..10 {
            feedbacks.push(DecisionFeedback {
                target_id: format!("target-{}", i),
                option_id: "low-accuracy-option".to_string(),
                was_correct: i < 3, // 30% accuracy
                notes: None,
                timestamp: 1234567890,
            });
        }

        let suggestions = reflect_on_feedback(&feedbacks);
        assert!(suggestions.iter().any(|s| s.contains("low-accuracy-option")));
    }
}