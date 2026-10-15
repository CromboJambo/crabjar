use serde_json::Value;
use std::collections::HashMap;

/// Deduplicate findings: merge findings that refer to the same code location with similar descriptions.
pub fn deduplicate_findings(findings: &[Value]) -> Vec<Value> {
    if findings.is_empty() {
        return vec![];
    }

    let mut seen = HashMap::new();
    let mut result = Vec::new();

    for finding in findings {
        let file = finding["file"].as_str().unwrap_or("");
        let line = finding["line"].as_u64().unwrap_or(0);
        let description = finding["description"].as_str().unwrap_or("").to_lowercase();

        // Create a dedup key: file + line (within 5 lines) + description similarity
        let key = format!(
            "{}:{}:{}",
            file,
            line / 5,
            description.chars().take(30).collect::<String>()
        );

        if !seen.contains_key(&key) {
            seen.insert(key, true);
            result.push(finding.clone());
        }
    }

    result
}

/// Reflect on severity: adjust based on context.
pub fn reflect_severity(findings: &[Value]) -> Vec<Value> {
    let mut result = Vec::with_capacity(findings.len());

    for finding in findings {
        let mut updated = finding.clone();
        let severity = finding["severity"].as_str().unwrap_or("minor");
        let finding_type = finding["type"].as_str().unwrap_or("");
        let file = finding["file"].as_str().unwrap_or("");

        // Security issues in test files are less severe
        if finding_type == "security" && file.contains("test") {
            updated["severity"] = Value::String("minor".to_string());
        }

        // Bugs in non-test source files are at least major
        if finding_type == "bug" && !file.contains("test") {
            if severity == "minor" || severity == "nit" {
                updated["severity"] = Value::String("major".to_string());
            }
        }

        result.push(updated);
    }

    result
}

/// Full post-processing pipeline: deduplicate, reflect, sort.
pub fn process_findings(raw_findings: &[Value]) -> Vec<Value> {
    // Step 1: Deduplicate
    let deduped = deduplicate_findings(raw_findings);

    // Step 2: Reflect on severity
    let reflected = reflect_severity(&deduped);

    // Step 3: Sort by severity (critical first)
    let mut sorted = reflected;
    sorted.sort_by(|a, b| {
        let sev_a = severity_rank(a["severity"].as_str().unwrap_or("minor"));
        let sev_b = severity_rank(b["severity"].as_str().unwrap_or("minor"));
        sev_b.cmp(&sev_a)
    });

    sorted
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "critical" => 4,
        "major" => 3,
        "minor" => 2,
        "nit" => 1,
        _ => 0,
    }
}

/// Generate a reflection summary for the review.
pub fn generate_reflection(findings: &[Value]) -> String {
    let total = findings.len();
    if total == 0 {
        return "No significant issues found.".to_string();
    }

    let mut critical = 0;
    let mut major = 0;
    let mut minor = 0;
    let mut nits = 0;

    for f in findings {
        match f["severity"].as_str().unwrap_or("minor") {
            "critical" => critical += 1,
            "major" => major += 1,
            "minor" => minor += 1,
            "nit" => nits += 1,
            _ => {}
        }
    }

    let mut summary = format!("Review complete: {} issues found.", total);
    if critical > 0 {
        summary.push_str(&format!(" {} critical", critical));
    }
    if major > 0 {
        summary.push_str(&format!(", {} major", major));
    }
    if minor > 0 {
        summary.push_str(&format!(", {} minor", minor));
    }
    if nits > 0 {
        summary.push_str(&format!(", {} nits", nits));
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deduplicate_same_location() {
        let findings = vec![
            serde_json::json!({"file": "a.rs", "line": 10, "severity": "minor", "type": "style", "description": "unused variable"}),
            serde_json::json!({"file": "a.rs", "line": 11, "severity": "minor", "type": "style", "description": "unused variable"}),
        ];

        let deduped = deduplicate_findings(&findings);
        assert_eq!(deduped.len(), 1);
    }

    #[test]
    fn test_deduplicate_different_locations() {
        let findings = vec![
            serde_json::json!({"file": "a.rs", "line": 10, "severity": "minor", "type": "style", "description": "unused variable"}),
            serde_json::json!({"file": "b.rs", "line": 20, "severity": "minor", "type": "style", "description": "unused variable"}),
        ];

        let deduped = deduplicate_findings(&findings);
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn test_reflect_severity_test_file_security() {
        let findings = vec![
            serde_json::json!({"file": "tests/foo.rs", "line": 5, "severity": "critical", "type": "security", "description": "injection"}),
        ];

        let reflected = reflect_severity(&findings);
        assert_eq!(reflected[0]["severity"].as_str().unwrap(), "minor");
    }

    #[test]
    fn test_process_findings_sorts_by_severity() {
        let findings = vec![
            serde_json::json!({"file": "a.rs", "line": 1, "severity": "minor", "type": "style", "description": "nitpick"}),
            serde_json::json!({"file": "b.rs", "line": 2, "severity": "critical", "type": "bug", "description": "crash"}),
        ];

        let processed = process_findings(&findings);
        assert_eq!(processed[0]["severity"].as_str().unwrap(), "critical");
    }
}
