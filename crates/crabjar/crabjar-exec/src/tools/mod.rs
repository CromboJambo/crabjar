//! Tool registry for crabjar-exec.
//!
//! Provides a way to register and look up tools that can be called from scripts.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// A registered tool that can be called from execution scripts.
pub struct Tool {
    /// Human-readable name of the tool.
    pub name: String,
    /// Description of what the tool does.
    pub description: String,
    /// The actual implementation (takes JSON args, returns JSON result).
    pub handler: Arc<dyn Fn(&[Value]) -> Result<Value, String> + Send + Sync>,
}

impl Tool {
    /// Create a new tool with the given name, description, and handler.
    pub fn new<F>(name: &str, description: &str, handler: F) -> Self
    where
        F: Fn(&[Value]) -> Result<Value, String> + Send + Sync + 'static,
    {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            handler: Arc::new(handler),
        }
    }

    /// Invoke the tool with the given arguments.
    pub fn call(&self, args: &[Value]) -> Result<Value, String> {
        (self.handler)(args)
    }
}

/// Registry of available tools for code execution.
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Tool>,
}

impl ToolRegistry {
    /// Create an empty tool registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool in the registry.
    pub fn register(&mut self, tool: Tool) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// Look up a tool by name.
    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }

    /// Call a tool by name with the given arguments.
    pub fn call(&self, name: &str, args: &[Value]) -> Result<Value, String> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| format!("Unknown tool: {}", name))?;
        tool.call(args)
    }

    /// List all registered tool names.
    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    /// Get the number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}
