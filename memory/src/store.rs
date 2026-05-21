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
        let sql = match (provenance_id.is_empty(), source_id.is_empty(), source_doc.is_empty()) {
            (false, false, false) => "SELECT id, content, tags, metadata, active FROM knowledge_entries WHERE active = 1 AND provenance_id = ? AND source_id = ? AND metadata->>'$.source_doc' = ?",
            (true, false, false) => "SELECT id, content, tags, metadata, active FROM knowledge_entries WHERE active = 1 AND source_id = ? AND metadata->>'$.source_doc' = ?",
            (false, true, false) => "SELECT id, content, tags, metadata, active FROM knowledge_entries WHERE active = 1 AND provenance_id = ? AND metadata->>'$.source_doc' = ?",
            (false, false, true) => "SELECT id, content, tags, metadata, active FROM knowledge_entries WHERE active = 1 AND provenance_id = ? AND source_id = ?",
            (true, false, true) => "SELECT id, content, tags, metadata, active FROM knowledge_entries WHERE active = 1 AND source_id = ?",
            (false, true, true) => "SELECT id, content, tags, metadata, active FROM knowledge_entries WHERE active = 1 AND provenance_id = ?",
            (true, true, true) => "SELECT id, content, tags, metadata, active FROM knowledge_entries WHERE active = 1",
        };
        let mut stmt = self.conn.prepare(sql)?;

        let cursor = match (provenance_id.is_empty(), source_id.is_empty(), source_doc.is_empty()) {
            (false, false, false) => stmt.query_map(rusqlite::params![provenance_id, source_id, source_doc], raw_knowledge_row)?,
            (true, false, false) => stmt.query_map(rusqlite::params![source_id, source_doc], raw_knowledge_row)?,
            (false, true, false) => stmt.query_map(rusqlite::params![provenance_id, source_doc], raw_knowledge_row)?,
            (false, false, true) => stmt.query_map(rusqlite::params![provenance_id, source_id], raw_knowledge_row)?,
            (true, false, true) => stmt.query_map(rusqlite::params![source_id], raw_knowledge_row)?,
            (false, true, true) => stmt.query_map(rusqlite::params![provenance_id], raw_knowledge_row)?,
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

    pub fn find_active_by_provenance(
        &self,
        source_type: &str,
        provenance_id: &str,
    ) -> StoreResult<Option<KnowledgeRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, metadata, active FROM knowledge_entries WHERE active = 1 AND source_type = ? AND source_id = ?",
        )?;

        let mut cursor = stmt.query_map(
            rusqlite::params![source_type, provenance_id],
            raw_knowledge_row,
        )?;

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
            Ok(EventRow {
                id: row.get(0)?,
                event_type: row.get(1)?,
                timestamp: chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(&ts_str)
                    .unwrap()
                    .to_utc(),
            })
        })?;

        for row in cursor {
            rows.push(row?);
        }

        Ok(rows)
    }

    pub fn deactivate_by_provenance(
        &self,
        source_type: &str,
        provenance_id: &str,
        source: Source,
        reason: Option<&str>,
    ) -> StoreResult<usize> {
        let _source = serde_json::to_string(&source)?;
        let _reason = reason.unwrap_or("");
        let ids = self.matching_ids_by_index(source_type, provenance_id)?;
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
        let ids = self.matching_ids_by_index("", provenance_id)?;
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

    fn matching_ids_by_index(
        &self,
        source_type: &str,
        provenance_id: &str,
    ) -> StoreResult<Vec<i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM knowledge_entries WHERE active = 1 AND source_type = ? AND source_id = ?",
        )?;
        let cursor = stmt.query_map(rusqlite::params![source_type, provenance_id], |row| {
            row.get(0)
        })?;
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
