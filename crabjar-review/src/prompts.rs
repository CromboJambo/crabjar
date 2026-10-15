//! Scenario-tuned prompt templates for code review.
//! Each bundle receives a focused prompt with the diff, relevant rules, and instructions.

use crate::types::{ReviewBundle, ReviewRule, Severity};

/// Generate a scenario-tuned prompt for a specific review bundle.
pub fn generate_review_prompt(
    bundle: &ReviewBundle,
    rules: &[ReviewRule],
    _repo_root: &str,
) -> String {
    let mut prompt = String::new();

    // System context - focused on code review, not general-purpose
    prompt.push_str("You are a senior Rust engineer performing targeted code review.\n");
    prompt.push_str(
        "Review ONLY the changes shown. Focus on correctness, safety, and maintainability.\n\n",
    );

    // Bundle metadata
    if let Some(crate_name) = &bundle.crate_name {
        prompt.push_str(&format!("Crate: {}\n", crate_name));
    }
    prompt.push_str(&format!(
        "Files in this review unit: {}\n\n",
        bundle.file_count()
    ));

    // Relevant rules matched for this bundle
    let relevant_rules: Vec<&ReviewRule> = rules
        .iter()
        .filter(|r| matches_rule_for_bundle(r, bundle))
        .collect();

    if !relevant_rules.is_empty() {
        prompt.push_str("Check these specific concerns:\n");
        for rule in &relevant_rules {
            let severity_tag = match &rule.severity {
                Severity::High => "[HIGH]",
                Severity::Medium => "[MED]",
                Severity::Low => "[LOW]",
            };
            prompt.push_str(&format!(
                "- {} {}: {}\n",
                severity_tag, rule.id, rule.description
            ));
        }
        prompt.push('\n');
    }

    // Diff content for each file in the bundle
    for file in &bundle.files {
        prompt.push_str(&format!("### File: {}\n", file.path.display()));
        prompt.push_str(&format!(
            "Status: {} (+{} -{})\n\n",
            file.status.as_str(),
            file.additions,
            file.deletions
        ));
        // In production, this would be populated with actual diff content
        prompt.push_str("(diff content will be injected here)\n\n");
    }

    // Output format instructions - structured JSON, not free-form text
    prompt.push_str("## Output Format\n\n");
    prompt.push_str(
        "Return ONLY a JSON array of findings. Each finding must have this exact structure:\n\n",
    );
    prompt.push_str("{\n");
    prompt.push_str("  \"file\": \"path/to/file.rs\",\n");
    prompt.push_str("  \"line_start\": 42,\n");
    prompt.push_str("  \"line_end\": 45,\n");
    prompt.push_str("  \"severity\": \"high|medium|low\",\n");
    prompt.push_str("  \"rule_id\": \"check_error_handling\",\n");
    prompt.push_str("  \"message\": \"Clear, specific description of the issue\",\n");
    prompt.push_str("  \"suggestion\": \"How to fix it (optional but recommended)\"\n");
    prompt.push_str("}\n\n");
    prompt.push_str("If no issues are found, return an empty array: []\n");
    prompt.push_str(
        "Do NOT include any prose, explanation, or markdown formatting outside the JSON.\n",
    );

    prompt
}

/// Check if a rule is relevant to this bundle based on file types.
fn matches_rule_for_bundle(rule: &ReviewRule, bundle: &ReviewBundle) -> bool {
    for file in &bundle.files {
        let type_str = file.file_type.as_str();

        // Check applies_to (if specified, must match at least one file)
        if !rule.applies_to.is_empty() && !rule.applies_to.contains(&type_str.to_string()) {
            return false;
        }

        // Check not_applies_to (must not match any file)
        if rule.not_applies_to.contains(&type_str.to_string()) {
            return false;
        }
    }
    true
}

/// Generate a context retrieval prompt when the agent needs additional information.
pub fn generate_context_prompt(bundle: &ReviewBundle, question: &str) -> String {
    format!(
        "While reviewing this code change, I need to understand more context.\n\n\
         Review unit files:\n{}\n\n\
         Question: {}\n\n\
         Please provide relevant context from the repository.",
        bundle
            .files
            .iter()
            .map(|f| f.path.display().to_string())
            .collect::<Vec<_>>()
            .join("\n"),
        question
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChangedFile, FileStatus};
    use std::path::PathBuf;

    #[test]
    fn generates_prompt_with_rules() {
        let bundle = ReviewBundle {
            id: "test".to_string(),
            files: vec![ChangedFile {
                path: PathBuf::from("src/lib.rs"),
                file_type: crate::types::FileType::Source,
                status: FileStatus::Modified,
                additions: 10,
                deletions: 5,
            }],
            crate_name: Some("crabjar-test".to_string()),
        };

        let rules = vec![ReviewRule {
            id: "check_errors".to_string(),
            applies_to: vec!["source".to_string()],
            not_applies_to: vec![],
            severity: Severity::High,
            description: "Check error handling".to_string(),
        }];

        let prompt = generate_review_prompt(&bundle, &rules, "/tmp/repo");

        assert!(prompt.contains("crabjar-test"));
        assert!(prompt.contains("check_errors"));
        assert!(prompt.contains("[HIGH]"));
        assert!(prompt.contains("JSON array"));
    }
}
