//! crabjar-compress: Deterministic, content-aware tool result compression.
//!
//! Zero-LLM, zero-dependency pipeline that reduces token count 60-95% while
//! preserving semantic fidelity. Port of Headroom's Rust compression algorithms
//! to the CrabJar ecosystem.
//!
//! ## Usage
//! ```rust
//! use crabjar_compress::{compress_tool_result, CompressionConfig};
//!
//! let raw = r#"{"users": [{"name": "Alice", "email": "alice@example.com"}, ...]}"#;
//! let result = compress_tool_result(raw, &CompressionConfig::default());
//! println!("Original: {} chars", raw.len());
//! println!("Compressed: {} chars ({}% reduction)", result.compressed.len(), result.reduction_percent);
//! ```

mod content_router;
mod diff_compressor;
mod html_compressor;
mod json_compressor;
mod log_compressor;
mod prose_compressor;
mod search_compressor;

pub use content_router::{detect_content_type, ContentType};
pub use diff_compressor::compress_diff;
pub use html_compressor::compress_html;
pub use json_compressor::compress_json;
pub use log_compressor::compress_logs;
pub use prose_compressor::compress_prose;
pub use search_compressor::compress_search_results;

/// Compression result with metadata.
#[derive(Debug, Clone)]
pub struct CompressionResult {
    /// The compressed content.
    pub compressed: String,
    /// Original character count.
    pub original_chars: usize,
    /// Compressed character count.
    pub compressed_chars: usize,
    /// Percentage reduction (0-100).
    pub reduction_percent: u8,
    /// Content type that was detected.
    pub content_type: ContentType,
}

impl CompressionResult {
    fn new(original: &str, compressed: String, content_type: ContentType) -> Self {
        let original_chars = original.len();
        let compressed_chars = compressed.len();
        let reduction = if original_chars > 0 {
            let diff = (original_chars as i64 - compressed_chars as i64).max(0);
            ((diff as f64 / original_chars as f64) * 100.0).min(100.0) as u8
        } else {
            0
        };
        Self {
            compressed,
            original_chars,
            compressed_chars,
            reduction_percent: reduction,
            content_type,
        }
    }
}

/// Configuration for tool result compression.
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Minimum token threshold to trigger compression (default: 100).
    pub min_tokens: usize,
    /// Maximum tokens after compression (default: 500).
    pub max_tokens: usize,
    /// JSON: maximum array elements to keep (default: 20).
    pub json_array_limit: usize,
    /// JSON: maximum string length before truncation (default: 100).
    pub json_string_limit: usize,
    /// Search: maximum results to keep (default: 10).
    pub search_result_limit: usize,
    /// Log: maximum lines to keep (default: 50).
    pub log_line_limit: usize,
    /// Diff: maximum files to include (default: 5).
    pub diff_file_limit: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            min_tokens: 100,
            max_tokens: 500,
            json_array_limit: 20,
            json_string_limit: 100,
            search_result_limit: 10,
            log_line_limit: 50,
            diff_file_limit: 5,
        }
    }
}

/// Compress a tool result string using content-type-aware strategies.
///
/// This is the primary API — it detects the content type and applies the
/// appropriate compression strategy. If compression fails or produces no
/// savings, the original text is returned unchanged.
pub fn compress_tool_result(text: &str, config: &CompressionConfig) -> CompressionResult {
    let content_type = detect_content_type(text);

    // Only compress if above minimum threshold (rough estimate: 4 chars per token)
    let estimated_tokens = text.len() / 4;
    if estimated_tokens < config.min_tokens {
        return CompressionResult::new(text, text.to_string(), content_type);
    }

    match content_type {
        ContentType::Json => compress_json(text, config),
        ContentType::Search => compress_search_results(text, config),
        ContentType::Log => compress_logs(text, config),
        ContentType::Diff => compress_diff(text, config),
        ContentType::Html => compress_html(text, config),
        ContentType::Prose => compress_prose(text, config),
    }
}

/// Estimate token count from character count.
pub fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_small_text_noop() {
        let text = "Short text";
        let config = CompressionConfig::default();
        let result = compress_tool_result(text, &config);
        assert_eq!(result.compressed, text);
        assert_eq!(result.reduction_percent, 0);
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("hello"), 1);
        assert_eq!(estimate_tokens(&"a".repeat(400)), 100);
    }
}
