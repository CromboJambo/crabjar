//! Structural vulnerability rules for PegStructuralAnalyzer.

use crate::types::{AnalyzerType, Evidence, Finding, Severity};
use std::path::Path;

use super::peg_analyzer::{
    PegFeature, byte_to_line, count_params_from_captures, estimate_depth, simple_hash, truncate,
};

/// Check a single structural feature against vulnerability patterns (no recursion).
pub(crate) fn check_structural_rules_single(
    feature: &PegFeature,
    path: &Path,
    language: &str,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    const MAX_PARAMS: usize = 8;
    const MAX_DEPTH: usize = 6;

    // Rule 1: Unsafe block detection (Rust) — check text content since grammar has no dedicated unsafe rule.
    if language == "rust" && feature.text.contains("unsafe ") {
        let line = byte_to_line(&feature.text, 0);
        findings.push(Finding {
            id: format!("PEG-UNSAFE-{}-{}", simple_hash(&path.display().to_string()), feature.start),
            title: "Unsafe code block detected".to_string(),
            description: format!(
                "Structural parser identified an unsafe block at line {} in {}. Unsafe blocks bypass Rust's safety guarantees and require careful review.",
                line, path.display()
            ),
            severity: Severity::Medium,
            cwe: Some("CWE-119".to_string()),
            analyzer: AnalyzerType::PegStructural,
            evidence: vec![Evidence {
                kind: "structural_match".to_string(),
                data: serde_json::json!({
                    "rule": feature.rule,
                    "text": truncate(&feature.text, 200),
                    "span": [feature.start, feature.end],
                    "line": line,
                    "captures": feature.captures,
                }),
                location: Some(path.display().to_string()),
            }],
            remediation: "Review the unsafe block for memory safety issues. Consider replacing with safe abstractions where possible.".to_string(),
            inference_validated: false,
            confidence: Some(0.85),
        });
    }

    // Rule 2: Function with many parameters (code smell). Grammar rule: FuncDef.
    if feature.rule == "FuncDef" {
        let param_count = count_params_from_captures(&feature.captures);
        if param_count > MAX_PARAMS {
            let line = byte_to_line(&feature.text, 0);
            findings.push(Finding {
                id: format!("PEG-MANY-PARAMS-{}-{}", simple_hash(&path.display().to_string()), feature.start),
                title: format!("Function has {} parameters (threshold: {})", param_count, MAX_PARAMS),
                description: format!(
                    "Structural parser identified a function with {} parameters at line {}. High parameter counts often indicate the function does too much or needs a struct/parameter object.",
                    param_count, line
                ),
                severity: Severity::Low,
                cwe: None,
                analyzer: AnalyzerType::PegStructural,
                evidence: vec![Evidence {
                    kind: "structural_match".to_string(),
                    data: serde_json::json!({
                        "rule": feature.rule,
                        "param_count": param_count,
                        "line": line,
                        "captures": feature.captures,
                    }),
                    location: Some(path.display().to_string()),
                }],
                remediation: "Consider grouping parameters into a struct or reducing the function's responsibility.".to_string(),
                inference_validated: false,
                confidence: Some(0.75),
            });
        }
    }

    // Rule 3: Deeply nested structures (potential complexity issue). Grammar rules: Block, ImplBody.
    if feature.rule == "Block" || feature.rule == "ImplBody" {
        let depth = estimate_depth(&feature.children);
        if depth > MAX_DEPTH {
            let line = byte_to_line(&feature.text, 0);
            findings.push(Finding {
                id: format!("PEG-DEEP-NEST-{}-{}", simple_hash(&path.display().to_string()), feature.start),
                title: format!("Deeply nested block detected (depth {})", depth),
                description: format!(
                    "Structural parser identified a block with nesting depth {} at line {}. Deep nesting reduces readability and increases cognitive load.",
                    depth, line
                ),
                severity: Severity::Low,
                cwe: None,
                analyzer: AnalyzerType::PegStructural,
                evidence: vec![Evidence {
                    kind: "structural_match".to_string(),
                    data: serde_json::json!({
                        "rule": feature.rule,
                        "depth": depth,
                        "line": line,
                    }),
                    location: Some(path.display().to_string()),
                }],
                remediation: "Consider extracting nested logic into separate functions or using early returns.".to_string(),
                inference_validated: false,
                confidence: Some(0.7),
            });
        }
    }

    // Rule 4: Raw pointer usage (Rust) — check text content across any rule.
    if language == "rust" && (feature.text.contains("*mut") || feature.text.contains("*const")) {
        let line = byte_to_line(&feature.text, 0);
        findings.push(Finding {
            id: format!("PEG-RAW-PTR-{}-{}", simple_hash(&path.display().to_string()), feature.start),
            title: "Raw pointer usage detected".to_string(),
            description: format!(
                "Structural parser identified raw pointer type at line {}. Raw pointers bypass Rust's borrow checker and require manual memory safety reasoning.",
                line
            ),
            severity: Severity::Medium,
            cwe: Some("CWE-824".to_string()),
            analyzer: AnalyzerType::PegStructural,
            evidence: vec![Evidence {
                kind: "structural_match".to_string(),
                data: serde_json::json!({
                    "rule": feature.rule,
                    "text": truncate(&feature.text, 100),
                    "line": line,
                }),
                location: Some(path.display().to_string()),
            }],
            remediation: "Prefer safe references (&T, &mut T) or smart pointers (Arc, Box, Rc) over raw pointers unless FFI or unsafe code requires them.".to_string(),
            inference_validated: false,
            confidence: Some(0.8),
        });
    }

    // Rule 5: eval()/exec() usage (Python).
    if language == "python"
        && feature.rule.contains("call")
        && let Some(func_name) = feature.captures.get("name")
        && (func_name == "eval" || func_name == "exec")
    {
        let line = byte_to_line(&feature.text, 0);
        findings.push(Finding {
                    id: format!("PEG-EVAL-{}-{}", simple_hash(&path.display().to_string()), feature.start),
                    title: format!("Dynamic code execution via {}()", func_name),
                    description: format!(
                        "Structural parser identified a call to {}() at line {}. Dynamic code execution can lead to arbitrary code injection if the argument is influenced by untrusted input.",
                        func_name, line
                    ),
                    severity: Severity::High,
                    cwe: Some("CWE-94".to_string()),
                    analyzer: AnalyzerType::PegStructural,
                    evidence: vec![Evidence {
                        kind: "structural_match".to_string(),
                        data: serde_json::json!({
                            "rule": feature.rule,
                            "function": func_name,
                            "text": truncate(&feature.text, 200),
                            "line": line,
                            "captures": feature.captures,
                        }),
                        location: Some(path.display().to_string()),
                    }],
                    remediation: "Avoid eval()/exec() with untrusted input. Use ast.literal_eval() for data parsing or implement explicit logic instead.".to_string(),
                    inference_validated: false,
                    confidence: Some(0.9),
                });
    }

    // Rule 6: SQL string concatenation (Python).
    if language == "python" && feature.rule.contains("binary_op") {
        let text_lower = feature.text.to_lowercase();
        if (text_lower.contains("execute")
            || text_lower.contains("cursor")
            || text_lower.contains("query"))
            && feature.text.contains('+')
            && (feature.text.contains('\'') || feature.text.contains('"'))
        {
            let line = byte_to_line(&feature.text, 0);
            findings.push(Finding {
                    id: format!("PEG-SQL-CONCAT-{}-{}", simple_hash(&path.display().to_string()), feature.start),
                    title: "Potential SQL injection via string concatenation".to_string(),
                    description: format!(
                        "Structural parser identified string concatenation in a context that appears to involve SQL at line {}. Use parameterized queries instead of string building.",
                        line
                    ),
                    severity: Severity::High,
                    cwe: Some("CWE-89".to_string()),
                    analyzer: AnalyzerType::PegStructural,
                    evidence: vec![Evidence {
                        kind: "structural_match".to_string(),
                        data: serde_json::json!({
                            "rule": feature.rule,
                            "text": truncate(&feature.text, 300),
                            "line": line,
                        }),
                        location: Some(path.display().to_string()),
                    }],
                    remediation: "Use parameterized queries (e.g., cursor.execute(\"SELECT * FROM t WHERE id = ?\", (id,))) instead of string concatenation.".to_string(),
                    inference_validated: false,
                    confidence: Some(0.75),
                });
        }
    }

    // Rule 7: Shell command with string formatting (Python).
    if language == "python"
        && feature.rule.contains("call")
        && let Some(func_name) = feature.captures.get("name")
        && (func_name == "system" || func_name == "subprocess_call" || func_name == "Popen")
    {
        let text_lower = feature.text.to_lowercase();
        if text_lower.contains("format") || (text_lower.contains("+") && text_lower.contains('\''))
        {
            let line = byte_to_line(&feature.text, 0);
            findings.push(Finding {
                        id: format!("PEG-SHELL-INJECT-{}-{}", simple_hash(&path.display().to_string()), feature.start),
                        title: "Potential shell injection via string formatting".to_string(),
                        description: format!(
                            "Structural parser identified a shell command call with string formatting at line {}. This is vulnerable to shell injection if the formatted values are untrusted.",
                            line
                        ),
                        severity: Severity::High,
                        cwe: Some("CWE-78".to_string()),
                        analyzer: AnalyzerType::PegStructural,
                        evidence: vec![Evidence {
                            kind: "structural_match".to_string(),
                            data: serde_json::json!({
                                "rule": feature.rule,
                                "function": func_name,
                                "text": truncate(&feature.text, 300),
                                "line": line,
                            }),
                            location: Some(path.display().to_string()),
                        }],
                        remediation: "Use subprocess.run() with a list of arguments (shell=False) instead of string formatting.".to_string(),
                        inference_validated: false,
                        confidence: Some(0.8),
                    });
        }
    }

    findings
}
