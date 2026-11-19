//! PEG-based structural code analyzer using pesti-structural grammars.
//!
//! This analyzer leverages grammar-based parsing with semantic roles, annotated
//! captures, and error recovery to detect vulnerabilities that flat token
//! matching misses. It loads language-specific PEG grammars (Rust, Python) and
//! uses the `PegEvaluator` to extract structural features from source code.
//!
//! Key advantages over the static_code analyzer:
//! - Grammar-based parsing respects language structure (not regex on tokens)
//! - Semantic roles identify function definitions, calls, assignments, etc.
//! - Annotated captures expose names, types, and arguments with provenance
//! - Error recovery allows analysis of partially malformed code
//! - Structural similarity detection finds near-duplicate vulnerable patterns

use crate::types::{Analyzer, AnalyzerOutput, AnalyzerType, Evidence, Finding, Severity};
use crate::{HuntConfig, HunterError};
use pesti_structural::evaluator::{ParseResult, PegEvaluator};
use pesti_structural::grammar::StructuralGrammar;
use pesti_structural::lexer::tokenize_for_language;
use pesti_structural::parser::parse_peg_file;
use pesti_structural::tree::TreeNode;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use super::peg_analyzer_rules::check_structural_rules_single;

/// Structural feature extracted from a PEG parse tree.
#[derive(Debug, Clone)]
pub(crate) struct PegFeature {
    /// The grammar rule that matched this node.
    pub(crate) rule: String,
    /// Start byte offset in source.
    pub(crate) start: usize,
    /// End byte offset in source.
    pub(crate) end: usize,
    /// Matched source text.
    pub(crate) text: String,
    /// Named captures within this node.
    pub(crate) captures: HashMap<String, String>,
    /// Child features (recursive).
    pub(crate) children: Vec<PegFeature>,
}

/// PEG-based structural analyzer.
pub struct PegStructuralAnalyzer {
    /// Loaded grammars by language name.
    grammars: HashMap<String, StructuralGrammar>,
}

impl PegStructuralAnalyzer {
    /// Create a new PEG structural analyzer with built-in Rust and Python grammars.
    pub fn new() -> Result<Self, HunterError> {
        let mut grammars = HashMap::new();

        // Load Rust grammar from the pesti-structural crate's grammars directory.
        let rust_grammar_path =
            find_grammar_file("rust.peg").or_else(|| find_grammar_file("rust_structural.peg"));
        if let Some(path) = rust_grammar_path {
            match parse_peg_file(&path) {
                Ok(grammar) => {
                    eprintln!(
                        "[peg-analyzer] Loaded Rust PEG grammar from {}",
                        path.display()
                    );
                    grammars.insert("rust".to_string(), grammar);
                }
                Err(e) => {
                    eprintln!(
                        "[peg-analyzer] Failed to load Rust PEG grammar from {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        // Load Python grammar.
        if let Some(path) = find_grammar_file("python.peg") {
            match parse_peg_file(&path) {
                Ok(grammar) => {
                    eprintln!(
                        "[peg-analyzer] Loaded Python PEG grammar from {}",
                        path.display()
                    );
                    grammars.insert("python".to_string(), grammar);
                }
                Err(e) => {
                    eprintln!(
                        "[peg-analyzer] Failed to load Python PEG grammar from {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        if grammars.is_empty() {
            return Err(HunterError::AnalysisFailed(
                "No PEG grammars could be loaded".to_string(),
            ));
        }

        Ok(Self { grammars })
    }

    /// Analyze a single source file using structural PEG parsing.
    pub fn analyze_file(&self, path: &Path, content: &str) -> Result<Vec<Finding>, HunterError> {
        // Determine language from file extension.
        let language = detect_language(path);
        let language_key = match language.as_str() {
            "rust" => "rust",
            "python" => "python",
            _ => return Ok(Vec::new()), // Unsupported language, skip.
        };

        let grammar = match self.grammars.get(language_key) {
            Some(g) => g,
            None => return Ok(Vec::new()),
        };

        // Tokenize using the language-appropriate lexer.
        let (tokens, lex_errors) = tokenize_for_language(content, language_key);

        // Create evaluator and parse with error recovery.
        let eval = PegEvaluator::new(grammar, &tokens);
        let result: ParseResult = eval.parse_with_recovery();

        // Collect findings from the parse tree.
        let mut findings = Vec::new();

        if let Some(root_node) = &result.tree {
            // Traverse the entire parse tree and check each node.
            traverse_and_check(root_node, content, path, language_key, &mut findings);
        }

        // Report lexical errors as low-severity findings (code quality signal).
        for lex_err in &lex_errors {
            let line = byte_to_line(content, lex_err.span.start);
            findings.push(Finding {
                id: format!(
                    "PEG-LEX-{}-{}",
                    simple_hash(&path.display().to_string()),
                    lex_err.span.start
                ),
                title: "Lexical anomaly detected".to_string(),
                description: format!(
                    "Structural parser encountered a lexical issue at line {}: {}",
                    line, lex_err.message
                ),
                severity: Severity::Low,
                cwe: None,
                analyzer: AnalyzerType::PegStructural,
                evidence: vec![Evidence {
                    kind: "lex_error".to_string(),
                    data: serde_json::json!({
                        "message": lex_err.message,
                        "span_start": lex_err.span.start,
                        "span_end": lex_err.span.end,
                        "line": line,
                    }),
                    location: Some(path.display().to_string()),
                }],
                remediation: "Review the flagged syntax for potential issues.".to_string(),
                inference_validated: false,
                confidence: Some(0.5),
            });
        }

        // Deduplicate by id.
        findings.dedup_by_key(|f| f.id.clone());

        Ok(findings)
    }

    /// Analyze a directory of source files using structural PEG parsing.
    pub fn analyze_directory(&self, dir: &Path) -> Result<Vec<Finding>, HunterError> {
        let mut all_findings = Vec::new();
        scan_directory(dir, self, &mut all_findings);
        Ok(all_findings)
    }
}

#[async_trait::async_trait]
impl Analyzer for PegStructuralAnalyzer {
    async fn analyze(&self, config: &HuntConfig) -> Result<AnalyzerOutput, HunterError> {
        let target_path = Path::new(&config.target);

        let findings = if target_path.is_dir() {
            self.analyze_directory(target_path)?
        } else if target_path.is_file() {
            let content = std::fs::read_to_string(target_path).map_err(|e| {
                HunterError::AnalysisFailed(format!("Failed to read {}: {}", config.target, e))
            })?;
            self.analyze_file(target_path, &content)?
        } else {
            return Err(HunterError::TargetNotFound(config.target.clone()));
        };

        Ok(AnalyzerOutput { findings })
    }

    fn clone_box(&self) -> Box<dyn Analyzer> {
        Box::new(PegStructuralAnalyzer {
            grammars: self.grammars.clone(),
        })
    }
}

/// Recursive directory scanner (mirrors static_code.rs approach).
fn scan_directory(dir: &Path, analyzer: &PegStructuralAnalyzer, findings: &mut Vec<Finding>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            // Skip hidden dirs and target/build artifacts.
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            scan_directory(&path, analyzer, findings);
        } else if matches!(detect_language(&path).as_str(), "rust" | "python")
            && let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(file_findings) = analyzer.analyze_file(&path, &content)
        {
            findings.extend(file_findings);
        }
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Detect the language from a file path extension.
pub(crate) fn detect_language(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "rs" => "rust".to_string(),
        "py" => "python".to_string(),
        _ => "unknown".to_string(),
    }
}

/// Find a PEG grammar file by searching known locations.
fn find_grammar_file(name: &str) -> Option<PathBuf> {
    let candidates = [
        // Canonical location in pesti-structural crate.
        PathBuf::from(format!(
            "../pesti/crates/pesti-structural/grammars/{}",
            name
        )),
        // Test data fallback.
        PathBuf::from(format!(
            "../pesti/crates/pesti-structural/testdata/{}",
            name
        )),
        // Absolute fallbacks.
        PathBuf::from(format!(
            "/home/crombo/projects/active/pesti/crates/pesti-structural/grammars/{}",
            name
        )),
        PathBuf::from(format!(
            "/home/crombo/projects/active/pesti/crates/pesti-structural/testdata/{}",
            name
        )),
    ];
    for path in candidates {
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Convert a byte offset to a line number (1-indexed).
pub(crate) fn byte_to_line(source: &str, byte_offset: usize) -> usize {
    source[..byte_offset.min(source.len())]
        .matches('\n')
        .count()
        + 1
}

/// Extract a structural feature from a parse tree node.
fn extract_feature(rule_name: &str, node: &TreeNode, source: &str) -> PegFeature {
    let text = node.span.text(source).to_string();
    let mut captures = HashMap::new();
    for (name, capture_node) in &node.captures {
        captures.insert(name.clone(), capture_node.span.text(source).to_string());
    }
    let children = node
        .children
        .iter()
        .map(|child| extract_feature(&child.label, child, source))
        .collect();

    PegFeature {
        rule: rule_name.to_string(),
        start: node.span.start,
        end: node.span.end,
        text,
        captures,
        children,
    }
}

/// Recursively traverse a parse tree and check each node against vulnerability rules.
fn traverse_and_check(
    node: &TreeNode,
    content: &str,
    path: &Path,
    language: &str,
    findings: &mut Vec<Finding>,
) {
    // Extract feature from this node.
    let feature = extract_feature(&node.label, node, content);

    // Check this node against vulnerability patterns.
    findings.extend(check_structural_rules_single(&feature, path, language));

    // Recurse into children.
    for child in &node.children {
        traverse_and_check(child, content, path, language, findings);
    }

    // Also recurse into captures (where the actual parsed content lives).
    for capture_node in node.captures.values() {
        traverse_and_check(capture_node, content, path, language, findings);
    }
}

/// Count parameters from captures (heuristic).
pub(crate) fn count_params_from_captures(captures: &HashMap<String, String>) -> usize {
    if let Some(params) = captures.get("params").or(captures.get("parameters")) {
        params.split(',').filter(|s| !s.trim().is_empty()).count()
    } else {
        0
    }
}

/// Estimate nesting depth from children.
pub(crate) fn estimate_depth(children: &[PegFeature]) -> usize {
    if children.is_empty() {
        return 0;
    }
    1 + children
        .iter()
        .map(|c| estimate_depth(&c.children))
        .max()
        .unwrap_or(0)
}

/// Truncate a string to a maximum length.
pub(crate) fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.min(s.len())])
    }
}

/// Compute a simple u64 hash of a string for ID generation.
pub(crate) fn simple_hash(s: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}
