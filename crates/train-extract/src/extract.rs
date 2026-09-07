use crate::error::{TrainExtractError, TrainExtractResult};
use agent_context::KnowledgeKind;
use rusqlite::Connection;
use std::collections::HashMap;

/// Extracted knowledge entry from the knowledge store.
#[derive(Debug, Clone)]
pub struct KnowledgeEntry {
    pub id: i64,
    pub content: String,
    pub kind: KnowledgeKind,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub weight: f64,
    pub source: String,
    pub source_type: String,
    pub source_id: String,
    pub provenance_id: String,
    pub active: bool,
    /// Unix timestamp (seconds) of when the entry was created.
    pub created_at: i64,
}

/// Extracted event from mirror-log.
#[derive(Debug, Clone)]
pub struct LogEvent {
    pub id: String,
    pub content: String,
    pub source: String,
    pub meta: Option<String>,
    pub timestamp: i64,
}

/// A chunk of a state-doc section (reserved for the chunking pipeline;
/// not populated by `extract()` yet).
#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: String,
    pub event_id: String,
    pub content: String,
    pub chunk_index: usize,
}

/// A resolved state-doc annotation (from the agent-context state-docs store).
#[derive(Debug, Clone)]
pub struct Annotation {
    pub id: String,
    pub doc_name: String,
    pub kind: String,
    pub message: String,
    pub status: String,
    /// Reason recorded when the annotation was resolved.
    pub resolution_reason: Option<String>,
    pub confidence: f64,
    pub created_at: i64,
}

/// Configuration for data extraction.
#[derive(Debug, Clone)]
pub struct ExtractConfig {
    /// Only include entries with these tags (empty = all tags).
    pub tags: Vec<String>,
    /// Maximum number of entries to extract per source.
    pub max_entries: usize,
    /// Only include events with timestamp >= this unix timestamp (0 = all).
    pub since: i64,
    /// Include mirror-log events.
    pub include_events: bool,
}

impl Default for ExtractConfig {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            max_entries: 10000,
            since: 0,
            include_events: true,
        }
    }
}

/// Parse a knowledge kind from the `kind` column.
///
/// Accepts plain strings (`pattern`) as well as JSON-quoted values
/// (`"pattern"`), and falls back to [`KnowledgeKind::Context`] for anything
/// unrecognized so one bad row can't fail the whole extraction.
fn parse_kind(raw: &str) -> KnowledgeKind {
    let trimmed = raw.trim();
    // Plain variant name (the canonical on-disk form, per memory/src/schema.rs).
    if let Ok(kind) = serde_json::from_str(&format!("\"{trimmed}\"")) {
        return kind;
    }
    // Already JSON-quoted.
    if let Ok(kind) = serde_json::from_str::<KnowledgeKind>(trimmed) {
        return kind;
    }
    KnowledgeKind::Context
}

/// List a table's column names (via PRAGMA table_info).
fn table_columns(conn: &Connection, table: &str) -> TrainExtractResult<Vec<String>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let cols: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(cols)
}

/// Extract data from the knowledge store and mirror-log databases.
pub fn extract(
    knowledge_conn: &Connection,
    mirror_log_conn: Option<&Connection>,
    config: &ExtractConfig,
) -> TrainExtractResult<ExtractedData> {
    let mut data = ExtractedData::default();

    data.knowledge_entries = extract_knowledge_entries(knowledge_conn, config)?;
    data.sample_count = data.knowledge_entries.len();

    if config.include_events {
        data.events = extract_events(mirror_log_conn, config)?;
    }

    if data.is_empty() {
        return Err(TrainExtractError::EmptyDataset);
    }

    Ok(data)
}

/// Extract knowledge entries from the knowledge store.
///
/// The canonical agent-context schema (memory/src/schema.rs) has a fixed set
/// of columns; newer stores may add `created_at`. We probe the actual table
/// with PRAGMA and build the SELECT to match, so both shapes work.
fn extract_knowledge_entries(
    conn: &Connection,
    config: &ExtractConfig,
) -> TrainExtractResult<Vec<KnowledgeEntry>> {
    let columns = table_columns(conn, "knowledge_entries")?;

    // Base columns that must exist (per memory/src/schema.rs).
    const BASE: [&str; 8] = [
        "id", "content", "kind", "tags", "metadata", "weight", "source", "active",
    ];
    for col in BASE {
        if !columns.contains(&col.to_string()) {
            return Err(TrainExtractError::Database(format!(
                "knowledge_entries is missing required column '{col}'"
            )));
        }
    }

    // Optional columns: fall back to defaults when absent.
    let optional = |col: &str, default: &str| -> String {
        if columns.iter().any(|c| c == col) {
            col.to_string()
        } else {
            format!("'{default}' AS {col}")
        }
    };

    let sql = format!(
        "SELECT id, content, kind, tags, metadata, weight, source, {}, {}, {}, active, {}
         FROM knowledge_entries
         WHERE active = 1
         ORDER BY id DESC
         LIMIT ?1",
        optional("source_type", ""),
        optional("source_id", ""),
        optional("provenance_id", ""),
        if columns.iter().any(|c| c == "created_at") {
            "COALESCE(created_at, 0)".to_string()
        } else {
            "0 AS created_at".to_string()
        },
    );

    let mut stmt = conn.prepare(&sql)?;

    let rows = stmt.query_map([config.max_entries as i64], |row| {
        let tags_str: String = row.get(3)?;
        let metadata_str: String = row.get(4)?;
        let kind_str: String = row.get(2)?;

        Ok(KnowledgeEntry {
            id: row.get(0)?,
            content: row.get(1)?,
            kind: parse_kind(&kind_str),
            tags: serde_json::from_str(&tags_str).unwrap_or_default(),
            metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
            weight: row.get(5)?,
            source: row.get(6)?,
            source_type: row.get(7)?,
            source_id: row.get(8)?,
            provenance_id: row.get(9)?,
            active: row.get(10)?,
            created_at: row.get(11)?,
        })
    })?;

    let entries: Vec<KnowledgeEntry> = rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
        TrainExtractError::Export(format!("failed to read knowledge entries: {e}"))
    })?;

    // Apply tag filter if specified
    let filtered = if config.tags.is_empty() {
        entries
    } else {
        entries
            .into_iter()
            .filter(|entry| {
                config.tags.iter().any(|tag| entry.tags.iter().any(|t| t == tag))
            })
            .collect()
    };

    tracing::debug!(entries_extracted = filtered.len(), "Extracted knowledge entries");

    Ok(filtered)
}

/// Extract events from the mirror-log database.
fn extract_events(
    conn: Option<&Connection>,
    config: &ExtractConfig,
) -> TrainExtractResult<Vec<LogEvent>> {
    let Some(conn) = conn else {
        return Ok(Vec::new());
    };

    let mut stmt = conn.prepare(
        "SELECT id, content, source, meta, timestamp
         FROM events
         WHERE (?1 = 0 OR timestamp >= ?1)
         ORDER BY timestamp DESC
         LIMIT ?2",
    )?;

    let rows = stmt.query_map([config.since, config.max_entries as i64], |row| {
        Ok(LogEvent {
            id: row.get(0)?,
            content: row.get(1)?,
            source: row.get(2)?,
            meta: row.get(3)?,
            timestamp: row.get(4)?,
        })
    })?;

    let events: Vec<LogEvent> = rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
        TrainExtractError::Export(format!("failed to read events: {e}"))
    })?;

    tracing::debug!(events_extracted = events.len(), "Extracted mirror-log events");

    Ok(events)
}

/// Combined extracted data ready for formatting.
#[derive(Debug, Default)]
pub struct ExtractedData {
    pub knowledge_entries: Vec<KnowledgeEntry>,
    pub events: Vec<LogEvent>,
    /// State-doc chunks (reserved; not populated by `extract()` yet).
    pub chunks: Vec<Chunk>,
    /// Resolved state-doc annotations (reserved; not populated by `extract()` yet).
    pub annotations: Vec<Annotation>,
    /// Number of source samples (entries + events) extracted.
    pub sample_count: usize,
}

impl ExtractedData {
    /// Returns true if no data was extracted.
    pub fn is_empty(&self) -> bool {
        self.knowledge_entries.is_empty() && self.events.is_empty()
    }

    /// Get a summary of the extracted data.
    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({
            "knowledge_entries": self.knowledge_entries.len(),
            "events": self.events.len(),
            "chunks": self.chunks.len(),
            "annotations": self.annotations.len(),
            "total_samples": self.sample_count,
        })
    }

    /// Group knowledge entries by tag for weighted sampling.
    pub fn tag_distribution(&self) -> HashMap<String, usize> {
        let mut dist = HashMap::new();
        for entry in &self.knowledge_entries {
            for tag in &entry.tags {
                *dist.entry(tag.clone()).or_insert(0) += 1;
            }
        }
        dist
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Test fixtures matching the real schemas:
    /// - knowledge_entries: agent-context schema (memory/src/schema.rs) — no created_at
    /// - events: mirror-log schema (~/.mirror-log/mirror.db) — timestamp, no active
    fn make_test_db(dir: &tempfile::TempDir) -> (Connection, Connection) {
        let kconn = Connection::open(dir.path().join("knowledge.db")).unwrap();
        kconn.execute_batch(
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
        mconn.execute_batch(
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

        mconn.execute(
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
