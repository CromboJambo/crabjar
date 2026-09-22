//! Semantic decision-making via embedding similarity (Jev/SemIf pattern)
//! Evaluates observations against decision criteria using cosine similarity.

use serde_json::json;
use std::fs;

/// Compute dot product of two vectors
fn dot_product(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Compute L2 norm of a vector
fn l2_norm(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// Compute cosine similarity between two vectors
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot = dot_product(a, b);
    let norm_a = l2_norm(a);
    let norm_b = l2_norm(b);
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Evaluate an observation against multiple decision criteria
/// Returns the best matching criterion with its similarity score
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

/// Decision command handler
pub fn handle(
    subcmd: &str,
    observation: Option<&str>,
    criteria_file: Option<&str>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // Usage: crabjar decide evaluate <observation> <criteria_json_file>
    if subcmd != "evaluate" || observation.is_none() || criteria_file.is_none() {
        return Ok(json!({
            "success": true,
            "message": "Semantic decision evaluation (Jev/SemIf pattern)",
            "usage": "crabjar decide evaluate <observation_text> <criteria.json>"
        }));
    }

    let observation = observation.unwrap();
    let criteria_file = criteria_file.unwrap();

    // Load criteria from JSON file
    let content = fs::read_to_string(criteria_file)?;
    let criteria_data: Vec<serde_json::Value> = serde_json::from_str(&content)?;

    // In production, this would use mirror-log's EmbeddingService to get real embeddings.
    // For now, demonstrate the architecture with a simple hash-based embedding (768-dim).
    fn embed(text: &str) -> Vec<f64> {
        let mut emb = vec![0.0f64; 768];
        for (i, byte) in text.bytes().enumerate() {
            let idx = i % 768;
            emb[idx] += byte as f64;
        }
        // Normalize
        let norm = l2_norm(&emb);
        if norm > 0.0 {
            for v in emb.iter_mut() {
                *v /= norm;
            }
        }
        emb
    }

    let obs_emb = embed(observation);
    let mut results = Vec::new();

    for criterion in &criteria_data {
        let name = criterion["name"].as_str().unwrap_or("unknown");
        let text = criterion["text"].as_str().unwrap_or("");
        let crit_emb = embed(text);
        let sim = cosine_similarity(&obs_emb, &crit_emb);
        results.push(json!({
            "criterion": name,
            "similarity": sim,
            "matches": sim > 0.5
        }));
    }

    // Find best match
    let criteria_pairs: Vec<(&str, Vec<f64>)> = criteria_data.iter().map(|c| {
        (c["name"].as_str().unwrap_or("unknown"), embed(c["text"].as_str().unwrap_or("")))
    }).collect();
    let eval_pairs: Vec<(&str, &[f64])> = criteria_pairs.iter().map(|(n, e)| (*n, e.as_slice())).collect();
    let best = evaluate(&obs_emb, &eval_pairs);

    Ok(json!({
        "success": true,
        "observation": observation,
        "results": results,
        "best_match": best.map(|(name, sim)| json!({"criterion": name, "similarity": sim})),
        "architecture": "Jev/SemIf non-autoregressive semantic branching",
        "note": "Using hash-based embeddings for demo. Production uses mirror-log EmbeddingService."
    }))
}
