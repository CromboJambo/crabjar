// crabjar/memory/src/context/budget.rs
// ContextBudget and ContextQueryResult — cumulative token tracking.

use serde::{Deserialize, Serialize};

/// Tracks cumulative token usage across fragments.
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
        use crate::context::constants::DEFAULT_CONTEXT_BUDGET;
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
    pub fn reserve(&mut self, token_count: usize) -> Result<(), super::ContextError> {
        if self.exhausted {
            return Err(super::ContextError::ExceedsBudget {
                used: self.used,
                budget: self.budget,
                remaining: 0,
            });
        }

        if !self.can_fit(token_count) {
            return Err(super::ContextError::ExceedsBudget {
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

/// Result of a bounded context query — includes the budget state alongside
/// the fragments so the caller can decide whether to include more.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextQueryResult {
    /// The fragments returned by the query.
    pub fragments: Vec<crate::context::fragment::ContextFragment>,
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
    pub fn new(
        fragments: Vec<crate::context::fragment::ContextFragment>,
        budget: &ContextBudget,
    ) -> Self {
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
