//! Hermes-compatible memory tool (P2-1).
//!
//! Reads/writes MEMORY.md and USER.md files in ~/.hermes/memories/ using
//! the § delimiter format that Hermes Agent uses for persistent memory.

use std::fs;
use std::path::PathBuf;

/// Character budgets matching Hermes defaults
const MEMORY_MAX_CHARS: usize = 2200;
const USER_MAX_CHARS: usize = 1375;

/// Entry delimiter used by Hermes memory files
const ENTRY_DELIMITER: &str = "\n§\n";

#[derive(Debug, Clone)]
pub struct MemoryStore {
    memories_dir: PathBuf,
}

impl MemoryStore {
    pub fn new() -> Result<Self, String> {
        let home = dirs::home_dir().ok_or_else(|| "could not determine home directory".to_string())?;
        Ok(Self {
            memories_dir: home.join(".hermes").join("memories"),
        })
    }

    fn file_path(&self, target: &str) -> PathBuf {
        let filename = match target {
            "user" => "USER.md",
            _ => "MEMORY.md",
        };
        self.memories_dir.join(filename)
    }

    fn max_chars(&self, target: &str) -> usize {
        if target == "user" { USER_MAX_CHARS } else { MEMORY_MAX_CHARS }
    }

    /// Read all entries from a memory file
    pub fn read(&self, target: &str) -> Result<String, String> {
        let path = self.file_path(target);
        if !path.exists() {
            return Ok(String::new());
        }
        fs::read_to_string(&path).map_err(|e| format!("failed to read {}: {}", path.display(), e))
    }

    /// Add a new entry to memory, checking budget
    pub fn add_entry(&self, target: &str, content: &str) -> Result<MemoryOperationResult, String> {
        let current = self.read(target)?;
        let max = self.max_chars(target);

        // Calculate new size (account for delimiter if not first entry)
        let delimiter_size = if current.is_empty() { 0 } else { ENTRY_DELIMITER.chars().count() };
        let new_size = current.chars().count() + delimiter_size + content.chars().count();

        if new_size > max {
            return Err(format!(
                "memory full: would exceed {} char budget (current: {}, entry: {}, total would be: {})",
                max,
                current.chars().count(),
                content.chars().count(),
                new_size
            ));
        }

        let mut updated = current;
        if !updated.is_empty() {
            updated.push_str(ENTRY_DELIMITER);
        }
        updated.push_str(content);

        self.write(target, &updated)?;

        Ok(MemoryOperationResult {
            message: format!("Added entry to {} ({} chars used)", target, updated.chars().count()),
            current_chars: updated.chars().count(),
            max_chars: max,
            used_percent: ((updated.chars().count() as f64) / (max as f64)) * 100.0,
            removed: false,
        })
    }

    /// Patch an existing entry (replace old_text with new_text within one entry)
    pub fn patch_entry(&self, target: &str, old_text: &str, new_text: &str) -> Result<MemoryOperationResult, String> {
        let current = self.read(target)?;
        if !current.contains(old_text) {
            return Err(format!("old text not found in {} memory", target));
        }

        let updated = current.replace(old_text, new_text);
        let max = self.max_chars(target);

        if updated.chars().count() > max {
            return Err(format!(
                "patch would exceed {} char budget ({} chars)",
                max,
                updated.chars().count()
            ));
        }

        self.write(target, &updated)?;

        Ok(MemoryOperationResult {
            message: format!("Patched entry in {}", target),
            current_chars: updated.chars().count(),
            max_chars: max,
            used_percent: ((updated.chars().count() as f64) / (max as f64)) * 100.0,
            removed: false,
        })
    }

    /// Remove an entry by matching a substring within it
    pub fn remove_entry_by_substring(&self, target: &str, match_text: &str) -> Result<MemoryOperationResult, String> {
        let current = self.read(target)?;
        if current.is_empty() {
            return Err(format!("{} memory is empty", target));
        }

        // Split into entries and find the one containing match_text
        let entries: Vec<&str> = current.split(ENTRY_DELIMITER).collect();
        let mut found = false;
        let mut updated_entries: Vec<String> = Vec::new();

        for entry in entries {
            if entry.contains(match_text) {
                found = true;
            } else {
                updated_entries.push(entry.to_string());
            }
        }

        if !found {
            return Err(format!("no entry containing '{}' found in {} memory", match_text, target));
        }

        let updated = updated_entries.join(ENTRY_DELIMITER);
        self.write(target, &updated)?;

        Ok(MemoryOperationResult {
            message: format!("Removed entry from {}", target),
            current_chars: updated.chars().count(),
            max_chars: self.max_chars(target),
            used_percent: 0.0,
            removed: true,
        })
    }

    /// Write content to a memory file
    fn write(&self, target: &str, content: &str) -> Result<(), String> {
        let path = self.file_path(target);

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("failed to create dir {}: {}", parent.display(), e))?;
        }

        fs::write(&path, content).map_err(|e| format!("failed to write {}: {}", path.display(), e))
    }
}

#[derive(Debug)]
pub struct MemoryOperationResult {
    pub message: String,
    pub current_chars: usize,
    pub max_chars: usize,
    pub used_percent: f64,
    pub removed: bool,
}
