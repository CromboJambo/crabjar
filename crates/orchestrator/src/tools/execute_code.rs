//! Code execution tool — runs Rhai scripts with crabjar tool bindings.
//!
//! Provides `execute_code` for programmatic tool calling with loops,
//! conditionals, and data processing between calls. Pure Rust (Rhai VM),
//! no Python dependency.

use serde_json::{json, Value};

/// Handle an execute_code tool call.
pub async fn handle(args: &[String]) -> Result<String, String> {
    if args.is_empty() || args[0].is_empty() {
        return Err("Error: execute_code requires JSON arguments".to_string());
    }

    let input: Value = serde_json::from_str(&args[0]).map_err(|e| format!("JSON parse error: {}", e))?;
    let code = input["code"].as_str().ok_or("Missing required field: code")?;

    // Create execution context with current working directory
    let workdir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    let ctx = crabjar_exec::ExecutionContext::new(workdir)
        .with_timeout(300)
        .with_max_tool_calls(50);

    let executor = crabjar_exec::CodeExecutor::new(ctx);

    match executor.execute(code).await {
        Ok(result) => {
            let result_json = json!({
                "success": true,
                "result": result,
            });
            Ok(serde_json::to_string_pretty(&result_json).map_err(|e| e.to_string())?)
        }
        Err(e) => {
            let error_json = json!({
                "success": false,
                "error": format!("{}", e),
            });
            Ok(serde_json::to_string_pretty(&error_json).map_err(|e2| e2.to_string())?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_code_basic() {
        let result = handle(&[r#"{"code": "let x = 2 + 3; json(x.to_string())"}"#.to_string()]).await;
        assert!(result.is_ok());
        let value: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(value["success"], true);
        assert_eq!(value["result"], 5);
    }

    #[tokio::test]
    async fn test_execute_code_loop() {
        let result = handle(&[r#"{"code": "let sum = 0; for i in 1..=10 { sum += i }; json(sum.to_string())"}"#.to_string()]).await;
        assert!(result.is_ok());
        let value: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(value["success"], true);
        assert_eq!(value["result"], 55);
    }

    #[tokio::test]
    async fn test_execute_code_error() {
        let result = handle(&[r#"{"code": "let x = undefined_var;"}"#.to_string()]).await;
        assert!(result.is_ok()); // Returns success=false in JSON, not Err
        let value: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(value["success"], false);
    }

    #[tokio::test]
    async fn test_execute_code_missing_code() {
        let result = handle(&[r#"{}"#.to_string()]).await;
        assert!(result.is_err());
    }
}
