//! crabjar-review-benchmark — Run code review pipeline benchmarks.
//!
//! Usage: crabjar-review-benchmark [benchmark-dir]
//!
//! Benchmark format: JSON files with PR metadata, findings, and expected rule IDs.
//! Example benchmark data in crabjar-review/benchmarks/.

use std::fs;
use std::path::PathBuf;

use crabjar_review::benchmark::{BenchmarkPR, run_all_benchmarks};

fn load_benchmarks(benchmark_dir: &str) -> Vec<BenchmarkPR> {
    let mut prs = Vec::new();

    let path = PathBuf::from(benchmark_dir);
    if !path.exists() {
        eprintln!("Benchmark directory not found: {}", benchmark_dir);
        return prs;
    }

    for entry in fs::read_dir(&path).expect("Failed to read benchmark dir") {
        let entry = entry.expect("Failed to read entry");
        let file_path = entry.path();

        if !file_path.extension().map_or(false, |ext| ext == "json") {
            continue;
        }

        let content = fs::read_to_string(&file_path).expect("Failed to read benchmark file");
        let data: serde_json::Value =
            serde_json::from_str(&content).expect("Invalid JSON in benchmark file");

        let pr = BenchmarkPR {
            name: data["name"].as_str().unwrap_or("unknown").to_string(),
            findings: data["findings"]
                .as_array()
                .map(|arr| arr.to_vec())
                .unwrap_or_default(),
            expected_rule_ids: data["expected_rule_ids"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default(),
        };

        prs.push(pr);
    }

    prs
}

fn main() {
    let benchmark_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "crabjar-review/benchmarks".to_string());

    println!("Loading benchmarks from: {}", benchmark_dir);
    let prs = load_benchmarks(&benchmark_dir);

    if prs.is_empty() {
        println!("No benchmark PRs found.");
        return;
    }

    println!("Loaded {} benchmark PRs", prs.len());
    run_all_benchmarks(&prs);
}
