//! Storage layer — SQLite-backed file metadata + tantivy BM25 index on disk.
//!
//! Two-tier storage:
//! 1. SQLite for file metadata (path, size, mtime, extension) and incremental change detection
//! 2. Tantivy index on disk for fast BM25 full-text search

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term};
use tracing::{debug, warn};

use crate::indexer::{IndexedFile, FileIndexer};

/// SQLite schema version. Bump this when the schema changes.
const SCHEMA_VERSION: i32 = 1;

/// Tantivy index directory name.
const TANTIVY_INDEX_DIR: &str = ".crabjar_search_index";

/// Storage backend for file search metadata and indexing.
pub struct SearchStorage {
    /// SQLite connection for file metadata.
    db_conn: Connection,
    /// Path to the tantivy index on disk.
    index_path: PathBuf,
    /// Tantivy index reader (for queries).
    index_reader: IndexReader,
    /// Tantivy index writer (for writes).
    index_writer: IndexWriter,
    /// Schema for field lookups.
    schema: Schema,
}

impl SearchStorage {
    /// Open or create a search storage at the given directory.
    pub fn open(root_dir: &Path) -> Result<Self, String> {
        let index_path = root_dir.join(TANTIVY_INDEX_DIR);

        // Ensure index directory exists
        fs::create_dir_all(&index_path).map_err(|e| format!("Failed to create index dir: {}", e))?;

        // Open SQLite database in the same directory
        let db_path = root_dir.join(".crabjar_search.db");
        let db_conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open search DB: {}", e))?;

        // Initialize schema
        Self::init_schema(&db_conn)?;

        // Open or create tantivy index
        let (index, index_writer) = Self::open_tantivy_index(&index_path)?;

        // Create index reader for queries
        let index_reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| format!("Failed to create index reader: {}", e))?;

        Ok(Self {
            db_conn,
            index_path: index_path.clone(),
            index_reader,
            index_writer,
            schema: index.schema().clone(),
        })
    }

    /// Initialize the SQLite schema.
    fn init_schema(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS files (
                path TEXT PRIMARY KEY,
                relative_path TEXT NOT NULL,
                size INTEGER NOT NULL,
                mtime INTEGER NOT NULL,
                extension TEXT NOT NULL,
                indexed_at INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_files_mtime ON files(mtime);
            CREATE INDEX IF NOT EXISTS idx_files_extension ON files(extension);

            -- Schema version tracking
            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )
        .map_err(|e| format!("Failed to create schema: {}", e))?;

        // Check schema version
        let version: Option<i32> = conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to read schema version: {}", e))?;

        if version != Some(SCHEMA_VERSION) {
            // Schema needs migration or fresh init
            conn.execute(
                "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', ?)",
                params![SCHEMA_VERSION],
            )
            .map_err(|e| format!("Failed to update schema version: {}", e))?;
        }

        Ok(())
    }

    /// Open or create a tantivy index.
    fn open_tantivy_index(path: &Path) -> Result<(Index, IndexWriter), String> {
        let schema = Self::create_schema();

        // Try to open existing index first
        if path.join("tantivy").exists() || Self::has_index_files(path) {
            debug!(path = ?path, "Opening existing tantivy index");
            let index = Index::open_in_dir(path).map_err(|e| format!("Failed to open tantivy index: {}", e))?;

            let writer = index.writer(50_000_000) // 50MB heap
                .map_err(|e| format!("Failed to create index writer: {}", e))?;

            Ok((index, writer))
        } else {
            // Create new index with schema
            debug!(path = ?path, "Creating new tantivy index");
            let index = Index::create_in_dir(path, schema.clone())
                .map_err(|e| format!("Failed to create tantivy index: {}", e))?;

            let writer = index.writer(50_000_000)
                .map_err(|e| format!("Failed to create index writer: {}", e))?;

            Ok((index, writer))
        }
    }

    /// Check if tantivy index files exist.
    fn has_index_files(path: &Path) -> bool {
        path.join("tantivy").exists() || fs::read_dir(path).ok().is_some_and(|mut entries| {
            entries.any(|e| {
                e.as_ref().ok().is_some_and(|entry| {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    name_str.starts_with("tantivy") || name_str == "meta" || name_str.ends_with(".json")
                })
            })
        })
    }

    /// Create the tantivy schema for file search.
    fn create_schema() -> Schema {
        let mut builder = Schema::builder();

        // File path (stored, searchable)
        builder.add_text_field("path", TEXT | STORED);

        // File content (tokenized for BM25)
        builder.add_text_field("content", TEXT);

        // Modification time (stored, indexed for sorting)
        builder.add_i64_field("mtime", INDEXED | STORED);

        // File size (stored)
        builder.add_u64_field("size", STORED);

        // File extension (stored, searchable)
        builder.add_text_field("extension", TEXT | STORED);

        builder.build()
    }

    /// Index a single file.
    pub fn index_file(&mut self, file: &IndexedFile, content: &str) -> Result<(), String> {
        // Tokenize content
        let tokens = FileIndexer::tokenize(content);
        let content_text = tokens.join(" ");

        // Get field handles from schema
        let path_field = self.schema.get_field("path").map_err(|e| format!("Missing 'path' field: {}", e))?;
        let content_field = self.schema.get_field("content").map_err(|e| format!("Missing 'content' field: {}", e))?;
        let mtime_field = self.schema.get_field("mtime").map_err(|e| format!("Missing 'mtime' field: {}", e))?;
        let size_field = self.schema.get_field("size").map_err(|e| format!("Missing 'size' field: {}", e))?;
        let extension_field = self.schema.get_field("extension").map_err(|e| format!("Missing 'extension' field: {}", e))?;

        // Add to tantivy index using doc! macro
        let doc = doc!(
            path_field => file.relative_path.clone(),
            content_field => content_text,
            mtime_field => file.mtime as i64,
            size_field => file.size,
            extension_field => file.extension.clone(),
        );

        self.index_writer
            .add_document(doc)
            .map_err(|e| format!("Failed to add document to index: {}", e))?;

        // Update SQLite metadata
        let indexed_at = Utc::now().timestamp() as u64;
        self.db_conn.execute(
            "INSERT OR REPLACE INTO files (path, relative_path, size, mtime, extension, indexed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                file.path.to_string_lossy(),
                file.relative_path.clone(),
                file.size,
                file.mtime,
                file.extension,
                indexed_at,
            ],
        )
        .map_err(|e| format!("Failed to update DB: {}", e))?;

        Ok(())
    }

    /// Remove a file from the index.
    pub fn remove_file(&mut self, path: &Path) -> Result<(), String> {
        let path_str = path.to_string_lossy().to_string();

        // Get field handle for path
        let path_field = self.schema.get_field("path").map_err(|e| format!("Missing 'path' field: {}", e))?;

        // Delete from tantivy (returns number of deleted documents)
        self.index_writer
            .delete_term(Term::from_field_text(path_field, &path_str));

        // Delete from SQLite
        self.db_conn.execute("DELETE FROM files WHERE path = ?1", params![&path_str])
            .map_err(|e| format!("Failed to remove from DB: {}", e))?;

        Ok(())
    }

    /// Commit pending changes to the tantivy index.
    pub fn commit(&mut self) -> Result<(), String> {
        self.index_writer.commit().map_err(|e| format!("Failed to commit index: {}", e))?;
        Ok(())
    }

    /// Search the index with a query string.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
        let searcher = self.index_reader.searcher();

        // Get field handles from schema
        let content_field = self.schema.get_field("content").map_err(|e| format!("Missing 'content' field: {}", e))?;
        let path_field = self.schema.get_field("path").map_err(|e| format!("Missing 'path' field: {}", e))?;
        let mtime_field = self.schema.get_field("mtime").map_err(|e| format!("Missing 'mtime' field: {}", e))?;
        let size_field = self.schema.get_field("size").map_err(|e| format!("Missing 'size' field: {}", e))?;
        let extension_field = self.schema.get_field("extension").map_err(|e| format!("Missing 'extension' field: {}", e))?;

        // Parse the query using tantivy's query parser
        let query_parser = QueryParser::for_index(self.index_reader.searcher().index(), vec![content_field]);

        let parsed_query = query_parser.parse_query(query).map_err(|e| format!("Failed to parse query: {}", e))?;

        // Execute search
        let top_docs = searcher.search(&parsed_query, &TopDocs::with_limit(limit)).map_err(|e| format!("Search failed: {}", e))?;

        // Convert results to SearchResult
        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address).map_err(|e| format!("Failed to retrieve document: {}", e))?;

            // Extract fields from the document
            let path = Self::get_text_field(&retrieved_doc, &self.schema, path_field)?;
            let mtime = Self::get_i64_field(&retrieved_doc, &self.schema, mtime_field).unwrap_or(0);
            let size = Self::get_u64_field(&retrieved_doc, &self.schema, size_field).unwrap_or(0);
            let extension = Self::get_text_field(&retrieved_doc, &self.schema, extension_field).unwrap_or_default();

            results.push(SearchResult {
                path,
                score: _score,
                mtime: mtime as u64,
                size,
                extension,
            });
        }

        Ok(results)
    }

    /// Get all indexed files from SQLite.
    pub fn list_indexed_files(&self) -> Result<Vec<IndexedFile>, String> {
        let mut stmt = self.db_conn.prepare("SELECT path, relative_path, size, mtime, extension FROM files")
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let file_iter = stmt.query_map([], |row| {
            let path_str: String = row.get(0)?;
            Ok(IndexedFile {
                path: PathBuf::from(path_str),
                relative_path: row.get(1)?,
                size: row.get(2)?,
                mtime: row.get(3)?,
                extension: row.get(4)?,
            })
        }).map_err(|e| format!("Failed to execute query: {}", e))?;

        let mut files = Vec::new();
        for file_result in file_iter {
            match file_result {
                Ok(file) => files.push(file),
                Err(e) => warn!(error = %e, "Error reading indexed file"),
            }
        }

        Ok(files)
    }

    /// Get the count of indexed files.
    pub fn index_count(&self) -> Result<usize, String> {
        let count: i64 = self.db_conn.query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
            .map_err(|e| format!("Failed to get count: {}", e))?;

        Ok(count as usize)
    }

    /// Check if a file is indexed.
    pub fn is_indexed(&self, path: &Path) -> Result<bool, String> {
        let exists: bool = self.db_conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM files WHERE path = ?1)",
            params![path.to_string_lossy()],
            |row| row.get(0),
        ).map_err(|e| format!("Failed to check index status: {}", e))?;

        Ok(exists)
    }

    /// Clear all indexed data.
    pub fn clear(&mut self) -> Result<(), String> {
        // Delete all documents from tantivy index
        if let Err(e) = self.index_writer.delete_all_documents() {
            return Err(format!("Failed to delete all documents: {}", e));
        }

        // Clear SQLite
        self.db_conn.execute("DELETE FROM files", [])
            .map_err(|e| format!("Failed to clear DB: {}", e))?;

        Ok(())
    }

    /// Get the path to the index directory.
    pub fn index_path(&self) -> &Path {
        &self.index_path
    }

    /// Reload the index reader to see newly committed documents.
    pub fn reload(&mut self) -> Result<(), String> {
        self.index_reader.reload().map_err(|e| format!("Failed to reload index: {}", e))?;
        Ok(())
    }

    /// Helper to extract a text field from a document.
    fn get_text_field(doc: &TantivyDocument, _schema: &Schema, field: Field) -> Result<String, String> {
        let values = doc.get_first(field).ok_or_else(|| "Missing field".to_string())?;
        match values {
            OwnedValue::Str(s) => Ok(s.clone()),
            _ => Err(format!("Expected text field, got {:?}", values)),
        }
    }

    /// Helper to extract an i64 field from a document.
    fn get_i64_field(doc: &TantivyDocument, _schema: &Schema, field: Field) -> Option<i64> {
        let values = doc.get_first(field)?;
        match values {
            OwnedValue::I64(v) => Some(*v),
            _ => None,
        }
    }

    /// Helper to extract a u64 field from a document.
    fn get_u64_field(doc: &TantivyDocument, _schema: &Schema, field: Field) -> Option<u64> {
        let values = doc.get_first(field)?;
        match values {
            OwnedValue::U64(v) => Some(*v),
            _ => None,
        }
    }
}

/// Result of a file search query.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Relative path to the matched file.
    pub path: String,
    /// BM25 relevance score (higher = more relevant).
    pub score: f32,
    /// File modification time (Unix timestamp).
    pub mtime: u64,
    /// File size in bytes.
    pub size: u64,
    /// File extension.
    pub extension: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_create_and_open() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create storage (opens or creates)
        let storage = SearchStorage::open(temp_dir.path()).unwrap();

        assert_eq!(storage.index_count().unwrap(), 0);
    }

    #[test]
    fn test_index_and_search() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut storage = SearchStorage::open(temp_dir.path()).unwrap();

        // Create a test file
        let test_file = IndexedFile {
            path: PathBuf::from("/tmp/test.rs"),
            relative_path: "test.rs".to_string(),
            size: 100,
            mtime: 1234567890,
            extension: "rs".to_string(),
        };

        // Index the file
        storage.index_file(&test_file, "fn main() { println!(\"hello world\"); }").unwrap();
        storage.commit().unwrap();
        storage.reload().unwrap();

        // Verify it's indexed
        assert_eq!(storage.index_count().unwrap(), 1);
        assert!(storage.is_indexed(Path::new("/tmp/test.rs")).unwrap());

        // Search for content
        let results = storage.search("hello", 10).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].path, "test.rs");
    }

    #[test]
    fn test_remove_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut storage = SearchStorage::open(temp_dir.path()).unwrap();

        let test_file = IndexedFile {
            path: PathBuf::from("/tmp/remove_test.rs"),
            relative_path: "remove_test.rs".to_string(),
            size: 50,
            mtime: 1234567890,
            extension: "rs".to_string(),
        };

        storage.index_file(&test_file, "fn test() {}").unwrap();
        storage.commit().unwrap();

        // Remove the file
        storage.remove_file(Path::new("/tmp/remove_test.rs")).unwrap();
        storage.commit().unwrap();

        assert_eq!(storage.index_count().unwrap(), 0);
    }
}
