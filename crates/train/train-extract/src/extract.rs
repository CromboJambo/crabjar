use crate::error::{TrainExtractError, TrainExtractResult};
use crate::events;
use crate::extract_types::{ExtractConfig, ExtractedData};
use crate::knowledge;
use rusqlite::Connection;

/// Extract data from the knowledge store and mirror-log databases.
pub fn extract(
    knowledge_conn: &Connection,
    mirror_log_conn: Option<&Connection>,
    config: &ExtractConfig,
) -> TrainExtractResult<ExtractedData> {
    let mut data = ExtractedData::default();

    data.knowledge_entries =
        knowledge::extract_knowledge_entries(knowledge_conn, &config.tags, config.max_entries)?;
    data.sample_count = data.knowledge_entries.len();

    if config.include_events {
        data.events = events::extract_events(mirror_log_conn, config.since, config.max_entries)?;
    }

    if data.is_empty() {
        return Err(TrainExtractError::EmptyDataset);
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract_types::ExtractConfig;
    use agent_context::KnowledgeKind;
    use tempfile::tempdir;

    fn make_test_db(dir: &tempfile::TempDir) -> (Connection, Connection) {
        let kconn = Connection::open(dir.path().join("knowledge.db")).unwrap();
        kconn
            .execute_batch(
                "CREATE TABLE knowledge_entries (
                id INTEGER PRIMARY KEY,
                content TEXT NOT NULL,
                kind TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '[]',
                metadata TEXT NOT NULL DEFAULT '{}',
                weight REAL NOT NULL DEFAULT 1.0,
                source TEXT NOT NULL DEFAULT 'user',
                active INTEGER NOT NULL DEFAULT 1,
                source_type TEXT NOT NULL DEFAULT '',
                source_id TEXT NOT NULL DEFAULT '',
                provenance_id TEXT NOT NULL DEFAULT ''
            )",
            )
            .unwrap();

        let mconn = Connection::open(dir.path().join("mirror.db")).unwrap();
        mconn
            .execute_batch(
                "CREATE TABLE events (
                id TEXT PRIMARY KEY,
                timestamp INTEGER NOT NULL,
                source TEXT NOT NULL,
                content TEXT NOT NULL,
                meta TEXT
            )",
            )
            .unwrap();

        (kconn, mconn)
    }

    #[test]
    fn extract_returns_empty_when_no_entries() {
        let dir = tempdir().unwrap();
        let (kconn, mconn) = make_test_db(&dir);
        let config = ExtractConfig::default();
        let result = extract(&kconn, Some(&mconn), &config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TrainExtractError::EmptyDataset
        ));
    }

    #[test]
    fn extract_knowledge_entries_with_data() {
        let dir = tempdir().unwrap();
        let (kconn, mconn) = make_test_db(&dir);

        kconn.execute(
            "INSERT INTO knowledge_entries (id, content, kind, tags, metadata, weight, source, active, source_type, source_id)
             VALUES (1, 'test content', 'pattern', '[\"test\", \"rust\"]', '{}', 1.0, 'user', 1, 'file', 'test-1')",
            [],
        )
        .unwrap();

        let config = ExtractConfig::default();
        let data = extract(&kconn, Some(&mconn), &config).unwrap();
        assert_eq!(data.knowledge_entries.len(), 1);
        assert_eq!(data.knowledge_entries[0].content, "test content");
        assert_eq!(data.knowledge_entries[0].kind, KnowledgeKind::Pattern);
        assert_eq!(
            data.knowledge_entries[0].tags,
            vec!["test".to_string(), "rust".to_string()]
        );
    }

    #[test]
    fn extract_parses_json_quoted_kind() {
        let dir = tempdir().unwrap();
        let (kconn, _) = make_test_db(&dir);

        kconn.execute(
            "INSERT INTO knowledge_entries (id, content, kind, tags, metadata, weight, source, active)
             VALUES (1, 'test', '\"instruction\"', '[]', '{}', 1.0, 'user', 1)",
            [],
        )
        .unwrap();

        let config = ExtractConfig::default();
        let data = extract(&kconn, None, &config).unwrap();
        assert_eq!(data.knowledge_entries[0].kind, KnowledgeKind::Instruction);
    }

    #[test]
    fn extract_filters_by_tag() {
        let dir = tempdir().unwrap();
        let (kconn, mconn) = make_test_db(&dir);

        kconn.execute(
            "INSERT INTO knowledge_entries (id, content, kind, tags, metadata, weight, source, active, source_id)
             VALUES (1, 'rust entry', 'pattern', '[\"rust\"]', '{}', 1.0, 'user', 1, 'test-1')",
            [],
        )
        .unwrap();
        kconn.execute(
            "INSERT INTO knowledge_entries (id, content, kind, tags, metadata, weight, source, active, source_id)
             VALUES (2, 'python entry', 'pattern', '[\"python\"]', '{}', 1.0, 'user', 1, 'test-2')",
            [],
        )
        .unwrap();

        let config = ExtractConfig {
            tags: vec!["rust".to_string()],
            ..ExtractConfig::default()
        };
        let data = extract(&kconn, Some(&mconn), &config).unwrap();
        assert_eq!(data.knowledge_entries.len(), 1);
        assert_eq!(data.knowledge_entries[0].content, "rust entry");
    }

    #[test]
    fn extract_filters_inactive_entries() {
        let dir = tempdir().unwrap();
        let (kconn, _) = make_test_db(&dir);

        kconn.execute(
            "INSERT INTO knowledge_entries (id, content, kind, tags, metadata, weight, source, active)
             VALUES (1, 'active entry', 'pattern', '[]', '{}', 1.0, 'user', 1)",
            [],
        )
        .unwrap();
        kconn.execute(
            "INSERT INTO knowledge_entries (id, content, kind, tags, metadata, weight, source, active)
             VALUES (2, 'inactive entry', 'pattern', '[]', '{}', 1.0, 'user', 0)",
            [],
        )
        .unwrap();

        let config = ExtractConfig::default();
        let data = extract(&kconn, None, &config).unwrap();
        assert_eq!(data.knowledge_entries.len(), 1);
        assert_eq!(data.knowledge_entries[0].content, "active entry");
    }

    #[test]
    fn extract_includes_events() {
        let dir = tempdir().unwrap();
        let (kconn, mconn) = make_test_db(&dir);

        kconn.execute(
            "INSERT INTO knowledge_entries (id, content, kind, tags, metadata, weight, source, active)
             VALUES (1, 'test entry', 'pattern', '[\"test\"]', '{}', 1.0, 'user', 1)",
            [],
        )
        .unwrap();

        mconn
            .execute(
                "INSERT INTO events (id, timestamp, source, content, meta)
             VALUES ('evt-1', 1000000, 'file', 'test event', null)",
                [],
            )
            .unwrap();

        let config = ExtractConfig::default();
        let data = extract(&kconn, Some(&mconn), &config).unwrap();
        assert_eq!(data.events.len(), 1);
        assert_eq!(data.events[0].content, "test event");
    }

    #[test]
    fn extract_filters_events_by_since() {
        let dir = tempdir().unwrap();
        let (kconn, mconn) = make_test_db(&dir);

        kconn.execute(
            "INSERT INTO knowledge_entries (id, content, kind, tags, metadata, weight, source, active)
             VALUES (1, 'test entry', 'pattern', '[]', '{}', 1.0, 'user', 1)",
            [],
        )
        .unwrap();

        mconn.execute(
            "INSERT INTO events (id, timestamp, source, content) VALUES ('old', 1000000, 'file', 'old event')",
            [],
        )
        .unwrap();
        mconn.execute(
            "INSERT INTO events (id, timestamp, source, content) VALUES ('new', 2000000, 'file', 'new event')",
            [],
        )
        .unwrap();

        let config = ExtractConfig {
            since: 1500000,
            ..ExtractConfig::default()
        };
        let data = extract(&kconn, Some(&mconn), &config).unwrap();
        assert_eq!(data.events.len(), 1);
        assert_eq!(data.events[0].content, "new event");
    }

    #[test]
    fn extract_without_mirror_log_returns_empty_events() {
        let dir = tempdir().unwrap();
        let (kconn, _) = make_test_db(&dir);

        kconn.execute(
            "INSERT INTO knowledge_entries (id, content, kind, tags, metadata, weight, source, active)
             VALUES (1, 'test entry', 'pattern', '[\"test\"]', '{}', 1.0, 'user', 1)",
            [],
        )
        .unwrap();

        let config = ExtractConfig {
            include_events: true,
            ..ExtractConfig::default()
        };
        let data = extract(&kconn, None, &config).unwrap();
        assert!(data.events.is_empty());
    }

    #[test]
    fn extract_summary_works() {
        let dir = tempdir().unwrap();
        let (kconn, mconn) = make_test_db(&dir);

        kconn.execute(
            "INSERT INTO knowledge_entries (id, content, kind, tags, metadata, weight, source, active)
             VALUES (1, 'test', 'pattern', '[\"tag1\", \"tag2\"]', '{}', 1.0, 'user', 1)",
            [],
        )
        .unwrap();

        let config = ExtractConfig::default();
        let data = extract(&kconn, Some(&mconn), &config).unwrap();
        let summary = data.summary();
        assert_eq!(summary["knowledge_entries"], 1);
        assert_eq!(summary["total_samples"], 1);
    }

    #[test]
    fn extract_tag_distribution_works() {
        let dir = tempdir().unwrap();
        let (kconn, mconn) = make_test_db(&dir);

        kconn.execute(
            "INSERT INTO knowledge_entries (id, content, kind, tags, metadata, weight, source, active)
             VALUES (1, 'a', 'pattern', '[\"rust\", \"pattern\"]', '{}', 1.0, 'user', 1)",
            [],
        )
        .unwrap();

        let config = ExtractConfig::default();
        let data = extract(&kconn, Some(&mconn), &config).unwrap();
        let dist = data.tag_distribution();
        assert_eq!(dist.get("rust"), Some(&1));
        assert_eq!(dist.get("pattern"), Some(&1));
    }

    #[test]
    fn extract_respects_max_entries() {
        let dir = tempdir().unwrap();
        let (kconn, _) = make_test_db(&dir);

        for i in 1..=5 {
            kconn.execute(
                "INSERT INTO knowledge_entries (id, content, kind, tags, metadata, weight, source, active)
                 VALUES (?, ?, 'pattern', '[]', '{}', 1.0, 'user', 1)",
                rusqlite::params![i, format!("entry {i}")],
            )
            .unwrap();
        }

        let config = ExtractConfig {
            max_entries: 3,
            ..ExtractConfig::default()
        };
        let data = extract(&kconn, None, &config).unwrap();
        assert_eq!(data.knowledge_entries.len(), 3);
    }
}
