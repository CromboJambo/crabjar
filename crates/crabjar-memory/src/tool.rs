//! Hermes-compatible memory tool implementation.
//!
//! Provides the same tool schema and response format as Hermes Agent's
//! memory tool, enabling drop-in replacement for agents.
//!
//! Optional mirror-log integration provides provenance tracking and
//! attention-based decision filtering for memory operations.

use serde::{Deserialize, Serialize};

#[cfg(feature = "mirror-log")]
use std::sync::Arc;

use super::error::{MemoryError, MemoryResult};
use super::store::{MemoryStore, MemoryTarget};
#[cfg(feature = "mirror-log")]
use super::mirror_log::{
    log_batch_applied, log_entry_added, log_entry_patched, log_entry_removed, MemoryAuditLogger,
};

/// Tool operation types.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryAction {
    Add,
    Patch,
    Remove,
}

/// Single memory operation (for batch processing).
#[derive(Debug, Clone, Deserialize)]
pub struct MemoryOperation {
    pub action: MemoryAction,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub old_text: Option<String>,
}

/// Tool input parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct MemoryToolInput {
    /// Which store to modify: "memory" or "user".
    pub target: String,
    /// Single operation action (overridden by operations array).
    #[serde(default)]
    pub action: Option<MemoryAction>,
    /// New entry text or replacement content.
    #[serde(default)]
    pub content: Option<String>,
    /// Substring to find for patch/remove.
    #[serde(default)]
    pub old_text: Option<String>,
    /// Batch operations (takes precedence over single action).
    #[serde(default)]
    pub operations: Option<Vec<MemoryOperation>>,
}

/// Tool response.
#[derive(Debug, Clone, Serialize)]
pub struct MemoryToolResponse {
    pub success: bool,
    pub message: String,
    pub current_chars: usize,
    pub max_chars: usize,
    pub usage_percent: u32,
    pub entries_count: usize,
}

impl MemoryToolResponse {
    fn ok(message: &str, store: &MemoryStore, target: &MemoryTarget) -> MemoryResult<Self> {
        let cap = store.capacity_info(target)?;
        let entries = store.read_entries(target)?;
        Ok(MemoryToolResponse {
            success: true,
            message: message.to_string(),
            current_chars: cap.current_chars,
            max_chars: cap.max_chars,
            usage_percent: cap.usage_percent(),
            entries_count: entries.len(),
        })
    }

    fn err(error: &MemoryError, store: &MemoryStore, target: &MemoryTarget) -> MemoryResult<Self> {
        let cap = store.capacity_info(target)?;
        let entries = store.read_entries(target)?;
        Ok(MemoryToolResponse {
            success: false,
            message: error.to_string(),
            current_chars: cap.current_chars,
            max_chars: cap.max_chars,
            usage_percent: cap.usage_percent(),
            entries_count: entries.len(),
        })
    }
}

/// Hermes-compatible memory tool.
pub struct MemoryTool {
    store: MemoryStore,
    #[cfg(feature = "mirror-log")]
    audit_logger: Option<Arc<MemoryAuditLogger>>,
}

impl MemoryTool {
    /// Create a new memory tool with the given base directory.
    pub fn new(base_dir: impl Into<std::path::PathBuf>) -> Self {
        MemoryTool {
            store: MemoryStore::new(base_dir),
            #[cfg(feature = "mirror-log")]
            audit_logger: None,
        }
    }

    /// Attach an optional mirror-log audit logger for provenance tracking.
    #[cfg(feature = "mirror-log")]
    pub fn with_audit_logger(mut self, logger: Arc<MemoryAuditLogger>) -> Self {
        self.audit_logger = Some(logger);
        self
    }

    /// Parse target string into `MemoryTarget` enum.
    fn parse_target(target: &str) -> MemoryResult<MemoryTarget> {
        match target.to_lowercase().as_str() {
            "memory" => Ok(MemoryTarget::Memory),
            "user" => Ok(MemoryTarget::User),
            _ => Err(MemoryError::InvalidParams(format!(
                "Unknown target: {target}. Must be 'memory' or 'user'."
            ))),
        }
    }

    /// Execute a memory tool call.
    pub fn execute(&self, input: MemoryToolInput) -> MemoryResult<MemoryToolResponse> {
        let target = Self::parse_target(&input.target)?;

        // Handle batch operations
        if let Some(operations) = &input.operations {
            return self.execute_batch(&target, operations);
        }

        // Single operation
        match input.action {
            Some(MemoryAction::Add) => {
                let content = input.content.ok_or_else(|| {
                    MemoryError::InvalidParams("content is required for add action".to_string())
                })?;

                match self.store.add_entry(&target, content) {
                    Ok(()) => MemoryToolResponse::ok("Added memory entry", &self.store, &target),
                    Err(e) => MemoryToolResponse::err(&e, &self.store, &target),
                }
            }
            Some(MemoryAction::Patch) => {
                let content = input.content.ok_or_else(|| {
                    MemoryError::InvalidParams("content is required for patch action".to_string())
                })?;
                let old_text = input.old_text.ok_or_else(|| {
                    MemoryError::InvalidParams("old_text is required for patch action".to_string())
                })?;

                match self.store.patch_entry(&target, &old_text, &content) {
                    Ok(()) => MemoryToolResponse::ok("Patched memory entry", &self.store, &target),
                    Err(e) => MemoryToolResponse::err(&e, &self.store, &target),
                }
            }
            Some(MemoryAction::Remove) => {
                let old_text = input.old_text.ok_or_else(|| {
                    MemoryError::InvalidParams("old_text is required for remove action".to_string())
                })?;

                match self.store.remove_entry(&target, &old_text) {
                    Ok(()) => MemoryToolResponse::ok("Removed memory entry", &self.store, &target),
                    Err(e) => MemoryToolResponse::err(&e, &self.store, &target),
                }
            }
            None => Err(MemoryError::InvalidParams(
                "action is required when operations array is not provided".to_string(),
            )),
        }
    }

    /// Execute a memory tool call (async version that supports audit logging).
    #[cfg(feature = "mirror-log")]
    pub async fn execute_async(&self, input: MemoryToolInput) -> MemoryResult<MemoryToolResponse> {
        let target = Self::parse_target(&input.target)?;

        // Handle batch operations
        if let Some(operations) = &input.operations {
            return self.execute_batch_async(&target, operations).await;
        }

        // Single operation
        match input.action {
            Some(MemoryAction::Add) => {
                let content = input.content.ok_or_else(|| {
                    MemoryError::InvalidParams("content is required for add action".to_string())
                })?;

                match self.store.add_entry(&target, content.clone()) {
                    Ok(()) => {
                        if let Some(logger) = &self.audit_logger {
                            let _ = log_entry_added(logger, &input.target, &content).await;
                        }
                        MemoryToolResponse::ok("Added memory entry", &self.store, &target)
                    }
                    Err(e) => MemoryToolResponse::err(&e, &self.store, &target),
                }
            }
            Some(MemoryAction::Patch) => {
                let content = input.content.ok_or_else(|| {
                    MemoryError::InvalidParams("content is required for patch action".to_string())
                })?;
                let old_text = input.old_text.ok_or_else(|| {
                    MemoryError::InvalidParams("old_text is required for patch action".to_string())
                })?;

                match self.store.patch_entry(&target, &old_text, &content) {
                    Ok(()) => {
                        if let Some(logger) = &self.audit_logger {
                            let _ = log_entry_patched(logger, &input.target, &old_text, &content).await;
                        }
                        MemoryToolResponse::ok("Patched memory entry", &self.store, &target)
                    }
                    Err(e) => MemoryToolResponse::err(&e, &self.store, &target),
                }
            }
            Some(MemoryAction::Remove) => {
                let old_text = input.old_text.ok_or_else(|| {
                    MemoryError::InvalidParams("old_text is required for remove action".to_string())
                })?;

                match self.store.remove_entry(&target, &old_text) {
                    Ok(()) => {
                        if let Some(logger) = &self.audit_logger {
                            let _ = log_entry_removed(logger, &input.target, &old_text).await;
                        }
                        MemoryToolResponse::ok("Removed memory entry", &self.store, &target)
                    }
                    Err(e) => MemoryToolResponse::err(&e, &self.store, &target),
                }
            }
            None => Err(MemoryError::InvalidParams(
                "action is required when operations array is not provided".to_string(),
            )),
        }
    }

    /// Execute batch operations atomically.
    fn execute_batch(
        &self,
        target: &MemoryTarget,
        operations: &[MemoryOperation],
    ) -> MemoryResult<MemoryToolResponse> {
        // Read current state
        let mut entries = self.store.read_entries(target)?;

        // Apply all operations in memory first (atomic)
        for op in operations {
            match &op.action {
                MemoryAction::Add => {
                    if let Some(content) = &op.content
                        && !entries.iter().any(|e| e == content) {
                            entries.push(content.clone());
                        }
                }
                MemoryAction::Patch => {
                    if let (Some(old_text), Some(new_text)) = (&op.old_text, &op.content) {
                        for entry in &mut entries {
                            if entry.contains(old_text) {
                                *entry = entry.replace(old_text, new_text);
                                break;
                            }
                        }
                    }
                }
                MemoryAction::Remove => {
                    if let Some(old_text) = &op.old_text {
                        entries.retain(|e| !e.contains(old_text));
                    }
                }
            }
        }

        // Check capacity of final state
        let serialized = super::parser::serialize_entries(&entries);
        let cap = target.capacity();
        if serialized.chars().count() > cap.max_chars {
            return MemoryToolResponse::err(
                &MemoryError::Full {
                    current_chars: serialized.chars().count(),
                    max_chars: cap.max_chars,
                    would_exceed_by: serialized.chars().count() - cap.max_chars,
                    closest_entries: entries.clone(),
                },
                &self.store,
                target,
            );
        }

        // Write final state
        self.store.write_entries(target, &entries)?;

        MemoryToolResponse::ok(
            &format!("Applied {} operations", operations.len()),
            &self.store,
            target,
        )
    }

    /// Execute batch operations atomically (async version with audit logging).
    #[cfg(feature = "mirror-log")]
    async fn execute_batch_async(
        &self,
        target: &MemoryTarget,
        operations: &[MemoryOperation],
    ) -> MemoryResult<MemoryToolResponse> {
        // Read current state
        let mut entries = self.store.read_entries(target)?;

        // Apply all operations in memory first (atomic)
        for op in operations {
            match &op.action {
                MemoryAction::Add => {
                    if let Some(content) = &op.content {
                        if !entries.iter().any(|e| e == content) {
                            entries.push(content.clone());
                        }
                    }
                }
                MemoryAction::Patch => {
                    if let (Some(old_text), Some(new_text)) = (&op.old_text, &op.content) {
                        for entry in &mut entries {
                            if entry.contains(old_text) {
                                *entry = entry.replace(old_text, new_text);
                                break;
                            }
                        }
                    }
                }
                MemoryAction::Remove => {
                    if let Some(old_text) = &op.old_text {
                        entries.retain(|e| !e.contains(old_text));
                    }
                }
            }
        }

        // Check capacity of final state
        let serialized = super::parser::serialize_entries(&entries);
        let cap = target.capacity();
        if serialized.chars().count() > cap.max_chars {
            return MemoryToolResponse::err(
                &MemoryError::Full {
                    current_chars: serialized.chars().count(),
                    max_chars: cap.max_chars,
                    would_exceed_by: serialized.chars().count() - cap.max_chars,
                    closest_entries: entries.clone(),
                },
                &self.store,
                target,
            );
        }

        // Write final state
        self.store.write_entries(target, &entries)?;

        // Log batch operation
        if let Some(logger) = &self.audit_logger {
            let _ = log_batch_applied(logger, &target.filename(), operations.len()).await;
        }

        MemoryToolResponse::ok(
            &format!("Applied {} operations", operations.len()),
            &self.store,
            target,
        )
    }

    /// Get the tool schema for Hermes integration.
    #[must_use]
    pub fn schema() -> &'static str {
        r#"{
  "name": "memory",
  "description": "Save durable facts to persistent memory that survive across sessions. Memory is injected into every future turn, so keep entries compact and high-signal.",
  "parameters": {
    "type": "object",
    "properties": {
      "action": {
        "type": "string",
        "enum": ["add", "patch", "remove"],
        "description": "The action to perform (single-op shape). Omit when using 'operations'."
      },
      "target": {
        "type": "string",
        "enum": ["memory", "user"],
        "description": "Which memory store: 'memory' for personal notes, 'user' for user profile."
      },
      "content": {
        "type": "string",
        "description": "Required for 'add' and 'patch'. For 'add', the new entry. For 'patch', the text that replaces old_text; the rest of the entry is kept."
      },
      "old_text": {
        "type": "string",
        "description": "REQUIRED for 'patch' and 'remove': text from one entry. 'patch' replaces exactly this span (it must occur once in the entry); 'remove' deletes the whole entry."
      },
      "operations": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "action": { "type": "string" },
            "content": { "type": "string" },
            "old_text": { "type": "string" }
          },
          "required": ["action"]
        },
        "description": "Batch shape: a list of operations applied atomically in one call against the final char budget."
      }
    },
    "required": ["target"]
  }
}"#
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_add_entry() {
        let dir = tempdir().unwrap();
        let tool = MemoryTool::new(dir.path());

        let input: MemoryToolInput = serde_json::from_str(
            r#"{"target": "memory", "action": "add", "content": "Test entry"}"#,
        )
        .unwrap();

        let response = tool.execute(input).unwrap();
        assert!(response.success);
        assert_eq!(response.entries_count, 1);
    }

    #[test]
    fn test_batch_operations() {
        let dir = tempdir().unwrap();
        let tool = MemoryTool::new(dir.path());

        let input: MemoryToolInput = serde_json::from_str(
            r#"{
                "target": "memory",
                "operations": [
                    {"action": "add", "content": "Entry one"},
                    {"action": "add", "content": "Entry two"}
                ]
            }"#,
        )
        .unwrap();

        let response = tool.execute(input).unwrap();
        assert!(response.success);
        assert_eq!(response.entries_count, 2);
    }
}