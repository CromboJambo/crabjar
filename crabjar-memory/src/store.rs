//! File I/O for memory stores (MEMORY.md and USER.md).

use std::fs;
use std::path::PathBuf;

use super::capacity::MemoryCapacity;
use super::error::{MemoryError, MemoryResult};
use super::parser;

/// Target store identifier.
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryTarget {
    /// Agent notes (MEMORY.md).
    Memory,
    /// User profile (USER.md).
    User,
}

impl MemoryTarget {
    /// Get the filename for this target.
    pub fn filename(&self) -> &'static str {
        match self {
            MemoryTarget::Memory => "MEMORY.md",
            MemoryTarget::User => "USER.md",
        }
    }

    /// Create capacity tracker for this target.
    pub fn capacity(&self) -> MemoryCapacity {
        match self {
            MemoryTarget::Memory => MemoryCapacity::for_memory(),
            MemoryTarget::User => MemoryCapacity::for_user(),
        }
    }
}

/// Memory store that reads/writes Hermes-compatible memory files.
pub struct MemoryStore {
    /// Base directory for memory files (typically ~/.hermes/memories/).
    base_dir: PathBuf,
}

impl MemoryStore {
    /// Create a new memory store with the given base directory.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        MemoryStore {
            base_dir: base_dir.into(),
        }
    }

    /// Get the path to a memory file for the given target.
    fn file_path(&self, target: &MemoryTarget) -> PathBuf {
        self.base_dir.join(target.filename())
    }

    /// Validate that a path stays within the base directory (defense against path traversal).
    fn validate_path_within_base(&self, path: &PathBuf) -> MemoryResult<()> {
        // Canonicalize both paths to resolve symlinks and .. components
        let canonical_base = self.base_dir.canonicalize().map_err(|e| {
            MemoryError::Io(e)
        })?;

        let canonical_path = path.canonicalize().or_else(|_| {
            // File may not exist yet — create parent and try again
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).ok();
            }
            path.canonicalize()
        }).map_err(|e| {
            MemoryError::Io(e)
        })?;

        if !canonical_path.starts_with(&canonical_base) {
            return Err(MemoryError::InvalidParams(format!(
                "Path traversal detected: {} is outside base dir {}",
                path.display(),
                self.base_dir.display()
            )));
        }

        Ok(())
    }

    /// Read all entries from a memory store.
    pub fn read_entries(&self, target: &MemoryTarget) -> MemoryResult<Vec<String>> {
        let path = self.file_path(target);
        if !path.exists() {
            return Ok(vec![]);
        }

        let content = fs::read_to_string(&path)?;
        Ok(parser::parse_entries(&content))
    }

    /// Write entries to a memory store.
    pub fn write_entries(&self, target: &MemoryTarget, entries: &[String]) -> MemoryResult<()> {
        let path = self.file_path(target);

        // Validate path stays within base directory (redteam finding #17)
        self.validate_path_within_base(&path)?;

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = parser::serialize_entries(entries);
        fs::write(&path, &content)?;
        Ok(())
    }

    /// Add a new entry to the store.
    pub fn add_entry(&self, target: &MemoryTarget, entry: String) -> MemoryResult<()> {
        let mut entries = self.read_entries(target)?;

        // Check for duplicates
        if entries.iter().any(|e| e == &entry) {
            return Err(MemoryError::Duplicate(entry));
        }

        // Check capacity
        let mut cap = target.capacity();
        let current_content = if entries.is_empty() {
            String::new()
        } else {
            parser::serialize_entries(&entries)
        };
        cap.set_current(&current_content);

        // Calculate size of new entry with delimiter
        let delimiter = "\n§\n";
        let additional_size = if entries.is_empty() {
            entry.len() + 1 // Just the entry + newline
        } else {
            delimiter.len() + entry.len() + 1
        };

        if !cap.would_fit(&"X".repeat(additional_size)) {
            return Err(MemoryError::Full {
                current_chars: cap.current_chars,
                max_chars: cap.max_chars,
                would_exceed_by: additional_size - cap.remaining(),
                closest_entries: entries.clone(),
            });
        }

        entries.push(entry);
        self.write_entries(target, &entries)?;
        Ok(())
    }

    /// Patch an existing entry (replace substring within entry).
    pub fn patch_entry(
        &self,
        target: &MemoryTarget,
        old_text: &str,
        new_text: &str,
    ) -> MemoryResult<()> {
        let entries = self.read_entries(target)?;

        // Find entry containing old_text
        let matches = parser::find_entries_containing(&entries, old_text);
        if matches.is_empty() {
            return Err(MemoryError::NotFound {
                old_text: old_text.to_string(),
                closest_entries: entries.clone(),
            });
        }
        if matches.len() > 1 {
            return Err(MemoryError::Ambiguous {
                old_text: old_text.to_string(),
                matching_count: matches.len(),
            });
        }

        let idx = matches[0];
        let mut entries = entries;
        entries[idx] = entries[idx].replace(old_text, new_text);

        self.write_entries(target, &entries)?;
        Ok(())
    }

    /// Remove an entry by substring match.
    pub fn remove_entry(&self, target: &MemoryTarget, old_text: &str) -> MemoryResult<()> {
        let entries = self.read_entries(target)?;

        // Find entry containing old_text
        let matches = parser::find_entries_containing(&entries, old_text);
        if matches.is_empty() {
            return Err(MemoryError::NotFound {
                old_text: old_text.to_string(),
                closest_entries: entries.clone(),
            });
        }
        if matches.len() > 1 {
            return Err(MemoryError::Ambiguous {
                old_text: old_text.to_string(),
                matching_count: matches.len(),
            });
        }

        let idx = matches[0];
        let mut entries = entries;
        entries.remove(idx);

        self.write_entries(target, &entries)?;
        Ok(())
    }

    /// Get capacity info for a target.
    pub fn capacity_info(&self, target: &MemoryTarget) -> MemoryResult<MemoryCapacity> {
        let path = self.file_path(target);
        if !path.exists() {
            return Ok(target.capacity());
        }

        let content = fs::read_to_string(&path)?;
        let mut cap = target.capacity();
        cap.set_current(&content);
        Ok(cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::tempdir;

    #[test]
    fn test_add_and_read_entry() {
        let dir = tempdir().unwrap();
        let store = MemoryStore::new(dir.path());

        store
            .add_entry(&MemoryTarget::Memory, "Test entry".to_string())
            .unwrap();

        let entries = store.read_entries(&MemoryTarget::Memory).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], "Test entry");
    }

    #[test]
    fn test_patch_entry() {
        let dir = tempdir().unwrap();
        let store = MemoryStore::new(dir.path());

        store
            .add_entry(&MemoryTarget::Memory, "Old text here".to_string())
            .unwrap();

        store
            .patch_entry(&MemoryTarget::Memory, "Old", "New")
            .unwrap();

        let entries = store.read_entries(&MemoryTarget::Memory).unwrap();
        assert_eq!(entries[0], "New text here");
    }

    #[test]
    fn test_remove_entry() {
        let dir = tempdir().unwrap();
        let store = MemoryStore::new(dir.path());

        store
            .add_entry(&MemoryTarget::Memory, "Entry to remove".to_string())
            .unwrap();

        store
            .remove_entry(&MemoryTarget::Memory, "to remove")
            .unwrap();

        let entries = store.read_entries(&MemoryTarget::Memory).unwrap();
        assert!(entries.is_empty());
    }
}
