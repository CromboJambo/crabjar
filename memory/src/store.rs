#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn store_open_with_tempdir_works() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        assert!(store.conn.is_open());
    }

    #[test]
    fn store_open_with_memory_works() {
        let store = Store::open(":memory:").unwrap();
        assert!(store.conn.is_open());
    }

    #[test]
    fn store_insert_works() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Instruction);
        let id = store.insert(entry).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn store_insert_with_tags_works() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Pattern)
            .tags(["deploy", "stale"]);
        let id = store.insert(entry).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn store_insert_with_weight_works() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Example)
            .weight(0.5);
        let id = store.insert(entry).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn store_insert_with_metadata_works() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Context)
            .meta("source_type", "agent")
            .meta("provenance_id", "abc");
        let id = store.insert(entry).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn store_insert_with_stale_works() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let stale_date = chrono::Utc::now() + chrono::Duration::days(7);
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Instruction)
            .stale(stale_date);
        let id = store.insert(entry).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn store_deactivate_existing_entry_works() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Instruction);
        let id = store.insert(entry).unwrap();
        store.deactivate(id, Source::User, Some("superseded")).unwrap();
    }

    #[test]
    fn store_deactivate_nonexistent_entry_errors() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let result = store.deactivate(999, Source::User, Some("superseded"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Knowledge entry 999"));
    }

    #[test]
    fn store_query_with_tags_works() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Instruction)
            .tags(["deploy"]);
        let id = store.insert(entry).unwrap();
        let rows = store.query(&["deploy"], 10, "").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, id);
    }

    #[test]
    fn store_query_with_empty_tags_returns_empty() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Instruction)
            .tags(["deploy"]);
        store.insert(entry).unwrap();
        let rows = store.query(&[], 10, "").unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn store_query_with_limit_works() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let entry1 = KnowledgeEntry::new("content 1", KnowledgeKind::Instruction)
            .tags(["deploy"]);
        let entry2 = KnowledgeEntry::new("content 2", KnowledgeKind::Instruction)
            .tags(["deploy"]);
        store.insert(entry1).unwrap();
        store.insert(entry2).unwrap();
        let rows = store.query(&["deploy"], 1, "").unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn store_query_with_non_matching_tags_returns_empty() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Instruction)
            .tags(["deploy"]);
        store.insert(entry).unwrap();
        let rows = store.query(&["build"], 10, "").unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn store_find_active_by_provenance_works() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Instruction)
            .meta("source_type", "agent")
            .meta("source_id", "abc");
        let id = store.insert(entry).unwrap();
        let result = store.find_active_by_provenance("agent", "abc").unwrap();
        assert_eq!(result, Some(id));
    }

    #[test]
    fn store_find_active_by_provenance_not_found_returns_none() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let result = store.find_active_by_provenance("agent", "nonexistent").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn store_deactivate_by_provenance_works() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Instruction)
            .meta("source_type", "agent")
            .meta("source_id", "abc");
        let id = store.insert(entry).unwrap();
        let affected = store.deactivate_by_provenance("agent", "abc", Source::User, Some("superseded")).unwrap();
        assert_eq!(affected, 1);
    }

    #[test]
    fn store_deactivate_by_provenance_no_match_returns_zero() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let affected = store.deactivate_by_provenance("agent", "nonexistent", Source::User, Some("superseded")).unwrap();
        assert_eq!(affected, 0);
    }

    #[test]
    fn store_deactivate_by_provenance_id_works() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Instruction)
            .meta("provenance_id", "abc");
        let id = store.insert(entry).unwrap();
        let affected = store.deactivate_by_provenance_id("abc", Source::User, Some("superseded")).unwrap();
        assert_eq!(affected, 1);
    }

    #[test]
    fn store_deactivate_by_provenance_id_no_match_returns_zero() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let affected = store.deactivate_by_provenance_id("nonexistent", Source::User, Some("superseded")).unwrap();
        assert_eq!(affected, 0);
    }

    #[test]
    fn store_verify_with_placeholder_checksum_returns_empty() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Instruction);
        store.insert(entry).unwrap();
        let bad_ids = store.verify().unwrap();
        assert!(bad_ids.is_empty());
    }

    #[test]
    fn store_verify_with_missing_entries_returns_empty() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let bad_ids = store.verify().unwrap();
        assert!(bad_ids.is_empty());
    }

    #[test]
    fn store_decay_weight_with_no_stale_after_returns_one() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Instruction);
        let id = store.insert(entry).unwrap();
        let weight = store.decay_weight(id).unwrap();
        assert_eq!(weight, 1.0);
    }

    #[test]
    fn store_decay_weight_with_stale_after_past_returns_decay() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let stale_date = chrono::Utc::now() - chrono::Duration::days(10);
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Instruction)
            .stale(stale_date);
        let id = store.insert(entry).unwrap();
        let weight = store.decay_weight(id).unwrap();
        assert!(weight < 1.0);
    }

    #[test]
    fn store_decay_weight_with_stale_after_future_returns_one() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let stale_date = chrono::Utc::now() + chrono::Duration::days(7);
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Instruction)
            .stale(stale_date);
        let id = store.insert(entry).unwrap();
        let weight = store.decay_weight(id).unwrap();
        assert_eq!(weight, 1.0);
    }

    #[test]
    fn store_decay_weight_nonexistent_entry_errors() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let result = store.decay_weight(999);
        assert!(result.is_err());
    }

    #[test]
    fn store_events_returns_recent_events() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Instruction);
        let id = store.insert(entry).unwrap();
        let events = store.events(10).unwrap();
        assert!(!events.is_empty());
    }

    #[test]
    fn store_events_with_limit_works() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let entry1 = KnowledgeEntry::new("content 1", KnowledgeKind::Instruction);
        let entry2 = KnowledgeEntry::new("content 2", KnowledgeKind::Instruction);
        let entry3 = KnowledgeEntry::new("content 3", KnowledgeKind::Instruction);
        store.insert(entry1).unwrap();
        store.insert(entry2).unwrap();
        store.insert(entry3).unwrap();
        let events = store.events(1).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn store_events_empty_db_returns_empty() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("test.db")).unwrap();
        let events = store.events(10).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn knowledge_kind_display_instruction() {
        assert_eq!(format!("{}", KnowledgeKind::Instruction), "instruction");
    }

    #[test]
    fn knowledge_kind_display_pattern() {
        assert_eq!(format!("{}", KnowledgeKind::Pattern), "pattern");
    }

    #[test]
    fn knowledge_kind_display_example() {
        assert_eq!(format!("{}", KnowledgeKind::Example), "example");
    }

    #[test]
    fn knowledge_kind_display_context() {
        assert_eq!(format!("{}", KnowledgeKind::Context), "context");
    }

    #[test]
    fn source_display_user() {
        assert_eq!(format!("{}", Source::User), "user");
    }

    #[test]
    fn source_display_agent() {
        assert_eq!(format!("{}", Source::Agent), "agent");
    }

    #[test]
    fn source_display_system() {
        assert_eq!(format!("{}", Source::System), "system");
    }

    #[test]
    fn knowledge_entry_new_defaults() {
        let entry = KnowledgeEntry::new("test", KnowledgeKind::Instruction);
        assert_eq!(entry.content, "test");
        assert_eq!(entry.kind, KnowledgeKind::Instruction);
        assert!(entry.tags.is_empty());
        assert_eq!(entry.weight, 1.0);
        assert_eq!(entry.source, Source::User);
    }

    #[test]
    fn knowledge_entry_tags_builder_works() {
        let entry = KnowledgeEntry::new("test", KnowledgeKind::Instruction)
            .tags(["a", "b", "c"]);
        assert_eq!(entry.tags, vec!["a", "b", "c"]);
    }

    #[test]
    fn knowledge_entry_weight_builder_works() {
        let entry = KnowledgeEntry::new("test", KnowledgeKind::Instruction)
            .weight(0.5);
        assert_eq!(entry.weight, 0.5);
    }

    #[test]
    fn knowledge_entry_meta_builder_works() {
        let entry = KnowledgeEntry::new("test", KnowledgeKind::Instruction)
            .meta("key", "value");
        assert!(entry.metadata.as_object().unwrap().contains_key("key"));
    }

    #[test]
    fn knowledge_entry_stale_builder_works() {
        let stale_date = chrono::Utc::now() + chrono::Duration::days(7);
        let entry = KnowledgeEntry::new("test", KnowledgeKind::Instruction)
            .stale(stale_date);
        assert!(entry.metadata.as_object().unwrap().contains_key("stale_after"));
    }

    #[test]
    fn knowledge_kind_equality_works() {
        assert_eq!(KnowledgeKind::Instruction, KnowledgeKind::Instruction);
        assert_ne!(KnowledgeKind::Instruction, KnowledgeKind::Pattern);
    }

    #[test]
    fn source_equality_works() {
        assert_eq!(Source::User, Source::User);
        assert_ne!(Source::User, Source::Agent);
    }

    #[test]
    fn knowledge_entry_clone_works() {
        let entry = KnowledgeEntry::new("test", KnowledgeKind::Instruction);
        let cloned = entry.clone();
        assert_eq!(cloned.content, entry.content);
        assert_eq!(cloned.kind, entry.kind);
    }

    #[test]
    fn knowledge_row_clone_works() {
        let row = KnowledgeRow {
            id: 1,
            content: "test".to_string(),
            tags: vec!["a".to_string()],
            metadata: serde_json::json!({}),
            active: true,
        };
        let cloned = row.clone();
        assert_eq!(cloned.id, row.id);
    }

    #[test]
    fn event_kind_clone_works() {
        let kind = EventKind {
            kind: "insert".to_string(),
            target_id: Some(1),
            payload: Some(serde_json::json!({}),
            source: "user".to_string(),
            ts: "2026-01-01".to_string(),
        };
        let cloned = kind.clone();
        assert_eq!(cloned.kind, kind.kind);
    }

    #[test]
    fn knowledge_kind_serialize_works() {
        let serialized = serde_json::to_string(&KnowledgeKind::Instruction).unwrap();
        assert_eq!(serialized, "\"instruction\"");
    }

    #[test]
    fn knowledge_kind_deserialize_works() {
        let deserialized: KnowledgeKind = serde_json::from_str("\"instruction\"").unwrap();
        assert_eq!(deserialized, KnowledgeKind::Instruction);
    }

    #[test]
    fn source_serialize_works() {
        let serialized = serde_json::to_string(&Source::User).unwrap();
        assert_eq!(serialized, "\"user\"");
    }

    #[test]
    fn source_deserialize_works() {
        let deserialized: Source = serde_json::from_str("\"user\"").unwrap();
        assert_eq!(deserialized, Source::User);
    }

    #[test]
    fn knowledge_entry_serialize_works() {
        let entry = KnowledgeEntry::new("test", KnowledgeKind::Instruction);
        let serialized = serde_json::to_string(&entry).unwrap();
        assert!(serialized.contains("test"));
    }

    #[test]
    fn knowledge_entry_deserialize_works() {
        let serialized = serde_json::to_string(&KnowledgeEntry::new("test", KnowledgeKind::Instruction)).unwrap();
        let deserialized: KnowledgeEntry = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.content, "test");
    }

    #[test]
    fn knowledge_row_serialize_works() {
        let row = KnowledgeRow {
            id: 1,
            content: "test".to_string(),
            tags: vec!["a".to_string()],
            metadata: serde_json::json!({}),
            active: true,
        };
        let serialized = serde_json::to_string(&row).unwrap();
        assert!(serialized.contains("test"));
    }

    #[test]
    fn knowledge_row_deserialize_works() {
        let serialized = serde_json::to_string(&KnowledgeRow {
            id: 1,
            content: "test".to_string(),
            tags: vec!["a".to_string()],
            metadata: serde_json::json!({}),
            active: true,
        }).unwrap();
        let deserialized: KnowledgeRow = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.id, 1);
    }

    #[test]
    fn event_row_serialize_works() {
        let row = EventRow {
            id: 1,
            event_type: "insert".to_string(),
            timestamp: chrono::Utc::now(),
        };
        let serialized = serde_json::to_string(&row).unwrap();
        assert!(serialized.contains("insert"));
    }

    #[test]
    fn event_kind_serialize_works() {
        let kind = EventKind {
            kind: "insert".to_string(),
            target_id: Some(1),
            payload: Some(serde_json::json!({}),
            source: "user".to_string(),
            ts: "2026-01-01".to_string(),
        };
        let serialized = serde_json::to_string(&kind).unwrap();
        assert!(serialized.contains("insert"));
    }
}
