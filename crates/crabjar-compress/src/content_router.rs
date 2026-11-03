//! Content type detection and routing for compression strategies.

/// Detected content types that map to specific compression strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Json,
    Search,
    Log,
    Diff,
    Html,
    Prose,
}

/// Detect the content type of a tool result string.
///
/// Uses heuristic parsing to identify JSON, search results, logs, diffs, HTML,
/// or generic prose. The detection order matters — more specific types are checked first.
pub fn detect_content_type(text: &str) -> ContentType {
    let trimmed = text.trim();

    // Check for JSON (most structured)
    if is_json(trimmed) {
        return ContentType::Json;
    }

    // Check for search results (file:line:content pattern)
    if is_search_results(trimmed) {
        return ContentType::Search;
    }

    // Check for diff format
    if is_diff(trimmed) {
        return ContentType::Diff;
    }

    // Check for HTML
    if is_html(trimmed) {
        return ContentType::Html;
    }

    // Check for log format
    if is_logs(trimmed) {
        return ContentType::Log;
    }

    // Default to prose
    ContentType::Prose
}

fn is_json(text: &str) -> bool {
    // Quick heuristic: starts with { or [ and can be parsed as JSON
    let trimmed = text.trim();
    if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
        return false;
    }
    serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
}

fn is_search_results(text: &str) -> bool {
    // Look for file:line:content pattern (grep-style output)
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return false;
    }
    // Count lines matching the pattern
    let mut matches = 0;
    for line in &lines {
        if line.contains(':') && line.split(':').count() >= 3 {
            matches += 1;
        }
    }
    // If most lines match the pattern, it's search results
    matches > lines.len() / 2
}

fn is_diff(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with("diff --git")
        || trimmed.contains("@@ ")
        || (trimmed.starts_with("--- ") && trimmed.contains("+++ "))
}

fn is_html(text: &str) -> bool {
    let trimmed = text.trim().to_lowercase();
    trimmed.starts_with("<!doctype html>")
        || (trimmed.starts_with("<html") && trimmed.ends_with("</html>"))
        || (trimmed.starts_with("<div") && trimmed.contains("</div>"))
}

fn is_logs(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return false;
    }

    // Look for log-level keywords or timestamp patterns
    let mut score = 0;
    for line in &lines {
        let lower = line.to_lowercase();
        if lower.contains("error")
            || lower.contains("warn")
            || lower.contains("info")
            || lower.contains("debug")
            || lower.contains("fatal")
        {
            score += 2;
        }
        // Check for timestamp patterns (ISO or syslog-style)
        if line.starts_with('[') && line.contains("]") {
            score += 1;
        }
    }

    score > lines.len() / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_json_object() {
        assert_eq!(
            detect_content_type(r#"{"key": "value"}"#),
            ContentType::Json
        );
    }

    #[test]
    fn test_detect_json_array() {
        assert_eq!(detect_content_type(r#"[1, 2, 3]"#), ContentType::Json);
    }

    #[test]
    fn test_detect_search_results() {
        let search = "file.rs:10:fn foo() {}\nfile.rs:20:fn bar() {}";
        assert_eq!(detect_content_type(search), ContentType::Search);
    }

    #[test]
    fn test_detect_diff() {
        let diff = "diff --git a/file b/file\nindex 123..456\n--- a/file\n+++ b/file\n@@ -1,3 +1,3 @@\n-old\n+new";
        assert_eq!(detect_content_type(diff), ContentType::Diff);
    }

    #[test]
    fn test_detect_html() {
        let html = "<!DOCTYPE html><html><body>Hello</body></html>";
        assert_eq!(detect_content_type(html), ContentType::Html);
    }

    #[test]
    fn test_detect_logs() {
        let logs = "[INFO] Starting service\n[ERROR] Connection failed\n[WARN] Retrying...";
        assert_eq!(detect_content_type(logs), ContentType::Log);
    }

    #[test]
    fn test_detect_prose() {
        let prose = "This is a regular paragraph of text that doesn't match any specific format.";
        assert_eq!(detect_content_type(prose), ContentType::Prose);
    }
}
