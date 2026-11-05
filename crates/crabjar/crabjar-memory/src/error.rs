//! Error types for the memory system.

use std::fmt;

/// Result type for memory operations.
pub type MemoryResult<T> = Result<T, MemoryError>;

/// Errors that can occur during memory operations.
#[derive(Debug)]
pub enum MemoryError {
    /// File I/O error.
    Io(std::io::Error),
    /// Memory store is full.
    Full {
        current_chars: usize,
        max_chars: usize,
        would_exceed_by: usize,
        closest_entries: Vec<String>,
    },
    /// Substring not found in any entry.
    NotFound {
        old_text: String,
        closest_entries: Vec<String>,
    },
    /// Substring matches multiple entries (ambiguous).
    Ambiguous {
        old_text: String,
        matching_count: usize,
    },
    /// Invalid operation parameters.
    InvalidParams(String),
    /// Duplicate entry detected.
    Duplicate(String),
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::Io(e) => write!(f, "I/O error: {e}"),
            MemoryError::Full {
                current_chars,
                max_chars,
                would_exceed_by,
                ..
            } => {
                write!(
                    f,
                    "Memory full: {current_chars} chars used of {max_chars} limit. Adding this entry would exceed by {would_exceed_by} characters."
                )
            }
            MemoryError::NotFound { old_text, .. } => {
                write!(f, "Entry not found containing: {old_text}")
            }
            MemoryError::Ambiguous {
                old_text,
                matching_count,
            } => {
                write!(
                    f,
                    "Substring '{old_text}' matches {matching_count} entries (ambiguous). Use a more specific substring."
                )
            }
            MemoryError::InvalidParams(msg) => write!(f, "Invalid parameters: {msg}"),
            MemoryError::Duplicate(entry) => write!(f, "Duplicate entry: {entry}"),
        }
    }
}

impl std::error::Error for MemoryError {}

impl From<std::io::Error> for MemoryError {
    fn from(e: std::io::Error) -> Self {
        MemoryError::Io(e)
    }
}
