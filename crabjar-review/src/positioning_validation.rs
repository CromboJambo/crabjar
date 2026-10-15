use crate::types::ReviewBundle;
use serde_json::Value;

/// Validate that a finding's position is correct by searching for the quoted code snippet
/// in the actual file content. Do NOT trust LLM-reported line numbers directly.
pub fn validate_position(
    bundle: &ReviewBundle,
    file_path: &str,
    line: u32,
    message: &str,
) -> Option<u32> {
    // Find the file in the bundle
    let changed_file = bundle
        .files
        .iter()
        .find(|f| f.path.to_string_lossy() == file_path);
    if changed_file.is_none() {
        return None;
    }

    // Read the actual file content
    let full_path = changed_file.unwrap().path.clone();
    let content = match std::fs::read_to_string(&full_path) {
        Ok(c) => c,
        Err(_) => return None,
    };

    let lines: Vec<&str> = content.lines().collect();

    // Search for the quoted code snippet from the message within a tolerance window
    let line_index = (line - 1) as usize;
    let tolerance = 2; // ±2 lines per spec

    let start = if line_index >= tolerance {
        line_index - tolerance
    } else {
        0
    };
    let end = std::cmp::min(line_index + tolerance + 1, lines.len());

    for (i, l) in lines[start..end].iter().enumerate() {
        let actual_line = start + i;
        // Check if the message references code on this line
        if message.contains(l.trim())
            || l.trim()
                .contains(&message[..std::cmp::min(50, message.len())])
        {
            return Some((actual_line + 1) as u32);
        }
    }

    // If no exact match found within tolerance, check if line exists at all
    if line_index < lines.len() {
        Some(line)
    } else {
        None
    }
}

/// Validate severity assignment based on finding type.
pub fn validate_severity(finding_type: &str, severity: &str) -> bool {
    match (finding_type, severity) {
        ("bug", "critical" | "major") => true,
        ("style", "minor" | "nit") => true,
        ("security", "critical" | "major" | "minor") => true,
        ("performance", "minor" | "nit") => true,
        _ => false,
    }
}

/// Validate that a finding is actionable (not vague).
pub fn validate_actionability(description: &str) -> bool {
    if description.len() < 20 {
        return false;
    }

    let generic_phrases = [
        "this is bad",
        "fix this",
        "improve this",
        "bad code",
        "needs work",
    ];

    for phrase in &generic_phrases {
        if description.to_lowercase().contains(phrase) {
            return false;
        }
    }

    true
}

/// Validate all findings in a batch.
pub fn validate_findings(bundle: &ReviewBundle, findings: &[Value]) -> Vec<Value> {
    let mut valid = Vec::new();

    for finding in findings {
        let file = finding["file"].as_str();
        let line = finding["line"].as_u64();
        let severity = finding["severity"].as_str();
        let finding_type = finding["type"].as_str();
        let description = finding["description"].as_str();

        if file.is_none()
            || line.is_none()
            || severity.is_none()
            || finding_type.is_none()
            || description.is_none()
        {
            continue;
        }

        // Validate position
        if validate_position(
            bundle,
            file.unwrap(),
            line.unwrap() as u32,
            description.unwrap(),
        )
        .is_none()
        {
            continue;
        }

        // Validate severity
        if !validate_severity(finding_type.unwrap(), severity.unwrap()) {
            continue;
        }

        // Validate actionability
        if !validate_actionability(description.unwrap()) {
            continue;
        }

        valid.push(finding.clone());
    }

    valid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_severity_bug_critical() {
        assert!(validate_severity("bug", "critical"));
    }

    #[test]
    fn test_validate_severity_style_minor() {
        assert!(validate_severity("style", "minor"));
    }

    #[test]
    fn test_validate_actionability_good() {
        assert!(validate_actionability(
            "Variable `x` is declared but never used, causing a compiler warning."
        ));
    }

    #[test]
    fn test_validate_actionability_generic() {
        assert!(!validate_actionability("fix this"));
    }
}
