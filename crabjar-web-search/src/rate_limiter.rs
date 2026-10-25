//! Rate limiting logic for web search backends.

use std::time::{Duration, Instant};

/// Simple rate limiter that tracks requests per time window.
pub struct RateLimiter {
    max_requests: usize,
    window_duration: Duration,
    last_reset: Instant,
    request_count: usize,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_duration: Duration) -> Self {
        Self {
            max_requests,
            window_duration,
            last_reset: Instant::now(),
            request_count: 0,
        }
    }

    /// Check if a request is allowed. Returns Ok(()) if allowed, Err(WebSearchError) if rate limited.
    pub fn check(&mut self) -> Result<(), String> {
        let elapsed = self.last_reset.elapsed();
        
        // Reset counter if window has passed
        if elapsed >= self.window_duration {
            self.request_count = 0;
            self.last_reset = Instant::now();
        }

        if self.request_count >= self.max_requests {
            return Err(format!(
                "Rate limit exceeded: {} requests in {:?} window",
                self.request_count, self.window_duration
            ));
        }

        self.request_count += 1;
        Ok(())
    }
}