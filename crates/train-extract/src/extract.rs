use crate::error::{TrainExtractError, TrainExtractResult};
use crate::events;
use crate::extract_types::{ExtractConfig, ExtractedData};
use crate::knowledge;
use rusqlite::Connection;

/// Extract training data from the Crabjar knowledge store.
///
/// Connects to the provided database path and extracts:
/// - Knowledge entries (with optional tag filtering)
/// - Log events (if include_events is true)
pub fn extract(
    conn: &Connection,
    event_conn: Option<&Connection>,
    config: &ExtractConfig,
) -> TrainExtractResult<ExtractedData> {
    // Extract knowledge entries
    let knowledge_entries =
        knowledge::extract_knowledge_entries(conn, &config.tags, config.max_entries)?;

    // Extract events if requested
    let extracted_events = if config.include_events {
        match events::extract_events(event_conn, config.since, config.max_entries) {
            Ok(events) => events,
            Err(e) => {
                tracing::warn!("Failed to extract events: {}", e);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let total_samples = knowledge_entries.len() + extracted_events.len();
    tracing::info!(
        "Extraction complete: {} knowledge entries, {} events ({} total samples)",
        knowledge_entries.len(),
        extracted_events.len(),
        total_samples
    );

    Ok(ExtractedData {
        knowledge_entries,
        events: extracted_events,
        chunks: Vec::new(),
        annotations: Vec::new(),
        sample_count: total_samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract_types::{ExtractConfig, ExtractedData};
    use std::fs;

    fn create_test_db(path: &str) -> TrainExtractResult<Connection> {
        let conn = Connection::open(path)?;
        // Create knowledge_entries table with the canonical schema (per memory/src/schema.rs).
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS knowledge_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'context',
                tags TEXT NOT NULL DEFAULT '[]',
                metadata TEXT NOT NULL DEFAULT '{}',
                weight REAL NOT NULL DEFAULT 1.0,
                source TEXT NOT NULL DEFAULT '',
                active INTEGER NOT NULL DEFAULT 1
            )",
        )?;

        // Insert test data
        conn.execute(
            "INSERT INTO knowledge_entries (content, kind, tags, metadata, weight, source) VALUES (?, ?, ?, ?, ?, ?)",
            [
                "Test pattern content",
                "pattern",
                "[\"test\"]",
                "{}",
                "1.0",
                "test",
            ],
        )?;

        Ok(conn)
    }

    #[test]
    fn test_extract_knowledge() -> TrainExtractResult<()> {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test.db").to_string_lossy().to_string();

        let conn = create_test_db(&db_path)?;

        let config = ExtractConfig {
            tags: vec![String::from("test")],
            max_entries: 100,
            since: 0,
            include_events: false,
        };

        let result = extract(&conn, None, &config)?;

        assert_eq!(result.knowledge_entries.len(), 1);
        assert!(result.knowledge_entries[0].content.contains("Test pattern"));
        assert_eq!(result.sample_count, 1);

        Ok(())
    }

    #[test]
    fn test_extract_all_tags() -> TrainExtractResult<()> {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test2.db").to_string_lossy().to_string();

        let conn = create_test_db(&db_path)?;

        let config = ExtractConfig {
            tags: Vec::new(), // No filter - get all
            max_entries: 100,
            since: 0,
            include_events: false,
        };

        let result = extract(&conn, None, &config)?;

        assert_eq!(result.knowledge_entries.len(), 1);

        Ok(())
    }

    #[test]
    fn test_extracted_data_summary() {
        let data = ExtractedData {
            knowledge_entries: vec![crate::extract_types::KnowledgeEntry {
                id: 1,
                content: String::from("test"),
                kind: agent_context::KnowledgeKind::Pattern,
                tags: vec!["tag1".to_string()],
                metadata: serde_json::json!({}),
                weight: 1.0,
                source: String::new(),
                source_type: String::new(),
                source_id: String::new(),
                provenance_id: String::new(),
                active: true,
                created_at: 0,
            }],
            events: vec![],
            chunks: vec![],
            annotations: vec![],
            sample_count: 1,
        };

        let summary = data.summary();
        assert_eq!(summary["knowledge_entries"], 1);
        assert_eq!(summary["events"], 0);
    }

    #[test]
    fn test_tag_distribution() {
        let data = ExtractedData {
            knowledge_entries: vec![crate::extract_types::KnowledgeEntry {
                id: 1,
                content: String::from("test"),
                kind: agent_context::KnowledgeKind::Pattern,
                tags: vec!["tag1".to_string(), "tag2".to_string()],
                metadata: serde_json::json!({}),
                weight: 1.0,
                source: String::new(),
                source_type: String::new(),
                source_id: String::new(),
                provenance_id: String::new(),
                active: true,
                created_at: 0,
            }],
            events: vec![],
            chunks: vec![],
            annotations: vec![],
            sample_count: 1,
        };

        let dist = data.tag_distribution();
        assert_eq!(dist.get("tag1"), Some(&1));
        assert_eq!(dist.get("tag2"), Some(&1));
    }
}
