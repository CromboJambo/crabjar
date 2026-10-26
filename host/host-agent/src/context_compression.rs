/// Context compression for the agent loop.
///
/// Between turns, the agent's observation array can grow unboundedly,
/// consuming context window and degrading model quality. This module
/// provides a `ContextCompressor` that condenses observations while
/// preserving critical information.
///
/// ## Compression Strategy
///
/// 1. **Keep recent**: Always retain the last N observations (configurable)
/// 2. **Summarize older**: Group older observations by stage/kind and summarize
/// 3. **Token budget**: Enforce a maximum token count (default: 4096)
/// 4. **Phase-aware**: Different phases may need different compression ratios
use crabjar_host_core::work_item::Observation;
use std::collections::HashMap;

use crate::semantic_compression::compress_observations;
use crate::structural_compression::compress_code;

/// Configuration for context compression.
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Maximum tokens to retain after compression (default: 4096).
    pub max_tokens: usize,
    /// Number of recent observations to keep uncompressed (default: 10).
    pub recent_count: usize,
    /// Whether to enable compression at all (default: true).
    pub enabled: bool,
    /// Maximum number of summary entries (default: 5).
    pub max_summaries: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            max_tokens: 4096,
            recent_count: 10,
            enabled: true,
            max_summaries: 5,
        }
    }
}

impl CompressionConfig {
    /// Create a config optimized for short conversations (fewer summaries).
    pub fn for_short_conversation() -> Self {
        Self {
            max_tokens: 2048,
            recent_count: 5,
            enabled: true,
            max_summaries: 3,
        }
    }

    /// Create a config optimized for long conversations (more retention).
    pub fn for_long_conversation() -> Self {
        Self {
            max_tokens: 8192,
            recent_count: 20,
            enabled: true,
            max_summaries: 10,
        }
    }

    /// Disable compression entirely.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }
}

/// Compresses a list of observations into a condensed context string.
///
/// Returns the compressed context, or the original observations if
/// compression is disabled or the input is small enough.
pub struct ContextCompressor {
    config: CompressionConfig,
}

impl ContextCompressor {
    /// Create a new compressor with the given config.
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    /// Create with default config.
    pub fn default_compressor() -> Self {
        Self::new(CompressionConfig::default())
    }

    /// Compress observations into a context string.
    ///
    /// Strategy:
    /// 1. If compression is disabled or observation count < recent_count, return raw
    /// 2. Keep the last `recent_count` observations as-is
    /// 3. Group remaining observations by stage/kind
    /// 4. Generate summaries for each group
    /// 5. Enforce token budget
    pub fn compress(&self, observations: &[Observation]) -> String {
        if !self.config.enabled {
            return Self::observations_to_string(observations);
        }

        if observations.len() <= self.config.recent_count {
            return Self::observations_to_string(observations);
        }

        let recent = &observations[observations.len() - self.config.recent_count..];
        let older = &observations[..observations.len() - self.config.recent_count];

        let mut result = String::new();

        // Add summaries of older observations
        let summaries = self.summarize(older);
        if !summaries.is_empty() {
            result.push_str("## Summary of earlier observations:\n");
            for summary in &summaries {
                result.push_str(summary);
                result.push('\n');
            }
            result.push('\n');
        }

        // Add recent observations
        result.push_str("## Recent observations:\n");
        result.push_str(&Self::observations_to_string(recent));

        // Enforce token budget (rough estimate: ~4 chars per token)
        let estimated_tokens = result.len() / 4;
        if estimated_tokens > self.config.max_tokens {
            // Truncate to fit budget
            let target_len = self.config.max_tokens * 4;
            if result.len() > target_len {
                result.truncate(target_len);
                result.push_str("\n[truncated — context budget exceeded]");
            }
        }

        result
    }

    /// Summarize older observations by grouping them by stage/kind and applying semantic clustering.
    fn summarize(&self, observations: &[Observation]) -> Vec<String> {
        // Group by stage
        let mut by_stage: HashMap<&str, Vec<&Observation>> = HashMap::new();
        for obs in observations {
            by_stage.entry(obs.stage.as_str()).or_default().push(obs);
        }

        // Convert to sorted vec for deterministic output
        let mut stages: Vec<_> = by_stage.into_iter().collect();
        stages.sort_by_key(|(stage, _)| stage.to_string());

        let mut summaries = Vec::new();

        for (stage, obs_list) in stages.into_iter() {
            if summaries.len() >= self.config.max_summaries {
                break;
            }

            // Group by kind within stage
            let mut by_kind: HashMap<&str, Vec<&Observation>> = HashMap::new();
            for obs in &obs_list {
                by_kind.entry(obs.kind.as_str()).or_default().push(obs);
            }

            for (kind, kind_obs) in by_kind.into_iter() {
                if summaries.len() >= self.config.max_summaries {
                    break;
                }

                // Apply semantic clustering to merge similar observations
                let kind_obs_owned: Vec<Observation> = kind_obs.iter().map(|o| (*o).clone()).collect();
                let cluster_result = compress_observations(&kind_obs_owned);

                // Generate details from clustered observations
                let details: Vec<_> = cluster_result
                    .compressed
                    .iter()
                    .filter_map(|o| {
                        let trimmed = o.details.trim();
                        if trimmed.is_empty() {
                            return None;
                        }
                        // Apply structural compression to code-heavy content
                        let compressed = self.compress_observation(trimmed);
                        Some(compressed)
                    })
                    .collect();

                if details.is_empty() {
                    continue;
                }

                // If clustering merged observations, note the reduction
                let cluster_note = if cluster_result.cluster_count < kind_obs.len() {
                    format!(
                        " ({} similar merged into {})",
                        kind_obs.len(),
                        cluster_result.cluster_count
                    )
                } else {
                    String::new()
                };

                summaries.push(format!(
                    "- **{}** ({} items){}: {}",
                    stage,
                    kind,
                    cluster_note,
                    details
                        .iter()
                        .map(|d| if d.len() > 80 { &d[..80] } else { d })
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
        }

        summaries
    }

    /// Compress a single observation's details using structural tokenization.
    /// For code-heavy content, this preserves declarations and control flow while
    /// eliding large function bodies.
    fn compress_observation(&self, text: &str) -> String {
        // Heuristic: if the text contains code-like patterns (fn, struct, impl, etc.)
        // and is long enough to benefit from compression, use structural compression.
        let has_code_patterns = text.contains("fn ")
            || text.contains("struct ")
            || text.contains("impl ")
            || text.contains("let ")
            || text.contains("return ");

        if !has_code_patterns || text.len() < 100 {
            // Not code-heavy or too short to benefit — simple truncation
            return if text.len() > 200 {
                format!("... {}", &text[..200])
            } else {
                text.to_string()
            };
        }

        // Use structural compression with budget
        let result = compress_code(text, 30);
        if result.was_compressed() {
            return format!("[compressed code: {} elisions]", result.elided_count);
        }
        // No compression happened — fall back to simple truncation
        if text.len() > 200 {
            format!("... {}", &text[..200])
        } else {
            text.to_string()
        }
    }

    /// Convert observations to a string representation.
    fn observations_to_string(observations: &[Observation]) -> String {
        if observations.is_empty() {
            return String::new();
        }

        let mut result = String::new();
        for obs in observations {
            let ts = obs.timestamp.format("%H:%M:%S");
            let details = if obs.details.len() > 100 {
                format!("{}...", &obs.details[..100])
            } else {
                obs.details.clone()
            };
            result.push_str(&format!(
                "- [{}] {} ({}): {}\n",
                ts, obs.stage, obs.kind, details
            ));
        }
        result
    }

    /// Get the estimated token count of the compressed context.
    pub fn estimated_tokens(&self, observations: &[Observation]) -> usize {
        let compressed = self.compress(observations);
        compressed.len() / 4
    }

    /// Get a reference to the compression config.
    pub fn config(&self) -> &CompressionConfig {
        &self.config
    }
}

impl Default for ContextCompressor {
    fn default() -> Self {
        Self::default_compressor()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_obs(stage: &str, kind: &str, details: &str) -> Observation {
        Observation {
            id: uuid::Uuid::new_v4(),
            stage: stage.to_string(),
            kind: kind.to_string(),
            details: details.to_string(),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_compress_disabled() {
        let config = CompressionConfig::disabled();
        let compressor = ContextCompressor::new(config);

        let obs: Vec<_> = (0..5)
            .map(|i| make_obs("observe", "state", &format!("observation {}", i)))
            .collect();

        let compressed = compressor.compress(&obs);
        assert!(!compressed.is_empty());
    }

    #[test]
    fn test_compress_small_input() {
        let config = CompressionConfig::default();
        let compressor = ContextCompressor::new(config);

        // Fewer observations than recent_count — should not compress
        let obs: Vec<_> = (0..3)
            .map(|i| make_obs("observe", "state", &format!("observation {}", i)))
            .collect();

        let compressed = compressor.compress(&obs);
        assert!(compressed.contains("observation 0"));
        assert!(compressed.contains("observation 1"));
        assert!(compressed.contains("observation 2"));
    }

    #[test]
    fn test_compress_large_input() {
        let config = CompressionConfig {
            recent_count: 3,
            max_summaries: 2,
            ..CompressionConfig::default()
        };
        let compressor = ContextCompressor::new(config);

        // Many observations — should compress
        let obs: Vec<_> = (0..20)
            .map(|i| make_obs("observe", "state", &format!("observation {}", i)))
            .collect();

        let compressed = compressor.compress(&obs);
        assert!(compressed.contains("Summary of earlier"));
        assert!(compressed.contains("Recent observations"));
    }

    #[test]
    fn test_compress_empty() {
        let compressor = ContextCompressor::default_compressor();
        let result = compressor.compress(&[]);
        assert_eq!(result, "");
    }

    #[test]
    fn test_compressor_default() {
        let compressor = ContextCompressor::default_compressor();
        assert!(compressor.compress(&[]) == compressor.compress(&[]));
    }

    #[test]
    fn test_summary_grouping() {
        let config = CompressionConfig {
            recent_count: 2,
            max_summaries: 5,
            ..CompressionConfig::default()
        };
        let compressor = ContextCompressor::new(config);

        let obs: Vec<_> = (0..10)
            .map(|i| {
                let stage = if i < 5 { "plan" } else { "execute" };
                let kind = if i % 2 == 0 { "tool" } else { "result" };
                make_obs(stage, kind, &format!("detail {}", i))
            })
            .collect();

        let compressed = compressor.compress(&obs);
        assert!(compressed.contains("Summary of earlier"));
        assert!(compressed.contains("plan"));
        assert!(compressed.contains("execute"));
    }

    #[test]
    fn test_token_budget_enforcement() {
        let config = CompressionConfig {
            max_tokens: 100, // ~400 chars
            recent_count: 3,
            ..CompressionConfig::default()
        };
        let compressor = ContextCompressor::new(config.clone());

        let obs: Vec<_> = (0..50)
            .map(|i| make_obs("observe", "state", &format!("observation {}", i)))
            .collect();

        let compressed = compressor.compress(&obs);
        // Should be truncated to fit budget
        assert!(compressed.len() <= config.max_tokens * 4 + 50); // +50 for truncation message
    }

    #[test]
    fn test_config_short_conversation() {
        let config = CompressionConfig::for_short_conversation();
        assert_eq!(config.max_tokens, 2048);
        assert_eq!(config.recent_count, 5);
        assert_eq!(config.max_summaries, 3);
        assert!(config.enabled);
    }

    #[test]
    fn test_config_long_conversation() {
        let config = CompressionConfig::for_long_conversation();
        assert_eq!(config.max_tokens, 8192);
        assert_eq!(config.recent_count, 20);
        assert_eq!(config.max_summaries, 10);
        assert!(config.enabled);
    }

    #[test]
    fn test_compression_disabled_config() {
        let config = CompressionConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_structural_compression_for_code_observations() {
        let config = CompressionConfig {
            recent_count: 3,
            max_summaries: 5,
            ..CompressionConfig::default()
        };
        let compressor = ContextCompressor::new(config);

        // Create observations with code-heavy content that should trigger structural compression
        let large_function = "fn process_large_data(items: Vec<i32>) -> i32 {
            let mut sum = 0;
            for item in items {
                sum += item;
            }
            return sum * 2;
        }";

        // Place code observation early enough to be in "older" bucket (indices 0-11 with recent_count=3)
        let obs: Vec<_> = (0..15)
            .map(|i| {
                if i == 5 {
                    make_obs("execute", "tool", large_function)
                } else {
                    make_obs(
                        "observe",
                        "state",
                        &format!("observation {} — some regular text", i),
                    )
                }
            })
            .collect();

        let compressed = compressor.compress(&obs);

        // Should have summarized older observations including the code-heavy one
        assert!(compressed.contains("Summary of earlier"));
        // The code observation should have been structurally compressed
        // (not just truncated to first 200 chars)
        assert!(!compressed.contains("let mut sum = 0"));
    }

    #[test]
    fn test_non_code_observations_use_simple_truncation() {
        let config = CompressionConfig {
            recent_count: 3,
            max_summaries: 5,
            ..CompressionConfig::default()
        };
        let compressor = ContextCompressor::new(config);

        // Long non-code text
        let long_text = "This is a long observation with no code patterns. It's just regular text that goes on and on without any Rust-like keywords that would trigger structural compression.";

        let obs_code = make_obs("execute", "tool", long_text);

        let obs: Vec<_> = (0..15)
            .map(|i| {
                if i == 12 {
                    return obs_code.clone();
                }
                make_obs(
                    "observe",
                    "state",
                    &format!("observation {} — some regular text", i),
                )
            })
            .collect();

        let compressed = compressor.compress(&obs);

        // Non-code observations use simple truncation
        assert!(compressed.contains("Summary of earlier"));
        // Should truncate but not apply structural compression markers
        assert!(!compressed.contains("[compressed code:"));
    }
}
