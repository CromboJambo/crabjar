//! Search result compression: group by file, keep top N matches.

use super::{CompressionConfig, CompressionResult};

/// Compress search results (grep-style output).
pub fn compress_search_results(text: &str, config: &CompressionConfig) -> CompressionResult {
    let lines: Vec<&str> = text.lines().collect();

    if lines.len() <= config.search_result_limit {
        return CompressionResult::new(text, text.to_string(), super::ContentType::Search);
    }

    // Group by file
    let mut grouped: std::collections::HashMap<String, Vec<&str>> =
        std::collections::HashMap::new();
    for line in &lines {
        if let Some(file) = extract_file(line) {
            grouped.entry(file).or_default().push(*line);
        }
    }

    // Build compressed output
    let mut output = String::new();
    let mut total_files = 0;
    let mut shown_files = 0;

    for (file, matches) in grouped.iter() {
        total_files += 1;
        if shown_files >= config.search_result_limit / 2 {
            break;
        }
        shown_files += 1;

        output.push_str(&format!("{}: {} match(es)\n", file, matches.len()));
        // Show first few matches for each file
        for match_line in matches.iter().take(3) {
            let line_num = extract_line_number(match_line);
            if let Some(num) = line_num {
                output.push_str(&format!("  {}:...\n", num));
            }
        }
    }

    if total_files > shown_files {
        output.push_str(&format!(
            "... [{} more file(s) with matches]\n",
            total_files - shown_files
        ));
    }

    CompressionResult::new(text, output, super::ContentType::Search)
}

fn extract_file(line: &str) -> Option<String> {
    // Pattern: file.rs:line:content or file.rs:line
    let parts: Vec<&str> = line.split(':').collect();
    if parts.len() >= 2 && parts[0].contains('.') {
        Some(parts[0].to_string())
    } else {
        None
    }
}

fn extract_line_number(line: &str) -> Option<u32> {
    let parts: Vec<&str> = line.split(':').collect();
    if parts.len() >= 2 {
        parts[1].parse::<u32>().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_search_results() {
        let mut search = String::new();
        for i in 1..30 {
            search.push_str(&format!("file{}.rs:{}:some match\n", i % 5, i));
        }
        let config = CompressionConfig {
            search_result_limit: 6,
            ..Default::default()
        };
        let result = compress_search_results(&search, &config);
        assert!(result.compressed.contains("match(es)"));
        assert!(result.compressed.contains("more file(s)"));
    }
}
