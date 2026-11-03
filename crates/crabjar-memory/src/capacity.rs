//! Character budget tracking and enforcement for memory stores.

use super::{MEMORY_CHAR_LIMIT, USER_CHAR_LIMIT};

/// Capacity information for a memory store.
#[derive(Debug, Clone)]
pub struct MemoryCapacity {
    /// Current character count (including delimiters).
    pub current_chars: usize,
    /// Maximum allowed characters.
    pub max_chars: usize,
}

impl MemoryCapacity {
    /// Create capacity tracker for the memory store (agent notes).
    #[must_use]
    pub fn for_memory() -> Self {
        MemoryCapacity {
            current_chars: 0,
            max_chars: MEMORY_CHAR_LIMIT,
        }
    }

    /// Create capacity tracker for the user store (user profile).
    #[must_use]
    pub fn for_user() -> Self {
        MemoryCapacity {
            current_chars: 0,
            max_chars: USER_CHAR_LIMIT,
        }
    }

    /// Update current character count from file content.
    pub fn set_current(&mut self, content: &str) {
        self.current_chars = content.chars().count();
    }

    /// Check if adding content would exceed the limit.
    ///
    /// `additional_content` is the new entry text plus delimiter overhead.
    #[must_use]
    pub fn would_fit(&self, additional_content: &str) -> bool {
        let additional_chars = additional_content.chars().count();
        self.current_chars + additional_chars <= self.max_chars
    }

    /// Calculate how many characters would be exceeded.
    #[must_use]
    pub fn exceed_by(&self, additional_content: &str) -> usize {
        let additional_chars = additional_content.chars().count();
        if self.would_fit(additional_content) {
            return 0;
        }
        self.current_chars + additional_chars - self.max_chars
    }

    /// Get usage percentage (0-100).
    #[must_use]
    pub fn usage_percent(&self) -> u32 {
        if self.max_chars == 0 {
            return 0;
        }
        ((self.current_chars as f64 / self.max_chars as f64) * 100.0).round() as u32
    }

    /// Get remaining characters.
    #[must_use]
    pub fn remaining(&self) -> usize {
        if self.current_chars >= self.max_chars {
            return 0;
        }
        self.max_chars - self.current_chars
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_capacity_limits() {
        let cap = MemoryCapacity::for_memory();
        assert_eq!(cap.max_chars, MEMORY_CHAR_LIMIT);
        assert_eq!(cap.max_chars, 2200);
    }

    #[test]
    fn test_user_capacity_limits() {
        let cap = MemoryCapacity::for_user();
        assert_eq!(cap.max_chars, USER_CHAR_LIMIT);
        assert_eq!(cap.max_chars, 1375);
    }

    #[test]
    fn test_would_fit() {
        let mut cap = MemoryCapacity::for_memory();
        cap.set_current("Existing content");
        assert!(cap.would_fit("Small addition"));
        assert!(!cap.would_fit(&"X".repeat(3000)));
    }

    #[test]
    fn test_usage_percent() {
        let mut cap = MemoryCapacity::for_memory();
        cap.set_current(&"X".repeat(1100)); // 50% of 2200
        assert_eq!(cap.usage_percent(), 50);
    }
}
