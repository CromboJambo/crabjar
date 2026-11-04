//! Go pattern detection for semantic annotation.
//!
//! Detects goroutine usage, interface patterns, and error handling idioms.

use std::collections::HashMap;

/// Detect goroutine patterns in Go source.
pub fn detect_goroutine_patterns(source: &str) -> HashMap<String, String> {
    let mut tags = HashMap::new();

    if source.contains("go ") && (source.contains('(') || source.contains('{')) {
        tags.insert("goroutine".to_string(), "true".to_string());
    }
    if source.contains("chan ") {
        tags.insert("channel".to_string(), "true".to_string());
    }

    tags
}

/// Detect interface patterns in Go source.
pub fn detect_interface_patterns(source: &str) -> HashMap<String, String> {
    let mut tags = HashMap::new();

    if source.contains("interface{") || source.contains("type ") && source.contains("interface") {
        tags.insert("interface".to_string(), "true".to_string());
    }

    tags
}

/// Detect Go error handling patterns.
pub fn detect_error_handling_patterns(source: &str) -> HashMap<String, String> {
    let mut tags = HashMap::new();

    if source.contains(", err :=") || source.contains(", err :=") {
        tags.insert("error_checking".to_string(), "idiomatic".to_string());
    } else if source.contains("panic(") {
        tags.insert("error_handling".to_string(), "panic".to_string());
    }

    tags
}
