//! Context retrieval tools for review agents.
//! Bounded tool calls to prevent token explosion during review.

use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Tracks tool call usage per bundle to enforce limits.
pub struct ToolCallTracker {
    calls_per_bundle: HashMap<String, u32>,
    limit: u32,
}

impl ToolCallTracker {
    pub fn new(limit: u32) -> Self {
        Self {
            calls_per_bundle: HashMap::new(),
            limit,
        }
    }

    /// Record a tool call for a bundle. Returns Ok(()) if within limit, Err if exceeded.
    pub fn record_call(&mut self, bundle_id: &str) -> Result<()> {
        let count = self
            .calls_per_bundle
            .entry(bundle_id.to_string())
            .or_insert(0);
        *count += 1;
        if *count > self.limit {
            anyhow::bail!(
                "Context retrieval limit exceeded for bundle {} ({} calls, limit {})",
                bundle_id,
                count,
                self.limit
            );
        }
        Ok(())
    }

    /// Get remaining calls for a bundle.
    pub fn remaining(&self, bundle_id: &str) -> u32 {
        let used = *self.calls_per_bundle.get(bundle_id).unwrap_or(&0);
        self.limit.saturating_sub(used)
    }
}

/// Read a file's contents for context retrieval.
pub fn read_file_at_path(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))
}

/// Search the codebase for a pattern (simplified grep-like search).
/// In production, this would use ripgrep or similar.
pub fn search_codebase(root: &Path, pattern: &str) -> Result<Vec<String>> {
    let mut results = Vec::new();
    search_dir(root, pattern, &mut results)?;
    Ok(results)
}

fn search_dir(dir: &Path, pattern: &str, results: &mut Vec<String>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip common non-source directories
        if path.is_dir() {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name == "target" || name == ".git" || name == "node_modules" {
                continue;
            }
            search_dir(&path, pattern, results)?;
        } else if path
            .extension()
            .map_or(false, |ext| ext == "rs" || ext == "toml" || ext == "md")
        {
            if let Ok(content) = fs::read_to_string(&path) {
                for (line_num, line) in content.lines().enumerate() {
                    if line.contains(pattern) {
                        results.push(format!(
                            "{}:{}: {}",
                            path.display(),
                            line_num + 1,
                            line.trim()
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

/// List all changed files in a review session for cross-file context.
pub fn list_changed_files(bundles: &[crate::types::ReviewBundle]) -> Vec<String> {
    let mut files = Vec::new();
    for bundle in bundles {
        for file in &bundle.files {
            files.push(file.path.display().to_string());
        }
    }
    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_call_tracker_enforces_limit() {
        let mut tracker = ToolCallTracker::new(3);
        assert!(tracker.record_call("bundle-1").is_ok());
        assert!(tracker.record_call("bundle-1").is_ok());
        assert!(tracker.record_call("bundle-1").is_ok());
        assert_eq!(tracker.remaining("bundle-1"), 0);
        assert!(tracker.record_call("bundle-1").is_err());
    }

    #[test]
    fn tool_call_tracker_per_bundle() {
        let mut tracker = ToolCallTracker::new(2);
        assert!(tracker.record_call("bundle-1").is_ok());
        assert!(tracker.record_call("bundle-2").is_ok());
        assert_eq!(tracker.remaining("bundle-1"), 1);
        assert_eq!(tracker.remaining("bundle-2"), 1);
    }
}
