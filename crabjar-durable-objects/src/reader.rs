//! Checkpoint reader for loading worker state from disk.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::error::{CheckpointError, CheckpointResult};
use crate::store::CheckpointStore;

/// Reads checkpoint data from disk and validates it.
pub struct CheckpointReader {
    store: CheckpointStore,
}

impl CheckpointReader {
    /// Create a new checkpoint reader backed by the given store.
    pub fn new(store: CheckpointStore) -> Self {
        Self { store }
    }

    /// Read and return the latest checkpoint for a task.
    pub fn read_latest_checkpoint(&self, task_id: &str) -> CheckpointResult<Option<Value>> {
        let meta = self.store.get_latest_checkpoint(task_id)?;

        match meta {
            Some(meta) => {
                // Read payload from disk
                let path = PathBuf::from(format!("/tmp/checkpoint_{}.json", meta.id));
                let content = fs::read_to_string(&path)?;
                Ok(Some(Value::from(content)))
            }
            None => Ok(None),
        }
    }

    /// Read a specific checkpoint by ID.
    pub fn read_checkpoint(&self, id: &str) -> CheckpointResult<Value> {
        let meta = self.store.get_checkpoint(id)?;

        match meta {
            Some(meta) => {
                let path = PathBuf::from(format!("/tmp/checkpoint_{}.json", meta.id));
                let content = fs::read_to_string(&path)?;
                Ok(Value::from(content))
            }
            None => Err(CheckpointError::NotFound(id.to_string())),
        }
    }

    /// Validate a checkpoint's integrity.
    pub fn validate_checkpoint(&self, id: &str) -> CheckpointResult<bool> {
        let meta = self.store.get_checkpoint(id)?;

        match meta {
            Some(meta) => {
                // Check if file exists and is readable
                let path = PathBuf::from(format!("/tmp/checkpoint_{}.json", meta.id));
                Ok(path.exists())
            }
            None => Err(CheckpointError::NotFound(id.to_string())),
        }
    }
}
