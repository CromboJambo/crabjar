//! JEV Evaluation Client
//!
//! Wraps llama-server HTTP API for embedding-based semantic decision-making.

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

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
        }
    }

    /// Evaluate an observation against decision criteria
    pub async fn evaluate(
        &self,
        observation: &str,
        criteria_file: &str,
    ) -> Result<DecisionResult> {
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
        if best_similarity >= criteria.threshold {
            Ok(DecisionResult {
                option: best_option.id.clone(),
                confidence: best_similarity,
                rationale: format!(
                    "High similarity to '{}' criterion ({:.3} vs threshold {:.2})",
                    best_option.id, best_similarity, criteria.threshold
                ),
            })
        } else {
            Ok(DecisionResult {
                option: String::new(),
                confidence: best_similarity,
                rationale: format!(
                    "Best match below threshold ({:.3} < {:.2}); route to human review",
                    best_similarity, criteria.threshold
                ),
            })
        }
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
            .post(format!("{}/api/v1/embeddings", self.config.url))
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