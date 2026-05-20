#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn provider_registry_new_empty() {
        let registry = ProviderRegistry::new();
        assert!(registry.providers.is_empty());
    }

    #[test]
    fn provider_registry_default_empty() {
        let registry = ProviderRegistry::default();
        assert!(registry.providers.is_empty());
    }

    #[test]
    fn provider_registry_discover_empty_project_returns_empty() {
        let dir = tempdir().unwrap();
        let providers = ProviderRegistry::discover(dir.path()).unwrap();
        assert!(providers.is_empty());
    }

    #[test]
    fn provider_discover_new_provenance_id() {
        let dir = tempdir().unwrap();
        let providers = ProviderRegistry::discover(dir.path()).unwrap();
        for provider in providers {
            assert!(!provider.provenance.provenance_id.is_empty());
        }
    }

    #[test]
    fn provider_discover_provenance_timestamp() {
        let dir = tempdir().unwrap();
        let providers = ProviderRegistry::discover(dir.path()).unwrap();
        for provider in providers {
            assert!(provider.provenance.ingestion_timestamp > 0);
        }
    }

    #[test]
    fn provider_discover_provenance_provider_id() {
        let dir = tempdir().unwrap();
        let providers = ProviderRegistry::discover(dir.path()).unwrap();
        for provider in providers {
            assert!(!provider.provenance.provider_id.is_empty());
        }
    }

    #[test]
    fn provider_discover_provenance_data_path() {
        let dir = tempdir().unwrap();
        let providers = ProviderRegistry::discover(dir.path()).unwrap();
        for provider in providers {
            assert!(!provider.provenance.data_path.is_empty());
        }
    }

    #[test]
    fn provider_discover_provenance_format() {
        let dir = tempdir().unwrap();
        let providers = ProviderRegistry::discover(dir.path()).unwrap();
        for provider in providers {
            assert!(!provider.provenance.format.is_empty());
        }
    }

    #[test]
    fn provider_clone_works() {
        let provider = Provider {
            name: "claude".to_string(),
            data_path: std::path::PathBuf::from("/tmp"),
            format: DataFormat::Jsonl,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "/tmp".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: 123,
            },
        };
        let cloned = provider.clone();
        assert_eq!(cloned.name, provider.name);
    }

    #[test]
    fn data_format_jsonl_display() {
        assert_eq!(format!("{:?}", DataFormat::Jsonl), "Jsonl");
    }

    #[test]
    fn data_format_sqlite_display() {
        assert_eq!(format!("{:?}", DataFormat::Sqlite), "Sqlite");
    }

    #[test]
    fn data_format_clone_works() {
        let format = DataFormat::Jsonl;
        let cloned = format.clone();
        assert_eq!(cloned, format);
    }

    #[test]
    fn provenance_entry_clone_works() {
        let entry = ProvenanceEntry {
            provenance_id: "abc".to_string(),
            provider_id: "claude".to_string(),
            data_path: "/tmp".to_string(),
            format: "jsonl".to_string(),
            ingestion_timestamp: 123,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.provenance_id, entry.provenance_id);
    }

    #[test]
    fn provenance_entry_serialize_works() {
        let entry = ProvenanceEntry {
            provenance_id: "abc".to_string(),
            provider_id: "claude".to_string(),
            data_path: "/tmp".to_string(),
            format: "jsonl".to_string(),
            ingestion_timestamp: 123,
        };
        let serialized = serde_json::to_string(&entry).unwrap();
        assert!(serialized.contains("abc"));
    }

    #[test]
    fn provenance_entry_deserialize_works() {
        let serialized = serde_json::to_string(&ProvenanceEntry {
            provenance_id: "abc".to_string(),
            provider_id: "claude".to_string(),
            data_path: "/tmp".to_string(),
            format: "jsonl".to_string(),
            ingestion_timestamp: 123,
        }).unwrap();
        let deserialized: ProvenanceEntry = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.provenance_id, "abc");
    }

    #[test]
    fn provenance_entry_zero_timestamp() {
        let entry = ProvenanceEntry {
            provenance_id: "abc".to_string(),
            provider_id: "claude".to_string(),
            data_path: "/tmp".to_string(),
            format: "jsonl".to_string(),
            ingestion_timestamp: 0,
        };
        let serialized = serde_json::to_string(&entry).unwrap();
        assert!(serialized.contains("0"));
    }

    #[test]
    fn provenance_entry_large_timestamp() {
        let entry = ProvenanceEntry {
            provenance_id: "abc".to_string(),
            provider_id: "claude".to_string(),
            data_path: "/tmp".to_string(),
            format: "jsonl".to_string(),
            ingestion_timestamp: 1_000_000_000,
        };
        let serialized = serde_json::to_string(&entry).unwrap();
        assert!(serialized.contains("1000000000"));
    }

    #[test]
    fn session_data_clone_works() {
        let data = SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "claude".to_string(),
            task_category: "test".to_string(),
            project: Some("test".to_string()),
            message_id: Some("abc".to_string()),
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "/tmp".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: 123,
            },
        };
        let cloned = data.clone();
        assert_eq!(cloned.provider, data.provider);
    }

    #[test]
    fn session_data_serialize_works() {
        let data = SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "claude".to_string(),
            task_category: "test".to_string(),
            project: Some("test".to_string()),
            message_id: Some("abc".to_string()),
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "/tmp".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: 123,
            },
        };
        let serialized = serde_json::to_string(&data).unwrap();
        assert!(serialized.contains("claude"));
    }

    #[test]
    fn session_data_deserialize_works() {
        let serialized = serde_json::to_string(&SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "claude".to_string(),
            task_category: "test".to_string(),
            project: Some("test".to_string()),
            message_id: Some("abc".to_string()),
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "/tmp".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: 123,
            },
        }).unwrap();
        let deserialized: SessionData = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.provider, "claude");
    }

    #[test]
    fn session_data_empty_provider() {
        let data = SessionData {
            provider: "".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 0,
            output_tokens: 0,
            model: "".to_string(),
            task_category: "".to_string(),
            project: None,
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "/tmp".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: 123,
            },
        };
        let serialized = serde_json::to_string(&data).unwrap();
        assert!(serialized.contains("abc"));
    }

    #[test]
    fn session_data_null_project() {
        let data = SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "claude".to_string(),
            task_category: "test".to_string(),
            project: None,
            message_id: Some("abc".to_string()),
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "/tmp".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: 123,
            },
        };
        let serialized = serde_json::to_string(&data).unwrap();
        assert!(serialized.contains("claude"));
    }

    #[test]
    fn session_data_null_message_id() {
        let data = SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 100,
            output_tokens: 100,
            model: "claude".to_string(),
            task_category: "test".to_string(),
            project: Some("test".to_string()),
            message_id: None,
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "/tmp".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: 123,
            },
        };
        let serialized = serde_json::to_string(&data).unwrap();
        assert!(serialized.contains("claude"));
    }

    #[test]
    fn session_data_large_tokens() {
        let data = SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 1_000_000,
            output_tokens: 1_000_000,
            model: "claude".to_string(),
            task_category: "test".to_string(),
            project: Some("test".to_string()),
            message_id: Some("abc".to_string()),
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "/tmp".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: 123,
            },
        };
        let serialized = serde_json::to_string(&data).unwrap();
        assert!(serialized.contains("claude"));
    }

    #[test]
    fn session_data_zero_tokens() {
        let data = SessionData {
            provider: "claude".to_string(),
            date: chrono::NaiveDateTime::default(),
            input_tokens: 0,
            output_tokens: 0,
            model: "claude".to_string(),
            task_category: "test".to_string(),
            project: Some("test".to_string()),
            message_id: Some("abc".to_string()),
            provenance: ProvenanceEntry {
                provenance_id: "abc".to_string(),
                provider_id: "claude".to_string(),
                data_path: "/tmp".to_string(),
                format: "jsonl".to_string(),
                ingestion_timestamp: 123,
            },
        };
        let serialized = serde_json::to_string(&data).unwrap();
        assert!(serialized.contains("claude"));
    }
}
