use crate::types::{ChangedFile, ReviewRule};
use std::path::Path;
use std::fs;

/// Load review rules from a TOML file.
pub fn load_rules(path: &Path) -> Result<Vec<ReviewRule>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read rules file {:?}: {}", path, e))?;

    let parsed: toml::Value = content.parse()
        .map_err(|e| format!("Failed to parse TOML in {:?}: {}", path, e))?;

    let rules_array = parsed.get("rules")
        .and_then(|r| r.as_array())
        .ok_or_else(|| "Rules file must contain a 'rules' array".to_string())?;

    let mut rules = Vec::new();
    for (i, rule_value) in rules_array.iter().enumerate() {
        let rule: ReviewRule = rule_value.clone().try_into()
            .map_err(|e| format!("Failed to parse rule at index {}: {}", i, e))?;
        rules.push(rule);
    }

    Ok(rules)
}

/// Match review rules against a file based on its type.
pub fn match_rules<'a>(file: &ChangedFile, rules: &'a [ReviewRule]) -> Vec<&'a ReviewRule> {
    let file_type = file.file_type.as_str();

    rules.iter()
        .filter(|rule| {
            // Check applies_to (if specified)
            if !rule.applies_to.is_empty() && !rule.applies_to.contains(&file_type.to_string()) {
                return false;
            }

            // Check not_applies_to
            if rule.not_applies_to.contains(&file_type.to_string()) {
                return false;
            }

            true
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChangedFile, FileStatus};
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    fn make_file(path: &str, file_type: crate::types::FileType) -> ChangedFile {
        ChangedFile {
            path: PathBuf::from(path),
            file_type,
            status: FileStatus::Modified,
            additions: 10,
            deletions: 5,
        }
    }

    #[test]
    fn test_load_rules() {
        let rules_toml = r#"
[[rules]]
id = "check_errors"
applies_to = ["source"]
severity = "high"
description = "Check error handling"

[[rules]]
id = "check_style"
applies_to = ["source", "test"]
severity = "low"
description = "Check code style"
"#;

        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, rules_toml).unwrap();

        let rules = load_rules(temp_file.path()).unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].id, "check_errors");
        assert_eq!(rules[1].id, "check_style");
    }

    #[test]
    fn test_match_rules_source() {
        let source_file = make_file("src/main.rs", crate::types::FileType::Source);
        let test_file = make_file("tests/test.rs", crate::types::FileType::Test);

        let rules = vec![
            ReviewRule {
                id: "source_only".to_string(),
                applies_to: vec!["source".to_string()],
                not_applies_to: vec![],
                severity: crate::types::Severity::High,
                description: "".to_string(),
            },
            ReviewRule {
                id: "all_files".to_string(),
                applies_to: vec![],
                not_applies_to: vec!["generated".to_string()],
                severity: crate::types::Severity::Medium,
                description: "".to_string(),
            },
        ];

        let source_matches = match_rules(&source_file, &rules);
        assert_eq!(source_matches.len(), 2);

        let test_matches = match_rules(&test_file, &rules);
        assert_eq!(test_matches.len(), 1);
        assert_eq!(test_matches[0].id, "all_files");
    }
}
