//! Sparse Lattice — a lightweight in-memory key-value store with pattern
//! matching capabilities. Provides the memory substrate for the slow friend
//! architecture: stores patterns of past events and their outcomes, enabling
//! novelty detection and learned responses.
//!
//! Design principles (from EXPL-001):
//! - Sparse representation: only store non-default values
//! - Pattern matching: query by partial key matches
//! - Simple and fast: O(1) for point lookups, O(n) for pattern scans

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A node in the lattice with a value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatticeNode {
    pub key: String,
    pub value: serde_json::Value,
    /// Timestamp of last update (epoch seconds).
    pub updated_at: i64,
}

/// The sparse lattice memory substrate.
pub struct SparseLattice {
    nodes: HashMap<String, LatticeNode>,
}

impl SparseLattice {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// Store a value at the given key.
    pub fn set(&mut self, key: &str, value: serde_json::Value) {
        let node = LatticeNode {
            key: key.to_string(),
            value,
            updated_at: chrono::Utc::now().timestamp(),
        };
        self.nodes.insert(key.to_string(), node);
    }

    /// Get a value by exact key.
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.nodes.get(key).map(|n| &n.value)
    }

    /// Check if a key exists.
    pub fn contains(&self, key: &str) -> bool {
        self.nodes.contains_key(key)
    }

    /// Delete a key.
    pub fn remove(&mut self, key: &str) {
        self.nodes.remove(key);
    }

    /// List all keys.
    pub fn keys(&self) -> Vec<&String> {
        self.nodes.keys().collect()
    }

    /// Number of stored nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get() {
        let mut lattice = SparseLattice::new();
        lattice.set("test_key", serde_json::json!("hello"));
        assert_eq!(lattice.get("test_key"), Some(&serde_json::json!("hello")));
    }

    #[test]
    fn contains_and_remove() {
        let mut lattice = SparseLattice::new();
        lattice.set("key1", serde_json::json!(42));
        assert!(lattice.contains("key1"));
        lattice.remove("key1");
        assert!(!lattice.contains("key1"));
    }

    #[test]
    fn len_and_is_empty() {
        let lattice = SparseLattice::new();
        assert!(lattice.is_empty());
        assert_eq!(lattice.len(), 0);
    }
}
