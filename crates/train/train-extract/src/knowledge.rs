use crate::error::{TrainExtractError, TrainExtractResult};
use crate::extract_types::KnowledgeEntry;
use agent_context::KnowledgeKind;
use rusqlite::Connection;

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

/// Extract knowledge entries from the knowledge store.
///
/// The canonical agent-context schema (memory/src/schema.rs) has a fixed set
/// of columns; newer stores may add `created_at`. We probe the actual table
/// with PRAGMA and build the SELECT to match, so both shapes work.
pub fn extract_knowledge_entries(
    conn: &Connection,
    tags: &[String],
    max_entries: usize,
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

    let rows = stmt.query_map([max_entries as i64], |row| {
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

    let entries: Vec<KnowledgeEntry> = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| TrainExtractError::Export(format!("failed to read knowledge entries: {e}")))?;

    // Apply tag filter if specified
    let filtered = if tags.is_empty() {
        entries
    } else {
        entries
            .into_iter()
            .filter(|entry| tags.iter().any(|tag| entry.tags.iter().any(|t| t == tag)))
            .collect()
    };

    tracing::debug!(
        entries_extracted = filtered.len(),
        "Extracted knowledge entries"
    );

    Ok(filtered)
}
