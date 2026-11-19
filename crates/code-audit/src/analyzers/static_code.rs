//! Static code vulnerability analyzer using pesti structural tokenizer and syn AST parsing.
//! Detects: hardcoded credentials, unsafe deserialization, string concatenation
//! (potential injection), overly complex functions, and suspicious patterns.

use crate::types::*;
use crate::{HuntConfig, HunterError};
use pesti_structural_tokenizer::errors::{Severity as PestiSeverity, TokenizerError};
use pesti_structural_tokenizer::semantic::{ComplexityMetrics, analyze_complexity};
use pesti_structural_tokenizer::{StructuralTokenizer, TokenKind};

#[derive(Debug, Default)]
pub struct StaticCodeAnalyzer {
    patterns: Vec<VulnPattern>,
}

/// A vulnerability pattern to detect.
#[derive(Clone, Debug)]
struct VulnPattern {
    name: &'static str,
    cwe: Option<&'static str>,
    severity: Severity,
    description: &'static str,
    remediation: &'static str,
}

impl StaticCodeAnalyzer {
    pub fn new() -> Self {
        let patterns = vec![
            VulnPattern {
                name: "hardcoded_credentials",
                cwe: Some("CWE-798"),
                severity: Severity::High,
                description: "Hardcoded credentials or secrets detected in source code",
                remediation: "Use environment variables or a secrets manager instead of hardcoding credentials.",
            },
            VulnPattern {
                name: "unsafe_deserialization",
                cwe: Some("CWE-502"),
                severity: Severity::Medium,
                description: "Deserialization of untrusted data without validation",
                remediation: "Validate and sanitize input before deserializing. Use safe deserialization formats.",
            },
            VulnPattern {
                name: "string_concat_injection",
                cwe: Some("CWE-89"),
                severity: Severity::High,
                description: "String concatenation used in query/command construction (potential injection)",
                remediation: "Use parameterized queries or prepared statements instead of string concatenation.",
            },
            VulnPattern {
                name: "complex_function",
                cwe: None,
                severity: Severity::Low,
                description: "Function exceeds complexity threshold (many control flow statements)",
                remediation: "Refactor complex functions into smaller, more focused functions.",
            },
        ];

        Self { patterns }
    }

    fn analyze_file(&self, path: &str, content: &str) -> Vec<Finding> {
        let mut findings = Vec::new();

        // AST-based detection for hardcoded credentials (fixes false positives)
        findings.extend(self.find_hardcoded_credentials_ast(path, content));

        // AST-based detection for SQL injection
        findings.extend(self.find_sql_injection(path, content));

        // AST-based detection for command injection
        findings.extend(self.find_command_injection(path, content));

        // AST-based detection for path traversal
        findings.extend(self.find_path_traversal(path, content));

        // Tokenize using pesti structural tokenizer for structural analysis
        let tokenizer = StructuralTokenizer::new();
        let tokens = match tokenizer.tokenize(content) {
            Ok(tokens) => tokens,
            Err(e) => {
                // Convert legacy TokenizeError to structured form for rich context reporting
                let (pos, msg) = match e {
                    pesti_structural_tokenizer::TokenizeError::Parse { pos, msg } => (pos, msg),
                };
                let structured = TokenizerError::syntax(content, pos, msg);

                // Report tokenization failure as a finding — files with syntax errors
                // are themselves worth flagging in a vulnerability scan
                let (line, col, snippet) = if let Some(ctx) = &structured.context {
                    (ctx.line, ctx.col, ctx.snippet.clone())
                } else {
                    (0, 0, String::new())
                };

                findings.push(Finding {
                    id: format!("tokenize-failure-{}", path),
                    title: "Structural analysis skipped — tokenization failed".to_string(),
                    description: format!(
                        "Pesti tokenizer could not parse this file ({}). \
                         Structural pattern detection (unsafe deserialization, complexity) \
                         was skipped. Only AST-based findings are available for this file.",
                        structured.message
                    ),
                    severity: match structured.severity {
                        PestiSeverity::Fatal => Severity::Medium,
                        PestiSeverity::Recoverable => Severity::Low,
                        PestiSeverity::Warning => Severity::Low,
                    },
                    cwe: None,
                    analyzer: AnalyzerType::StaticCode,
                    evidence: vec![Evidence {
                        kind: "tokenization_error".to_string(),
                        data: serde_json::json!({
                            "file": path,
                            "line": line,
                            "column": col,
                            "error_message": structured.message,
                            "severity": format!("{:?}", structured.severity),
                            "snippet": snippet,
                        }),
                        location: Some(format!("{}:{}", path, if line > 0 { line } else { 1 })),
                    }],
                    remediation: "Fix syntax errors in this file to enable full structural vulnerability analysis.".to_string(),
                    inference_validated: false,
                    confidence: None,
                });

                // Still return AST-based findings we already collected
                return findings;
            }
        };

        // Run structural pattern detection using pesti tokens
        for pattern in &self.patterns {
            if pattern.name == "hardcoded_credentials" || pattern.name == "string_concat_injection"
            {
                continue; // Already handled via AST
            }

            let matches = match pattern.name {
                "unsafe_deserialization" => find_unsafe_deserialization(&tokens),
                "complex_function" => {
                    // Returns Vec<(usize, ComplexityMetrics)> - function start index and complexity metrics
                    let complex = find_complex_functions(content, &tokens);
                    for (idx, metrics) in &complex {
                        let line_num = get_line_number(content, &tokens, *idx);
                        findings.push(Finding {
                            id: format!("{}-{}", pattern.name, findings.len()),
                            title: "Complex function detected".to_string(),
                            description: format!(
                                "Function has cyclomatic complexity {} (threshold: 10), max nesting depth {}",
                                metrics.cyclomatic, metrics.max_depth
                            ),
                            severity: Severity::Low,
                            cwe: None,
                            analyzer: AnalyzerType::StaticCode,
                            evidence: vec![Evidence {
                                kind: "code_snippet".to_string(),
                                data: serde_json::json!({
                                    "file": path,
                                    "line": line_num,
                                    "cyclomatic_complexity": metrics.cyclomatic,
                                    "max_nesting_depth": metrics.max_depth,
                                    "control_flow_count": metrics.control_flow_count,
                                    "snippet": get_snippet(content, *idx, &tokens).unwrap_or_default()
                                }),
                                location: Some(format!("{}:{}", path, line_num)),
                            }],
                            remediation: pattern.remediation.to_string(),
                            inference_validated: false,
                            confidence: None,
                        });
                    }
                    Vec::new() // Return empty since we handled it inline
                }
                _ => Vec::new(),
            };

            for token_idx in matches {
                let line_num = get_line_number(content, &tokens, token_idx);
                findings.push(Finding {
                    id: format!("{}-{}", pattern.name, findings.len()),
                    title: pattern.name.to_string(),
                    description: pattern.description.to_string(),
                    severity: pattern.severity,
                    cwe: pattern.cwe.map(|s| s.to_string()),
                    analyzer: AnalyzerType::StaticCode,
                    evidence: vec![Evidence {
                        kind: "source_code".to_string(),
                        data: serde_json::json!({
                            "file": path,
                            "line": line_num,
                            "snippet": get_snippet(content, token_idx, &tokens).unwrap_or_default()
                        }),
                        location: Some(format!("{}:{}", path, line_num)),
                    }],
                    remediation: pattern.remediation.to_string(),
                    inference_validated: false,
                    confidence: None,
                });
            }
        }

        findings
    }



    /// Get the name of a local variable.
    pub(crate) fn local_name(&self, local: &syn::Local) -> String {
        if let syn::Pat::Ident(pat_ident) = &local.pat {
            return pat_ident.ident.to_string();
        }
        "unknown".to_string()
    }
}

#[async_trait::async_trait]
impl Analyzer for StaticCodeAnalyzer {
    async fn analyze(&self, config: &HuntConfig) -> Result<AnalyzerOutput, HunterError> {
        let target = &config.target;

        // Check if it's a file or directory
        if std::path::Path::new(target).is_dir() {
            // Scan all .rs files in the directory (recursive)
            let mut findings = Vec::new();
            scan_directory(target, &mut |path, content| {
                findings.extend(self.analyze_file(path, content));
            });
            return Ok(AnalyzerOutput { findings });
        } else {
            // Single file
            let content = std::fs::read_to_string(target).map_err(HunterError::Io)?;
            let findings = self.analyze_file(target, &content);
            return Ok(AnalyzerOutput { findings });
        }
    }

    fn clone_box(&self) -> Box<dyn Analyzer> {
        Box::new(self.clone())
    }
}

impl Clone for StaticCodeAnalyzer {
    fn clone(&self) -> Self {
        Self {
            patterns: self.patterns.clone(),
        }
    }
}

/// Find the line number where a variable is declared by searching the source content.
pub(crate) fn find_line_for_var(content: &str, name: &str) -> usize {
    // Search for "let name =" or "const NAME =" or "static NAME =" patterns
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains(&format!("let {}", name))
            || line.contains(&format!("const {}", name.to_uppercase()))
            || line.contains(&format!("static {}", name.to_uppercase()))
        {
            return i + 1;
        }
    }
    // Fallback: search for just the variable name
    for (i, line) in lines.iter().enumerate() {
        if line.contains(name) {
            return i + 1;
        }
    }
    1
}

fn get_line_number(
    content: &str,
    tokens: &[pesti_structural_tokenizer::StructuralToken],
    idx: usize,
) -> usize {
    if idx >= tokens.len() {
        return 1;
    }
    // Use precise location if available
    if let Some(loc) = &tokens[idx].location {
        return loc.line;
    }
    // Fall back to counting newlines before this token's position
    let pos = tokens[idx].span.0;
    content[..pos.min(content.len())].matches('\n').count() + 1
}

fn get_snippet(
    content: &str,
    idx: usize,
    tokens: &[pesti_structural_tokenizer::StructuralToken],
) -> Option<String> {
    if idx >= tokens.len() {
        return None;
    }
    // Use precise location if available
    if let Some(loc) = &tokens[idx].location {
        let start = loc.start_byte;
        let end = loc.end_byte.min(content.len());
        if start <= content.len() {
            let snippet = &content[start..end];
            if snippet.chars().count() > 100 {
                return Some(format!(
                    "{}...",
                    snippet.chars().take(100).collect::<String>()
                ));
            }
            return Some(snippet.to_string());
        }
    }
    // Fall back to old span-based approach
    let (start, end) = tokens[idx].span;
    if start > content.len() || end > content.len() {
        return Some(tokens[idx].text.clone());
    }
    let snippet = &content[start..end.min(content.len())];
    // Truncate long snippets
    if snippet.chars().count() > 100 {
        return Some(format!(
            "{}...",
            snippet.chars().take(100).collect::<String>()
        ));
    }
    Some(snippet.to_string())
}

fn scan_directory(dir: &str, analyzer: &mut dyn FnMut(&str, &str)) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries {
        let path = match entry {
            Ok(e) => e.path(),
            Err(_) => continue,
        };

        if path.is_dir() {
            scan_directory(path.to_str().unwrap_or(""), analyzer);
        } else if path
            .extension()
            .is_some_and(|ext| ext == "rs" || ext == "py" || ext == "js")
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            analyzer(path.to_str().unwrap_or(""), &content);
        }
    }
}

/// Find unsafe deserialization patterns using pesti structural tokens
fn find_unsafe_deserialization(
    tokens: &[pesti_structural_tokenizer::StructuralToken],
) -> Vec<usize> {
    let mut matches = Vec::new();
    // Look for CallExpr or MethodCall with dangerous deserialize function names
    for (i, token) in tokens.iter().enumerate() {
        match &token.kind {
            TokenKind::Ident(name)
                if name.contains("deserialize")
                    || name.contains("from_slice")
                    || name.contains("from_str") =>
            {
                matches.push(i);
            }
            _ => {}
        }
    }
    matches
}

/// Find overly complex functions using pesti's analyze_complexity().
/// Returns (token_index_of_fn_decl, complexity_metrics) for each function
/// exceeding the cyclomatic complexity threshold.
fn find_complex_functions(
    content: &str,
    tokens: &[pesti_structural_tokenizer::StructuralToken],
) -> Vec<(usize, ComplexityMetrics)> {
    const CYCLOMATIC_THRESHOLD: u32 = 10;

    let fn_decl_indices: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| matches!(t.kind, TokenKind::FnDecl))
        .map(|(i, _)| i)
        .collect();

    let mut results = Vec::new();

    for (fn_idx, &decl_i) in fn_decl_indices.iter().enumerate() {
        // Determine the end boundary: next FnDecl or last token
        let end_idx = if fn_idx + 1 < fn_decl_indices.len() {
            fn_decl_indices[fn_idx + 1]
        } else {
            tokens.len()
        };

        if decl_i >= tokens.len() || end_idx <= decl_i {
            continue;
        }

        // Extract function source region using token spans
        let start = tokens[decl_i].span.0.min(content.len());
        let last_token_end = tokens[end_idx - 1].span.1.min(content.len());
        let func_source = &content[start..last_token_end.max(start)];

        // Run pesti's complexity analyzer on this function region
        if let Some(annotation) = analyze_complexity(func_source).get(&0) {
            let metrics = annotation.metrics;
            if metrics.cyclomatic > CYCLOMATIC_THRESHOLD {
                results.push((decl_i, metrics));
            }
        }
    }

    results
}
