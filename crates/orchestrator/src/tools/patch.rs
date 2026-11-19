//! First-class `patch` tool — fuzzy find-and-replace with multiple matching strategies.
//! Matches Hermes Agent API contract for drop-in parity.
//!
//! Matching strategies (tried in order):
//! 1. Exact match
//! 2. Whitespace-normalized match (collapse all whitespace runs to single space)
//! 3. Trailing-whitespace-ignored match
//! 4. Trimmed match (trim leading/trailing whitespace on each line)
//! 5. Case-insensitive exact match
//! 6. Case-insensitive trimmed match
//! 7. Line-by-line fuzzy (each line must match after trim, order preserved)
//! 8. Substring match with context tolerance (find old_string lines within file)
//! 9. Single-line fallback for single-line old_string

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

#[path = "patch_strategies.rs"]
mod patch_strategies;
use patch_strategies::*;

/// Handle the `patch` tool call.
///
/// Arguments: JSON string in args[0] with fields:
///   - path (required): file path to patch
///   - old_string (required): text to find and replace
///   - new_string (required): replacement text
///   - replace_all (optional, default false): replace all occurrences
pub async fn handle(args: &[String]) -> Result<String, String> {
    if args.is_empty() || args[0].is_empty() {
        return Err("Error: patch requires a JSON argument with 'path', 'old_string', and 'new_string' fields".to_string());
    }

    let request: Value = serde_json::from_str(&args[0])
        .map_err(|e| format!("Invalid JSON arguments: {}", e))?;

    let path_str = request["path"]
        .as_str()
        .ok_or("Error: 'path' field is required")?;

    let old_string = request["old_string"]
        .as_str()
        .ok_or("Error: 'old_string' field is required")?;

    let new_string = request["new_string"]
        .as_str()
        .ok_or("Error: 'new_string' field is required")?;

    let replace_all = request["replace_all"].as_bool().unwrap_or(false);

    if old_string.is_empty() {
        return Err("Error: 'old_string' must not be empty".to_string());
    }

    // Expand ~ to home directory
    let path = expand_path(path_str);

    // Read the file
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Error reading file {}: {}", path.display(), e))?;

    // Find and replace using fuzzy matching strategies
    let (new_content, matches_found, strategy_used) = find_and_replace(&content, old_string, new_string, replace_all);

    if matches_found == 0 {
        return Err(format!(
            "Error: could not find '{}' in file. Tried all 9 matching strategies.\nFile content preview:\n{}",
            truncate(old_string, 100),
            truncate(&content, 500)
        ));
    }

    // Write the patched file
    fs::write(&path, &new_content)
        .map_err(|e| format!("Error writing file {}: {}", path.display(), e))?;

    // Generate a simple diff summary
    let old_lines = content.lines().count();
    let new_lines = new_content.lines().count();

    Ok(serde_json::to_string_pretty(&json!({
        "status": "success",
        "path": path.display().to_string(),
        "matches_replaced": matches_found,
        "strategy_used": strategy_used,
        "old_line_count": old_lines,
        "new_line_count": new_lines
    })).map_err(|e| e.to_string())?)
}

/// Find and replace using multiple fuzzy matching strategies.
fn find_and_replace(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> (String, usize, &'static str) {
    // Strategy 1: Exact match
    if let Some(result) = try_exact(content, old_string, new_string, replace_all) {
        return result;
    }

    // Strategy 2: Whitespace-normalized match
    if let Some(result) = try_whitespace_normalized(content, old_string, new_string, replace_all) {
        return (result.0, result.1, "whitespace-normalized");
    }

    // Strategy 3: Trailing-whitespace-ignored match
    if let Some(result) = try_trailing_ws_ignored(content, old_string, new_string, replace_all) {
        return (result.0, result.1, "trailing-ws-ignored");
    }

    // Strategy 4: Trimmed match (trim each line)
    if let Some(result) = try_trimmed_lines(content, old_string, new_string, replace_all) {
        return (result.0, result.1, "trimmed-lines");
    }

    // Strategy 5: Case-insensitive exact match
    if let Some(result) = try_case_insensitive(content, old_string, new_string, replace_all) {
        return (result.0, result.1, "case-insensitive");
    }

    // Strategy 6: Case-insensitive trimmed match
    if let Some(result) = try_case_insensitive_trimmed(content, old_string, new_string, replace_all) {
        return (result.0, result.1, "case-insensitive-trimmed");
    }

    // Strategy 7: Line-by-line fuzzy (each line matches after trim, order preserved)
    if let Some(result) = try_line_by_line_fuzzy(content, old_string, new_string, replace_all) {
        return (result.0, result.1, "line-by-line-fuzzy");
    }

    // Strategy 8: Substring match with context tolerance
    if let Some(result) = try_context_tolerance(content, old_string, new_string, replace_all) {
        return (result.0, result.1, "context-tolerance");
    }

    // Strategy 9: Single-line fallback
    if let Some(result) = try_single_line_fallback(content, old_string, new_string, replace_all) {
        return (result.0, result.1, "single-line-fallback");
    }

    (content.to_string(), 0, "none")
}

fn try_exact(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Option<(String, usize, &'static str)> {
    if !content.contains(old_string) {
        return None;
    }
    let count = content.matches(old_string).count();
    if replace_all {
        Some((content.replace(old_string, new_string), count, "exact"))
    } else if count == 1 {
        Some((content.replacen(old_string, new_string, 1), 1, "exact"))
    } else {
        None // Ambiguous — need more context
    }
}

/// Strategy 2: Whitespace-normalized match.
fn try_whitespace_normalized(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Option<(String, usize)> {
    let norm_old = normalize_ws(old_string);
    if norm_old.is_empty() {
        return None;
    }

    // Find the region in content that matches when normalized
    let lines: Vec<&str> = content.lines().collect();
    let old_lines: Vec<&str> = old_string.lines().collect();
    if old_lines.is_empty() {
        return None;
    }

    for start in 0..=lines.len().saturating_sub(old_lines.len()) {
        let candidate: String = lines[start..start + old_lines.len()].join("\n");
        if normalize_ws(&candidate) == norm_old {
            let count = count_normalized_matches(&lines, &norm_old, old_lines.len());
            if replace_all || count == 1 {
                let mut result_lines = lines.clone();
                result_lines.splice(start..start + old_lines.len(), new_string.lines());
                return Some((result_lines.join("\n") + "\n", 1));
            }
        }
    }
    None
}

/// Strategy 3: Trailing-whitespace-ignored match.
fn try_trailing_ws_ignored(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Option<(String, usize)> {
    let stripped_old = strip_trailing_ws(old_string);
    if !content.contains(&stripped_old) {
        return None;
    }

    // Find the actual span with trailing whitespace
    let lines: Vec<&str> = content.lines().collect();
    let old_lines: Vec<&str> = stripped_old.lines().collect();
    if old_lines.is_empty() {
        return None;
    }

    for start in 0..=lines.len().saturating_sub(old_lines.len()) {
        let candidate: String = lines[start..start + old_lines.len()]
            .iter()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n");
        if candidate == stripped_old {
            let mut result_lines = lines.clone();
            result_lines.splice(start..start + old_lines.len(), new_string.lines());
            return Some((result_lines.join("\n") + "\n", 1));
        }
    }
    None
}

/// Strategy 4: Trimmed match (trim leading/trailing whitespace on each line).
fn try_trimmed_lines(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Option<(String, usize)> {
    let trimmed_old: Vec<String> = old_string.lines().map(|l| l.trim().to_string()).collect();
    if trimmed_old.is_empty() {
        return None;
    }

    let lines: Vec<&str> = content.lines().collect();
    for start in 0..=lines.len().saturating_sub(trimmed_old.len()) {
        let candidate: Vec<String> = lines[start..start + trimmed_old.len()]
            .iter()
            .map(|l| l.trim().to_string())
            .collect();
        if candidate == trimmed_old {
            let mut result_lines = lines.clone();
            result_lines.splice(start..start + trimmed_old.len(), new_string.lines());
            return Some((result_lines.join("\n") + "\n", 1));
        }
    }
    None
}

/// Strategy 5: Case-insensitive exact match.
fn try_case_insensitive(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Option<(String, usize)> {
    let lower_content = content.to_lowercase();
    let lower_old = old_string.to_lowercase();
    if !lower_content.contains(&lower_old) {
        return None;
    }

    // Find the actual span (preserving original case)
    if let Some(pos) = lower_content.find(&lower_old) {
        let end = pos + old_string.len();
        let before = &content[..pos];
        let after = &content[end..];
        return Some((format!("{}{}", before, new_string) + after, 1));
    }
    None
}

/// Strategy 6: Case-insensitive trimmed match.
fn try_case_insensitive_trimmed(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Option<(String, usize)> {
    let trimmed_old: Vec<String> = old_string.lines().map(|l| l.trim().to_lowercase()).collect();
    if trimmed_old.is_empty() {
        return None;
    }

    let lines: Vec<&str> = content.lines().collect();
    for start in 0..=lines.len().saturating_sub(trimmed_old.len()) {
        let candidate: Vec<String> = lines[start..start + trimmed_old.len()]
            .iter()
            .map(|l| l.trim().to_lowercase())
            .collect();
        if candidate == trimmed_old {
            let mut result_lines = lines.clone();
            result_lines.splice(start..start + trimmed_old.len(), new_string.lines());
            return Some((result_lines.join("\n") + "\n", 1));
        }
    }
    None
}

/// Strategy 7: Line-by-line fuzzy (each line must match after trim, order preserved).
fn try_line_by_line_fuzzy(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Option<(String, usize)> {
    let old_trimmed: Vec<String> = old_string.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).map(|s| s.to_string()).collect();
    if old_trimmed.is_empty() {
        return None;
    }

    let lines: Vec<&str> = content.lines().collect();
    for start in 0..=lines.len().saturating_sub(old_trimmed.len()) {
        let candidate: Vec<String> = lines[start..start + old_trimmed.len()]
            .iter()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .map(|s| s.to_string())
            .collect();
        if candidate == old_trimmed {
            let mut result_lines = lines.clone();
            result_lines.splice(start..start + old_trimmed.len(), new_string.lines());
            return Some((result_lines.join("\n") + "\n", 1));
        }
    }
    None
}

/// Strategy 8: Substring match with context tolerance.
fn try_context_tolerance(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Option<(String, usize)> {
    // Find the first line of old_string and search for it in content
    let first_line = old_string.lines().next()?.trim();
    if first_line.is_empty() {
        return None;
    }

    let lines: Vec<&str> = content.lines().collect();
    let old_lines_count = old_string.lines().count();

    for (i, line) in lines.iter().enumerate() {
        if line.trim().contains(first_line) || first_line.contains(line.trim()) {
            // Found a potential match — check if surrounding context is close enough
            if i + old_lines_count <= lines.len() {
                let candidate: Vec<String> = lines[i..i + old_lines_count]
                    .iter()
                    .map(|l| l.trim().to_string())
                    .collect();
                let expected: Vec<String> = old_string.lines().map(|l| l.trim().to_string()).collect();

                // Check if at least 80% of lines match
                let matches = candidate.iter().zip(expected.iter()).filter(|(a, b)| a == b).count();
                if matches >= (expected.len() as f64 * 0.8) as usize {
                    let mut result_lines = lines.clone();
                    result_lines.splice(i..i + old_lines_count, new_string.lines());
                    return Some((result_lines.join("\n") + "\n", 1));
                }
            }
        }
    }
    None
}

/// Strategy 9: Single-line fallback for single-line old_string.
fn try_single_line_fallback(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Option<(String, usize)> {
    if old_string.lines().count() != 1 {
        return None;
    }

    let target = old_string.trim();
    if target.is_empty() {
        return None;
    }

    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains(target) || target.contains(line.trim()) {
            let mut result_lines = lines.clone();
            result_lines[i] = new_string;
            return Some((result_lines.join("\n") + "\n", 1));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_patch_exact_match() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "hello world\nfoo bar\n").unwrap();

        let result = handle(&[format!(r#"{{"path": "{}", "old_string": "hello world", "new_string": "goodbye world"}}"#, file_path.display())]).await;
        assert!(result.is_ok());

        let output: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["matches_replaced"], 1);
        assert_eq!(output["strategy_used"], "exact");

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("goodbye world"));
    }

    #[tokio::test]
    async fn test_patch_whitespace_normalized() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "  hello   world  \n").unwrap();

        let result = handle(&[format!(r#"{{"path": "{}", "old_string": "hello world", "new_string": "hi there"}}"#, file_path.display())]).await;
        assert!(result.is_ok());

        let output: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["strategy_used"], "whitespace-normalized");
    }

    #[tokio::test]
    async fn test_patch_trimmed_lines() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "  line one\n  line two\n").unwrap();

        let result = handle(&[format!(r#"{{"path": "{}", "old_string": "line one\nline two", "new_string": "first\nsecond"}}"#, file_path.display())]).await;
        assert!(result.is_ok());

        let output: Value = serde_json::from_str(&result.unwrap()).unwrap();
        // Either whitespace-normalized or trimmed-lines strategy should work here
        assert!(output["strategy_used"].as_str().unwrap() == "whitespace-normalized" || output["strategy_used"].as_str().unwrap() == "trimmed-lines");
    }

    #[tokio::test]
    async fn test_patch_case_insensitive() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello World\n").unwrap();

        let result = handle(&[format!(r#"{{"path": "{}", "old_string": "hello world", "new_string": "Hi There"}}"#, file_path.display())]).await;
        assert!(result.is_ok());

        let output: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["strategy_used"], "case-insensitive");
    }

    #[tokio::test]
    async fn test_patch_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "hello world\n").unwrap();

        let result = handle(&[format!(r#"{{"path": "{}", "old_string": "nonexistent text that is very long and unique to ensure no match", "new_string": "replacement"}}"#, file_path.display())]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("could not find"));
    }

    #[tokio::test]
    async fn test_patch_replace_all() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "foo\nbar\nfoo\nbaz\nfoo\n").unwrap();

        let result = handle(&[format!(r#"{{"path": "{}", "old_string": "foo", "new_string": "qux", "replace_all": true}}"#, file_path.display())]).await;
        assert!(result.is_ok());

        let output: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["matches_replaced"], 3);

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(!content.contains("foo"));
        assert_eq!(content.matches("qux").count(), 3);
    }
}
