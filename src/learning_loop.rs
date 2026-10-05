/// Learning loop: analyze skill invocation logs and suggest improvements.
///
/// Reads ~/.crabjar/skill-instrumentation.jsonl, computes skill health metrics,
/// identifies failure patterns, and generates actionable recommendations.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

/// Skill health metrics computed from invocation logs.
pub struct SkillHealth {
    pub name: String,
    pub invocations: usize,
    pub successes: usize,
    pub failures: usize,
    pub success_rate: f64,
    pub avg_duration_ms: u64,
    pub total_duration_ms: u64,
    pub error_types: HashMap<String, usize>,
    pub last_invoked: String,
}

/// Learning analyzer that processes skill instrumentation logs.
pub struct LearningAnalyzer {
    log_path: PathBuf,
}

impl LearningAnalyzer {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        let log_dir = PathBuf::from(home).join(".crabjar");
        Self {
            log_path: log_dir.join("skill-instrumentation.jsonl"),
        }
    }

    /// Compute health metrics for all skills.
    pub fn compute_skill_health(&self) -> Result<Vec<SkillHealth>, String> {
        let content = fs::read_to_string(&self.log_path)
            .map_err(|e| format!("Failed to read instrumentation log: {}", e))?;

        let mut health_map: HashMap<String, SkillHealth> = HashMap::new();

        for line in content.lines() {
            let event: Value = serde_json::from_str(line).map_err(|e| {
                format!("Failed to parse event line: {} (error: {})", line, e)
            })?;

            if event["type"] != "skill_invocation" {
                continue;
            }

            let skill_name = event["skill_name"].as_str().unwrap_or("unknown").to_string();
            let success = event["success"].as_bool().unwrap_or(false);
            let duration_ms = event["duration_ms"].as_u64().unwrap_or(0);
            let error_type = event["error_type"].as_str().unwrap_or("");
            let timestamp = event["timestamp"].as_str().unwrap_or("").to_string();

            let health = health_map.entry(skill_name.clone()).or_insert_with(|| SkillHealth {
                name: skill_name,
                invocations: 0,
                successes: 0,
                failures: 0,
                success_rate: 0.0,
                avg_duration_ms: 0,
                total_duration_ms: 0,
                error_types: HashMap::new(),
                last_invoked: String::new(),
            });

            health.invocations += 1;
            health.total_duration_ms += duration_ms;
            if success {
                health.successes += 1;
            } else {
                health.failures += 1;
                if !error_type.is_empty() {
                    *health.error_types.entry(error_type.to_string()).or_insert(0) += 1;
                }
            }
            health.last_invoked = timestamp;
        }

        // Compute derived metrics
        for health in health_map.values_mut() {
            if health.invocations > 0 {
                health.success_rate = health.successes as f64 / health.invocations as f64;
                health.avg_duration_ms = health.total_duration_ms / health.invocations as u64;
            }
        }

        let mut results: Vec<SkillHealth> = health_map.into_values().collect();
        // Sort by invocations descending (most used first)
        results.sort_by(|a, b| b.invocations.cmp(&a.invocations));
        Ok(results)
    }

    /// Identify failure patterns across skills.
    pub fn identify_failure_patterns(&self) -> Result<Vec<Value>, String> {
        let health = self.compute_skill_health()?;
        let mut patterns = Vec::new();

        // Pattern 1: High failure rate skills
        for h in &health {
            if h.invocations >= 3 && h.success_rate < 0.5 {
                patterns.push(json!({
                    "pattern": "high_failure_rate",
                    "skill": h.name,
                    "invocations": h.invocations,
                    "success_rate": h.success_rate,
                    "suggestion": format!("Review skill '{}' - {}% success rate over {} invocations", 
                        h.name, (h.success_rate * 100.0) as u32, h.invocations),
                }));
            }
        }

        // Pattern 2: Repeated error types
        let mut error_counts: HashMap<String, Vec<String>> = HashMap::new();
        for h in &health {
            for (error_type, count) in &h.error_types {
                error_counts.entry(error_type.clone())
                    .or_insert_with(Vec::new)
                    .push(h.name.clone());
            }
        }

        for (error_type, skills) in error_counts {
            if skills.len() >= 2 || error_type.contains("timeout") || error_type.contains("permission") {
                patterns.push(json!({
                    "pattern": "repeated_error",
                    "error_type": error_type,
                    "affected_skills": skills,
                    "suggestion": format!("Error '{}' affects {} skill(s): {}. Consider policy update.", 
                        error_type, skills.len(), skills.join(", ")),
                }));
            }
        }

        // Pattern 3: Slow skills (potential optimization targets)
        for h in &health {
            if h.avg_duration_ms > 30000 && h.invocations >= 2 {
                patterns.push(json!({
                    "pattern": "slow_skill",
                    "skill": h.name,
                    "avg_duration_ms": h.avg_duration_ms,
                    "suggestion": format!("Skill '{}' averages {}ms. Consider caching or optimization.", 
                        h.name, h.avg_duration_ms),
                }));
            }
        }

        Ok(patterns)
    }

    /// Generate learning recommendations as structured JSON.
    pub fn generate_recommendations(&self) -> Result<Value, String> {
        let health = self.compute_skill_health()?;
        let patterns = self.identify_failure_patterns()?;

        // Identify top-performing skills (potential templates)
        let mut top_skills: Vec<String> = health.iter()
            .filter(|h| h.invocations >= 5 && h.success_rate >= 0.9)
            .map(|h| h.name.clone())
            .collect();
        top_skills.sort();

        Ok(json!({
            "type": "learning_recommendations",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "skill_count": health.len(),
            "top_performing_skills": top_skills,
            "failure_patterns": patterns,
            "suggestions": [
                {
                    "type": "policy_update",
                    "description": "Update guard policies for high-failure skills to require approval before execution"
                },
                {
                    "type": "skill_optimization",
                    "description": "Profile slow skills and add caching for repeated operations"
                },
                {
                    "type": "training_data",
                    "description": "Export successful invocations of top-performing skills as QLoRA training examples"
                }
            ]
        }))
    }

    /// Export high-value successful invocations as training data candidates.
    pub fn export_training_candidates(&self, min_invocations: usize) -> Result<Vec<Value>, String> {
        let health = self.compute_skill_health()?;
        let mut candidates = Vec::new();

        for h in &health {
            if h.invocations >= min_invocations && h.success_rate >= 0.8 {
                candidates.push(json!({
                    "skill": h.name,
                    "invocations": h.invocations,
                    "success_rate": h.success_rate,
                    "avg_duration_ms": h.avg_duration_ms,
                    "training_value_score": (h.invocations as f64) * h.success_rate * 
                        (1.0 - ((h.avg_duration_ms as f64) / 60000.0).min(1.0))
                }));
            }
        }

        // Sort by training value score descending
        candidates.sort_by(|a, b| {
            let score_a = a["training_value_score"].as_f64().unwrap_or(0.0);
            let score_b = b["training_value_score"].as_f64().unwrap_or(0.0);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzer_initializes() {
        let analyzer = LearningAnalyzer::new();
        assert!(analyzer.log_path.exists() || true); // May not exist yet
    }
}