//! JEV Evaluation Client
//!
//! Wraps llama-server HTTP API for embedding-based semantic decision-making.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::cache::DecisionCache;
use crate::state_store::StateStore;

/// Embedding backend configuration
#[derive(Debug, Clone)]
pub struct BackendConfig {
    pub url: String,
    pub model_id: String,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8080".to_string(),
            model_id: "qwen3.5-4b-q4_k_s".to_string(),
        }
    }
}

/// Decision result from JEV evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionResult {
    pub option: String,
    pub confidence: f64,
    pub rationale: String,
}

/// JEV Evaluator that communicates with llama-server embedding backend
pub struct JevEvaluator {
    client: reqwest::Client,
    config: BackendConfig,
    criteria_cache: HashMap<String, DecisionCriteria>,
    decision_cache: Mutex<DecisionCache>,
}

/// Decision criteria loaded from JSON file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionCriteria {
    pub version: String,
    pub name: String,
    pub description: String,
    pub options: Vec<CriterionOption>,
    pub threshold: f64,
}

/// Individual decision criterion option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionOption {
    pub id: String,
    pub description: String,
}

/// Embedding API response from llama-server
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f64>,
}

impl JevEvaluator {
    /// Create a new JEV evaluator with the given backend configuration
    pub fn new(config: BackendConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
            criteria_cache: HashMap::new(),
            decision_cache: Mutex::new(DecisionCache::default_config()),
        }
    }

    /// Get cache statistics for monitoring
    pub fn cache_stats(&self) -> super::cache::CacheStats {
        let cache = self.decision_cache.lock().unwrap();
        cache.stats()
    }

    /// Evaluate an observation against decision criteria
    pub async fn evaluate(
        &self,
        observation: &str,
        criteria_file: &str,
        state_store: Option<&StateStore>,
    ) -> Result<DecisionResult> {
        // Compute cache key hash for logging
        let obs_hash = compute_observation_hash(observation, criteria_file);

        // Check cache first for recurring observation patterns
        let mut cache = self.decision_cache.lock().unwrap();
        if let Some(cached) = cache.get(observation, criteria_file) {
            tracing::debug!(
                "JEV cache hit: {} ({:.1}% hit rate)",
                &criteria_file[..criteria_file.len().min(40)],
                cache.hit_rate()
            );

            // Log cache hit to decision history
            if let Some(store) = state_store {
                let _ = store.log_decision(
                    &obs_hash,
                    criteria_file,
                    &cached.option,
                    cached.confidence,
                    true,
                );
            }

            return Ok(cached);
        }

        // Cache miss - compute decision via embedding API
        drop(cache); // Release lock before expensive API call

        // Load or retrieve cached criteria
        let criteria = self.load_criteria(criteria_file).await?;

        // Embed observation and criteria options
        let obs_embedding = self.embed(observation).await?;
        let option_embeddings = self.embed_options(&criteria.options).await?;

        // Compute similarities and find best match
        let mut best_option = &criteria.options[0];
        let mut best_similarity = f64::MIN;

        for (option, embedding) in criteria.options.iter().zip(option_embeddings.iter()) {
            let similarity = cosine_similarity(&obs_embedding, embedding);
            if similarity > best_similarity {
                best_similarity = similarity;
                best_option = option;
            }
        }

        // Apply threshold and format response
        let result = if best_similarity >= criteria.threshold {
            DecisionResult {
                option: best_option.id.clone(),
                confidence: best_similarity,
                rationale: format!(
                    "High similarity to '{}' criterion ({:.3} vs threshold {:.2})",
                    best_option.id, best_similarity, criteria.threshold
                ),
            }
        } else {
            DecisionResult {
                option: String::new(),
                confidence: best_similarity,
                rationale: format!(
                    "Best match below threshold ({:.3} < {:.2}); route to human review",
                    best_similarity, criteria.threshold
                ),
            }
        };

        // Cache the result for future similar observations
        let mut cache = self.decision_cache.lock().unwrap();
        cache.set(observation, criteria_file, result.clone());
        tracing::debug!(
            "JEV cache miss (computed): {} ({:.1}% hit rate)",
            &criteria_file[..criteria_file.len().min(40)],
            cache.hit_rate()
        );

        // Log computed decision to history
        if let Some(store) = state_store {
            let _ = store.log_decision(
                &obs_hash,
                criteria_file,
                &result.option,
                result.confidence,
                false,
            );
        }

        Ok(result)
    }

    /// Load decision criteria from file (with caching)
    async fn load_criteria(&self, path: &str) -> Result<DecisionCriteria> {
        // Simple cache check (not thread-safe, but sufficient for single-threaded use)
        if let Some(cached) = self.criteria_cache.get(path) {
            return Ok(cached.clone());
        }

        let content = tokio::fs::read_to_string(path).await?;
        let criteria: DecisionCriteria = serde_json::from_str(&content)?;

        // Note: Can't insert into cache here because self is immutable.
        // Cache would require interior mutability (RefCell) or making this method &mut self.
        Ok(criteria)
    }

    /// Embed a single text string
    async fn embed(&self, text: &str) -> Result<Vec<f64>> {
        let body = serde_json::json!({
            "model": self.config.model_id,
            "input": text
        });

        let response = self
            .client
            .post(format!("{}/v1/embeddings", self.config.url))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Embedding API returned status {}: {}",
                response.status(),
                response.text().await?
            ));
        }

        let embedding_response: EmbeddingResponse = response.json().await?;
        Ok(embedding_response.data[0].embedding.clone())
    }

    /// Embed multiple texts (criteria options)
    async fn embed_options(&self, options: &[CriterionOption]) -> Result<Vec<Vec<f64>>> {
        let mut embeddings = Vec::with_capacity(options.len());
        for option in options {
            let embedding = self.embed(&option.description).await?;
            embeddings.push(embedding);
        }
        Ok(embeddings)
    }
}

/// Compute cosine similarity between two vectors
fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot_product: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let magnitude_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

/// Compute a hash of the observation + criteria for caching and logging.
fn compute_observation_hash(observation: &str, criteria_file: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(criteria_file.as_bytes());
    hasher.update(b"\n");
    hasher.update(observation.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..16].to_string()
}

/// Evaluate a structural target against criteria, using fast-path condition
/// evaluation first and falling back to embedding-based JEV evaluation.
pub async fn evaluate_structural_target(
    evaluator: &JevEvaluator,
    target: &crate::decision::structural::EvaluationTarget,
    criteria_path: &str,
    state_store: Option<&StateStore>,
) -> Result<DecisionResult> {
    // First, try direct structural condition evaluation (fast path)
    if let Some(result) = evaluate_structural_conditions(target, criteria_path) {
        return Ok(result);
    }

    // Fall back to embedding-based JEV evaluation
    let observation = target.to_observation();
    evaluator
        .evaluate(&observation, criteria_path, state_store)
        .await
}

/// Evaluate structural conditions directly against the target.
/// Returns Some(DecisionResult) if a condition matched, None to fall back to embedding evaluation.
pub fn evaluate_structural_conditions(
    target: &crate::decision::structural::EvaluationTarget,
    criteria_path: &str,
) -> Option<DecisionResult> {
    // Read and parse the criteria file
    let criteria_content = match std::fs::read_to_string(criteria_path) {
        Ok(content) => content,
        Err(_) => return None, // File not found or unreadable — fall back to JEV
    };

    let criteria: serde_json::Value = match serde_json::from_str(&criteria_content) {
        Ok(value) => value,
        Err(_) => return None, // Invalid JSON — fall back to JEV
    };

    // Check each option for structural conditions
    if let Some(options) = criteria["options"].as_array() {
        for option in options {
            if let Some(condition) = option["conditions"].as_str() {
                if crate::decision::condition::evaluate_condition(target, condition) {
                    return Some(DecisionResult {
                        option: option["id"].as_str().unwrap_or("unknown").to_string(),
                        confidence: 1.0,
                        rationale: format!("Structural condition matched: {}", condition),
                    });
                }
            }
        }
    }

    None // No conditions matched — fall back to JEV embedding evaluation
}
