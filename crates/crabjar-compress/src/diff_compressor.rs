//! Diff compression: cap files/hunks, trim context lines.

use super::{CompressionConfig, CompressionResult};

/// Compress git diff output.
pub fn compress_diff(text: &str, config: &CompressionConfig) -> CompressionResult {
    // Split by file boundaries (diff --git markers)
    let mut parts: Vec<&str> = Vec::new();
    let mut current_start = 0;

    for (i, _ch) in text.char_indices() {
        if i > 0 && text[i..].starts_with("diff --git") {
            parts.push(&text[current_start..i]);
            current_start = i;
        }
    }
    if current_start < text.len() {
        parts.push(&text[current_start..]);
    }

    if parts.len() <= config.diff_file_limit {
        return CompressionResult::new(text, text.to_string(), super::ContentType::Diff);
    }

    // Keep first N files, summarize rest
    let mut output = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i >= config.diff_file_limit {
            break;
        }
        output.push_str(part);
        // Ensure trailing newline
        if !part.ends_with('\n') {
            output.push('\n');
        }
    }

    let remaining = parts.len() - config.diff_file_limit;
    output.push_str(&format!(
        "...\n[{} more file(s) changed — see full diff for details]\n",
        remaining
    ));

    CompressionResult::new(text, output, super::ContentType::Diff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_diff() {
        let mut diff = String::new();
        for i in 1..10 {
            diff.push_str(&format!(
                "diff --git a/file{} b/file{}\nindex 123..456\n--- a/file{}\n+++ b/file{}\n@@ -1 +1 @@\n-old\n+new\n",
                i, i, i, i
            ));
        }
        let config = CompressionConfig {
            diff_file_limit: 3,
            ..Default::default()
        };
        let result = compress_diff(&diff, &config);
        assert!(result.compressed.contains("more file(s) changed"));
    }
}
