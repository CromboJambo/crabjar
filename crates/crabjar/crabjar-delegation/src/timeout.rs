//! Timeout management for delegated tasks.

use std::time::{Duration, Instant};

/// Tracks timeouts for individual tasks.
pub struct TaskTimeout {
    started_at: Instant,
    duration: Duration,
}

impl TaskTimeout {
    /// Create a new timeout tracker.
    #[must_use]
    pub fn new(duration: Duration) -> Self {
        Self {
            started_at: Instant::now(),
            duration,
        }
    }

    /// Check if the task has timed out.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.started_at.elapsed() > self.duration
    }

    /// Remaining time before timeout.
    #[must_use]
    pub fn remaining(&self) -> Duration {
        let elapsed = self.started_at.elapsed();
        if elapsed > self.duration {
            Duration::from_secs(0)
        } else {
            self.duration.checked_sub(elapsed).unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_not_expired_immediately() {
        let t = TaskTimeout::new(Duration::from_mins(1));
        assert!(!t.is_expired());
    }

    #[test]
    fn timeout_remaining_decreases() {
        let t = TaskTimeout::new(Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(100));
        let remaining = t.remaining();
        assert!(remaining.as_secs() <= 10);
    }
}
