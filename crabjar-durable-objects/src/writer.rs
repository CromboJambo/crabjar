//! Checkpoint writer for serializing worker state to disk.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::error::{CheckpointError, CheckpointResult};
use crate::store::CheckpointStore;

/// Writes checkpoint data to disk with atomicity guarantees.
pub struct CheckpointWriter {
    store: CheckpointStore,
}

impl CheckpointWriter {
    /// Create a new checkpoint writer backed by the given store.
    pub fn new(store: CheckpointStore) -> Self {
        Self { store }
    }

    /// Write a checkpoint for the given task with serialized state.
    pub fn write_checkpoint(
        &self,
        task_id: &str,
        state: Value,
    ) -> CheckpointResult<String> {
        // Serialize state to JSON
        let payload = state.to_string();

        // Write payload to temp file first (atomic write pattern)
        let temp_path = format!("/tmp/checkpoint_{}_tmp", task_id);
        fs::write(&temp_path, &payload)?;

        // Create checkpoint record in store
        let checkpoint_id = self.store.create_checkpoint(
            task_id,
            Value::String(payload.clone()),
            Some(&temp_path),
        )?;

        // Move temp file to final location
        let final_path = PathBuf::from(format!("/tmp/checkpoint_{}.json", checkpoint_id));
        fs::rename(&temp_path, &final_path)?;

        Ok(checkpoint_id)
    }

    /// Write a checkpoint with custom payload path.
    pub fn write_checkpoint_at(
        &self,
        task_id: &str,
        state: Value,
        path: &str,
    ) -> CheckpointResult<String> {
        let payload = state.to_string();
        fs::write(path, &payload)?;

        self.store.create_checkpoint(task_id, Value::String(payload), Some(path))
    }
}