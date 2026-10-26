//! Prose compression: head + key sentences + tail.

use super::{CompressionConfig, CompressionResult};

/// Compress generic prose by keeping head, key sentences, and tail.
pub fn compress_prose(text: &str, _config: &CompressionConfig) -> CompressionResult {
    let lines: Vec<&str> = text.lines().collect();

    if lines.len() <= 20 {
        return CompressionResult::new(text, text.to_string(), super::ContentType::Prose);
    }

    // Keep first few lines (head)
    let head_lines = lines.iter().take(3).cloned().collect::<Vec<_>>();

    // Find "key" sentences (those with important keywords or longer length)
    let mut key_lines: Vec<&str> = Vec::new();
    for line in &lines[3..] {
        if is_key_sentence(line) && key_lines.len() < 5 {
            key_lines.push(*line);
        }
    }

    // Keep last few lines (tail)
    let tail_start = lines.len().saturating_sub(2);
    let tail_lines = lines.iter().skip(tail_start).cloned().collect::<Vec<_>>();

    // Build compressed output
    let mut output = String::new();
    for line in head_lines {
        output.push_str(line);
        output.push('\n');
    }
    if key_lines.is_empty() {
        output.push_str("... [content omitted]\n");
    } else {
        output.push_str("--- Key points ---\n");
        for line in key_lines {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                output.push_str("- ");
                output.push_str(trimmed);
                output.push('\n');
            }
        }
    }
    for line in tail_lines {
        output.push_str(line);
        output.push('\n');
    }

    CompressionResult::new(text, output, super::ContentType::Prose)
}

fn is_key_sentence(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }

    // Longer lines are more likely to be important
    if trimmed.len() > 100 {
        return true;
    }

    // Lines with action words or conclusions
    let lower = trimmed.to_lowercase();
    let keywords = [
        "important",
        "note",
        "warning",
        "error",
        "success",
        "failed",
        "completed",
        "result",
    ];
    for kw in &keywords {
        if lower.contains(kw) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_prose() {
        let mut prose = String::new();
        for i in 1..30 {
            prose.push_str(&format!("Line {} of content.\n", i));
        }
        let config = CompressionConfig::default();
        let result = compress_prose(&prose, &config);
        assert!(
            result.compressed.contains("Key points")
                || result.compressed.contains("content omitted")
        );
    }
}
