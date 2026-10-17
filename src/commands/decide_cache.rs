//! JEV Decision Cache Manager
//!
//! Stores and retrieves prior decisions for similar observations.
//! Uses SHA-256 hashing of observation text as cache key.
//! Implements TTL-based expiration and LRU eviction.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Cached decision entry with metadata for efficiency tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedDecision {
    /// Original observation text (truncated to 256 chars for storage)
    pub observation_preview: String,
    /// Full SHA-256 hash of the original observation
    pub observation_hash: String,
    /// Decision result (option ID selected)
    pub decision_option: String,
    /// Confidence score at time of caching
    pub confidence: f64,
    /// Components invoked to reach this decision
    pub components_invoked: Vec<String>,
    /// Computation time in milliseconds
    pub computation_time_ms: u64,
    /// Timestamp when cached (Unix epoch seconds)
    pub cached_at: u64,
    /// Time-to-live in seconds (0 = use default)
    pub ttl_seconds: u64,
    /// Number of times this cache entry has been hit
    pub hit_count: u64,
}

/// Cache statistics for efficiency metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_inserts: u64,
    pub total_hits: u64,
    pub total_misses: u64,
    pub total_evictions: u64,
    pub total_expirations: u64,
    pub current_size: usize,
    pub last_accessed_at: Option<u64>,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_hits + self.total_misses;
        if total == 0 {
            return 0.0;
        }
        self.total_hits as f64 / total as f64
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "total_inserts": self.total_inserts,
            "total_hits": self.total_hits,
            "total_misses": self.total_misses,
            "total_evictions": self.total_evictions,
            "total_expirations": self.total_expirations,
            "current_size": self.current_size,
            "hit_rate": self.hit_rate(),
            "last_accessed_at": self.last_accessed_at
        })
    }
}

/// Decision cache with TTL expiration and LRU eviction
pub struct DecisionCache {
    /// Map from observation hash to cached decision
    entries: HashMap<String, CachedDecision>,
    /// LRU order (most recently used at front)
    lru_order: VecDeque<String>,
    /// Maximum number of entries before eviction
    max_size: usize,
    /// Default TTL in seconds
    default_ttl_seconds: u64,
    /// Cache statistics
    stats: CacheStats,
}

impl DecisionCache {
    /// Create a new decision cache with given capacity and TTL
    pub fn new(max_size: usize, default_ttl_seconds: u64) -> Self {
        Self {
            entries: HashMap::new(),
            lru_order: VecDeque::new(),
            max_size,
            default_ttl_seconds,
            stats: CacheStats {
                total_inserts: 0,
                total_hits: 0,
                total_misses: 0,
                total_evictions: 0,
                total_expirations: 0,
                current_size: 0,
                last_accessed_at: None,
            },
        }
    }

    /// Get current Unix timestamp in seconds
    fn now_seconds() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Check if a cache entry has expired
    fn is_expired(entry: &CachedDecision) -> bool {
        let ttl = if entry.ttl_seconds > 0 {
            entry.ttl_seconds
        } else {
            // Use default TTL (passed separately)
            return false;
        };

        let now = Self::now_seconds();
        now - entry.cached_at >= ttl
    }

    /// Look up a cached decision by observation hash
    pub fn get(&mut self, hash: &str) -> Option<CachedDecision> {
        let now = Self::now_seconds();
        self.stats.last_accessed_at = Some(now);

        // Check if entry exists and hasn't expired
        match self.entries.get(hash) {
            Some(entry) if !Self::is_expired(entry) => {
                // Update LRU order (move to front)
                self.lru_order.retain(|h| h != hash);
                self.lru_order.push_front(hash.to_string());

                // Increment hit count and stats
                let mut entry = entry.clone();
                entry.hit_count += 1;
                let result = entry.clone();
                self.entries.insert(hash.to_string(), entry);
                self.stats.total_hits += 1;

                Some(result)
            }
            Some(_) => {
                // Entry exists but is expired
                self.evict_expired();
                self.stats.total_expirations += 1;
                self.stats.total_misses += 1;
                None
            }
            None => {
                self.stats.total_misses += 1;
                None
            }
        }
    }

    /// Store a decision in the cache
    pub fn set(&mut self, hash: String, entry: CachedDecision) {
        // Remove existing entry if present
        if self.entries.contains_key(&hash) {
            self.lru_order.retain(|h| h != &hash);
        }

        // Evict if at capacity
        while self.entries.len() >= self.max_size {
            self.evict_lru();
        }

        // Insert new entry
        self.lru_order.push_front(hash);
        self.entries.insert(entry.observation_hash.clone(), entry);
        self.stats.total_inserts += 1;
        self.stats.current_size = self.entries.len();
    }

    /// Evict least recently used entry
    fn evict_lru(&mut self) {
        if let Some(lru_key) = self.lru_order.pop_back() {
            self.entries.remove(&lru_key);
            self.stats.total_evictions += 1;
            self.stats.current_size = self.entries.len();
        }
    }

    /// Evict all expired entries
    fn evict_expired(&mut self) {
        let now = Self::now_seconds();
        let default_ttl = self.default_ttl_seconds;

        let expired_keys: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, entry)| {
                let ttl = if entry.ttl_seconds > 0 {
                    entry.ttl_seconds
                } else {
                    default_ttl
                };
                now - entry.cached_at >= ttl
            })
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            self.lru_order.retain(|h| h != &key);
            self.entries.remove(&key);
            self.stats.total_expirations += 1;
        }

        self.stats.current_size = self.entries.len();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        self.stats.clone()
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.lru_order.clear();
        self.stats.current_size = 0;
    }

    /// Get current cache size
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(hash: &str, option: &str) -> CachedDecision {
        CachedDecision {
            observation_preview: "test obs".to_string(),
            observation_hash: hash.to_string(),
            decision_option: option.to_string(),
            confidence: 0.85,
            components_invoked: vec!["embedding".to_string()],
            computation_time_ms: 150,
            cached_at: DecisionCache::now_seconds(),
            ttl_seconds: 0,
            hit_count: 0,
        }
    }

    #[test]
    fn cache_set_and_get() {
        let mut cache = DecisionCache::new(10, 3600);
        cache.set(
            "hash123".to_string(),
            make_entry("hash123", "restart_service"),
        );

        let retrieved = cache.get("hash123");
        assert!(retrieved.is_some());
        let r = retrieved.unwrap();
        assert_eq!(r.decision_option, "restart_service");
        assert_eq!(r.hit_count, 1);
    }

    #[test]
    fn cache_miss() {
        let mut cache = DecisionCache::new(10, 3600);
        let result = cache.get("nonexistent");
        assert!(result.is_none());
        assert_eq!(cache.stats().total_misses, 1);
    }

    #[test]
    fn cache_lru_eviction() {
        let mut cache = DecisionCache::new(2, 3600);
        cache.set("h1".to_string(), make_entry("h1", "opt1"));
        cache.set("h2".to_string(), make_entry("h2", "opt2"));

        // Access h1 to make it recently used
        let _ = cache.get("h1");

        // Add third entry — should evict h2 (LRU)
        cache.set("h3".to_string(), make_entry("h3", "opt3"));

        assert!(cache.get("h1").is_some());
        assert!(cache.get("h2").is_none());
        assert!(cache.get("h3").is_some());
        assert_eq!(cache.stats().total_evictions, 1);
    }

    #[test]
    fn cache_hit_rate() {
        let mut cache = DecisionCache::new(10, 3600);
        cache.set("h".to_string(), make_entry("h", "opt"));

        let _ = cache.get("h"); // Hit
        let _ = cache.get("h"); // Hit
        let _ = cache.get("miss"); // Miss
        let _ = cache.get("h"); // Hit

        let stats = cache.stats();
        assert_eq!(stats.total_hits, 3);
        assert_eq!(stats.total_misses, 1);
        assert!((stats.hit_rate() - 0.75).abs() < 0.001);
    }
}
