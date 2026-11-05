//! Semantic pattern detection for C source.

use std::collections::HashMap;

/// Detect pointer patterns.
pub fn detect_pointer_patterns(source: &str) -> HashMap<String, String> {
    let mut tags = HashMap::new();

    if source.contains('*') {
        tags.insert("pointer_usage".to_string(), "true".to_string());
    }
    if source.contains("**") || source.contains("(*") {
        tags.insert("pointer_to_pointer".to_string(), "true".to_string());
    }

    tags
}

/// Detect memory management patterns.
pub fn detect_memory_patterns(source: &str) -> HashMap<String, String> {
    let mut tags = HashMap::new();

    if source.contains("malloc") || source.contains("calloc") || source.contains("realloc") {
        tags.insert("dynamic_allocation".to_string(), "true".to_string());
    }
    if source.contains("free") {
        tags.insert("manual_deallocation".to_string(), "true".to_string());
    }

    tags
}

/// Detect preprocessor patterns.
pub fn detect_preprocessor_patterns(source: &str) -> HashMap<String, String> {
    let mut tags = HashMap::new();

    if source.contains("#ifdef") || source.contains("#ifndef") {
        tags.insert("conditional_compilation".to_string(), "true".to_string());
    }
    if source.contains("#include") {
        tags.insert("has_includes".to_string(), "true".to_string());
    }
    if source.contains("#define") {
        tags.insert("has_macros".to_string(), "true".to_string());
    }

    tags
}
