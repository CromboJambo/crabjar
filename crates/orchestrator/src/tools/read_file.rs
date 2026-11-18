//! First-class `read_file` tool — line-numbered reads with pagination.
//! Matches Hermes Agent API contract for drop-in parity.

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

/// Handle the `read_file` tool call.
///
/// Arguments: JSON string in args[0] with fields:
///   - path (required): file path to read
///   - offset (optional, default 1): line number to start from (1-indexed)
///   - limit (optional, default 2000, max 2000): max lines to read
pub async fn handle(args: &[String]) -> Result<String, String> {
    if args.is_empty() || args[0].is_empty() {
        return Err("Error: read_file requires a JSON argument with 'path' field".to_string());
    }

    let request: Value = serde_json::from_str(&args[0])
        .map_err(|e| format!("Invalid JSON arguments: {}", e))?;

    let path_str = request["path"]
        .as_str()
        .ok_or("Error: 'path' field is required")?;

    let offset = request["offset"].as_u64().unwrap_or(1).max(1) as usize;
    let limit = request["limit"].as_u64().unwrap_or(2000).min(2000) as usize;

    // Expand ~ to home directory
    let path = expand_path(path_str);

    // Check if file exists
    if !path.exists() {
        return Err(format!("Error: file not found: {}", path.display()));
    }

    if !path.is_file() {
        return Err(format!("Error: not a regular file: {}", path.display()));
    }

    // Read the file
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Error reading file {}: {}", path.display(), e))?;

    let total_lines = content.lines().count();

    if total_lines == 0 {
        return Ok(serde_json::to_string_pretty(&json!({
            "content": "",
            "total_lines": 0,
            "offset": offset,
            "limit": limit,
            "truncated": false
        })).map_err(|e| e.to_string())?);
    }

    // Extract the requested range
    let lines: Vec<&str> = content.lines().collect();
    let start_idx = (offset - 1).min(total_lines);
    let end_idx = (start_idx + limit).min(total_lines);

    if start_idx >= total_lines {
        return Ok(serde_json::to_string_pretty(&json!({
            "content": "",
            "total_lines": total_lines,
            "offset": offset,
            "limit": limit,
            "truncated": false,
            "note": format!("Offset {} is past end of file ({} lines)", offset, total_lines)
        })).map_err(|e| e.to_string())?);
    }

    // Format with line numbers: "LINE_NUM|CONTENT"
    let numbered: Vec<String> = lines[start_idx..end_idx]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{}|{}", start_idx + i + 1, line))
        .collect();

    let result_content = numbered.join("\n");
    let truncated = end_idx < total_lines;

    let mut result = json!({
        "content": result_content,
        "total_lines": total_lines,
        "offset": offset,
        "limit": limit,
        "truncated": truncated
    });

    if truncated {
        result["next_offset"] = json!(end_idx + 1);
    }

    Ok(serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?)
}

/// Expand leading ~ to the user's home directory.
fn expand_path(path_str: &str) -> PathBuf {
    if let Some(rest) = path_str.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_file_basic() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let mut f = fs::File::create(&file_path).unwrap();
        writeln!(f, "line one").unwrap();
        writeln!(f, "line two").unwrap();
        writeln!(f, "line three").unwrap();

        let result = handle(&[format!(r#"{{"path": "{}"}}"#, file_path.display())]).await;
        assert!(result.is_ok());
        let output: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["total_lines"], 3);
        assert!(output["content"].as_str().unwrap().contains("1|line one"));
        assert!(output["content"].as_str().unwrap().contains("2|line two"));
    }

    #[tokio::test]
    async fn test_read_file_with_offset() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let mut f = fs::File::create(&file_path).unwrap();
        writeln!(f, "line one").unwrap();
        writeln!(f, "line two").unwrap();
        writeln!(f, "line three").unwrap();

        let result = handle(&[format!(r#"{{"path": "{}", "offset": 2}}"#, file_path.display())]).await;
        assert!(result.is_ok());
        let output: Value = serde_json::from_str(&result.unwrap()).unwrap();
        let content = output["content"].as_str().unwrap();
        assert!(content.contains("2|line two"));
        assert!(!content.contains("1|line one"));
    }

    #[tokio::test]
    async fn test_read_file_with_limit() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let mut f = fs::File::create(&file_path).unwrap();
        for i in 1..=10 {
            writeln!(f, "line {}", i).unwrap();
        }

        let result = handle(&[format!(r#"{{"path": "{}", "limit": 3}}"#, file_path.display())]).await;
        assert!(result.is_ok());
        let output: Value = serde_json::from_str(&result.unwrap()).unwrap();
        let content = output["content"].as_str().unwrap();
        assert_eq!(content.lines().count(), 3);
        assert!(output["truncated"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_read_file_missing_path() {
        let result = handle(&[r#"{}"#.to_string()]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("'path' field is required"));
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let result = handle(&[r#"{"path": "/nonexistent/file.txt"}"#.to_string()]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("file not found"));
    }

    #[tokio::test]
    async fn test_read_file_empty_args() {
        let result = handle(&[]).await;
        assert!(result.is_err());
    }
}
