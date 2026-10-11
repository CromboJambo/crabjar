//! Crabjar Code Review Pipeline
//!
//! Deterministic hybrid architecture combining deterministic logic with agent capabilities
//! for high-precision, low-token code review. Inspired by alibaba/open-code-review.
//!
//! Architecture:
//!   PR/Commit → [Deterministic Pre-processor] → [Agent Review Units] → [Post-processor] → Comments
//!                File selection/bundling           Isolated sub-agents      Position/content validation
//!                Rule matching per file            Context retrieval        Precision optimization

pub mod types;
pub mod rules;
pub mod bundling;
pub mod file_selection;
pub mod prompts;
pub mod context_retrieval;
pub mod review_agent;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::types::{ChangedFile, FileStatus, FileType};

    /// Test that the full prompt generation pipeline produces valid output.
    #[test]
    fn test_prompt_generation_pipeline() {
        let bundle = types::ReviewBundle {
            id: "test-bundle".to_string(),
            files: vec![ChangedFile {
                path: std::path::PathBuf::from("src/lib.rs"),
                file_type: FileType::Source,
                status: FileStatus::Modified,
                additions: 10,
                deletions: 5,
            }],
            crate_name: Some("crabjar-test".to_string()),
        };

        let rules = rules::load_rules_from_str(
            r#"
[[rules]]
id = "check_errors"
applies_to = ["source"]
severity = "high"
description = "Check error handling"
"#,
        )
        .unwrap();

        let prompt = prompts::generate_review_prompt(&bundle, &rules, "/tmp/repo");

        // Verify prompt contains expected elements
        assert!(prompt.contains("crabjar-test"));
        assert!(prompt.contains("check_errors"));
        assert!(prompt.contains("[HIGH]"));
        assert!(prompt.contains("JSON array"));
    }

    /// Test context retrieval tracker with simulated tool calls.
    #[test]
    fn test_context_retrieval_tracking() {
        let mut tracker = context_retrieval::ToolCallTracker::new(5);

        // Simulate 3 tool calls for a bundle
        for i in 1..=3 {
            assert!(tracker.record_call("bundle-1").is_ok(), "Call {} should succeed", i);
        }
        assert_eq!(tracker.remaining("bundle-1"), 2);

        // Simulate exceeding the limit
        assert!(tracker.record_call("bundle-1").is_ok());
        assert!(tracker.record_call("bundle-1").is_ok());
        assert!(tracker.record_call("bundle-1").is_err(), "6th call should fail");
    }

    /// Test that review agent can parse structured JSON output.
    #[test]
    fn test_structured_output_parsing() {
        let json = r#"[
            {
                "file": "src/lib.rs",
                "line_start": 42,
                "line_end": 45,
                "severity": "high",
                "rule_id": "check_errors",
                "message": "Error not propagated",
                "suggestion": "Return the error"
            }
        ]"#;

        let findings = review_agent::parse_review_output(json).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "src/lib.rs");
        assert_eq!(findings[0].severity, types::Severity::High);
    }

    /// Test that non-JSON output is handled gracefully.
    #[test]
    fn test_non_json_output_handling() {
        let findings = review_agent::parse_review_output("This is not JSON").unwrap();
        assert_eq!(findings.len(), 0, "Non-JSON should produce empty findings");
    }

    /// Test that markdown-wrapped JSON is extracted correctly.
    #[test]
    fn test_markdown_json_extraction() {
        let wrapped = r#"Here are my findings:

```json
[{"file":"src/lib.rs","line_start":10,"severity":"medium","rule_id":"style","message":"Style issue"}]
```

Let me know if you need more details."#;

        let findings = review_agent::parse_review_output(wrapped).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "src/lib.rs");
    }

    /// End-to-end test: file selection → bundling → prompt generation.
    #[test]
    fn test_end_to_end_pipeline() {
        // Simulate changed files
        let changed_files = vec![
            ChangedFile {
                path: std::path::PathBuf::from("src/lib.rs"),
                file_type: FileType::Source,
                status: FileStatus::Modified,
                additions: 5,
                deletions: 2,
            },
            ChangedFile {
                path: std::path::PathBuf::from("src/lib_test.rs"),
                file_type: FileType::Test,
                status: FileStatus::Modified,
                additions: 3,
                deletions: 1,
            },
        ];

        // Bundle them together (source + test)
        let bundles = bundling::create_bundles(changed_files);
        assert_eq!(bundles.len(), 1, "Source and test should be in one bundle");
        assert_eq!(bundles[0].file_count(), 2);

        // Generate prompt for the bundle
        let rules = vec![];
        let prompt = prompts::generate_review_prompt(&bundles[0], &rules, "/tmp/repo");
        assert!(prompt.contains("lib.rs"));
        assert!(prompt.contains("lib_test.rs"));
    }
}
