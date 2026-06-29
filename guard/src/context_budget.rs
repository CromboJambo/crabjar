//! Lightweight context budget tracker for gate-level enforcement.
//!
//! This is a minimal version of `memory::context::ContextBudget` used only
//! for gate checks. It avoids pulling the full memory crate into guard's
//! dependency graph. The budget is checked at gate time; the actual fragment
//! construction and token counting live in `memory/`.

/// Default cumulative context budget (tokens). 128K covers most models.
pub const DEFAULT_CONTEXT_BUDGET: usize = 128_000;

/// Warning threshold: 80% of budget. At this level, log a warning but
/// allow the action to proceed (per Q12: "leave pretty open").
pub const CONTEXT_BUDGET_WARN_PCT: f64 = 0.8;

/// ContextBudget tracks cumulative token usage across fragments.
///
/// Checked at the gate: if usage >= budget, the gate returns
/// `GateResult::ContextExhausted`. Between warn_pct and 100%, a warning
/// is logged but the action is allowed (loose bounds per Q12).
#[derive(Debug, Clone)]
pub struct ContextBudget {
    /// Total budget in tokens.
    budget: usize,
    /// Tokens consumed so far.
    used: usize,
    /// Whether the budget has been exhausted.
    exhausted: bool,
}

impl ContextBudget {
    /// Create a new context budget with the given token limit.
    pub fn new(budget: usize) -> Self {
        Self {
            budget,
            used: 0,
            exhausted: false,
        }
    }

    /// Create a budget with the default budget (128K tokens).
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

    /// Tokens consumed so far.
    pub fn used(&self) -> usize {
        self.used
    }

    /// Total budget.
    pub fn budget(&self) -> usize {
        self.budget
    }

    /// Whether the budget has been exhausted.
    pub fn is_exhausted(&self) -> bool {
        self.exhausted
    }

    /// Check whether a fragment of the given token count would fit.
    pub fn can_fit(&self, token_count: usize) -> bool {
        !self.exhausted && self.used.saturating_add(token_count) <= self.budget
    }

    /// Check whether we're approaching the budget limit (>= warn_pct).
    /// Returns `Some(remaining)` if approaching, `None` if safe.
    pub fn warn_if_approaching(&self) -> Option<usize> {
        let warn_threshold = (self.budget as f64 * CONTEXT_BUDGET_WARN_PCT) as usize;
        if self.used >= warn_threshold && !self.exhausted {
            Some(self.remaining())
        } else {
            None
        }
    }

    /// Reserve tokens for a new fragment.
    ///
    /// Returns `Err(remaining)` if the budget is exhausted.
    pub fn reserve(&mut self, token_count: usize) -> Result<(), usize> {
        if self.exhausted {
            return Err(self.remaining());
        }

        if !self.can_fit(token_count) {
            return Err(self.remaining());
        }

        self.used += token_count;

        if self.used >= self.budget {
            self.exhausted = true;
        }

        Ok(())
    }

    /// Release tokens back to the budget (for rollback or fragment removal).
    pub fn release(&mut self, token_count: usize) {
        self.used = self.used.saturating_sub(token_count);
        // Unset exhausted if we have remaining budget after release
        if self.used < self.budget {
            self.exhausted = false;
        }
    }
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self::default_budget()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_default() {
        let budget = ContextBudget::default();
        assert_eq!(budget.budget(), DEFAULT_CONTEXT_BUDGET);
        assert_eq!(budget.used(), 0);
        assert_eq!(budget.remaining(), DEFAULT_CONTEXT_BUDGET);
        assert!(!budget.is_exhausted());
    }

    #[test]
    fn test_budget_reserve() {
        let mut budget = ContextBudget::new(1000);
        assert!(budget.reserve(500).is_ok());
        assert_eq!(budget.used(), 500);
        assert_eq!(budget.remaining(), 500);
    }

    #[test]
    fn test_budget_exhaustion() {
        let mut budget = ContextBudget::new(1000);
        assert!(budget.reserve(1000).is_ok());
        assert!(budget.is_exhausted());
        assert_eq!(budget.remaining(), 0);
        assert!(budget.reserve(1).is_err());
        assert_eq!(budget.reserve(1).unwrap_err(), 0);
    }

    #[test]
    fn test_budget_warn_threshold() {
        let mut budget = ContextBudget::new(1000);
        // At 70%, no warning
        budget.reserve(700).unwrap();
        assert!(budget.warn_if_approaching().is_none());

        // At 800/1000 = 80%, warning triggered
        budget.reserve(100).unwrap();
        assert_eq!(budget.remaining(), 200);
    }

    #[test]
    fn test_budget_release() {
        let mut budget = ContextBudget::new(1000);
        budget.reserve(1000).unwrap();
        assert!(budget.is_exhausted());

        budget.release(500);
        assert!(!budget.is_exhausted());
        assert_eq!(budget.remaining(), 500);
    }

    #[test]
    fn test_budget_can_fit() {
        let budget = ContextBudget::new(1000);
        assert!(budget.can_fit(500));
        assert!(!budget.can_fit(1001));
    }
}
