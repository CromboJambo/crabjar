// crabjar/memory/src/state_docs/indexer.rs
// High-level indexing orchestration — reads files, delegates to extract/insert.

use crate::state_docs::models::{Annotation};
use std::fs;
use std::path::Path;

use super::extract::*;
use super::insert::*;

// Re-export extraction functions for test compatibility (tests import via indexer module path)
pub use super::extract::{extract_metadata, extract_sections, extract_tables, extract_code_blocks};

/// Test-only wrappers — tests reference these by name via the indexer module path.
pub fn extract_metadata_for_test(content: &str) -> crate::state_docs::models::DocMetadata {
    extract_metadata(content)
}

pub fn extract_sections_for_test(content: &str) -> Vec<crate::state_docs::models::Section> {
    extract_sections(content)
}

pub fn extract_code_blocks_for_test(
    content: &str,
) -> Vec<crate::state_docs::models::CodeBlock> {
    extract_code_blocks(content)
}

pub fn extract_tables_for_test(content: &str) -> Vec<crate::state_docs::models::Table> {
    extract_tables(content)
}

/// Test wrapper for `index_all_docs` — tests import via indexer module path.
pub fn index_all_docs_for_test(
    conn: &rusqlite::Connection,
    docs_dir: &std::path::Path,
) -> Result<usize, crate::Error> {
    // Ensure schema is migrated before indexing (in-memory connections may not be pre-migrated)
    super::schema::migrate(conn).ok();
    index_all_docs(conn, docs_dir)
}

/// Test wrapper for `extract_confidence` — tests import via indexer module path.
pub fn extract_confidence_for_test(
    content: &str,
) -> Option<crate::state_docs::models::ConfidenceAssessment> {
    extract_confidence(content)
}

/// Index a single state-doc Markdown file into SQLite
pub fn index_doc(
    conn: &rusqlite::Connection,
    doc_path: &Path,
    overlay_entries: &[Annotation],
) -> Result<(), crate::Error> {
    let content = fs::read_to_string(doc_path)?;
    let metadata = extract_metadata(&content);
    let sections = extract_sections(&content);
    let tables = extract_tables(&content);
    let code_blocks = extract_code_blocks(&content);
    let confidence = extract_confidence(&content);

    // Insert doc metadata
    insert_doc_metadata(conn, doc_path, &metadata)?;

    // Insert sections
    for section in &sections {
        insert_section(conn, doc_path, section)?;
    }

    // Insert tables
    for table in &tables {
        insert_table(conn, doc_path, table)?;
    }

    // Insert code blocks
    for block in &code_blocks {
        insert_code_block(conn, doc_path, block)?;
    }

    // Insert confidence assessment
    if let Some(conf) = confidence {
        insert_confidence(conn, doc_path, &conf)?;
    }

    // Insert annotations linked by line
    for annotation in overlay_entries {
        insert_annotation(conn, doc_path, annotation)?;
    }

    Ok(())
}

/// Index all state-docs in a directory into SQLite
pub fn index_all_docs(conn: &rusqlite::Connection, docs_dir: &Path) -> Result<usize, crate::Error> {
    if !docs_dir.exists() {
        return Ok(0);
    }

    let mut count = 0;
    for entry in fs::read_dir(docs_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        if file_name == "README.md" {
            continue;
        }

        // Load overlay annotations for this doc
        let overlay_path = docs_dir
            .parent()
            .unwrap_or(docs_dir)
            .join("overlay")
            .join(format!(
                "{}.overlay.json",
                file_name.trim_end_matches(".md")
            ));

        let overlay_entries = if overlay_path.exists() {
            load_overlay(&overlay_path)?
        } else {
            Vec::new()
        };

        index_doc(conn, &path, &overlay_entries)?;
        count += 1;
    }

    Ok(count)
}

/// Load overlay annotations from JSON file
fn load_overlay(path: &Path) -> Result<Vec<Annotation>, crate::Error> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(crate::Error::Json)
}
