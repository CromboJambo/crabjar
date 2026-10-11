use crate::reflection::{process_findings, generate_reflection};
use serde_json::Value;

/// Benchmark PR with known issues (ground truth).
pub struct BenchmarkPR {
    pub name: String,
    pub findings: Vec<Value>, // Raw agent findings for this PR
    pub expected_rule_ids: Vec<String>, // rule_ids that should be found (ground truth)
}

/// Benchmark results for a single PR.
pub struct BenchmarkResults {
    pub pr_name: String,
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
}

impl BenchmarkResults {
    fn new(pr_name: &str) -> Self {
        Self {
            pr_name: pr_name.to_string(),
            true_positives: 0,
            false_positives: 0,
            false_negatives: 0,
            precision: 0.0,
            recall: 0.0,
            f1_score: 0.0,
        }
    }

    fn compute_metrics(&mut self) {
        let tp = self.true_positives as f64;
        let fp = self.false_positives as f64;
        let fn_ = self.false_negatives as f64;

        self.precision = if tp + fp > 0.0 { tp / (tp + fp) } else { 1.0 };
        self.recall = if tp + fn_ > 0.0 { tp / (tp + fn_) } else { 1.0 };

        if self.precision + self.recall > 0.0 {
            self.f1_score = 2.0 * (self.precision * self.recall) / (self.precision + self.recall);
        }
    }
}

/// Run the post-processor on a benchmark PR and evaluate against ground truth.
fn evaluate_pr(pr: &BenchmarkPR) -> BenchmarkResults {
    let mut results = BenchmarkResults::new(&pr.name);

    // Phase 3: Post-process findings (dedup, reflect, sort)
    let processed = process_findings(&pr.findings);

    // Extract rule_ids from processed findings
    let found_rule_ids: Vec<String> = processed.iter()
        .filter_map(|f| f["rule_id"].as_str().map(String::from))
        .collect();

    // Compare against expected (ground truth)
    for expected_id in &pr.expected_rule_ids {
        if found_rule_ids.contains(expected_id) {
            results.true_positives += 1;
        } else {
            results.false_negatives += 1;
        }
    }

    for found_id in found_rule_ids {
        if !pr.expected_rule_ids.contains(&found_id) {
            results.false_positives += 1;
        }
    }

    results.compute_metrics();
    results
}

/// Run all benchmarks and print summary.
pub fn run_all_benchmarks(prs: &[BenchmarkPR]) -> Vec<BenchmarkResults> {
    let mut all_results = Vec::new();

    for pr in prs {
        let results = evaluate_pr(pr);
        println!(
            "{} — Precision: {:.2}, Recall: {:.2}, F1: {:.2} (TP={}, FP={}, FN={})",
            results.pr_name,
            results.precision,
            results.recall,
            results.f1_score,
            results.true_positives,
            results.false_positives,
            results.false_negatives
        );
        all_results.push(results);
    }

    // Aggregate metrics
    let total_tp: usize = all_results.iter().map(|r| r.true_positives).sum();
    let total_fp: usize = all_results.iter().map(|r| r.false_positives).sum();
    let total_fn: usize = all_results.iter().map(|r| r.false_negatives).sum();

    let avg_precision: f64 = all_results.iter().map(|r| r.precision).sum::<f64>() / all_results.len() as f64;
    let avg_recall: f64 = all_results.iter().map(|r| r.recall).sum::<f64>() / all_results.len() as f64;
    let avg_f1: f64 = all_results.iter().map(|r| r.f1_score).sum::<f64>() / all_results.len() as f64;

    println!("\n=== Benchmark Summary ===");
    println!("PRs tested: {}", prs.len());
    println!("True positives: {}", total_tp);
    println!("False positives: {}", total_fp);
    println!("False negatives: {}", total_fn);
    println!("Average precision: {:.2}", avg_precision);
    println!("Average recall: {:.2}", avg_recall);
    println!("Average F1 score: {:.2}", avg_f1);

    all_results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_finding(rule_id: &str, file: &str, line: u32) -> Value {
        serde_json::json!({
            "file": file,
            "line": line,
            "severity": "high",
            "type": "bug",
            "rule_id": rule_id,
            "description": "Test finding for benchmark"
        })
    }

    #[test]
    fn test_benchmark_perfect_precision_recall() {
        let pr = BenchmarkPR {
            name: "perfect".to_string(),
            findings: vec![
                make_finding("check_errors", "src/a.rs", 10),
                make_finding("check_style", "src/b.rs", 20),
            ],
            expected_rule_ids: vec!["check_errors".to_string(), "check_style".to_string()],
        };

        let results = evaluate_pr(&pr);
        assert_eq!(results.true_positives, 2);
        assert_eq!(results.false_positives, 0);
        assert_eq!(results.false_negatives, 0);
        assert!((results.f1_score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_benchmark_false_positive() {
        let pr = BenchmarkPR {
            name: "false-positive".to_string(),
            findings: vec![
                make_finding("check_errors", "src/a.rs", 10),
                make_finding("check_style", "src/b.rs", 20), // extra, not expected
            ],
            expected_rule_ids: vec!["check_errors".to_string()],
        };

        let results = evaluate_pr(&pr);
        assert_eq!(results.true_positives, 1);
        assert_eq!(results.false_positives, 1);
        assert_eq!(results.false_negatives, 0);
    }

    #[test]
    fn test_benchmark_false_negative() {
        let pr = BenchmarkPR {
            name: "false-negative".to_string(),
            findings: vec![make_finding("check_errors", "src/a.rs", 10)],
            expected_rule_ids: vec!["check_errors".to_string(), "check_style".to_string()],
        };

        let results = evaluate_pr(&pr);
        assert_eq!(results.true_positives, 1);
        assert_eq!(results.false_positives, 0);
        assert_eq!(results.false_negatives, 1);
    }

    #[test]
    fn test_benchmark_deduplication_affects_metrics() {
        // Duplicate findings should be deduplicated by post-processor
        let pr = BenchmarkPR {
            name: "dedup".to_string(),
            findings: vec![
                make_finding("check_errors", "src/a.rs", 10),
                make_finding("check_errors", "src/a.rs", 11), // duplicate (same file, close line)
            ],
            expected_rule_ids: vec!["check_errors".to_string()],
        };

        let results = evaluate_pr(&pr);
        assert_eq!(results.true_positives, 1);
        assert_eq!(results.false_positives, 0); // dedup prevents FP
    }
}
