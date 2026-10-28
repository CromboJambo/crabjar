//! Semantic compression using hash-based embeddings for observation clustering.
//!
//! Groups similar observations together based on embedding similarity, then summarizes
//! clusters with a representative observation + count. Uses 768-dim hash embeddings
//! (no HTTP dependency) for offline/edge use cases.
//!
//! Implements the knowledge compression triad pattern:
//! - Compressed form: rule/procedure derived from clustered observations
//! - Supporting evidence: examples where the rule worked vs failed
//! - Source traceability: links back to original observation IDs

use crabjar_host_core::work_item::Observation;

/// Threshold for considering two observations "similar enough" to cluster together.
const SIMILARITY_THRESHOLD: f64 = 0.85;

/// Dimensionality of hash embeddings.
const EMBEDDING_DIM: usize = 768;

/// Minimum cluster size to promote to a rule.
const MIN_CLUSTER_SIZE: usize = 2;

/// Compute a hash-based embedding for an observation string.
fn embed(text: &str) -> Vec<f64> {
    let mut emb = vec![0.0f64; EMBEDDING_DIM];

    for (i, byte) in text.bytes().enumerate() {
        let idx1 = i % EMBEDDING_DIM;
        emb[idx1] += byte as f64;

        let idx2 = ((i * 7) + (byte as usize)) % EMBEDDING_DIM;
        emb[idx2] += (byte as f64) * 0.5;

        let idx3 = ((i ^ byte as usize) * 13) % EMBEDDING_DIM;
        emb[idx3] += (byte as f64) * 0.25;
    }

    let norm = l2_norm(&emb);
    if norm > 0.0 {
        for v in emb.iter_mut() {
            *v /= norm;
        }
    }
    emb
}

fn l2_norm(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f64>();
    let norm_a = l2_norm(a);
    let norm_b = l2_norm(b);
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Cluster observations based on semantic similarity.
fn cluster_observations(observations: &[Observation]) -> Vec<Vec<usize>> {
    let n = observations.len();
    if n == 0 {
        return vec![];
    }

    let embeddings: Vec<Vec<f64>> = observations
        .iter()
        .map(|obs| embed(obs.details.as_str()))
        .collect();

    let mut clusters: Vec<Vec<usize>> = vec![vec![0]];

    for i in 1..n {
        let mut best_cluster = None;
        let mut best_similarity = f64::MIN;

        for (cluster_idx, cluster) in clusters.iter().enumerate() {
            let rep_idx = cluster[0];
            let sim = cosine_similarity(&embeddings[i], &embeddings[rep_idx]);
            if sim > best_similarity {
                best_similarity = sim;
                best_cluster = Some(cluster_idx);
            }
        }

        if best_cluster.is_some() && best_similarity >= SIMILARITY_THRESHOLD {
            clusters[best_cluster.unwrap()].push(i);
        } else {
            clusters.push(vec![i]);
        }
    }

    clusters
}

/// Evidence tracking for a piece of compressed knowledge.
#[derive(Debug, Clone)]
pub struct CompressionEvidence {
    pub supporting_examples: Vec<String>,
    pub counterexamples: Vec<String>,
    pub accuracy: f64,
}

impl Default for CompressionEvidence {
    fn default() -> Self {
        Self {
            supporting_examples: Vec::new(),
            counterexamples: Vec::new(),
            accuracy: 1.0,
        }
    }
}

/// A piece of compressed knowledge with triad components.
#[derive(Debug, Clone)]
pub struct CompressedKnowledge {
    pub id: String,
    /// The compressed form (rule/procedure)
    pub content: String,
    /// Type of compression (rule, procedure, model)
    pub kind: String,
    /// Confidence score based on cluster consistency
    pub confidence: f64,
    /// Supporting evidence
    pub evidence: CompressionEvidence,
    /// Source observation IDs that produced this knowledge
    pub source_observations: Vec<String>,
}

/// Semantic compression result with triad components.
pub struct SemanticCompressionResult {
    /// Compressed observations (one per cluster, with count)
    pub compressed: Vec<Observation>,
    /// Knowledge items derived from clusters (triad-form)
    pub knowledge: Vec<CompressedKnowledge>,
    /// Original observation count
    pub original_count: usize,
    /// Number of clusters formed
    pub cluster_count: usize,
}

/// Apply semantic compression to observations using embedding similarity clustering.
/// Produces both compressed observations and formal knowledge items with triad components.
pub fn compress_observations(observations: &[Observation]) -> SemanticCompressionResult {
    let original_count = observations.len();

    if original_count <= 1 {
        return SemanticCompressionResult {
            compressed: observations.to_vec(),
            knowledge: Vec::new(),
            original_count,
            cluster_count: original_count,
        };
    }

    // Cluster observations by semantic similarity
    let clusters = cluster_observations(observations);

    // For each cluster, create a summarized observation and derive knowledge if large enough
    let mut compressed_obs = Vec::new();
    let mut knowledge_items = Vec::new();

    for (cluster_idx, cluster) in clusters.iter().enumerate() {
        if cluster.len() == 1 {
            // Single observation, return as-is
            compressed_obs.push(observations[cluster[0]].clone());
        } else {
            // Multiple similar observations — summarize with count
            let representative = &observations[cluster[0]];
            let mut summarized = representative.clone();

            // Collect source observation IDs
            let source_ids: Vec<String> = cluster
                .iter()
                .map(|&i| observations[i].id.to_string())
                .collect();

            // Calculate confidence based on cluster size (larger = more confident)
            let raw_confidence = 0.7 + (cluster.len() as f64 * 0.02);
            let confidence = if raw_confidence > 0.95 { 0.95 } else { raw_confidence };

            summarized.details = format!(
                "[{}] similar {} {}: {}",
                cluster.len(),
                representative.stage,
                representative.kind,
                representative.details
            );
            compressed_obs.push(summarized);

            // Derive knowledge item if cluster is large enough
            if cluster.len() >= MIN_CLUSTER_SIZE {
                let evidence = CompressionEvidence {
                    supporting_examples: source_ids.clone(),
                    counterexamples: Vec::new(),
                    accuracy: 1.0,
                };

                let knowledge = CompressedKnowledge {
                    id: format!("kn_{}_{}", cluster_idx, uuid::Uuid::new_v4().to_string()[..8].to_string()),
                    content: format!(
                        "If {} context with {} pattern, expect {} outcome",
                        representative.stage, representative.kind, representative.details
                    ),
                    kind: "rule".to_string(),
                    confidence,
                    evidence,
                    source_observations: source_ids,
                };

                knowledge_items.push(knowledge);
            }
        }
    }

    SemanticCompressionResult {
        compressed: compressed_obs,
        knowledge: knowledge_items,
        original_count,
        cluster_count: clusters.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_obs(stage: &str, kind: &str, details: &str) -> Observation {
        Observation {
            id: Uuid::new_v4(),
            stage: stage.to_string(),
            kind: kind.to_string(),
            details: details.to_string(),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_embed_produces_unit_vectors() {
        let emb = embed("hello world");
        assert_eq!(emb.len(), EMBEDDING_DIM);
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
    fn test_cluster_identical_observations() {
        let obs1 = make_obs("observe", "state", "identical content");
        let obs2 = make_obs("observe", "state", "identical content");
        let observations = vec![obs1, obs2];

        let clusters = cluster_observations(&observations);
        // Identical content should be in the same cluster
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].len(), 2);
    }

    #[test]
    fn test_cluster_different_observations() {
        // Use strings with completely different content to ensure low similarity
        let obs1 = make_obs("observe", "state", "checking disk usage on partition sda1");
        let obs2 = make_obs("observe", "state", "querying remote API endpoint for weather data");
        let observations = vec![obs1, obs2];

        let clusters = cluster_observations(&observations);
        // Different content should be in separate clusters
        assert_eq!(clusters.len(), 2);
    }

    #[test]
    fn test_compress_merges_similar() {
        let obs1 = make_obs("observe", "state", "similar content A");
        let obs2 = make_obs("observe", "state", "similar content B"); // Very similar, differs by one char
        let observations = vec![obs1, obs2];

        let result = compress_observations(&observations);
        // Should merge into one cluster with count
        assert_eq!(result.cluster_count, 1);
        assert_eq!(result.compressed.len(), 1);
        assert!(result.compressed[0].details.contains("[2]"));
    }

    #[test]
    fn test_compress_empty() {
        let observations = vec![];
        let result = compress_observations(&observations);
        assert_eq!(result.compressed.len(), 0);
        assert_eq!(result.cluster_count, 0);
    }

    #[test]
    fn test_compress_single() {
        let obs = make_obs("observe", "state", "single observation");
        let observations = vec![obs];
        let result = compress_observations(&observations);
        assert_eq!(result.compressed.len(), 1);
        assert_eq!(result.cluster_count, 1);
    }
}