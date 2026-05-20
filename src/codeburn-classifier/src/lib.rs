#[cfg(test)]
mod tests {
    use super::*;
    use codeburn_provider::SessionData;
    use codeburn_provider::ProvenanceEntry;

    #[test]
    fn task_classifier_new_default_rules() {
        let classifier = TaskClassifier::new();
        assert!(classifier.rules.len() >= 12);
    }

    #[test]
    fn task_classifier_classify_test_pattern() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "test".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "test");
    }

    #[test]
    fn task_classifier_classify_fix_pattern() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "fix".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "fix");
    }

    #[test]
    fn task_classifier_classify_refactor_pattern() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "refactor".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "refactor");
    }

    #[test]
    fn task_classifier_classify_design_pattern() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "design".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "design");
    }

    #[test]
    fn task_classifier_classify_docs_pattern() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "docs".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "documentation");
    }

    #[test]
    fn task_classifier_classify_debug_pattern() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "debug".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "debugging");
    }

    #[test]
    fn task_classifier_classify_arch_pattern() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "arch".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "architecture");
    }

    #[test]
    fn task_classifier_classify_deploy_pattern() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "deploy".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "deployment");
    }

    #[test]
    fn task_classifier_classify_review_pattern() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "review".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "review");
    }

    #[test]
    fn task_classifier_classify_edit_pattern() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "edit".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "edit");
    }

    #[test]
    fn task_classifier_classify_research_pattern() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "research".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "research");
    }

    #[test]
    fn task_classifier_classify_integrate_pattern() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "integrate".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "integration");
    }

    #[test]
    fn task_classifier_classify_no_match_returns_other() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "xyz-unknown".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "other");
    }

    #[test]
    fn task_classifier_classify_empty_sessions_works() {
        let classifier = TaskClassifier::new();
        let sessions: Vec<SessionData> = vec![];
        let result = classifier.classify(&sessions).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn task_classifier_classify_multi_sessions_works() {
        let classifier = TaskClassifier::new();
        let sessions = vec![
            SessionData {
                provider: "claude".to_string(),
                date: chrono::NaiveDateTime::default(),
                input_tokens: 100,
                output_tokens: 100,
                model: "test".to_string(),
                task_category: "".to_string(),
                project: None,
                message_id: None,
                provenance: ProvenanceEntry {
                    provenance_id: "abc".to_string(),
                    provider_id: "claude".to_string(),
                    data_path: "".to_string(),
                    format: "jsonl".to_string(),
                    ingestion_timestamp: chrono::Utc::now().timestamp(),
                },
            },
            SessionData {
                provider: "claude".to_string(),
                date: chrono::NaiveDateTime::default(),
                input_tokens: 200,
                output_tokens: 200,
                model: "fix".to_string(),
                task_category: "".to_string(),
                project: None,
                message_id: None,
                provenance: ProvenanceEntry {
                    provenance_id: "abc".to_string(),
                    provider_id: "claude".to_string(),
                    data_path: "".to_string(),
                    format: "jsonl".to_string(),
                    ingestion_timestamp: chrono::Utc::now().timestamp(),
                },
            },
        ];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].task_category, "test");
        assert_eq!(result[1].task_category, "fix");
    }

    #[test]
    fn task_classifier_classify_project_contains_pattern() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "xyz".to_string(),
            task_category: "".to_string(),
            project: Some("test-project".to_string()),
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "test");
    }

    #[test]
    fn task_classifier_classify_model_substring_match() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "claude-test".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "test");
    }

    #[test]
    fn task_classifier_classify_first_rule_wins() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "test-refactor".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "test");
    }

    #[test]
    fn task_classifier_classify_provenance_added() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "test".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].provenance.provider_id, "codeburn-classifier");
    }

    #[test]
    fn task_classifier_classify_format_changed() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "test".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].provenance.format, "classified");
    }

    #[test]
    fn task_classifier_default_works() {
        let classifier = TaskClassifier::default();
        assert!(classifier.rules.len() >= 12);
    }

    #[test]
    fn task_classifier_clone_works() {
        let classifier = TaskClassifier::new();
        let cloned = classifier.clone();
        assert_eq!(cloned.rules.len(), classifier.rules.len());
    }

    #[test]
    fn task_classifier_classify_works_for_empty_model() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "other");
    }

    #[test]
    fn task_classifier_classify_works_for_empty_project() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "xyz".to_string(),
            task_category: "".to_string(),
            project: Some("".to_string()),
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "other");
    }

    #[test]
    fn task_classifier_classify_works_for_large_tokens() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 1_000_000,
            output_tokens: 1_000_000,
            model: "test".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "test");
        assert_eq!(result[0].input_tokens, 1_000_000);
    }

    #[test]
    fn task_classifier_classify_works_for_zero_tokens() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 0,
            output_tokens: 0,
            model: "test".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "test");
    }

    #[test]
    fn task_classifier_classify_works_for_custom_provider() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "custom-provider".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "test".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "custom-provider".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "test");
    }

    #[test]
    fn task_classifier_classify_with_message_id() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "test".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: Some("msg-123".to_string()),
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "test");
    }

    #[test]
    fn task_category_display_test() {
        assert_eq!(format!("{}", TaskCategory::Test), "test");
    }

    #[test]
    fn task_category_display_fix() {
        assert_eq!(format!("{}", TaskCategory::Fix), "fix");
    }

    #[test]
    fn task_category_display_refactor() {
        assert_eq!(format!("{}", TaskCategory::Refactor), "refactor");
    }

    #[test]
    fn task_category_display_design() {
        assert_eq!(format!("{}", TaskCategory::Design), "design");
    }

    #[test]
    fn task_category_display_documentation() {
        assert_eq!(format!("{}", TaskCategory::Documentation), "documentation");
    }

    #[test]
    fn task_category_display_debugging() {
        assert_eq!(format!("{}", TaskCategory::Debugging), "debugging");
    }

    #[test]
    fn task_category_display_architecture() {
        assert_eq!(format!("{}", TaskCategory::Architecture), "architecture");
    }

    #[test]
    fn task_category_display_integration() {
        assert_eq!(format!("{}", TaskCategory::Integration), "integration");
    }

    #[test]
    fn task_category_display_deployment() {
        assert_eq!(format!("{}", TaskCategory::Deployment), "deployment");
    }

    #[test]
    fn task_category_display_review() {
        assert_eq!(format!("{}", TaskCategory::Review), "review");
    }

    #[test]
    fn task_category_display_other() {
        assert_eq!(format!("{}", TaskCategory::Other), "other");
    }

    #[test]
    fn task_category_display_edit() {
        assert_eq!(format!("{}", TaskCategory::Edit), "edit");
    }

    #[test]
    fn task_category_display_research() {
        assert_eq!(format!("{}", TaskCategory::Research), "research");
    }

    #[test]
    fn task_classifier_classify_works_for_special_chars_in_model() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "test!@#".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "test");
    }

    #[test]
    fn task_classifier_classify_works_for_unicode_in_model() {
        let classifier = TaskClassifier::new();
        let sessions = vec![SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "test🎉".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: chrono::Utc::now().timestamp(),
            },
        }];
        let result = classifier.classify(&sessions).unwrap();
        assert_eq!(result[0].task_category, "test");
    }
}
