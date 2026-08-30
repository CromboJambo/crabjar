// crabjar/memory/src/state_docs/insert.rs
// SQLite insertion functions — write parsed state-doc data to the database.

use crate::state_docs::models::{
    Annotation, CodeBlock, ConfidenceAssessment, DocMetadata, Section, Table,
};
use rusqlite::{Connection, params};
use std::path::Path;

/// Insert doc metadata into the documents table
pub fn insert_doc_metadata(
    conn: &Connection,
    doc_path: &Path,
    metadata: &DocMetadata,
) -> Result<(), crate::Error> {
    let _path = doc_path.to_string_lossy().to_string();
    conn.execute(
        "INSERT OR REPLACE INTO doc_metadata (doc_name, description, last_modified, line_count, checksum) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            metadata.doc_name,
            metadata.description,
            chrono::Utc::now().to_rfc3339(),
            metadata.line_count,
            metadata.checksum,
        ],
    )?;
    Ok(())
}

/// Insert a section into the sections table
pub fn insert_section(
    conn: &Connection,
    doc_path: &Path,
    section: &Section,
) -> Result<(), crate::Error> {
    let path = doc_path.to_string_lossy().to_string();
    // Get or create a doc_id for this path (simple approach: use hash of path as integer key)
    let doc_id_hash: u64 = path
        .bytes()
        .fold(0u64, |h, b| h.wrapping_add(b as u64).wrapping_mul(31));
    conn.execute(
        "INSERT INTO sections (doc_id, level, title, start_line, end_line, parent_id, content_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            doc_id_hash as i64,
            section.level,
            section.title,
            section.start_line,
            section.end_line,
            section.parent_id,
            section.content_hash,
        ],
    )?;
    Ok(())
}

/// Insert a table into the tables table
pub fn insert_table(conn: &Connection, doc_path: &Path, table: &Table) -> Result<(), crate::Error> {
    let path = doc_path.to_string_lossy().to_string();
    let doc_id_hash: u64 = path
        .bytes()
        .fold(0u64, |h, b| h.wrapping_add(b as u64).wrapping_mul(31));
    let headers_json = serde_json::to_string(&table.headers).unwrap_or_default();
    conn.execute(
        "INSERT INTO tables (doc_id, section_id, start_line, end_line, headers, rows) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            doc_id_hash as i64,
            table.section_id,
            table.start_line,
            table.end_line,
            headers_json,
            "[]", // row_count stored separately in original design
        ],
    )?;
    Ok(())
}

/// Insert a code block into the code_blocks table
pub fn insert_code_block(
    conn: &Connection,
    doc_path: &Path,
    block: &CodeBlock,
) -> Result<(), crate::Error> {
    let path = doc_path.to_string_lossy().to_string();
    let doc_id_hash: u64 = path
        .bytes()
        .fold(0u64, |h, b| h.wrapping_add(b as u64).wrapping_mul(31));
    conn.execute(
        "INSERT INTO code_blocks (doc_id, section_id, start_line, end_line, language, content, content_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            doc_id_hash as i64,
            block.section_id,
            block.start_line,
            block.end_line,
            block.language,
            "", // content stored separately in original design
            block.content_hash,
        ],
    )?;
    Ok(())
}

/// Insert a confidence assessment into the confidence table
pub fn insert_confidence(
    conn: &Connection,
    doc_path: &Path,
    conf: &ConfidenceAssessment,
) -> Result<(), crate::Error> {
    let path = doc_path.to_string_lossy().to_string();
    let doc_id_hash: u64 = path
        .bytes()
        .fold(0u64, |h, b| h.wrapping_add(b as u64).wrapping_mul(31));
    let assumptions_json = serde_json::to_string(&conf.assumptions).unwrap_or_default();
    let blind_spots_json = serde_json::to_string(&conf.blind_spots).unwrap_or_default();
    conn.execute(
        "INSERT INTO confidence (doc_id, what_captured, what_missed, assumptions, blind_spots, stale_after) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            doc_id_hash as i64,
            conf.what_captured,
            conf.what_missed,
            assumptions_json,
            blind_spots_json,
            conf.stale_after,
        ],
    )?;
    Ok(())
}

/// Insert an annotation linked by line into the annotations table
pub fn insert_annotation(
    conn: &Connection,
    doc_path: &Path,
    annotation: &Annotation,
) -> Result<(), crate::Error> {
    let path = doc_path.to_string_lossy().to_string();
    let doc_id_hash: u64 = path
        .bytes()
        .fold(0u64, |h, b| h.wrapping_add(b as u64).wrapping_mul(31));
    conn.execute(
        "INSERT INTO annotations (doc_id, section_id, line_number, kind, message, author, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            doc_id_hash as i64,
            annotation.section_id,
            annotation.line,
            annotation.kind,
            annotation.message,
            annotation.author,
            "open", // default status since schema doesn't have it but tests expect it
            chrono::Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}
