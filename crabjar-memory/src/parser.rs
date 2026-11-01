//! Parser for Hermes-compatible memory file format.
//!
//! Entries are separated by `§` (U+00A7 SECTION SIGN) followed by a newline.
//! Multi-line entries are supported — text continues until the next delimiter or EOF.

/// Entry delimiter: section sign followed by newline.
const DELIMITER: &str = "§\n";

/// Parse memory file content into individual entries.
///
/// Empty entries (whitespace only) are filtered out.
pub fn parse_entries(content: &str) -> Vec<String> {
    if content.is_empty() {
        return vec![];
    }

    let mut entries = Vec::new();
    // Split on §\n delimiter
    for part in content.split(DELIMITER) {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            entries.push(trimmed.to_string());
        }
    }
    entries
}

/// Serialize entries back to memory file format.
///
/// Entries are joined with the § delimiter and a trailing newline is added.
pub fn serialize_entries(entries: &[String]) -> String {
    if entries.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    for (i, entry) in entries.iter().enumerate() {
        if i > 0 {
            result.push_str(DELIMITER);
        }
        result.push_str(entry);
    }
    result.push('\n');
    result
}

/// Find entries containing a given substring.
pub fn find_entries_containing(entries: &[String], substring: &str) -> Vec<usize> {
    let mut matches = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        if entry.contains(substring) {
            matches.push(i);
        }
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_entry() {
        let content = "First memory entry\n";
        let entries = parse_entries(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], "First memory entry");
    }

    #[test]
    fn test_parse_multiple_entries() {
        let content = "Entry one\n§\nEntry two\n§\nEntry three\n";
        let entries = parse_entries(content);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0], "Entry one");
        assert_eq!(entries[1], "Entry two");
        assert_eq!(entries[2], "Entry three");
    }

    #[test]
    fn test_parse_multiline_entry() {
        let content = "Line one\nLine two\n§\nSecond entry\n";
        let entries = parse_entries(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], "Line one\nLine two");
    }

    #[test]
    fn test_parse_empty_content() {
        let entries = parse_entries("");
        assert!(entries.is_empty());
    }

    #[test]
    fn test_serialize_roundtrip() {
        let original = vec![
            "Entry one".to_string(),
            "Entry two".to_string(),
            "Entry three".to_string(),
        ];
        let serialized = serialize_entries(&original);
        let parsed = parse_entries(&serialized);
        assert_eq!(parsed, original);
    }

    #[test]
    fn test_find_entries_containing() {
        let entries = vec![
            "First entry about Rust".to_string(),
            "Second entry about Python".to_string(),
            "Third entry mentioning Rust again".to_string(),
        ];
        let matches = find_entries_containing(&entries, "Rust");
        assert_eq!(matches, vec![0, 2]);
    }
}
