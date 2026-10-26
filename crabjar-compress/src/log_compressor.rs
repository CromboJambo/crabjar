//! Log compression: severity scoring, deduplicate warnings, keep errors.

use super::{CompressionConfig, CompressionResult};

/// Compress log output by severity and deduplication.
pub fn compress_logs(text: &str, config: &CompressionConfig) -> CompressionResult {
    let lines: Vec<&str> = text.lines().collect();

    if lines.len() <= config.log_line_limit {
        return CompressionResult::new(text, text.to_string(), super::ContentType::Log);
    }

    // Score each line by severity
    let mut scored: Vec<(i32, &str)> = Vec::with_capacity(lines.len());
    for line in &lines {
        scored.push((severity_score(line), *line));
    }

    // Sort by severity (highest first)
    scored.sort_by_key(|b| std::cmp::Reverse(b.0));

    // Keep high-severity lines, summarize low-severity
    let mut output = String::new();
    let mut error_count = 0;
    let mut warn_count = 0;
    let mut info_count = 0;
    let mut shown_errors = 0;

    for (score, line) in &scored {
        if *score >= 3 {
            // Error or fatal — always show (up to a limit)
            if shown_errors < 10 {
                output.push_str(line);
                output.push('\n');
                shown_errors += 1;
            }
            error_count += 1;
        } else if *score >= 2 {
            warn_count += 1;
        } else {
            info_count += 1;
        }
    }

    // Add summary
    output.push_str("--- Log Summary ---\n");
    if error_count > shown_errors {
        output.push_str(&format!(
            "{} errors (showing {})\n",
            error_count, shown_errors
        ));
    } else if error_count > 0 {
        output.push_str(&format!("{} errors\n", error_count));
    }
    if warn_count > 0 {
        output.push_str(&format!("{} warnings\n", warn_count));
    }
    if info_count > 0 {
        output.push_str(&format!("{} info/debug lines\n", info_count));
    }

    CompressionResult::new(text, output, super::ContentType::Log)
}

fn severity_score(line: &str) -> i32 {
    let lower = line.to_lowercase();
    if lower.contains("fatal") || lower.contains("panic") {
        5
    } else if lower.contains("error") || lower.contains("exception") {
        4
    } else if lower.contains("warn") {
        3
    } else if lower.contains("info") {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_logs() {
        let logs = "[INFO] Starting\n[WARN] Slow query\n[ERROR] Connection failed\n[ERROR] Timeout\n[INFO] Retrying";
        let config = CompressionConfig {
            log_line_limit: 3,
            ..Default::default()
        };
        let result = compress_logs(logs, &config);
        assert!(result.compressed.contains("Log Summary"));
    }
}
