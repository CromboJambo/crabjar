//! Go pattern detection for semantic annotation.
//!
//! Detects goroutine usage, interface patterns, and error handling idioms.

use std::collections::HashMap;

/// Detect goroutine patterns in Go source.
pub fn detect_goroutine_patterns(source: &str) -> HashMap<String, String> {
    let mut tags = HashMap::new();

    if source.contains("go ") && (source.contains('(') || source.contains('.')) {
        tags.insert("goroutines".to_string(), "true".to_string());
    }

    tags
}

/// Detect channel patterns in Go source.
pub fn detect_channel_patterns(source: &str) -> HashMap<String, String> {
    let mut tags = HashMap::new();

    if source.contains("chan ") || source.contains("<-") {
        tags.insert("channels".to_string(), "true".to_string());
    }

    tags
}

/// Detect error handling patterns in Go source.
pub fn detect_error_patterns(source: &str) -> HashMap<String, String> {
    let mut tags = HashMap::new();

    if source.contains("defer ") {
        tags.insert("defer_used".to_string(), "true".to_string());
    }
    if source.contains("recover()") || source.contains("panic(") {
        tags.insert("panic_recover".to_string(), "true".to_string());
    }

    tags
}
