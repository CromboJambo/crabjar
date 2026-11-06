//! Filesystem operations plugin for CrabJar.
//! Demonstrates dynamic component loading with host imports for fs operations.

use crabjar_plugin::types::{PluginError, ToolResult};
use std::fs;
use std::path::Path;

pub struct FsOpsPlugin;

impl FsOpsPlugin {
    pub fn new() -> Self {
        FsOpsPlugin
    }

    fn list_tools(&self) -> Vec<String> {
        vec![
            "read_file".to_string(),
            "write_file".to_string(),
            "list_dir".to_string(),
            "exists".to_string(),
            "remove_file".to_string(),
        ]
    }

    fn handle_tool_call(
        &self,
        tool_name: &str,
        args_json: &str,
    ) -> Result<ToolResult, PluginError> {
        let args: serde_json::Value = serde_json::from_str(args_json)
            .map_err(|e| PluginError::InvalidArgument(format!("Failed to parse args: {}", e)))?;

        match tool_name {
            "read_file" => self.read_file(&args),
            "write_file" => self.write_file(&args),
            "list_dir" => self.list_dir(&args),
            "exists" => self.exists(&args),
            "remove_file" => self.remove_file(&args),
            _ => Err(PluginError::ToolNotFound(tool_name.to_string())),
        }
    }

    fn read_file(&self, args: &serde_json::Value) -> Result<ToolResult, PluginError> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| PluginError::InvalidArgument("Missing 'path' argument".to_string()))?;

        let content = fs::read_to_string(path)
            .map_err(|e| PluginError::Internal(format!("Failed to read file {}: {}", path, e)))?;

        Ok(ToolResult {
            success: true,
            output: serde_json::json!({ "content": content }),
        })
    }

    fn write_file(&self, args: &serde_json::Value) -> Result<ToolResult, PluginError> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| PluginError::InvalidArgument("Missing 'path' argument".to_string()))?;
        let content = args["content"].as_str().ok_or_else(|| {
            PluginError::InvalidArgument("Missing 'content' argument".to_string())
        })?;

        fs::write(path, content)
            .map_err(|e| PluginError::Internal(format!("Failed to write file {}: {}", path, e)))?;

        Ok(ToolResult {
            success: true,
            output: serde_json::json!({ "path": path }),
        })
    }

    fn list_dir(&self, args: &serde_json::Value) -> Result<ToolResult, PluginError> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| PluginError::InvalidArgument("Missing 'path' argument".to_string()))?;

        let entries = fs::read_dir(path)
            .map_err(|e| {
                PluginError::Internal(format!("Failed to list directory {}: {}", path, e))
            })?
            .filter_map(|entry| {
                entry
                    .ok()
                    .map(|e| e.file_name().to_string_lossy().to_string())
            })
            .collect::<Vec<String>>();

        Ok(ToolResult {
            success: true,
            output: serde_json::json!({ "entries": entries }),
        })
    }

    fn exists(&self, args: &serde_json::Value) -> Result<ToolResult, PluginError> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| PluginError::InvalidArgument("Missing 'path' argument".to_string()))?;

        let result = Path::new(path).exists();

        Ok(ToolResult {
            success: true,
            output: serde_json::json!({ "exists": result }),
        })
    }

    fn remove_file(&self, args: &serde_json::Value) -> Result<ToolResult, PluginError> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| PluginError::InvalidArgument("Missing 'path' argument".to_string()))?;

        fs::remove_file(path)
            .map_err(|e| PluginError::Internal(format!("Failed to remove file {}: {}", path, e)))?;

        Ok(ToolResult {
            success: true,
            output: serde_json::json!({ "path": path }),
        })
    }
}
