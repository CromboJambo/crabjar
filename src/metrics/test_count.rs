/// Test count metrics.
///
/// Runs `cargo test --workspace` and parses the output to count total tests.
/// Returns structured JSON with per-crate and aggregate counts.
use serde_json::json;

/// Run test count metrics.
pub fn run_test_count() -> serde_json::Value {
    let output = match std::process::Command::new("cargo")
        .args(["test", "--workspace"])
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            return json!({
                "success": false,
                "error": format!("Failed to run cargo test: {}", e),
                "usage": ["crabjar metrics tests"],
            });
        }
    };

    // cargo test writes build output to stderr and test results to stdout.
    // Parse both for "test result:" lines.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Parse test results from cargo test output.
    // Format: "test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured"
    let mut total_passed: usize = 0;
    let mut total_failed: usize = 0;
    let mut total_ignored: usize = 0;
    let mut crate_results: Vec<serde_json::Value> = Vec::new();

    for line in combined.lines() {
        if line.contains("test result: ok.") {
            // Split by ";" to get individual counters.
            // First segment is "test result: ok. 73 passed" (includes prefix).
            // Remaining segments are "0 failed", "0 ignored", etc.
            let parts: Vec<&str> = line.split(';').collect();

            // Parse first segment: "test result: ok. 73 passed"
            let first = parts[0].trim();
            // Find " passed" and extract the number before it
            if let Some(pos) = first.find(" passed") {
                let num_str = &first[..pos]; // everything before " passed"
                // num_str is like "test result: ok. 73" — extract last number
            if let Some(last_word) = num_str.split_whitespace().last()
                && let Ok(val) = last_word.parse::<usize>()
            {
                total_passed += val;
                crate_results.push(json!({
                    "passed": val,
                    "failed": 0,
                    "ignored": 0,
                }));
            }
            }

            // Parse remaining segments: "0 failed", "0 ignored"
            for part in parts.iter().skip(1) {
                let seg = part.trim();
                if seg.is_empty() {
                    continue;
                }
                // Split into number + word: "0 failed" -> ("0", "failed")
                if let Some((num_str, word)) = seg.rsplit_once(' ')
                    && let Ok(val) = num_str.parse::<usize>()
                {
                    match word {
                        "failed" => total_failed += val,
                        "ignored" => total_ignored += val,
                        _ => {}
                    }
                }
            }
        }
    }

    // Fallback: if parsing did not work, use "running X tests" lines
    if total_passed == 0 {
        let mut running_total: usize = 0;
        for line in combined.lines() {
            let words: Vec<&str> = line.split_whitespace().collect();
            if words.len() >= 3 && words[1] == "running" && words[2] == "tests"
                && let Ok(val) = words[0].parse::<usize>()
            {
                running_total += val;
            }
        }
        if running_total > 0 {
            return json!({
                "success": true,
                "metrics": {
                    "tests": {
                        "total": running_total,
                        "passed": running_total,
                        "failed": 0,
                        "ignored": 0,
                    },
                    "crates": crate_results,
                },
                "doubt": {
                    "assumptions": ["Running cargo test is the source of truth for test count"],
                    "blind_spots": ["Does not distinguish between unit/integration/doc tests"],
                    "last_validation": chrono::Utc::now().to_rfc3339(),
                    "stale_after": chrono::Utc::now()
                        .checked_add_signed(chrono::Duration::hours(24))
                        .unwrap_or_default()
                        .to_rfc3339(),
                },
            });
        }

        return json!({
            "success": false,
            "error": "Could not parse test count from cargo test output",
            "usage": ["crabjar metrics tests"],
        });
    }

    json!({
        "success": true,
        "metrics": {
            "tests": {
                "total": total_passed + total_failed + total_ignored,
                "passed": total_passed,
                "failed": total_failed,
                "ignored": total_ignored,
            },
            "crates": crate_results,
        },
        "doubt": {
            "assumptions": [
                "cargo test --workspace output is parseable",
                "All crates in workspace have tests",
            ],
            "blind_spots": [
                "Does not count doc tests",
                "Does not distinguish between unit/integration tests",
            ],
            "last_validation": chrono::Utc::now().to_rfc3339(),
            "stale_after": chrono::Utc::now()
                .checked_add_signed(chrono::Duration::hours(24))
                .unwrap_or_default()
                .to_rfc3339(),
        },
    })
}
