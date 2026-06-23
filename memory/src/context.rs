// crabjar/memory/src/context.rs
// Bounded context fragments — inspired by Codex's context-fragments/ crate.
//
// Codex enforces hard token caps on everything injected into model context:
// - No unbounded items
// - No items larger than 10K tokens
// - Highlight items crossing 1K tokens as P0 requiring manual review
//
// Crabjar adopts this model for its knowledge store to prevent silent
// degradation of model quality in long conversations.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Constants (Codex parity)
// ---------------------------------------------------------------------------

/// Maximum tokens per fragment. Codex hard cap: 10K tokens.
pub const MAX_TOKENS_PER_FRAGMENT: usize = 10_000;

/// P0 alert threshold in tokens. Fragments exceeding this require manual
/// review before being included in model context. Codex convention: >1K tokens.
pub const P0_ALERT_TOKENS: usize = 1_000;

/// Default cumulative context budget (tokens). When the total context
/// budget is exhausted, new fragments are rejected with a hard error.
/// 128K tokens covers most models; callers can override.
pub const DEFAULT_CONTEXT_BUDGET: usize = 128_000;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ContextError {
    #[error("fragment exceeds max tokens: {actual} > {max}")]
    ExceedsMaxTokens { actual: usize, max: usize },

    #[error("fragment would exceed cumulative budget: {used} / {budget} tokens used, {remaining} remaining")]
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
// ContextFragment
// ---------------------------------------------------------------------------

/// A bounded context fragment — a single item injected into model context.
///
/// Every fragment carries a token count (not byte count) and is validated
/// against the hard cap (`MAX_TOKENS_PER_FRAGMENT`) at construction time.
/// Fragments approaching the P0 threshold (`P0_ALERT_TOKENS`) are flagged
/// so that manual review can be triggered.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextFragment {
    /// Unique identifier for this fragment (e.g., "doc:crabjar-state.md:Section 2.1").
    pub id: String,
    /// Human-readable label for display/logging.
    pub label: String,
    /// Raw content of the fragment.
    pub content: String,
    /// Token count of the content. Computed at construction time.
    pub token_count: usize,
    /// Whether this fragment is flagged for P0 alert (token_count > 1K).
    pub p0_flagged: bool,
    /// Optional metadata (source document, section, confidence, etc.).
    pub metadata: serde_json::Value,
}

impl ContextFragment {
    /// Create a new context fragment, validating token count against the
    /// hard cap.
    ///
    /// # Errors
    /// - `ContextError::ExceedsMaxTokens` if `token_count > MAX_TOKENS_PER_FRAGMENT`
    /// - `ContextError::ApproachingP0Threshold` if `token_count > P0_ALERT_TOKENS`
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        content: impl Into<String>,
        token_count: usize,
    ) -> Result<Self, ContextError> {
        let content = content.into();

        if token_count > MAX_TOKENS_PER_FRAGMENT {
            return Err(ContextError::ExceedsMaxTokens {
                actual: token_count,
                max: MAX_TOKENS_PER_FRAGMENT,
            });
        }

        let p0_flagged = token_count > P0_ALERT_TOKENS;

        Ok(Self {
            id: id.into(),
            label: label.into(),
            content,
            token_count,
            p0_flagged,
            metadata: serde_json::json!({}),
        })
    }

    /// Create a fragment without token count validation.
    ///
    /// # Safety
    /// The caller must ensure `token_count <= MAX_TOKENS_PER_FRAGMENT`.
    /// This is useful when token count has already been validated elsewhere.
    pub fn new_unchecked(
        id: impl Into<String>,
        label: impl Into<String>,
        content: impl Into<String>,
        token_count: usize,
    ) -> Self {
        let content = content.into();
        let p0_flagged = token_count > P0_ALERT_TOKENS;
        Self {
            id: id.into(),
            label: label.into(),
            content,
            token_count,
            p0_flagged,
            metadata: serde_json::json!({}),
        }
    }

    /// Add metadata key-value pair to this fragment.
    pub fn meta(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(val) = serde_json::to_value(value) {
            if let Some(obj) = self.metadata.as_object_mut() {
                obj.insert(key.into(), val);
            }
        }
        self
    }

    /// Add metadata from a pre-built JSON value.
    pub fn meta_value(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        if let Some(obj) = self.metadata.as_object_mut() {
            obj.insert(key.into(), value);
        }
        self
    }

    /// Total size in bytes (not tokens). Useful for memory budgeting.
    pub fn byte_size(&self) -> usize {
        self.content.len()
    }

    /// Whether this fragment is flagged for P0 alert.
    pub fn is_p0_flagged(&self) -> bool {
        self.p0_flagged
    }

    /// Whether this fragment is within safe token bounds (< P0 threshold).
    pub fn is_safe(&self) -> bool {
        !self.p0_flagged
    }

    /// Serialize this fragment to JSON for transport/storage.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "label": self.label,
            "content": self.content,
            "token_count": self.token_count,
            "p0_flagged": self.p0_flagged,
            "metadata": self.metadata,
        })
    }
}

// ---------------------------------------------------------------------------
// ContextBudget
// ---------------------------------------------------------------------------

/// Tracks cumulative token usage across fragments.
///
/// Codex's approach: every fragment injected into model context must be
/// tracked against a running budget. When the budget is exhausted, no more
/// fragments are accepted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBudget {
    /// Total budget in tokens.
    pub budget: usize,
    /// Tokens consumed so far.
    pub used: usize,
    /// Fragments added to this budget.
    pub fragment_count: usize,
    /// Whether the budget has been exhausted.
    pub exhausted: bool,
}

impl ContextBudget {
    /// Create a new context budget with the given token limit.
    pub fn new(budget: usize) -> Self {
        Self {
            budget,
            used: 0,
            fragment_count: 0,
            exhausted: false,
        }
    }

    /// Create a budget with the default `DEFAULT_CONTEXT_BUDGET`.
    pub fn default_budget() -> Self {
        Self::new(DEFAULT_CONTEXT_BUDGET)
    }

    /// Remaining tokens in the budget.
    pub fn remaining(&self) -> usize {
        if self.exhausted {
            0
        } else {
            self.budget.saturating_sub(self.used)
        }
    }

    /// Check whether a fragment of the given token count would fit.
    pub fn can_fit(&self, token_count: usize) -> bool {
        !self.exhausted && self.used.saturating_add(token_count) <= self.budget
    }

    /// Try to reserve tokens for a new fragment.
    ///
    /// # Errors
    /// - `ContextError::ExceedsBudget` if the fragment would exceed the budget.
    pub fn reserve(&mut self, token_count: usize) -> Result<(), ContextError> {
        if self.exhausted {
            return Err(ContextError::ExceedsBudget {
                used: self.used,
                budget: self.budget,
                remaining: 0,
            });
        }

        if !self.can_fit(token_count) {
            return Err(ContextError::ExceedsBudget {
                used: self.used,
                budget: self.budget,
                remaining: self.remaining(),
            });
        }

        self.used += token_count;
        self.fragment_count += 1;

        if self.used >= self.budget {
            self.exhausted = true;
        }

        Ok(())
    }

    /// Release tokens back to the budget (for rollback or fragment removal).
    pub fn release(&mut self, token_count: usize) {
        self.used = self.used.saturating_sub(token_count);
        if !self.exhausted && self.used < self.budget {
            self.exhausted = false;
        }
    }

    /// Get a summary of the budget state.
    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({
            "budget": self.budget,
            "used": self.used,
            "remaining": self.remaining(),
            "fragment_count": self.fragment_count,
            "exhausted": self.exhausted,
            "utilization_pct": if self.budget > 0 {
                (self.used as f64 / self.budget as f64 * 100.0).round()
            } else {
                0.0
            },
        })
    }
}

// ---------------------------------------------------------------------------
// ContextFragmentBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing `ContextFragment` instances with fluent API.
///
/// Mirrors `KnowledgeEntry`'s builder pattern for familiarity.
#[derive(Debug, Clone)]
pub struct ContextFragmentBuilder {
    id: Option<String>,
    label: Option<String>,
    content: Option<String>,
    token_count: Option<usize>,
    metadata: serde_json::Map<String, serde_json::Value>,
}

impl ContextFragmentBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            id: None,
            label: None,
            content: None,
            token_count: None,
            metadata: serde_json::Map::new(),
        }
    }

    /// Set the fragment ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the fragment label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the fragment content.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Set the token count.
    pub fn token_count(mut self, count: usize) -> Self {
        self.token_count = Some(count);
        self
    }

    /// Add a metadata key-value pair.
    pub fn meta(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(val) = serde_json::to_value(value) {
            self.metadata.insert(key.into(), val);
        }
        self
    }

    /// Add metadata from a pre-built JSON value.
    pub fn meta_value(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Build the fragment, validating token count.
    ///
    /// # Errors
    /// - `ContextError::ExceedsMaxTokens` if token count exceeds the hard cap.
    pub fn build(self) -> Result<ContextFragment, ContextError> {
        let id = self.id.unwrap_or_default();
        let label = self.label.unwrap_or_default();
        let content = self.content.unwrap_or_default();
        let token_count = self.token_count.ok_or(ContextError::InvalidTokenCount(0))?;

        ContextFragment::new(id, label, content, token_count)
            .map(|mut f| {
                if !self.metadata.is_empty() {
                    if let Some(obj) = f.metadata.as_object_mut() {
                        for (k, v) in self.metadata {
                            obj.insert(k, v);
                        }
                    }
                }
                f
            })
    }

    /// Build without validation (caller guarantees token count is valid).
    pub fn build_unchecked(self) -> ContextFragment {
        let id = self.id.unwrap_or_default();
        let label = self.label.unwrap_or_default();
        let content = self.content.unwrap_or_default();
        let token_count = self.token_count.unwrap_or(0);

        let mut f = ContextFragment::new_unchecked(id, label, content, token_count);
        if !self.metadata.is_empty() {
            if let Some(obj) = f.metadata.as_object_mut() {
                for (k, v) in self.metadata {
                    obj.insert(k, v);
                }
            }
        }
        f
    }
}

impl Default for ContextFragmentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Token estimation (approximate)
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
    let density = if chars > 0 { non_ascii as f64 / chars as f64 } else { 0.0 };

    // Adjust divisor based on density: higher density → smaller divisor → more tokens
    let divisor = if density > 0.5 {
        2.0 // Dense code/non-ASCII
    } else {
        4.0 // English text
    };

    (chars as f64 / divisor).ceil() as usize
}

// ---------------------------------------------------------------------------
// ContextQueryResult
// ---------------------------------------------------------------------------

/// Result of a bounded context query — includes the budget state alongside
/// the fragments so the caller can decide whether to include more.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextQueryResult {
    /// The fragments returned by the query.
    pub fragments: Vec<ContextFragment>,
    /// The budget state after the query.
    pub budget: serde_json::Value,
    /// Whether the budget was exhausted during this query.
    pub budget_exhausted: bool,
    /// Total tokens consumed by these fragments.
    pub total_tokens: usize,
    /// P0-flagged fragments (for manual review).
    pub p0_flagged: Vec<String>,
}

impl ContextQueryResult {
    pub fn new(fragments: Vec<ContextFragment>, budget: &ContextBudget) -> Self {
        let total_tokens: usize = fragments.iter().map(|f| f.token_count).sum();
        let p0_flagged: Vec<String> = fragments
            .iter()
            .filter(|f| f.p0_flagged)
            .map(|f| f.id.clone())
            .collect();

        Self {
            fragments,
            budget: budget.summary(),
            budget_exhausted: budget.exhausted,
            total_tokens,
            p0_flagged,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- ContextFragment --

    #[test]
    fn test_fragment_new_valid() {
        let frag = ContextFragment::new(
            "test-1",
            "Test Fragment",
            "hello world",
            5,
        )
        .unwrap();
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
                used,
                remaining,
                ..
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
        let frag2 = ContextFragment::new(
            "f2",
            "Frag 2",
            "content2",
            P0_ALERT_TOKENS + 1,
        )
        .unwrap();
        let mut budget = ContextBudget::new(2000);
        budget.reserve(100).unwrap();
        budget.reserve(P0_ALERT_TOKENS + 1).unwrap();

        let result = ContextQueryResult::new(vec![frag1, frag2], &budget);
        assert_eq!(result.p0_flagged.len(), 1);
        assert_eq!(result.p0_flagged[0], "f2");
    }
}
