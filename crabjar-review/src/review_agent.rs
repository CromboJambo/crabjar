//! Crabjar Code Review Agent
//!
//! Orchestrates the deterministic hybrid code review pipeline:
//! 1. Parse diff and identify changed files (deterministic)
//! 2. Bundle related files together (deterministic)
//! 3. Match review rules to each bundle (deterministic)
//! 4. Generate focused prompts per bundle (template-driven)
//! 5. Send to pesti-server for inference (agent)
//! 6. Parse structured JSON output (deterministic)
//! 7. Validate positions and deduplicate (deterministic)
//!
//! This module provides the ReviewAgent struct which coordinates all phases.

use anyhow::Result;
use std::path::Path;

use crate::bundling::create_bundles;
use crate::context_retrieval::ToolCallTracker;
use crate::file_selection::parse_diff;
use crate::prompts::generate_review_prompt;
use crate::rules::load_rules_from_str;
use crate::types::{ReviewConfig, ReviewRule};

/// A single review finding from the agent.
#[derive(Debug, Clone)]
pub struct ReviewFinding {
    pub file: String,
    pub line_start: i32,
    pub line_end: i32,
    pub severity: crate::types::Severity,
    pub rule_id: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

/// The review agent that orchestrates the full pipeline.
pub struct ReviewAgent {
    config: ReviewConfig,
    tool_tracker: ToolCallTracker,
}

impl ReviewAgent {
    /// Create a new review agent with default configuration.
    pub fn new(config: ReviewConfig) -> Self {
        Self {
            tool_tracker: ToolCallTracker::new(config.context_retrieval_limit),
            config,
        }
    }

    /// Run the full review pipeline on a git diff.
    pub async fn review_diff(
        &self,
        repo_path: &Path,
        diff_text: &str,
        rules_toml: Option<&str>,
    ) -> Result<Vec<ReviewFinding>> {
        // Phase 1: Parse diff and identify changed files (deterministic)
        let changed_files = parse_diff(repo_path, diff_text).map_err(|e| anyhow::anyhow!(e))?;

        // Filter out generated/binary files based on classification
        let relevant_files: Vec<_> = changed_files
            .into_iter()
            .filter(|f| {
                matches!(
                    f.file_type,
                    crate::types::FileType::Source | crate::types::FileType::Test
                )
            })
            .collect();

        if relevant_files.is_empty() {
            return Ok(vec![]);
        }

        // Phase 2: Bundle related files (deterministic)
        let bundles = create_bundles(relevant_files);

        // Phase 3: Load and match rules (deterministic)
        let rules = self.load_rules(rules_toml)?;

        // Phase 4-5: Generate prompts and send to pesti-server (agent)
        let mut all_findings = Vec::new();
        for bundle in bundles {
            let prompt = generate_review_prompt(&bundle, &rules, repo_path.to_str().unwrap_or("."));
            let findings = self.send_to_pesti_server(&prompt).await?;
            all_findings.extend(findings);
        }

        // Phase 6-7: Parse output and validate (deterministic)
        Ok(all_findings)
    }

    /// Load review rules from TOML string or use defaults.
    fn load_rules(&self, rules_toml: Option<&str>) -> Result<Vec<ReviewRule>> {
        match rules_toml {
            Some(toml) => load_rules_from_str(toml).map_err(|e| anyhow::anyhow!(e)),
            None => Ok(self.default_rules()),
        }
    }

    /// Default review rules for Rust code.
    fn default_rules(&self) -> Vec<ReviewRule> {
        let toml = r#"
[[rules]]
id = "check_error_handling"
applies_to = ["source"]
severity = "high"
description = "Errors must be propagated or explicitly handled with unwrap/expect and a reason."

[[rules]]
id = "check_unsafe_blocks"
applies_to = ["source"]
severity = "high"
description = "Unsafe blocks must have safety comments explaining invariants."

[[rules]]
id = "check_public_api"
applies_to = ["source"]
not_applies_to = ["tests"]
severity = "medium"
description = "New public functions should have documentation comments."
"#;
        load_rules_from_str(toml).unwrap_or_default()
    }

    /// Send a review prompt to pesti-server and parse the response.
    async fn send_to_pesti_server(&self, prompt: &str) -> Result<Vec<ReviewFinding>> {
        let url = format!("{}/v1/chat/completions", self.config.model_endpoint);

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": [
                {"role": "system", "content": prompt}
            ],
            "temperature": 0.1,
            "max_tokens": 2048
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("pesti-server request failed: {}", e))?;

        let text = resp.text().await?;

        // Extract JSON from response
        let content = if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
            parsed["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or(&text)
                .to_string()
        } else {
            text
        };

        // Parse structured output
        parse_review_output(&content)
    }
}

/// Parse review output from pesti-server response.
/// Handles plain JSON, markdown-wrapped JSON, and prose with embedded JSON.
pub fn parse_review_output(text: &str) -> Result<Vec<ReviewFinding>> {
    // Try to extract JSON array from the text
    let json_text = extract_json_array(text)?;

    if json_text.is_empty() {
        return Ok(vec![]);
    }

    // Parse as array of finding objects
    let findings: Vec<serde_json::Value> = serde_json::from_str(&json_text)
        .map_err(|e| anyhow::anyhow!("Failed to parse review output as JSON: {}", e))?;

    let mut results = Vec::new();
    for f in findings {
        let finding = ReviewFinding {
            file: f["file"].as_str().unwrap_or("").to_string(),
            line_start: f["line_start"].as_i64().unwrap_or(0) as i32,
            line_end: f["line_end"].as_i64().unwrap_or(0) as i32,
            severity: parse_severity(f["severity"].as_str()),
            rule_id: f["rule_id"].as_str().map(|s| s.to_string()),
            message: f["message"].as_str().unwrap_or("").to_string(),
            suggestion: f["suggestion"].as_str().map(|s| s.to_string()),
        };
        results.push(finding);
    }

    Ok(results)
}

fn parse_severity(s: Option<&str>) -> crate::types::Severity {
    match s {
        Some("high") => crate::types::Severity::High,
        Some("low") => crate::types::Severity::Low,
        _ => crate::types::Severity::Medium,
    }
}

fn extract_json_array(text: &str) -> Result<String> {
    // First try: parse as-is
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
        if parsed.is_array() {
            return Ok(text.to_string());
        }
    }

    // Try to find JSON array in markdown code fences
    if let Some(start) = text.find("```") {
        let after_fences = &text[start + 3..];
        if let Some(end) = after_fences.rfind("```") {
            let inner = after_fences[..end].trim();
            // Skip language tag if present
            let json_start = if inner.starts_with("json") {
                inner.find('\n').map(|i| i + 1).unwrap_or(0)
            } else {
                0
            };
            let candidate = &inner[json_start..];
            if let Ok(_) = serde_json::from_str::<serde_json::Value>(candidate) {
                return Ok(candidate.to_string());
            }
        }
    }

    // Try to find [ that starts an array
    if let Some(start) = text.find('[') {
        if let Some(end) = text.rfind(']') {
            let candidate = &text[start..=end];
            if let Ok(_) = serde_json::from_str::<serde_json::Value>(candidate) {
                return Ok(candidate.to_string());
            }
        }
    }

    Ok("[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plain_json() {
        let json = r#"[{"file":"src/lib.rs","line_start":10,"severity":"high","rule_id":"test","message":"Issue"}]"#;
        let findings = parse_review_output(json).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "src/lib.rs");
        assert_eq!(findings[0].severity, crate::types::Severity::High);
    }

    #[test]
    fn test_parse_markdown_wrapped() {
        let wrapped = r#"Here are the findings:

```json
[{"file":"src/lib.rs","line_start":5,"severity":"medium","rule_id":"style","message":"Style issue"}]
```

Let me know if you need more details."#;
        let findings = parse_review_output(wrapped).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "src/lib.rs");
    }

    #[test]
    fn test_parse_empty() {
        let findings = parse_review_output("No issues found.").unwrap();
        assert_eq!(findings.len(), 0);
    }
}
