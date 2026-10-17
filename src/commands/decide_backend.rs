//! Embedding backend abstraction for JEV semantic decision-making.
//! Supports llama-server HTTP backend and hash-based fallback.

use serde_json::json;

/// Error type for embedding backend operations.
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Backend unreachable at {url}: {detail}")]
    Unreachable { url: String, detail: String },
    #[error("Invalid response from backend: {0}")]
    InvalidResponse(String),
    #[error("Embedding dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
    #[error("Timeout after {seconds}s")]
    Timeout { seconds: u64 },
    #[error("Generic backend error: {0}")]
    Other(String),
}

/// Result type for backend operations.
pub type BackendResult<T> = Result<T, BackendError>;

/// Trait for embedding backends.
pub trait EmbeddingBackend {
    /// Name of the backend (e.g., "llama-server", "hash").
    fn name(&self) -> &str;

    /// Compute embedding for a single text string.
    fn embed(&self, text: &str) -> BackendResult<Vec<f64>>;

    /// Compute embeddings for multiple texts (batch).
    fn embed_batch(&self, texts: &[&str]) -> BackendResult<Vec<Vec<f64>>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text)?);
        }
        Ok(results)
    }

    /// Dimensionality of embeddings produced by this backend.
    fn embedding_dim(&self) -> usize;
}

/// Hash-based embedding backend for offline/testing use.
/// Produces 768-dimensional vectors using byte distribution hashing.
pub struct HashBackend {
    dim: usize,
}

impl HashBackend {
    pub fn new() -> Self {
        Self { dim: 768 }
    }
}

impl Default for HashBackend {
    fn default() -> Self {
        Self::new()
    }
}

fn l2_norm(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

impl EmbeddingBackend for HashBackend {
    fn name(&self) -> &str {
        "hash"
    }

    fn embed(&self, text: &str) -> BackendResult<Vec<f64>> {
        let mut emb = vec![0.0f64; self.dim];
        for (i, byte) in text.bytes().enumerate() {
            let idx = i % self.dim;
            emb[idx] += byte as f64;
        }
        // Normalize to unit vector
        let norm = l2_norm(&emb);
        if norm > 0.0 {
            for v in emb.iter_mut() {
                *v /= norm;
            }
        }
        Ok(emb)
    }

    fn embedding_dim(&self) -> usize {
        self.dim
    }
}

/// Llama-server HTTP backend using /api/v1/embeddings endpoint.
pub struct LlamaServerBackend {
    url: String,
    model_id: String,
    client: reqwest::blocking::Client,
    dim: usize,
}

impl LlamaServerBackend {
    pub fn new(url: &str) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        Self {
            url: url.to_string(),
            model_id: "Qwen3.5-4B-Q4_K_S".to_string(),
            client,
            dim: 1024, // Qwen3.5-4B produces 1024-dim embeddings
        }
    }

    fn parse_embedding_response(&self, body: &str) -> BackendResult<Vec<f64>> {
        let parsed: serde_json::Value =
            serde_json::from_str(body).map_err(|e| BackendError::InvalidResponse(e.to_string()))?;

        // Try llama-server response format: { "embeddings": [[...], ...] }
        if let Some(embeddings) = parsed.get("embeddings") {
            if let Some(arr) = embeddings.as_array() {
                if !arr.is_empty() {
                    return self.extract_vector(&arr[0]);
                }
            }
        }

        // Try OpenAI-compatible format: { "data": [{ "embedding": [...] }] }
        if let Some(data) = parsed.get("data") {
            if let Some(arr) = data.as_array() {
                for item in arr {
                    if let Some(emb) = item.get("embedding") {
                        return self.extract_vector(emb);
                    }
                }
            }
        }

        Err(BackendError::InvalidResponse(
            "No embedding found in response".to_string(),
        ))
    }

    fn extract_vector(&self, value: &serde_json::Value) -> BackendResult<Vec<f64>> {
        if let Some(arr) = value.as_array() {
            let mut vec = Vec::with_capacity(arr.len());
            for item in arr {
                if let Some(f) = item.as_f64() {
                    vec.push(f);
                } else {
                    return Err(BackendError::InvalidResponse(format!(
                        "Non-float value in embedding: {:?}",
                        item
                    )));
                }
            }
            Ok(vec)
        } else {
            Err(BackendError::InvalidResponse(
                "Embedding is not an array".to_string(),
            ))
        }
    }
}

impl EmbeddingBackend for LlamaServerBackend {
    fn name(&self) -> &str {
        "llama-server"
    }

    fn embed(&self, text: &str) -> BackendResult<Vec<f64>> {
        let body = json!({
            "model": self.model_id,
            "input": text
        });

        let response = self
            .client
            .post(format!("{}/v1/embeddings", self.url))
            .json(&body)
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    BackendError::Timeout { seconds: 30 }
                } else if e.is_connect() {
                    BackendError::Unreachable {
                        url: self.url.clone(),
                        detail: e.to_string(),
                    }
                } else {
                    BackendError::Http(e)
                }
            })?;

        let status = response.status();
        let body_text = response.text().map_err(BackendError::Http)?;

        if !status.is_success() {
            return Err(BackendError::InvalidResponse(format!(
                "HTTP {}: {}",
                status, body_text
            )));
        }

        self.parse_embedding_response(&body_text)
    }

    fn embedding_dim(&self) -> usize {
        self.dim
    }
}

/// Factory function to create backends by name.
pub fn create_backend(name: &str) -> Box<dyn EmbeddingBackend> {
    match name {
        "llama-server" | "llama_server" => {
            let url = std::env::var("CRABJAR_JEV_API_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string());
            Box::new(LlamaServerBackend::new(&url))
        }
        "hash" => Box::new(HashBackend::new()),
        _ => Box::new(HashBackend::new()), // Default to hash for unknown
    }
}

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a = l2_norm(a);
    let norm_b = l2_norm(b);
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Evaluate an observation against multiple decision criteria.
/// Returns the best matching criterion with its similarity score.
pub fn evaluate<'a>(
    observation_embedding: &[f64],
    criteria: &'a [(&str, &[f64])],
) -> Option<(&'a str, f64)> {
    let mut best: Option<(&str, f64)> = None;
    for (name, embedding) in criteria {
        let sim = cosine_similarity(observation_embedding, embedding);
        match best {
            None => best = Some((*name, sim)),
            Some((_, best_sim)) if sim > best_sim => best = Some((*name, sim)),
            _ => {}
        }
    }
    best
}

/// Evaluate and return all similarity scores.
pub fn evaluate_all<'a>(
    observation_embedding: &[f64],
    criteria: &'a [(&str, &[f64])],
) -> Vec<(&'a str, f64)> {
    criteria
        .iter()
        .map(|(name, emb)| (*name, cosine_similarity(observation_embedding, emb)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_backend_produces_unit_vectors() {
        let backend = HashBackend::new();
        let emb = backend.embed("hello world").unwrap();
        assert_eq!(emb.len(), 768);
        let norm = l2_norm(&emb);
        assert!((norm - 1.0).abs() < 1e-6, "Not a unit vector: {}", norm);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_evaluate_selects_best_match() {
        let obs = vec![1.0, 0.0];
        let close_emb = vec![0.9, 0.1];
        let far_emb = vec![0.0, 1.0];
        let criteria = vec![("close", close_emb.as_slice()), ("far", far_emb.as_slice())];
        let best = evaluate(&obs, &criteria);
        assert_eq!(best.map(|(n, _)| n), Some("close"));
    }

    #[test]
    fn test_create_backend_hash() {
        let backend = create_backend("hash");
        assert_eq!(backend.name(), "hash");
    }
}
