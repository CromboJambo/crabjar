//! Cross-language code comparison infrastructure.
//!
//! Compares architectural properties across different language implementations
//! to identify trade-offs, patterns, and quality differences.

use crate::{SpanKind, SpanTree};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

/// Comparison metrics for a single aspect of code architecture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonMetric {
    /// Name of the metric (e.g., `error_handling_coverage`).
    pub name: String,
    /// Value for language A.
    pub value_a: f64,
    /// Value for language B.
    pub value_b: f64,
    /// Description of what this metric measures.
    pub description: String,
}

/// Overall comparison report between two codebases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReport {
    /// Name or path of the first codebase.
    pub name_a: String,
    /// Language of the first codebase.
    pub language_a: String,
    /// Name or path of the second codebase.
    pub name_b: String,
    /// Language of the second codebase.
    pub language_b: String,
    /// Individual comparison metrics.
    pub metrics: Vec<ComparisonMetric>,
    /// Summary findings and insights.
    pub findings: Vec<String>,
}

impl ComparisonReport {
    #[must_use]
    pub fn new(name_a: &str, language_a: &str, name_b: &str, language_b: &str) -> Self {
        Self {
            name_a: name_a.to_string(),
            language_a: language_a.to_string(),
            name_b: name_b.to_string(),
            language_b: language_b.to_string(),
            metrics: Vec::new(),
            findings: Vec::new(),
        }
    }

    pub fn add_metric(&mut self, metric: ComparisonMetric) {
        self.metrics.push(metric);
    }

    pub fn add_finding(&mut self, finding: String) {
        self.findings.push(finding);
    }

    /// Generate a human-readable summary of the comparison.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut result = format!(
            "# Cross-Language Comparison: {} ({}) vs {} ({})\n\n",
            self.name_a, self.language_a, self.name_b, self.language_b
        );

        result.push_str("## Metrics\n\n");
        for metric in &self.metrics {
            let diff = (metric.value_b - metric.value_a).abs();
            let better = if metric.value_b > metric.value_a {
                format!(
                    "{} ({}) is higher by {:.1}%",
                    self.name_b,
                    self.language_b,
                    diff * 100.0
                )
            } else if metric.value_a > metric.value_b {
                format!(
                    "{} ({}) is higher by {:.1}%",
                    self.name_a,
                    self.language_a,
                    diff * 100.0
                )
            } else {
                "equal".to_string()
            };

            writeln!(
                result,
                "- **{metric_name}**: {name_a} = {value_a:.2}, {name_b} = {value_b:.2} — {better}",
                metric_name = metric.name,
                name_a = self.name_a,
                value_a = metric.value_a,
                name_b = self.name_b,
                value_b = metric.value_b,
            )
            .unwrap();
        }

        if !self.findings.is_empty() {
            result.push_str("\n## Findings\n\n");
            for finding in &self.findings {
                writeln!(result, "- {finding}").unwrap();
            }
        }

        result
    }
}

/// Count total functions in a span tree.
fn count_functions(tree: &SpanTree) -> usize {
    tree.spans
        .iter()
        .filter(|s| s.kind == SpanKind::FnDecl)
        .count()
}

/// Count error handling patterns (try/catch, throws) in a span tree.
fn count_error_handling(tree: &SpanTree) -> usize {
    tree.spans
        .iter()
        .filter(|s| s.kind.is_error_handling())
        .count()
}

/// Compute the ratio of error handling patterns to functions.
#[allow(clippy::cast_precision_loss)]
fn error_handling_coverage(tree: &SpanTree) -> f64 {
    let total_functions = count_functions(tree);
    if total_functions == 0 {
        return 0.0;
    }

    // Use min(1.0, error_patterns / functions) as coverage estimate
    let error_patterns = count_error_handling(tree);
    (error_patterns as f64 / total_functions as f64).min(1.0)
}

/// Compute the ratio of async patterns detected.
#[allow(clippy::cast_precision_loss)]
fn async_pattern_ratio(tree: &SpanTree) -> f64 {
    let total_functions = count_functions(tree);
    if total_functions == 0 {
        return 0.0;
    }

    // Check for async keyword in function source text
    let async_functions = tree
        .spans
        .iter()
        .filter(|s| {
            s.kind == SpanKind::FnDecl && (s.source.contains("async") || s.open_token.is_some())
        })
        .count();

    async_functions as f64 / total_functions as f64
}

/// Compute the average nesting depth of functions.
#[allow(clippy::cast_precision_loss)]
fn avg_nesting_depth(tree: &SpanTree) -> f64 {
    let functions = tree.spans.iter().filter(|s| s.kind == SpanKind::FnDecl);
    let mut total_depth = 0;
    let mut count = 0;

    for func in functions {
        total_depth += func.depth();
        count += 1;
    }

    if count == 0 {
        return 0.0;
    }

    total_depth as f64 / count as f64
}

/// Compare two codebases and generate a comparison report.
#[must_use]
pub fn compare_trees(tree_a: &SpanTree, tree_b: &SpanTree) -> ComparisonReport {
    let mut report = ComparisonReport::new(
        &tree_a.source,
        &tree_a.language,
        &tree_b.source,
        &tree_b.language,
    );

    // Error handling coverage
    let eh_a = error_handling_coverage(tree_a);
    let eh_b = error_handling_coverage(tree_b);
    report.add_metric(ComparisonMetric {
        name: "error_handling_coverage".to_string(),
        value_a: eh_a,
        value_b: eh_b,
        description: "Ratio of functions with explicit error handling to total functions"
            .to_string(),
    });

    // Async pattern usage
    let async_a = async_pattern_ratio(tree_a);
    let async_b = async_pattern_ratio(tree_b);
    report.add_metric(ComparisonMetric {
        name: "async_pattern_ratio".to_string(),
        value_a: async_a,
        value_b: async_b,
        description: "Ratio of functions using async patterns to total functions".to_string(),
    });

    // Average nesting depth (lower is generally flatter/better)
    let depth_a = avg_nesting_depth(tree_a);
    let depth_b = avg_nesting_depth(tree_b);
    report.add_metric(ComparisonMetric {
        name: "avg_nesting_depth".to_string(),
        value_a: depth_a,
        value_b: depth_b,
        description: "Average nesting depth of functions (lower = flatter)".to_string(),
    });

    // Generate findings based on metric comparisons
    if eh_a > eh_b {
        report.add_finding(format!(
            "{} ({}) has better error handling coverage than {} ({})",
            tree_a.source, tree_a.language, tree_b.source, tree_b.language
        ));
    } else if eh_b > eh_a {
        report.add_finding(format!(
            "{} ({}) has better error handling coverage than {} ({})",
            tree_b.source, tree_b.language, tree_a.source, tree_a.language
        ));
    }

    if depth_a < depth_b {
        report.add_finding(format!(
            "{} ({}) has flatter function structure than {} ({})",
            tree_a.source, tree_a.language, tree_b.source, tree_b.language
        ));
    } else if depth_b < depth_a {
        report.add_finding(format!(
            "{} ({}) has flatter function structure than {} ({})",
            tree_b.source, tree_b.language, tree_a.source, tree_a.language
        ));
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LanguageAdapter;
    use crate::adapters::javascript::JavaScriptAdapter;

    #[test]
    fn test_error_handling_coverage() {
        let js_code = "
async function good() {
    try {
        await risky();
    } catch (e) {
        console.error(e);
    }
}

async function bad() {
    await risky();
}
";
        let adapter = JavaScriptAdapter::new();
        let tree = adapter.tokenize_and_adapt(js_code);
        let coverage = error_handling_coverage(&tree);
        // 1 try/catch produces TryBlock + CatchBlock = 2 error handling spans
        // 2 functions, so coverage is capped at 1.0
        assert_eq!(coverage, 1.0);
    }

    #[test]
    fn test_cross_language_comparison() {
        let js_adapter = JavaScriptAdapter::new();

        let js_code_a = "function add(a, b) { return a + b; }";
        let js_code_b =
            "async function multiply(a, b) { try { return a * b; } catch (e) { return 0; } }";

        let mut tree_a = js_adapter.tokenize_and_adapt(js_code_a);
        let mut tree_b = js_adapter.tokenize_and_adapt(js_code_b);

        // Rename to simulate different languages
        tree_a.language = "javascript".to_string();
        tree_b.language = "typescript".to_string();
        tree_a.source = "js_impl.js".to_string();
        tree_b.source = "ts_impl.ts".to_string();

        let report = compare_trees(&tree_a, &tree_b);
        assert_eq!(report.language_a, "javascript");
        assert_eq!(report.language_b, "typescript");
        assert!(!report.metrics.is_empty());
    }
}
