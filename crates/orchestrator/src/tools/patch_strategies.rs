//! Matching strategies for the patch tool.
//!
//! Each strategy function attempts to find old_string in content using a specific
//! matching approach and returns the replacement result if successful.

use std::path::PathBuf;

/// Strategy 1: Exact string match.
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
    let trimmed_old: Vec<String> = old_string
        .lines()
        .map(|l| l.trim().to_lowercase())
        .collect();
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
    let old_trimmed: Vec<String> = old_string
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect();
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
                let expected: Vec<String> =
                    old_string.lines().map(|l| l.trim().to_string()).collect();

                // Check if at least 80% of lines match
                let matches = candidate
                    .iter()
                    .zip(expected.iter())
                    .filter(|(a, b)| a == b)
                    .count();
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

// --- Helper functions ---

pub(crate) fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn strip_trailing_ws(s: &str) -> String {
    s.lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn count_normalized_matches(
    lines: &[&str],
    norm_old: &str,
    old_line_count: usize,
) -> usize {
    let mut count = 0;
    for start in 0..=lines.len().saturating_sub(old_line_count) {
        let candidate: String = lines[start..start + old_line_count].join("\n");
        if normalize_ws(&candidate) == norm_old {
            count += 1;
        }
    }
    count
}

pub(crate) fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

/// Expand leading ~ to the user's home directory.
pub(crate) fn expand_path(path_str: &str) -> PathBuf {
    if let Some(rest) = path_str.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path_str)
}
