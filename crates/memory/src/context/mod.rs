// crabjar/memory/src/context/mod.rs
// Bounded context fragments — inspired by Codex's context-fragments/ crate.

pub mod budget;
pub mod constants;
pub mod fragment;

use thiserror::Error;

// Re-export submodules at the module root for backward compatibility
pub use budget::{ContextBudget, ContextQueryResult};
pub use constants::*;
pub use fragment::{ContextFragment, ContextFragmentBuilder};

// ---------------------------------------------------------------------------
// Error types (kept here since they're shared across all context subtypes)
// ---------------------------------------------------------------------------

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ContextError {
    #[error("fragment exceeds max tokens: {actual} > {max}")]
    ExceedsMaxTokens { actual: usize, max: usize },

    #[error(
        "fragment would exceed cumulative budget: {used} / {budget} tokens used, {remaining} remaining"
    )]
    ExceedsBudget {
        used: usize,
        budget: usize,
        remaining: usize,
    },

    #[error("fragment below P0 alert threshold: {tokens} tokens exceeds {P0_ALERT_TOKENS}")]
    ApproachingP0Threshold { tokens: usize },

    #[error("invalid token count: {0}")]
    InvalidTokenCount(usize),
}

// ---------------------------------------------------------------------------
// Token estimation (approximate) — kept at root level since it's a utility
// ---------------------------------------------------------------------------

/// Estimate the token count of a string using a simple heuristic.
///
/// This is an approximation — real tokenizers vary by model. The estimate
/// is used for budgeting; the actual token count should be computed by the
/// model's tokenizer when available.
///
/// Heuristic: ~4 characters per token for English text, ~2 for dense code.
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    let chars = text.chars().count();

    // Heuristic: English text ~4 chars/token, code ~2 chars/token
    // Simple heuristic: if the text has many non-ASCII chars, it's likely dense
    let non_ascii = text.chars().filter(|c| !c.is_ascii()).count();
    let density = if chars > 0 {
        non_ascii as f64 / chars as f64
    } else {
        0.0
    };

    // Adjust divisor based on density: higher density → smaller divisor → more tokens
    let divisor = if density > 0.5 {
        2.0 // Dense code/non-ASCII
    } else {
        4.0 // English text
    };

    (chars as f64 / divisor).ceil() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- ContextFragment --

    #[test]
    fn test_fragment_new_valid() {
        let frag = ContextFragment::new("test-1", "Test Fragment", "hello world", 5).unwrap();
        assert_eq!(frag.id, "test-1");
        assert_eq!(frag.label, "Test Fragment");
        assert_eq!(frag.content, "hello world");
        assert_eq!(frag.token_count, 5);
        assert!(!frag.p0_flagged);
        assert!(frag.is_safe());
    }

    #[test]
    fn test_fragment_new_exceeds_max() {
        let result = ContextFragment::new(
            "test-2",
            "Too Big",
            "x".repeat(MAX_TOKENS_PER_FRAGMENT + 1),
            MAX_TOKENS_PER_FRAGMENT + 1,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            ContextError::ExceedsMaxTokens { actual, max } => {
                assert_eq!(actual, MAX_TOKENS_PER_FRAGMENT + 1);
                assert_eq!(max, MAX_TOKENS_PER_FRAGMENT);
            }
            _ => panic!("expected ExceedsMaxTokens"),
        }
    }

    #[test]
    fn test_fragment_p0_flagged() {
        let frag = ContextFragment::new(
            "test-3",
            "P0 Fragment",
            "x".repeat(P0_ALERT_TOKENS + 1),
            P0_ALERT_TOKENS + 1,
        )
        .unwrap();
        assert!(frag.p0_flagged);
        assert!(!frag.is_safe());
    }

    #[test]
    fn test_fragment_new_unchecked() {
        let frag = ContextFragment::new_unchecked(
            "test-4",
            "Unchecked",
            "content",
            MAX_TOKENS_PER_FRAGMENT + 100, // Would fail new() but not new_unchecked
        );
        assert_eq!(frag.token_count, MAX_TOKENS_PER_FRAGMENT + 100);
    }

    #[test]
    fn test_fragment_meta() {
        let frag = ContextFragment::new("id", "label", "content", 5)
            .unwrap()
            .meta("key", "value")
            .meta("number", 42)
            .meta_value("nested", serde_json::json!({"a": 1}));
        assert_eq!(frag.metadata["key"], "value");
        assert_eq!(frag.metadata["number"], 42);
        assert_eq!(frag.metadata["nested"]["a"], 1);
    }

    #[test]
    fn test_fragment_to_json() {
        let frag = ContextFragment::new("id", "label", "content", 5).unwrap();
        let json = frag.to_json();
        assert_eq!(json["id"], "id");
        assert_eq!(json["token_count"], 5);
        assert_eq!(json["p0_flagged"], false);
    }

    #[test]
    fn test_fragment_byte_size() {
        let frag = ContextFragment::new("id", "label", "hello", 5).unwrap();
        assert_eq!(frag.byte_size(), 5);
    }

    // -- ContextBudget --

    #[test]
    fn test_budget_new() {
        let budget = ContextBudget::new(1000);
        assert_eq!(budget.budget, 1000);
        assert_eq!(budget.used, 0);
        assert_eq!(budget.remaining(), 1000);
        assert!(!budget.exhausted);
    }

    #[test]
    fn test_budget_reserve() {
        let mut budget = ContextBudget::new(1000);
        budget.reserve(500).unwrap();
        assert_eq!(budget.used, 500);
        assert_eq!(budget.remaining(), 500);
        assert_eq!(budget.fragment_count, 1);
    }

    #[test]
    fn test_budget_reserve_exceeds() {
        let mut budget = ContextBudget::new(1000);
        budget.reserve(500).unwrap();
        let result = budget.reserve(600);
        assert!(result.is_err());
        match result.unwrap_err() {
            ContextError::ExceedsBudget {
                used, remaining, ..
            } => {
                assert_eq!(used, 500);
                assert_eq!(remaining, 500);
            }
            _ => panic!("expected ExceedsBudget"),
        }
    }

    #[test]
    fn test_budget_reserve_exact() {
        let mut budget = ContextBudget::new(1000);
        budget.reserve(1000).unwrap();
        assert!(budget.exhausted);
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    fn test_budget_release() {
        let mut budget = ContextBudget::new(1000);
        budget.reserve(500).unwrap();
        budget.release(200);
        assert_eq!(budget.used, 300);
        assert_eq!(budget.remaining(), 700);
        assert!(!budget.exhausted);
    }

    #[test]
    fn test_budget_summary() {
        let mut budget = ContextBudget::new(1000);
        budget.reserve(300).unwrap();
        let summary = budget.summary();
        assert_eq!(summary["budget"], 1000);
        assert_eq!(summary["used"], 300);
        assert_eq!(summary["remaining"], 700);
        assert_eq!(summary["fragment_count"], 1);
        assert_eq!(summary["utilization_pct"], 30.0);
    }

    #[test]
    fn test_budget_default() {
        let budget = ContextBudget::default_budget();
        assert_eq!(budget.budget, DEFAULT_CONTEXT_BUDGET);
    }

    #[test]
    fn test_budget_can_fit() {
        let mut budget = ContextBudget::new(1000);
        assert!(budget.can_fit(500));
        budget.reserve(800).unwrap();
        assert!(!budget.can_fit(300));
        assert!(budget.can_fit(200));
    }

    // -- ContextFragmentBuilder --

    #[test]
    fn test_builder_build_valid() {
        let frag = ContextFragmentBuilder::new()
            .id("b-1")
            .label("Builder Fragment")
            .content("builder content")
            .token_count(10)
            .meta("source", "test")
            .build()
            .unwrap();
        assert_eq!(frag.id, "b-1");
        assert_eq!(frag.token_count, 10);
        assert_eq!(frag.metadata["source"], "test");
    }

    #[test]
    fn test_builder_build_exceeds() {
        let result = ContextFragmentBuilder::new()
            .id("b-2")
            .label("Too Big")
            .content("x")
            .token_count(MAX_TOKENS_PER_FRAGMENT + 1)
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_build_unchecked() {
        let frag = ContextFragmentBuilder::new()
            .id("b-3")
            .label("Unchecked")
            .content("x")
            .token_count(MAX_TOKENS_PER_FRAGMENT + 1)
            .build_unchecked();
        assert_eq!(frag.token_count, MAX_TOKENS_PER_FRAGMENT + 1);
    }

    #[test]
    fn test_builder_default() {
        let _ = ContextFragmentBuilder::default();
    }

    // -- estimate_tokens --

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_english() {
        // ~4 chars per token heuristic
        let text = "The quick brown fox jumps over the lazy dog";
        let tokens = estimate_tokens(text);
        assert!(tokens > 0);
        // "The quick brown fox jumps over the lazy dog" = 43 chars → ~11 tokens at 4 chars/token
        assert!(tokens <= 12);
    }

    #[test]
    fn test_estimate_tokens_dense_code() {
        // Dense code → ~2 chars per token
        let text = "fn main() { let x = 42; println!(\"{}\"); }";
        let tokens = estimate_tokens(text);
        assert!(tokens > 0);
    }

    // -- ContextQueryResult --

    #[test]
    fn test_query_result_new() {
        let frag1 = ContextFragment::new("f1", "Frag 1", "content1", 100).unwrap();
        let frag2 = ContextFragment::new("f2", "Frag 2", "content2", 200).unwrap();
        let mut budget = ContextBudget::new(1000);
        budget.reserve(100).unwrap();
        budget.reserve(200).unwrap();

        let result = ContextQueryResult::new(vec![frag1, frag2], &budget);
        assert_eq!(result.fragments.len(), 2);
        assert_eq!(result.total_tokens, 300);
        assert!(!result.budget_exhausted);
        assert!(result.p0_flagged.is_empty());
    }

    #[test]
    fn test_query_result_p0_flagged() {
        let frag1 = ContextFragment::new("f1", "Frag 1", "content1", 100).unwrap();
        let frag2 = ContextFragment::new("f2", "Frag 2", "content2", P0_ALERT_TOKENS + 1).unwrap();
        let mut budget = ContextBudget::new(2000);
        budget.reserve(100).unwrap();
        budget.reserve(P0_ALERT_TOKENS + 1).unwrap();

        let result = ContextQueryResult::new(vec![frag1, frag2], &budget);
        assert_eq!(result.p0_flagged.len(), 1);
        assert_eq!(result.p0_flagged[0], "f2");
    }
}
