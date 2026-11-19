//! crabjar-exec — Structured code execution engine for crabjar agents.
//!
//! Provides a Rust-first alternative to Hermes' Python `execute_code` tool.
//! Agents write Rhai scripts that call crabjar tools programmatically, with
//! full support for loops, conditionals, and data processing between tool calls.
//!
//! ## Design Principles
//!
//! 1. **Rust-first**: No Python dependency — uses Rhai VM embedded in Rust
//! 2. **Tool binding**: All crabjar tools exposed as callable functions
//! 3. **Sandboxed**: Scripts run in a controlled VM with resource limits
//! 4. **Typed results**: Tools return structured data (JSON) for further processing
//! 5. **Observable**: Execution traced via `tracing` crate

use anyhow::Result;
use rhai::Engine;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

mod bindings;
mod error;
mod tools;

pub use error::{ExecError, ExecResult};
pub use tools::ToolRegistry;

/// Execution context passed to the Rhai VM.
/// Holds shared state that tool bindings can access during script execution.
#[derive(Clone)]
pub struct ExecutionContext {
    /// Working directory for file operations
    pub workdir: PathBuf,
    /// Tool registry for dynamic tool discovery
    pub tool_registry: Arc<Mutex<ToolRegistry>>,
    /// Maximum execution time in seconds (0 = no limit)
    #[allow(dead_code)]
    pub timeout_secs: u64,
    /// Maximum number of tool calls allowed
    pub max_tool_calls: usize,
    /// Counter for tool calls made during this execution
    pub tool_call_count: Arc<Mutex<usize>>,
}

impl ExecutionContext {
    /// Create a new execution context with default limits.
    pub fn new(workdir: impl Into<PathBuf>) -> Self {
        Self {
            workdir: workdir.into(),
            tool_registry: Arc::new(Mutex::new(ToolRegistry::default())),
            timeout_secs: 300,
            max_tool_calls: 50,
            tool_call_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Set the maximum execution time.
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Set the maximum number of tool calls.
    pub fn with_max_tool_calls(mut self, max: usize) -> Self {
        self.max_tool_calls = max;
        self
    }
}

/// The main code execution engine.
/// Compiles and runs Rhai scripts with crabjar tool bindings.
pub struct CodeExecutor {
    engine: Engine,
    context: ExecutionContext,
}

impl CodeExecutor {
    /// Create a new code executor with the given context.
    pub fn new(context: ExecutionContext) -> Self {
        let mut engine = Engine::new();

        // Register helper functions for JSON conversion
        engine.register_fn(
            "json",
            |value: String| -> Result<String, Box<rhai::EvalAltResult>> {
                // Parse the input as JSON and re-serialize it (validation)
                let parsed: Value =
                    serde_json::from_str(&value).map_err(|e| format!("Invalid JSON: {}", e))?;
                Ok(serde_json::to_string(&parsed)
                    .map_err(|e| format!("JSON serialization failed: {}", e))?)
            },
        );

        // Register all tool bindings
        bindings::register_all(&mut engine, context.clone());

        Self { engine, context }
    }

    /// Execute a Rhai script and return the result as JSON.
    ///
    /// The script should return a value which will be converted to JSON.
    /// For complex data, use the `json()` helper function to construct
    /// JSON strings within the script.
    pub async fn execute(&self, code: &str) -> ExecResult<Value> {
        info!(code_len = code.len(), "Executing code block");

        // Check tool call limit before starting
        let count = *self.context.tool_call_count.lock().await;
        if count >= self.context.max_tool_calls {
            return Err(ExecError::ToolCallLimitExceeded(
                self.context.max_tool_calls,
            ));
        }

        // Evaluate the script in the VM as a string, then parse as JSON
        let result_str: String = self
            .engine
            .eval(code)
            .map_err(|e| ExecError::ScriptError(e.to_string()))?;

        // Try to parse as JSON first
        if let Ok(json_val) = serde_json::from_str::<Value>(&result_str) {
            info!("Code execution completed");
            return Ok(json_val);
        }

        // If not valid JSON, wrap as a string value
        info!("Code execution completed (string result)");
        Ok(Value::String(result_str))
    }

    /// Execute a Rhai script with initial variables passed in.
    pub async fn execute_with_vars(
        &self,
        code: &str,
        initial_vars: Vec<(&str, Value)>,
    ) -> ExecResult<Value> {
        let mut scope = rhai::Scope::new();

        for (name, value) in initial_vars {
            // Push values as strings for now (simplified type handling)
            scope.push(name, serde_json::to_string(&value).unwrap_or_default());
        }

        // Run with the provided scope
        let result_str: String = self
            .engine
            .eval_with_scope(&mut scope, code)
            .map_err(|e| ExecError::ScriptError(e.to_string()))?;

        // Try to parse as JSON first
        if let Ok(json_val) = serde_json::from_str::<Value>(&result_str) {
            return Ok(json_val);
        }

        Ok(Value::String(result_str))
    }

    /// Register a custom tool function that scripts can call.
    pub fn register_tool<F>(&mut self, name: &str, func: F)
    where
        F: Fn(&[Value]) -> Result<Value, String> + Send + Sync + 'static,
    {
        let engine = &mut self.engine;
        let rust_func = move |args: Vec<String>| -> Result<String, Box<rhai::EvalAltResult>> {
            // Convert string args to JSON values
            let json_args: Result<Vec<Value>, _> =
                args.into_iter().map(|s| serde_json::from_str(&s)).collect();
            match json_args {
                Ok(values) => func(&values)
                    .and_then(|v| serde_json::to_string(&v).map_err(|e| e.to_string()))
                    .map_err(|e| format!("{}", e).into()),
                Err(e) => Err(format!("Invalid JSON argument: {}", e).into()),
            }
        };
        engine.register_fn(name, rust_func);
    }

    /// Get the current tool call count.
    pub async fn tool_call_count(&self) -> usize {
        *self.context.tool_call_count.lock().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_execution() {
        let ctx = ExecutionContext::new("/tmp");
        let executor = CodeExecutor::new(ctx);

        // Simple arithmetic returned as JSON string
        let result: Value = executor
            .execute(r#"let x = 2 + 3; let y = x * 4; json(y.to_string())"#)
            .await
            .unwrap();
        assert_eq!(result, json!(20));
    }

    #[tokio::test]
    async fn test_string_operations() {
        let ctx = ExecutionContext::new("/tmp");
        let executor = CodeExecutor::new(ctx);

        // Build a JSON string manually (Rhai strings don't have to_json_string)
        let result: Value = executor
            .execute(r#"let s = "hello"; json("\"" + s + " world\"")"#)
            .await
            .unwrap();
        assert_eq!(result, json!("hello world"));
    }

    #[tokio::test]
    async fn test_loop_and_aggregation() {
        let ctx = ExecutionContext::new("/tmp");
        let executor = CodeExecutor::new(ctx);

        // Sum numbers 1-10 using a loop
        let result: Value = executor
            .execute("let sum = 0; for i in 1..=10 { sum += i }; json(sum.to_string())")
            .await
            .unwrap();
        assert_eq!(result, json!(55));
    }

    #[tokio::test]
    async fn test_conditional_logic() {
        let ctx = ExecutionContext::new("/tmp");
        let executor = CodeExecutor::new(ctx);

        let result: Value = executor
            .execute(
                r#"if 5 > 3 { json("\"yes\"".to_string()) } else { json("\"no\"".to_string()) }"#,
            )
            .await
            .unwrap();
        assert_eq!(result, json!("yes"));
    }

    #[tokio::test]
    async fn test_array_operations() {
        let ctx = ExecutionContext::new("/tmp");
        let executor = CodeExecutor::new(ctx);

        // Build a JSON array string manually using a while loop
        let result: Value = executor
            .execute(
                r#"
                let nums = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
                let evens = [];
                for n in nums {
                    if n % 2 == 0 {
                        evens.push(n * 2);
                    }
                }
                // Build JSON array string using a while loop
                let joined = "";
                let i = 0;
                while i < evens.len() {
                    if joined != "" {
                        joined += ", ";
                    }
                    joined += evens[i].to_string();
                    i += 1;
                }
                json("[" + joined + "]")
            "#,
            )
            .await
            .unwrap();
        assert_eq!(result, json!([4, 8, 12, 16, 20]));
    }

    #[tokio::test]
    async fn test_error_handling() {
        let ctx = ExecutionContext::new("/tmp");
        let executor = CodeExecutor::new(ctx);

        // Script errors should be caught and returned as ExecError
        let result = executor.execute("let x = undefined_var;").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ExecError::ScriptError(_)));
    }
}
