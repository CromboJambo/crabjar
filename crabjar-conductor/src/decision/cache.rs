//! Decision Result Cache
//!
//! LRU cache with TTL for JEV decision results to reduce redundant embedding
//! API calls. Similar observations against the same criteria file are hashed
//! and cached, providing significant latency improvements for recurring patterns.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use sha2::{Digest, Sha256};

use super::evaluate::DecisionResult;

/// Cache entry with TTL tracking
#[derive(Debug)]
struct CacheEntry {
    result: DecisionResult,
    inserted_at: Instant,
}

/// Decision cache with LRU eviction and TTL expiration
pub struct DecisionCache {
    entries: HashMap<String, CacheEntry>,
    max_entries: usize,
    ttl: Duration,
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl DecisionCache {
    /// Create a new decision cache
    ///
    /// # Arguments
    /// * `max_entries` - Maximum number of cached decisions before LRU eviction
    /// * `ttl` - Time-to-live for each cached entry
    pub fn new(max_entries: usize, ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
            ttl,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// Default cache configuration: 1000 entries, 24h TTL
    pub fn default_config() -> Self {
        Self::new(1000, Duration::from_secs(86400))
    }

    /// Compute cache key from observation and criteria file
    fn compute_key(observation: &str, criteria_file: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(observation.as_bytes());
        hasher.update(b"::"); // delimiter
        hasher.update(criteria_file.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    /// Get cached decision result, or None if not found/expired
    pub fn get(&mut self, observation: &str, criteria_file: &str) -> Option<DecisionResult> {
        let key = Self::compute_key(observation, criteria_file);

        if let Some(entry) = self.entries.get(&key) {
            // Check TTL expiration
            if entry.inserted_at.elapsed() < self.ttl {
                self.hits += 1;
                return Some(entry.result.clone());
            } else {
                // Expired - evict
                self.entries.remove(&key);
                self.evictions += 1;
            }
        }

        self.misses += 1;
        None
    }

    /// Store a decision result in the cache
    pub fn set(&mut self, observation: &str, criteria_file: &str, result: DecisionResult) {
        let key = Self::compute_key(observation, criteria_file);

        // LRU eviction if at capacity
        if self.entries.len() >= self.max_entries && !self.entries.contains_key(&key) {
            // Evict oldest entry (simple approach: evict first found)
            if let Some(oldest_key) = self.entries.keys().next().cloned() {
                self.entries.remove(&oldest_key);
                self.evictions += 1;
            }
        }

        self.entries.insert(key, CacheEntry {
            result,
            inserted_at: Instant::now(),
        });
    }

    /// Get cache hit rate as a percentage (0.0 - 100.0)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        (self.hits as f64 / total as f64) * 100.0
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.entries.len(),
            max_entries: self.max_entries,
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            hit_rate: self.hit_rate(),
        }
    }

    /// Clear all cached entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.evictions += 1; // Count as a bulk eviction
    }
}

/// Cache statistics for monitoring and reporting
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub max_entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub hit_rate: f64,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DecisionCache [{} entries, {:.1}% hit rate, {} hits, {} misses, {} evictions]",
            self.entries, self.hit_rate, self.hits, self.misses, self.evictions
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_result(option: &str) -> DecisionResult {
        DecisionResult {
            option: option.to_string(),
            confidence: 0.95,
            rationale: "test".to_string(),
        }
    }

    #[test]
    fn cache_set_and_get() {
        let mut cache = DecisionCache::new(10, Duration::from_secs(3600));
        let result = test_result("proceed");

        cache.set("observation", "criteria.json", result);
        let cached = cache.get("observation", "criteria.json");

        assert!(cached.is_some());
        assert_eq!(cached.unwrap().option, "proceed");
    }

    #[test]
    fn cache_miss_on_different_observation() {
        let mut cache = DecisionCache::new(10, Duration::from_secs(3600));
        cache.set("obs1", "criteria.json", test_result("proceed"));

        let cached = cache.get("obs2", "criteria.json");
        assert!(cached.is_none());
    }

    #[test]
    fn cache_miss_on_different_criteria() {
        let mut cache = DecisionCache::new(10, Duration::from_secs(3600));
        cache.set("obs", "criteria1.json", test_result("proceed"));

        let cached = cache.get("obs", "criteria2.json");
        assert!(cached.is_none());
    }

    #[test]
    fn cache_ttl_expiration() {
        let mut cache = DecisionCache::new(10, Duration::from_millis(10));
        cache.set("obs", "criteria.json", test_result("proceed"));

        std::thread::sleep(Duration::from_millis(50));

        let cached = cache.get("obs", "criteria.json");
        assert!(cached.is_none());
    }

    #[test]
    fn cache_lru_eviction() {
        let mut cache = DecisionCache::new(2, Duration::from_secs(3600));
        cache.set("obs1", "c.json", test_result("a"));
        cache.set("obs2", "c.json", test_result("b"));

        // Adding third should evict one
        cache.set("obs3", "c.json", test_result("c"));
        assert_eq!(cache.entries.len(), 2);
    }

    #[test]
    fn cache_hit_rate_tracking() {
        let mut cache = DecisionCache::new(10, Duration::from_secs(3600));
        cache.set("obs", "c.json", test_result("proceed"));

        // One hit
        cache.get("obs", "c.json");
        // One miss
        cache.get("other", "c.json");

        assert_eq!(cache.hit_rate(), 50.0);
    }
}
