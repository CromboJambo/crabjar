use crate::models::EventRow;
use crate::{KnowledgeEntry, KnowledgeRow, Source};
use rusqlite::Connection;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),
    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("internal error: {0}")]
    Internal(String),
}

pub type StoreResult<T> = Result<T, StoreError>;

pub struct Store {
    pub conn: Connection,
}

impl Store {
    pub fn open(path: impl Into<std::path::PathBuf>) -> StoreResult<Self> {
        let conn = Connection::open(path.into())?;
        Ok(Self { conn })
    }

    pub fn insert(&self, entry: KnowledgeEntry) -> StoreResult<i64> {
        let tags_str = serde_json::to_string(&entry.tags)?;
        let metadata_str = serde_json::to_string(&entry.metadata)?;
        let kind_str = serde_json::to_string(&entry.kind)?;
        let source_str = serde_json::to_string(&entry.source)?;

        self.conn.execute(
            "INSERT INTO knowledge_entries (content, kind, tags, metadata, weight, source, active, source_type, source_id, provenance_id) VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?, ?)",
            rusqlite::params![entry.content, kind_str, tags_str, metadata_str, entry.weight, source_str, entry.source_type, entry.source_id, entry.provenance_id],
        )?;

        let rowid = self.conn.last_insert_rowid();
        let ts = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO event_rows (event_type, timestamp) VALUES (?, ?)",
            rusqlite::params!["insert", ts],
        )?;

        Ok(rowid)
    }

    pub fn query(
        &self,
        tags: &[&str],
        limit: usize,
        provenance_id: &str,
        source_id: &str,
        source_doc: &str,
    ) -> StoreResult<Vec<KnowledgeRow>> {
        let mut rows = Vec::new();
        let sql = match (
            provenance_id.is_empty(),
            source_id.is_empty(),
            source_doc.is_empty(),
        ) {
            (false, false, false) => {
                "SELECT id, content, tags, metadata, active FROM knowledge_entries WHERE active = 1 AND provenance_id = ? AND source_id = ? AND metadata->>'$.source_doc' = ?"
            }
            (true, false, false) => {
                "SELECT id, content, tags, metadata, active FROM knowledge_entries WHERE active = 1 AND source_id = ? AND metadata->>'$.source_doc' = ?"
            }
            (false, true, false) => {
                "SELECT id, content, tags, metadata, active FROM knowledge_entries WHERE active = 1 AND provenance_id = ? AND metadata->>'$.source_doc' = ?"
            }
            (true, true, false) => {
                "SELECT id, content, tags, metadata, active FROM knowledge_entries WHERE active = 1 AND metadata->>'$.source_doc' = ?"
            }
            (false, false, true) => {
                "SELECT id, content, tags, metadata, active FROM knowledge_entries WHERE active = 1 AND provenance_id = ? AND source_id = ?"
            }
            (true, false, true) => {
                "SELECT id, content, tags, metadata, active FROM knowledge_entries WHERE active = 1 AND source_id = ?"
            }
            (false, true, true) => {
                "SELECT id, content, tags, metadata, active FROM knowledge_entries WHERE active = 1 AND provenance_id = ?"
            }
            (true, true, true) => {
                "SELECT id, content, tags, metadata, active FROM knowledge_entries WHERE active = 1"
            }
        };
        let mut stmt = self.conn.prepare(sql)?;

        let cursor = match (
            provenance_id.is_empty(),
            source_id.is_empty(),
            source_doc.is_empty(),
        ) {
            (false, false, false) => stmt.query_map(
                rusqlite::params![provenance_id, source_id, source_doc],
                raw_knowledge_row,
            )?,
            (true, false, false) => {
                stmt.query_map(rusqlite::params![source_id, source_doc], raw_knowledge_row)?
            }
            (false, true, false) => stmt.query_map(
                rusqlite::params![provenance_id, source_doc],
                raw_knowledge_row,
            )?,
            (true, true, false) => {
                stmt.query_map(rusqlite::params![source_doc], raw_knowledge_row)?
            }
            (false, false, true) => stmt.query_map(
                rusqlite::params![provenance_id, source_id],
                raw_knowledge_row,
            )?,
            (true, false, true) => {
                stmt.query_map(rusqlite::params![source_id], raw_knowledge_row)?
            }
            (false, true, true) => {
                stmt.query_map(rusqlite::params![provenance_id], raw_knowledge_row)?
            }
            (true, true, true) => stmt.query_map([], raw_knowledge_row)?,
        };

        for row in cursor {
            let row = row?;
            if row_matches_tags(&row, tags) {
                rows.push(row);
            }
            if rows.len() >= limit {
                break;
            }
        }

        Ok(rows)
    }

    pub fn find_active_by_source(
        &self,
        source_type: &str,
        source_id: &str,
    ) -> StoreResult<Option<KnowledgeRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, metadata, active FROM knowledge_entries WHERE active = 1 AND source_type = ? AND source_id = ?",
        )?;

        let mut cursor =
            stmt.query_map(rusqlite::params![source_type, source_id], raw_knowledge_row)?;

        if let Some(row) = cursor.next() {
            return Ok(Some(row?));
        }

        Ok(None)
    }

    pub fn events(&self, limit: usize) -> StoreResult<Vec<EventRow>> {
        let mut rows = Vec::new();
        let mut stmt = self
            .conn
            .prepare("SELECT id, event_type, timestamp FROM event_rows ORDER BY id DESC LIMIT ?")?;

        let cursor = stmt.query_map(rusqlite::params![limit], |row| {
            let ts_str: String = row.get(2)?;
            let ts = chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(&ts_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            Ok(EventRow {
                id: row.get(0)?,
                event_type: row.get(1)?,
                timestamp: ts,
            })
        })?;

        for row in cursor {
            rows.push(row?);
        }

        Ok(rows)
    }

    pub fn deactivate_by_source(
        &self,
        source_type: &str,
        source_id: &str,
        source: Source,
        reason: Option<&str>,
    ) -> StoreResult<usize> {
        let _source = serde_json::to_string(&source)?;
        let _reason = reason.unwrap_or("");
        let ids = self.matching_ids_by_source(source_type, source_id)?;
        let mut affected = 0;
        for id in ids {
            affected += self.conn.execute(
                "UPDATE knowledge_entries SET active = 0 WHERE id = ?",
                rusqlite::params![id],
            )?;
        }

        Ok(affected)
    }

    pub fn deactivate_by_provenance_id(
        &self,
        provenance_id: &str,
        _source: Source,
        reason: Option<&str>,
    ) -> StoreResult<usize> {
        let _reason = reason.unwrap_or("");
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM knowledge_entries WHERE active = 1 AND provenance_id = ?")?;
        let cursor = stmt.query_map(rusqlite::params![provenance_id], |row| {
            row.get::<usize, i64>(0)
        })?;
        let mut ids = Vec::new();
        for id in cursor {
            ids.push(id?);
        }
        let mut affected = 0;
        for id in ids {
            affected += self.conn.execute(
                "UPDATE knowledge_entries SET active = 0 WHERE id = ?",
                rusqlite::params![id],
            )?;
        }

        Ok(affected)
    }

    pub fn verify(&self) -> StoreResult<Vec<i64>> {
        let mut bad_ids = Vec::new();
        let mut stmt = self.conn.prepare(
            "SELECT id FROM knowledge_entries WHERE active = 1 AND (tags = '' OR metadata = '' OR content = '')",
        )?;

        let cursor = stmt.query_map(rusqlite::params![], |row| row.get(0))?;
        for id in cursor {
            bad_ids.push(id?);
        }

        Ok(bad_ids)
    }

    pub fn deactivate(&self, id: i64, source: Source, reason: Option<&str>) -> StoreResult<()> {
        let _source = serde_json::to_string(&source)?;
        let _reason = reason.unwrap_or("");
        self.conn.execute(
            "UPDATE knowledge_entries SET active = 0 WHERE id = ?",
            rusqlite::params![id],
        )?;

        Ok(())
    }

    fn matching_ids_by_source(&self, source_type: &str, source_id: &str) -> StoreResult<Vec<i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM knowledge_entries WHERE active = 1 AND source_type = ? AND source_id = ?",
        )?;
        let cursor = stmt.query_map(rusqlite::params![source_type, source_id], |row| row.get(0))?;
        let mut ids = Vec::new();

        for id in cursor {
            ids.push(id?);
        }

        Ok(ids)
    }
}

fn raw_knowledge_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<KnowledgeRow> {
    let tags_str: String = row.get(2)?;
    let metadata_str: String = row.get(3)?;
    Ok(KnowledgeRow {
        id: row.get(0)?,
        content: row.get(1)?,
        tags: serde_json::from_str(&tags_str).unwrap_or_default(),
        metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
        active: row.get(4)?,
    })
}

fn row_matches_tags(row: &KnowledgeRow, tags: &[&str]) -> bool {
    tags.is_empty()
        || tags
            .iter()
            .all(|wanted| row.tags.iter().any(|tag| tag == wanted))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KnowledgeKind;

    fn temp_store() -> (Store, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("knowledge.db");
        let conn = Connection::open(path).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS knowledge_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
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
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS event_rows (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                timestamp TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        let store = Store { conn };
        (store, dir)
    }

    fn insert_entry(store: &Store, content: &str, tags: &[&str]) -> i64 {
        let entry = KnowledgeEntry {
            content: content.to_string(),
            kind: KnowledgeKind::Pattern,
            tags: tags.iter().map(|s| s.to_string()).collect(),
            metadata: serde_json::json!({"source_doc": "test.md"}),
            weight: 1.0,
            source: Source::User,
            source_type: "file".to_string(),
            source_id: "test-file".to_string(),
            provenance_id: "prov-1".to_string(),
        };
        store.insert(entry).unwrap()
    }

    #[test]
    fn test_open_and_insert() {
        let (store, _dir) = temp_store();
        let entry = KnowledgeEntry::new("test content", KnowledgeKind::Pattern);
        let rowid = store.insert(entry).unwrap();
        assert!(rowid > 0);
    }

    #[test]
    fn test_query_all_empty() {
        let (store, _dir) = temp_store();
        let rows = store.query(&[], 10, "", "", "").unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn test_query_with_tags() {
        let (store, _dir) = temp_store();
        insert_entry(&store, "rust pattern", &["rust", "pattern"]);
        insert_entry(&store, "python pattern", &["python", "pattern"]);
        let rows = store.query(&["rust"], 10, "", "", "").unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].content.contains("rust"));
    }

    #[test]
    fn test_query_with_provenance_id() {
        let (store, _dir) = temp_store();
        insert_entry(&store, "content with provenance", &["test"]);
        let rows = store.query(&[], 10, "prov-1", "", "").unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_query_with_source_id() {
        let (store, _dir) = temp_store();
        insert_entry(&store, "content with source", &["test"]);
        let rows = store.query(&[], 10, "", "test-file", "").unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_query_with_source_doc() {
        let (store, _dir) = temp_store();
        insert_entry(&store, "content with doc", &["test"]);
        let rows = store.query(&[], 10, "", "", "test.md").unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_query_with_all_filters() {
        let (store, _dir) = temp_store();
        insert_entry(&store, "filtered content", &["test"]);
        let rows = store.query(&[], 10, "prov-1", "test-file", "test.md").unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_query_limit() {
        let (store, _dir) = temp_store();
        for i in 0..5 {
            store.insert(KnowledgeEntry {
                content: format!("entry-{}", i),
                kind: KnowledgeKind::Pattern,
                tags: vec!["test".to_string()],
                metadata: serde_json::json!({}),
                weight: 1.0,
                source: Source::User,
                source_type: "file".to_string(),
                source_id: "test".to_string(),
                provenance_id: "prov-1".to_string(),
            }).unwrap();
        }
        let rows = store.query(&[], 3, "", "", "").unwrap();
        assert!(rows.len() <= 3);
    }

    #[test]
    fn test_find_active_by_source() {
        let (store, _dir) = temp_store();
        store.insert(KnowledgeEntry {
            content: "found content".to_string(),
            kind: KnowledgeKind::Pattern,
            tags: vec!["test".to_string()],
            metadata: serde_json::json!({}),
            weight: 1.0,
            source: Source::User,
            source_type: "file".to_string(),
            source_id: "unique-id".to_string(),
            provenance_id: "prov-1".to_string(),
        }).unwrap();
        let row = store.find_active_by_source("file", "unique-id").unwrap();
        assert!(row.is_some());
        assert_eq!(row.unwrap().content, "found content");
    }

    #[test]
    fn test_find_active_by_source_not_found() {
        let (store, _dir) = temp_store();
        let row = store.find_active_by_source("file", "nonexistent").unwrap();
        assert!(row.is_none());
    }

    #[test]
    fn test_events() {
        let (store, _dir) = temp_store();
        insert_entry(&store, "test", &["test"]);
        let events = store.events(10).unwrap();
        assert!(!events.is_empty());
        assert_eq!(events[0].event_type, "insert");
    }

    #[test]
    fn test_events_limit() {
        let (store, _dir) = temp_store();
        insert_entry(&store, "test1", &["test"]);
        insert_entry(&store, "test2", &["test"]);
        insert_entry(&store, "test3", &["test"]);
        let events = store.events(2).unwrap();
        assert!(events.len() <= 2);
    }

    #[test]
    fn test_deactivate() {
        let (store, _dir) = temp_store();
        let id = insert_entry(&store, "deactivate me", &["test"]);
        store.deactivate(id, Source::User, Some("reason")).unwrap();
        let rows = store.query(&[], 10, "", "", "").unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn test_deactivate_by_source() {
        let (store, _dir) = temp_store();
        insert_entry(&store, "keep this", &["other"]);
        insert_entry(&store, "remove this", &["test"]);
        let affected = store.deactivate_by_source("file", "test-file", Source::User, Some("reason")).unwrap();
        assert!(affected >= 1);
    }

    #[test]
    fn test_deactivate_by_provenance_id() {
        let (store, _dir) = temp_store();
        insert_entry(&store, "prov content", &["test"]);
        let affected = store.deactivate_by_provenance_id("prov-1", Source::User, Some("reason")).unwrap();
        assert!(affected >= 1);
    }

    #[test]
    fn test_verify_empty() {
        let (store, _dir) = temp_store();
        let bad = store.verify().unwrap();
        assert!(bad.is_empty());
    }

    #[test]
    fn test_verify_bad_entries() {
        let (store, _dir) = temp_store();
        store.conn.execute(
            "INSERT INTO knowledge_entries (content, kind, tags, metadata, weight, source, source_type, source_id, provenance_id) VALUES ('', 'pattern', '[\"test\"]', '{}', 1.0, 'user', 'file', 'test-id', 'prov-1')",
            [],
        ).unwrap();
        let bad = store.verify().unwrap();
        assert!(!bad.is_empty());
    }

    #[test]
    fn test_query_tag_mismatch() {
        let (store, _dir) = temp_store();
        insert_entry(&store, "rust content", &["rust"]);
        let rows = store.query(&["python"], 10, "", "", "").unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn test_query_multiple_tag_match() {
        let (store, _dir) = temp_store();
        insert_entry(&store, "both tags", &["rust", "pattern"]);
        let rows = store.query(&["rust", "pattern"], 10, "", "", "").unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_query_provenance_no_match() {
        let (store, _dir) = temp_store();
        insert_entry(&store, "content", &["test"]);
        let rows = store.query(&[], 10, "wrong-prov", "", "").unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn test_query_source_doc_no_match() {
        let (store, _dir) = temp_store();
        insert_entry(&store, "content", &["test"]);
        let rows = store.query(&[], 10, "", "", "wrong-doc.md").unwrap();
        assert!(rows.is_empty());
    }
}
