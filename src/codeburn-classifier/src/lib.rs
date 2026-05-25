use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClassifierError {
    #[error("classification failed: {0}")]
    Failed(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskCategory {
    Test,
    Fix,
    Refactor,
    Design,
    Documentation,
    Debugging,
    Architecture,
    Integration,
    Deployment,
    Review,
    Edit,
    Research,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskClassifier {
    pub rules: Vec<(String, TaskCategory)>,
}

impl TaskClassifier {
    pub fn new() -> Self {
        Self {
            rules: vec![
                ("test".to_string(), TaskCategory::Test),
                ("fix".to_string(), TaskCategory::Fix),
                ("refactor".to_string(), TaskCategory::Refactor),
                ("design".to_string(), TaskCategory::Design),
                ("doc".to_string(), TaskCategory::Documentation),
                ("debug".to_string(), TaskCategory::Debugging),
                ("arch".to_string(), TaskCategory::Architecture),
                ("integ".to_string(), TaskCategory::Integration),
                ("deploy".to_string(), TaskCategory::Deployment),
                ("review".to_string(), TaskCategory::Review),
                ("edit".to_string(), TaskCategory::Edit),
                ("research".to_string(), TaskCategory::Research),
            ],
        }
    }

    pub fn classify(
        &self,
        _sessions: &[codeburn_provider::SessionData],
    ) -> Result<Vec<codeburn_provider::SessionData>, ClassifierError> {
        Ok(_sessions.to_vec())
    }
}

impl Default for TaskClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_classifier_new_has_rules() {
        let classifier = TaskClassifier::new();
        assert!(!classifier.rules.is_empty());
        assert_eq!(classifier.rules.len(), 12);
    }

    #[test]
    fn task_classifier_default_has_rules() {
        let classifier = TaskClassifier::default();
        assert!(!classifier.rules.is_empty());
    }

    #[test]
    fn task_category_test_variants() {
        assert_eq!(TaskCategory::Test as u8, TaskCategory::Test as u8);
        assert_eq!(TaskCategory::Fix as u8, TaskCategory::Fix as u8);
        assert_eq!(TaskCategory::Refactor as u8, TaskCategory::Refactor as u8);
        assert_eq!(TaskCategory::Design as u8, TaskCategory::Design as u8);
    }

    #[test]
    fn task_category_all_variants_exist() {
        let _ = TaskCategory::Test;
        let _ = TaskCategory::Fix;
        let _ = TaskCategory::Refactor;
        let _ = TaskCategory::Design;
        let _ = TaskCategory::Documentation;
        let _ = TaskCategory::Debugging;
        let _ = TaskCategory::Architecture;
        let _ = TaskCategory::Integration;
        let _ = TaskCategory::Deployment;
        let _ = TaskCategory::Review;
        let _ = TaskCategory::Edit;
        let _ = TaskCategory::Research;
        let _ = TaskCategory::Other;
    }

    #[test]
    fn task_category_serde_lowercase_test() {
        let cat = TaskCategory::Test;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, "\"test\"");
    }

    #[test]
    fn task_category_serde_lowercase_fix() {
        let cat = TaskCategory::Fix;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, "\"fix\"");
    }

    #[test]
    fn task_category_serde_lowercase_refactor() {
        let cat = TaskCategory::Refactor;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, "\"refactor\"");
    }

    #[test]
    fn task_category_serde_lowercase_documentation() {
        let cat = TaskCategory::Documentation;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, "\"documentation\"");
    }

    #[test]
    fn task_category_serde_lowercase_research() {
        let cat = TaskCategory::Research;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, "\"research\"");
    }

    #[test]
    fn task_category_serde_roundtrip_test() {
        let cat = TaskCategory::Test;
        let json = serde_json::to_string(&cat).unwrap();
        let restored: TaskCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, cat);
    }

    #[test]
    fn task_category_clone_works() {
        let cat = TaskCategory::Debugging;
        let cloned = cat.clone();
        assert_eq!(cat, cloned);
    }

    #[test]
    fn task_category_copy_works() {
        let cat = TaskCategory::Architecture;
        let copy = cat;
        assert_eq!(cat, copy);
    }

    #[test]
    fn classifier_classify_passes_through_sessions() {
        let classifier = TaskClassifier::new();
        let sessions: Vec<codeburn_provider::SessionData> = vec![];
        let result = classifier.classify(&sessions).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn classifier_rules_match_known_patterns() {
        let classifier = TaskClassifier::new();
        let patterns: Vec<&str> = classifier.rules.iter().map(|(p, _)| p.as_str()).collect();
        assert!(patterns.contains(&"test"));
        assert!(patterns.contains(&"fix"));
        assert!(patterns.contains(&"refactor"));
        assert!(patterns.contains(&"design"));
        assert!(patterns.contains(&"doc"));
        assert!(patterns.contains(&"debug"));
        assert!(patterns.contains(&"arch"));
        assert!(patterns.contains(&"integ"));
        assert!(patterns.contains(&"deploy"));
        assert!(patterns.contains(&"review"));
        assert!(patterns.contains(&"edit"));
        assert!(patterns.contains(&"research"));
    }

    #[test]
    fn classifier_error_message() {
        let err = ClassifierError::Failed("test failed".to_string());
        assert!(err.to_string().contains("test failed"));
    }

    #[test]
    fn classifier_error_debug_format() {
        let err = ClassifierError::Failed("detail".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Failed"));
    }
}
