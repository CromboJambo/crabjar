//! Semantic compression using hash-based embeddings for observation clustering.
//!
//! Groups similar observations together based on embedding similarity, then summarizes
//! clusters with a representative observation + count. Uses 768-dim hash embeddings
//! (no HTTP dependency) for offline/edge use cases.

use crabjar_host_core::work_item::Observation;

/// Threshold for considering two observations "similar enough" to cluster together.
const SIMILARITY_THRESHOLD: f64 = 0.85;

/// Dimensionality of hash embeddings.
const EMBEDDING_DIM: usize = 768;

/// Compute a hash-based embedding for an observation string.
/// Uses multiple hash functions to create distinct vectors for different inputs.
fn embed(text: &str) -> Vec<f64> {
    let mut emb = vec![0.0f64; EMBEDDING_DIM];

    // Use a simple but effective hashing approach:
    // For each byte, compute multiple hash values and distribute across dimensions
    for (i, byte) in text.bytes().enumerate() {
        // Primary hash: direct index
        let idx1 = i % EMBEDDING_DIM;
        emb[idx1] += byte as f64;

        // Secondary hash: different stride to spread out values
        let idx2 = ((i * 7) + (byte as usize)) % EMBEDDING_DIM;
        emb[idx2] += (byte as f64) * 0.5;

        // Tertiary hash: XOR-based
        let idx3 = ((i ^ byte as usize) * 13) % EMBEDDING_DIM;
        emb[idx3] += (byte as f64) * 0.25;
    }

    // L2 normalize to unit vector
    let norm = l2_norm(&emb);
    if norm > 0.0 {
        for v in emb.iter_mut() {
            *v /= norm;
        }
    }
    emb
}

/// Compute L2 norm of a vector.
fn l2_norm(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// Compute cosine similarity between two vectors.
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
/// Returns clusters where each cluster is a list of observation indices.
fn cluster_observations(observations: &[Observation]) -> Vec<Vec<usize>> {
    let n = observations.len();
    if n == 0 {
        return vec![];
    }

    // Compute embeddings for all observations
    let embeddings: Vec<Vec<f64>> = observations
        .iter()
        .map(|obs| embed(obs.details.as_str()))
        .collect();

    // Greedy clustering: each observation either joins an existing cluster or starts a new one
    let mut clusters: Vec<Vec<usize>> = vec![vec![0]];

    for i in 1..n {
        let mut best_cluster = None;
        let mut best_similarity = f64::MIN;

        // Find the most similar existing cluster representative
        for (cluster_idx, cluster) in clusters.iter().enumerate() {
            // Use first observation in cluster as representative
            let rep_idx = cluster[0];
            let sim = cosine_similarity(&embeddings[i], &embeddings[rep_idx]);
            if sim > best_similarity {
                best_similarity = sim;
                best_cluster = Some(cluster_idx);
            }
        }

        // If similar enough to an existing cluster, join it
        if best_cluster.is_some() && best_similarity >= SIMILARITY_THRESHOLD {
            clusters[best_cluster.unwrap()].push(i);
        } else {
            // Start new cluster
            clusters.push(vec![i]);
        }
    }

    clusters
}

/// Semantic compression result.
pub struct SemanticCompressionResult {
    /// Compressed observations (one per cluster, with count)
    pub compressed: Vec<Observation>,
    /// Original observation count
    pub original_count: usize,
    /// Number of clusters formed
    pub cluster_count: usize,
}

/// Apply semantic compression to observations using embedding similarity clustering.
pub fn compress_observations(observations: &[Observation]) -> SemanticCompressionResult {
    let original_count = observations.len();

    if original_count <= 1 {
        return SemanticCompressionResult {
            compressed: observations.to_vec(),
            original_count,
            cluster_count: original_count,
        };
    }

    // Cluster observations by semantic similarity
    let clusters = cluster_observations(observations);

    // For each cluster, create a summarized observation using the representative (first) observation
    let compressed: Vec<Observation> = clusters
        .iter()
        .map(|cluster| {
            if cluster.len() == 1 {
                // Single observation, return as-is
                observations[cluster[0]].clone()
            } else {
                // Multiple similar observations — summarize with count
                let representative = &observations[cluster[0]];
                let mut summarized = representative.clone();
                summarized.details = format!(
                    "[{}] similar {} {}: {}",
                    cluster.len(),
                    representative.stage,
                    representative.kind,
                    representative.details
                );
                summarized
            }
        })
        .collect();

    SemanticCompressionResult {
        compressed,
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