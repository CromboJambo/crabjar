//! File indexer — git-aware traversal with BM25 tokenization.
//!
//! Uses the `ignore` crate for .gitignore-aware file discovery and tantivy's tokenizer
//! for efficient full-text indexing of file contents.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use ignore::WalkBuilder;
use tantivy::tokenizer::{LowerCaser, SimpleTokenizer, TextAnalyzer};
use tracing::{debug, info, warn};

use crate::DEFAULT_EXTENSIONS;

/// Configuration for the file indexer.
#[derive(Debug, Clone)]
pub struct IndexConfig {
    /// Root directory to index (required).
    pub root: PathBuf,
    /// File extensions to include (empty = all files).
    pub extensions: Vec<String>,
    /// Patterns to ignore (merged with .gitignore/.ignore).
    pub ignore_patterns: Vec<String>,
    /// Maximum file size to index in bytes (default 1MB).
    pub max_file_size: u64,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            extensions: DEFAULT_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
            ignore_patterns: Vec::new(),
            max_file_size: 1_048_576, // 1MB
        }
    }
}

/// Metadata about an indexed file.
#[derive(Debug, Clone)]
pub struct IndexedFile {
    /// Absolute path to the file.
    pub path: PathBuf,
    /// Relative path from root.
    pub relative_path: String,
    /// File size in bytes.
    pub size: u64,
    /// Last modification time as Unix timestamp.
    pub mtime: u64,
    /// File extension (without dot).
    pub extension: String,
}

impl IndexedFile {
    fn new(path: &Path, root: &Path) -> Option<Self> {
        let absolute = path.canonicalize().ok()?;
        let relative = absolute.strip_prefix(root).ok()?.to_string_lossy().into_owned();
        let metadata = std::fs::metadata(&absolute).ok()?;

        if !metadata.is_file() {
            return None;
        }

        let size = metadata.len();
        let mtime = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let extension = absolute
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        Some(Self {
            path: absolute,
            relative_path: relative,
            size,
            mtime,
            extension,
        })
    }
}

/// File indexer that discovers and indexes files using BM25 tokenization.
pub struct FileIndexer {
    config: IndexConfig,
}

impl FileIndexer {
    /// Create a new file indexer with the given configuration.
    pub fn new(config: IndexConfig) -> Self {
        Self { config }
    }

    /// Discover files in the configured root directory.
    /// Returns all files that match the extension filter and size limit.
    pub fn discover_files(&self) -> Result<Vec<IndexedFile>, String> {
        let mut files = Vec::new();
        let extensions: std::collections::HashSet<&str> =
            self.config.extensions.iter().map(|s| s.as_str()).collect();

        // Build ignore patterns from config + .gitignore/.ignore
        let mut walk_builder = WalkBuilder::new(&self.config.root);
        walk_builder
            .hidden(true) // We handle hidden files ourselves
            .require_git(false) // Let ignore crate handle it
            .standard_filters(true) // Use standard gitignore patterns
            .parents(true);

        // Add custom ignore patterns
        for pattern in &self.config.ignore_patterns {
            walk_builder.add_ignore(pattern.as_str());
        }

        let walker = walk_builder.build();

        for entry_result in walker {
            match entry_result {
                Ok(entry) => {
                    if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                        continue;
                    }

                    // Check extension filter
                    if !extensions.is_empty() {
                        let ext = entry
                            .path()
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        if !extensions.contains(ext.as_str()) {
                            continue;
                        }
                    }

                    // Check file size
                    if let Ok(metadata) = entry.metadata()
                        && metadata.len() > self.config.max_file_size {
                        debug!(
                            path = ?entry.path(),
                            size = metadata.len(),
                            max = self.config.max_file_size,
                            "Skipping oversized file"
                        );
                        continue;
                    }

                    // Create IndexedFile
                    if let Some(file) = IndexedFile::new(entry.path(), &self.config.root) {
                        files.push(file);
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Error walking directory");
                }
            }
        }

        info!(count = files.len(), root = ?self.config.root, "File discovery complete");
        Ok(files)
    }

    /// Tokenize file content for BM25 indexing.
    pub fn tokenize(content: &str) -> Vec<String> {
        let mut tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(LowerCaser)
            .build();

        // tantivy 0.22 API: token_stream().process() takes a closure receiving &Token
        let mut tokens = Vec::new();
        tokenizer.token_stream(content).process(&mut |token| {
            if token.text.len() >= 3 {
                tokens.push(token.text.clone());
            }
        });

        tokens
    }

    /// Index a single file's content.
    pub fn index_content(&self, path: &Path, content: &str) -> Result<Vec<String>, String> {
        let tokens = Self::tokenize(content);
        debug!(path = ?path, token_count = tokens.len(), "Tokenized file");
        Ok(tokens)
    }

    /// Get the root directory being indexed.
    pub fn root(&self) -> &Path {
        &self.config.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_tokenize_basic() {
        let content = "Hello world! This is a test of the BM25 tokenizer.";
        let tokens = FileIndexer::tokenize(content);

        assert!(!tokens.is_empty());
        // Should contain lowercase words >= 3 chars
        assert!(tokens.iter().any(|t| t == "hello"));
        assert!(tokens.iter().any(|t| t == "world"));
        assert!(tokens.iter().any(|t| t == "test"));
        assert!(tokens.iter().any(|t| t == "bm25"));
    }

    #[test]
    fn test_tokenize_filters_short_tokens() {
        let content = "a ab abc def";
        let tokens = FileIndexer::tokenize(content);

        // Should filter out "a" and "ab" (less than 3 chars)
        assert!(!tokens.contains(&String::from("a")));
        assert!(!tokens.contains(&String::from("ab")));
        assert!(tokens.contains(&String::from("abc")));
    }

    #[test]
    fn test_indexed_file_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "fn main() {}").unwrap();

        let file = IndexedFile::new(&test_file, temp_dir.path()).unwrap();

        assert_eq!(file.extension, "rs");
        assert_eq!(file.size, 12); // "fn main() {}\n" or similar
    }
}
