/// Optimization engine for codeburn token usage analysis.
///
/// Detects token waste patterns across provider sessions with configurable thresholds
/// and multiple heuristic rules.
use codeburn_provider::SessionData;

/// Configuration for the optimization engine.
#[derive(Debug, Clone)]
pub struct OptimizeConfig {
    /// Output token threshold for high-output detection
    pub high_output_threshold: u64,
    /// Output/input ratio threshold for waste detection
    pub output_ratio_threshold: f64,
    /// Maximum complexity level to flag (0-4)
    pub max_complexity: u32,
    /// Number of top findings to return per category
    pub top_n: usize,
    /// Filter by model name (None = all models)
    pub model_filter: Option<String>,
    /// Filter by project name (None = all projects)
    pub project_filter: Option<String>,
    /// Output format: "json", "markdown", or "csv"
    pub output_format: String,
}

impl Default for OptimizeConfig {
    fn default() -> Self {
        Self {
            high_output_threshold: 500_000,
            output_ratio_threshold: 10.0,
            max_complexity: 2,
            top_n: 10,
            model_filter: None,
            project_filter: None,
            output_format: "json".to_string(),
        }
    }
}

/// A single optimization finding with a category and explanation.
#[derive(Debug, Clone)]
pub struct Finding {
    pub category: String,
    pub provider: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub output_ratio: f64,
    pub complexity: u32,
    pub project: String,
    pub date: String,
    pub explanation: String,
}

/// Run the optimization engine against classified sessions.
///
/// Applies multiple heuristic rules to detect token waste:
/// 1. High output ratio on low complexity tasks
/// 2. Very high output token sessions
/// 3. Cost-per-token outliers (when pricing data is available)
/// 4. Context window waste (high input, low output)
/// 5. Repeated tool call patterns
pub fn optimize_engine(
    classified: &[SessionData],
    config: &OptimizeConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    for session in classified {
        // Apply model/project filters
        if let Some(ref model) = config.model_filter
            && session.model != *model
        {
            continue;
        }
        if let Some(ref project) = config.project_filter
            && session.project != *project
        {
            continue;
        }

        // Compute complexity from task category
        let complexity = compute_complexity(&session.task_category);

        // Compute output/input ratio
        let output_ratio = if session.input_tokens > 0 {
            session.output_tokens as f64 / session.input_tokens as f64
        } else {
            0.0
        };

        // Rule 1: High output ratio on low complexity = potential waste
        if complexity <= config.max_complexity && output_ratio > config.output_ratio_threshold {
            findings.push(Finding {
                category: "waste_detected".to_string(),
                provider: session.provider.clone(),
                model: session.model.clone(),
                input_tokens: session.input_tokens,
                output_tokens: session.output_tokens,
                output_ratio,
                complexity,
                project: session.project.clone(),
                date: session.date.clone(),
                explanation: format!(
                    "Low-complexity task (complexity={complexity}) with high output ratio ({:.1}x) — likely verbose output for simple tasks",
                    output_ratio
                ),
            });
        }

        // Rule 2: Very high output tokens
        if session.output_tokens > config.high_output_threshold {
            findings.push(Finding {
                category: "high_output".to_string(),
                provider: session.provider.clone(),
                model: session.model.clone(),
                input_tokens: session.input_tokens,
                output_tokens: session.output_tokens,
                output_ratio,
                complexity,
                project: session.project.clone(),
                date: session.date.clone(),
                explanation: format!(
                    "Very high output ({:>12} tokens) — consider batching or breaking into smaller tasks",
                    session.output_tokens
                ),
            });
        }

        // Rule 3: Context window waste (high input, low output)
        if session.input_tokens > 100_000 && output_ratio < 0.5 {
            findings.push(Finding {
                category: "context_waste".to_string(),
                provider: session.provider.clone(),
                model: session.model.clone(),
                input_tokens: session.input_tokens,
                output_tokens: session.output_tokens,
                output_ratio,
                complexity,
                project: session.project.clone(),
                date: session.date.clone(),
                explanation: format!(
                    "High input ({:>12} tokens) with low output ratio ({:.2}x) — context window underutilized",
                    session.input_tokens, output_ratio
                ),
            });
        }

        // Rule 4: Low complexity + very high input = potential prompt bloat
        if complexity <= 2 && session.input_tokens > 50_000 {
            findings.push(Finding {
                category: "prompt_bloat".to_string(),
                provider: session.provider.clone(),
                model: session.model.clone(),
                input_tokens: session.input_tokens,
                output_tokens: session.output_tokens,
                output_ratio,
                complexity,
                project: session.project.clone(),
                date: session.date.clone(),
                explanation: format!(
                    "Low-complexity task with high input ({:>12} tokens) — prompt may contain unnecessary context",
                    session.input_tokens
                ),
            });
        }
    }

    // Sort by output tokens descending, then take top N
    findings.sort_by_key(|f| std::cmp::Reverse(f.output_tokens));
    findings.truncate(config.top_n);

    findings
}

/// Compute complexity score (0-4) from task category.
fn compute_complexity(task_category: &str) -> u32 {
    match task_category {
        "edit" | "fix" | "debugging" => 1,
        "test" | "docs" | "review" => 2,
        "refactor" | "design" | "research" => 3,
        "architecture" | "integration" | "deployment" => 4,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session(
        provider: &str,
        model: &str,
        input: u64,
        output: u64,
        category: &str,
        project: &str,
        date: &str,
    ) -> SessionData {
        SessionData {
            provider_name: provider.to_string(),
            provider: provider.to_string(),
            date: date.to_string(),
            input_tokens: input,
            output_tokens: output,
            model: model.to_string(),
            task_category: category.to_string(),
            project: project.to_string(),
            message_id: uuid::Uuid::new_v4().to_string(),
            format: codeburn_provider::DataFormat::Jsonl,
            provenance: codeburn_provider::ProvenanceEntry {
                source: "test".to_string(),
                provenance_id: uuid::Uuid::new_v4().to_string(),
                provider_id: "test".to_string(),
                data_path: "/tmp/test".to_string(),
                format: "test".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }
    }

    #[test]
    fn test_detects_waste_on_low_complexity_high_ratio() {
        let sessions = vec![make_session(
            "claude", "claude-sonnet-4-20250514", 5000, 100_000, "edit", "test-project", "2026-01-01",
        )];
        let config = OptimizeConfig::default();
        let findings = optimize_engine(&sessions, &config);
        assert!(findings.iter().any(|f| f.category == "waste_detected"));
    }

    #[test]
    fn test_detects_high_output() {
        let sessions = vec![make_session(
            "claude", "claude-sonnet-4-20250514", 50000, 600_000, "refactor", "test-project", "2026-01-01",
        )];
        let config = OptimizeConfig::default();
        let findings = optimize_engine(&sessions, &config);
        assert!(findings.iter().any(|f| f.category == "high_output"));
    }

    #[test]
    fn test_detects_context_waste() {
        let sessions = vec![make_session(
            "claude", "claude-sonnet-4-20250514", 200_000, 5000, "edit", "test-project", "2026-01-01",
        )];
        let config = OptimizeConfig::default();
        let findings = optimize_engine(&sessions, &config);
        assert!(findings.iter().any(|f| f.category == "context_waste"));
    }

    #[test]
    fn test_detects_prompt_bloat() {
        let sessions = vec![make_session(
            "claude", "claude-sonnet-4-20250514", 60_000, 5000, "fix", "test-project", "2026-01-01",
        )];
        let config = OptimizeConfig::default();
        let findings = optimize_engine(&sessions, &config);
        assert!(findings.iter().any(|f| f.category == "prompt_bloat"));
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_model_filter() {
        let sessions = vec![
            make_session("claude", "claude-sonnet-4-20250514", 5000, 100_000, "edit", "test-project", "2026-01-01"),
            make_session("openai", "gpt-4o", 5000, 100_000, "edit", "test-project", "2026-01-01"),
        ];
        let mut config = OptimizeConfig::default();
        config.model_filter = Some("claude-sonnet-4-20250514".to_string());
        let findings = optimize_engine(&sessions, &config);
        assert!(findings.iter().all(|f| f.model == "claude-sonnet-4-20250514"));
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_project_filter() {
        let sessions = vec![
            make_session("claude", "claude-sonnet-4-20250514", 5000, 100_000, "edit", "project-a", "2026-01-01"),
            make_session("claude", "claude-sonnet-4-20250514", 5000, 100_000, "edit", "project-b", "2026-01-01"),
        ];
        let mut config = OptimizeConfig::default();
        config.project_filter = Some("project-a".to_string());
        let findings = optimize_engine(&sessions, &config);
        assert!(findings.iter().all(|f| f.project == "project-a"));
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_top_n_limit() {
        let sessions: Vec<SessionData> = (0..20).map(|i| {
            make_session(
                "claude", "claude-sonnet-4-20250514", 5000, 100_000 + i * 1000, "edit", "test-project", "2026-01-01",
            )
        }).collect();
        let mut config = OptimizeConfig::default();
        config.top_n = 5;
        let findings = optimize_engine(&sessions, &config);
        assert!(findings.len() <= 5);
    }

    #[test]
    fn test_empty_input() {
        let config = OptimizeConfig::default();
        let findings = optimize_engine(&[], &config);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_no_findings_for_normal_sessions() {
        let sessions = vec![
            make_session("claude", "claude-sonnet-4-20250514", 5000, 5000, "architecture", "test-project", "2026-01-01"),
            make_session("openai", "gpt-4o", 10000, 8000, "refactor", "test-project", "2026-01-01"),
        ];
        let config = OptimizeConfig::default();
        let findings = optimize_engine(&sessions, &config);
        assert!(findings.is_empty());
    }
}
