//! Code review endpoint for the conductor service.

use crate::api::ConductorState;
use axum::{Json, extract::State};
use serde_json::json;
use tracing::{debug, info};

/// Submit a code review request.
pub async fn submit_review(
    State(_conductor): State<ConductorState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let repo_url = body["repo_url"].as_str().unwrap_or("").to_string();
    let pr_number = body["pr_number"].as_i64();
    let commit_sha = body["commit_sha"].as_str().unwrap_or("").to_string();
    let diff = body["diff"].as_str().unwrap_or("").to_string();

    info!(repo = %repo_url, pr = ?pr_number, sha = %commit_sha, "Submitting code review");

    // Run the deterministic pre-processor using git2 directly for the demo
    // In production, this would clone and inspect the actual repo
    let changed_files = parse_diff_to_changed_files(&diff);

    if changed_files.is_empty() {
        return Json(json!({
            "status": "completed",
            "findings": [],
            "message": "No relevant changed files detected"
        }));
    }

    info!(
        files = changed_files.len(),
        "Changed files selected for review"
    );

    // Smart bundling
    let bundles = crabjar_review::bundling::create_bundles(changed_files);
    info!(bundles = bundles.len(), "Files bundled for review");

    // Load default rules
    let default_rules = r#"
[[rules]]
id = "check_errors"
applies_to = ["source"]
not_applies_to = ["tests", "generated"]
severity = "high"
description = "Errors must be propagated or explicitly handled"

[[rules]]
id = "check_unreachable"
applies_to = ["source"]
severity = "medium"
description = "Check for unreachable code after return/break/continue"

[[rules]]
id = "check_style"
applies_to = ["source", "tests"]
severity = "low"
description = "Style and formatting conventions"
"#;

    let rules = match crabjar_review::rules::load_rules_from_str(default_rules) {
        Ok(r) => r,
        Err(e) => {
            return Json(json!({
                "status": "error",
                "message": format!("Failed to load review rules: {}", e)
            }));
        }
    };

    // Agent review phase (per bundle)
    let mut all_findings = Vec::new();

    for bundle in &bundles {
        // Match rules per file within the bundle
        for file in &bundle.files {
            let matched_rules = crabjar_review::rules::match_rules(file, &rules);

            if matched_rules.is_empty() {
                continue;
            }

            // Generate review prompt
            let scenario = "general".to_string();
            let owned_rules: Vec<crabjar_review::types::ReviewRule> =
                matched_rules.iter().map(|r| (*r).clone()).collect();
            let _prompt =
                crabjar_review::prompts::generate_review_prompt(bundle, &owned_rules, &scenario);

            debug!(bundle_files = bundle.files.len(), "Reviewing bundle");

            // Simulated agent output (replace with real pesti-server call)
            let mock_findings = simulate_bundle_review(bundle, &owned_rules);
            all_findings.extend(mock_findings);
        }
    }

    info!(
        findings = all_findings.len(),
        "Raw findings collected from review"
    );

    // Post-processing: deduplication and reflection
    let processed_findings = crabjar_review::reflection::process_findings(&all_findings);
    info!(
        processed_findings = processed_findings.len(),
        "Findings after post-processing"
    );

    // Generate summary
    let summary = crabjar_review::reflection::generate_reflection(&processed_findings);

    Json(json!({
        "status": "completed",
        "repo_url": repo_url,
        "pr_number": pr_number,
        "commit_sha": commit_sha,
        "bundles_reviewed": bundles.len(),
        "findings_count": processed_findings.len(),
        "findings": processed_findings,
        "summary": summary
    }))
}

/// Parse a unified diff into `ChangedFile` structs for review.
fn parse_diff_to_changed_files(diff: &str) -> Vec<crabjar_review::types::ChangedFile> {
    let mut files = Vec::new();
    let mut current_file = None;

    for line in diff.lines() {
        if line.starts_with("diff --git") {
            if let Some(f) = current_file.take() {
                files.push(f);
            }
            // Extract filename from "diff --git a/path b/path"
            if let Some(parts) = line.split_whitespace().nth(3) {
                let path = parts.strip_prefix("b/").unwrap_or(parts).to_string();
                current_file = Some(crabjar_review::types::ChangedFile {
                    path: std::path::PathBuf::from(&path),
                    file_type: classify_file_type(&path),
                    status: crabjar_review::types::FileStatus::Modified,
                    additions: 0,
                    deletions: 0,
                });
            }
        } else if line.starts_with("+++") && current_file.is_some() {
            // Count additions
            if let Some(f) = current_file.as_mut() {
                f.additions += 1;
            }
        } else if line.starts_with("---") && current_file.is_some() {
            // Count deletions
            if let Some(f) = current_file.as_mut() {
                f.deletions += 1;
            }
        }
    }

    if let Some(f) = current_file.take() {
        files.push(f);
    }

    files
}

/// Classify file type based on extension and path.
fn classify_file_type(path: &str) -> crabjar_review::types::FileType {
    if path.ends_with(".rs")
        || path.ends_with(".ts")
        || path.ends_with(".js")
        || path.ends_with(".py")
        || path.ends_with(".go")
    {
        crabjar_review::types::FileType::Source
    } else if path.contains("test") || path.ends_with("_test.rs") {
        crabjar_review::types::FileType::Test
    } else if path.ends_with(".json") || path.ends_with(".yaml") || path.ends_with(".toml") {
        crabjar_review::types::FileType::Config
    } else if path.ends_with(".md") {
        crabjar_review::types::FileType::Docs
    } else {
        crabjar_review::types::FileType::Unknown
    }
}

/// Simulate agent review for a bundle (placeholder until pesti-server integration).
fn simulate_bundle_review(
    bundle: &crabjar_review::types::ReviewBundle,
    matched_rules: &[crabjar_review::types::ReviewRule],
) -> Vec<serde_json::Value> {
    let mut findings = Vec::new();

    for file in &bundle.files {
        if let Ok(content) = std::fs::read_to_string(&file.path) {
            for rule in matched_rules {
                if rule.id.as_str() == "check_errors"
                    && content.contains("unwrap()")
                        && !file.path.to_string_lossy().contains("test")
                    {
                        findings.push(serde_json::json!({
                            "file": file.path.to_string_lossy(),
                            "line": 1,
                            "severity": "high",
                            "type": "bug",
                            "rule_id": rule.id,
                            "description": "unwrap() called without error handling"
                        }));
                    }
            }
        }
    }

    findings
}