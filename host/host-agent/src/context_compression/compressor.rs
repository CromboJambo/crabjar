use crabjar_host_core::work_item::Observation;
use std::collections::HashMap;

use crate::semantic_compression::{compress_observations, CompressedKnowledge};
use crate::structural_compression::compress_code;

use super::config::CompressionConfig;

/// Compresses a list of observations into a condensed context string.
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

    /// Compress observations into a context string and derive knowledge items.
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
        let (summaries, _knowledge) = summarize(&self.config, older);
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

    /// Compress observations and return both the compressed string and derived knowledge items.
    pub fn compress_with_knowledge(
        &self,
        observations: &[Observation],
    ) -> (String, Vec<CompressedKnowledge>) {
        if !self.config.enabled || observations.len() <= self.config.recent_count {
            return (Self::observations_to_string(observations), Vec::new());
        }

        let recent = &observations[observations.len() - self.config.recent_count..];
        let older = &observations[..observations.len() - self.config.recent_count];

        let mut result = String::new();
        let (summaries, knowledge) = summarize(&self.config, older);

        if !summaries.is_empty() {
            result.push_str("## Summary of earlier observations:\n");
            for summary in &summaries {
                result.push_str(summary);
                result.push('\n');
            }
            result.push('\n');
        }

        result.push_str("## Recent observations:\n");
        result.push_str(&Self::observations_to_string(recent));

        // Enforce token budget
        let estimated_tokens = result.len() / 4;
        if estimated_tokens > self.config.max_tokens {
            let target_len = self.config.max_tokens * 4;
            if result.len() > target_len {
                result.truncate(target_len);
                result.push_str("\n[truncated — context budget exceeded]");
            }
        }

        (result, knowledge)
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

/// Summarize older observations by grouping them by stage/kind and applying semantic clustering.
fn summarize(
    config: &CompressionConfig,
    observations: &[Observation],
) -> (Vec<String>, Vec<CompressedKnowledge>) {
    // Group by stage
    let mut by_stage: HashMap<&str, Vec<&Observation>> = HashMap::new();
    for obs in observations {
        by_stage.entry(obs.stage.as_str()).or_default().push(obs);
    }

    let mut stages: Vec<_> = by_stage.into_iter().collect();
    stages.sort_by_key(|(stage, _)| stage.to_string());

    let mut summaries = Vec::new();
    let mut all_knowledge = Vec::new();

    for (stage, obs_list) in stages.into_iter() {
        if summaries.len() >= config.max_summaries {
            break;
        }

        // Group by kind within stage
        let mut by_kind: HashMap<&str, Vec<&Observation>> = HashMap::new();
        for obs in &obs_list {
            by_kind.entry(obs.kind.as_str()).or_default().push(obs);
        }

        for (kind, kind_obs) in by_kind.into_iter() {
            if summaries.len() >= config.max_summaries {
                break;
            }

            // Apply semantic clustering to merge similar observations
            let kind_obs_owned: Vec<Observation> = kind_obs.iter().map(|o| (*o).clone()).collect();
            let cluster_result = compress_observations(&kind_obs_owned);

            // Collect knowledge items derived from this cluster group
            all_knowledge.extend(cluster_result.knowledge);

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
                    let compressed = compress_observation(trimmed);
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

    (summaries, all_knowledge)
}

/// Compress a single observation's details using structural tokenization.
fn compress_observation(text: &str) -> String {
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
