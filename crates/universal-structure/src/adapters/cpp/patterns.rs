//! C++ pattern detection helpers for semantic annotations.

use std::collections::HashMap;

/// Detect template patterns.
pub fn detect_template_patterns(source: &str) -> HashMap<String, String> {
    let mut tags = HashMap::new();

    if source.contains('<') && source.contains('>') {
        tags.insert("template_usage".to_string(), "true".to_string());
    }
    if source.contains("template <") {
        tags.insert("template_declaration".to_string(), "true".to_string());
    }

    tags
}

/// Detect smart pointer patterns.
pub fn detect_smart_pointer_patterns(source: &str) -> HashMap<String, String> {
    let mut tags = HashMap::new();

    if source.contains("std::unique_ptr") || source.contains("make_unique") {
        tags.insert("smart_pointer".to_string(), "unique_ptr".to_string());
    } else if source.contains("std::shared_ptr") || source.contains("make_shared") {
        tags.insert("smart_pointer".to_string(), "shared_ptr".to_string());
    }

    tags
}

/// Detect modern C++ features.
pub fn detect_modern_cpp_patterns(source: &str) -> HashMap<String, String> {
    let mut tags = HashMap::new();

    if source.contains("auto") && (source.contains('=') || source.contains('(')) {
        tags.insert("type_inference".to_string(), "true".to_string());
    }
    if source.contains("nullptr") {
        tags.insert("modern_null".to_string(), "nullptr".to_string());
    }
    if source.contains("override") {
        tags.insert("virtual_override".to_string(), "true".to_string());
    }
    if source.contains("final") {
        tags.insert("class_final".to_string(), "true".to_string());
    }

    tags
}
