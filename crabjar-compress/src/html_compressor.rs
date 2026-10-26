//! HTML compression: strip tags → prose compression.

use super::prose_compressor::compress_prose;
use super::{CompressionConfig, CompressionResult};

/// Compress HTML by stripping tags and applying prose compression.
pub fn compress_html(text: &str, config: &CompressionConfig) -> CompressionResult {
    // Strip HTML tags using regex
    let tag_re = regex::Regex::new(r"<[^>]*>").unwrap();
    let text_only = tag_re.replace_all(text.trim(), " ").to_string();

    // Collapse multiple spaces
    let space_re = regex::Regex::new(r"\s+").unwrap();
    let clean = space_re.replace_all(&text_only, " ").to_string();

    // Apply prose compression to the text content
    compress_prose(&clean, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_html() {
        let html = "<html><body><h1>Title</h1><p>Some paragraph text here.</p></body></html>";
        let config = CompressionConfig::default();
        let result = compress_html(html, &config);
        assert!(!result.compressed.contains("<"));
    }
}
