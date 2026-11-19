//! First-class `write_file` tool — full file overwrite with parent directory creation.
//! Matches Hermes Agent API contract for drop-in parity.

use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

/// Handle the `write_file` tool call.
///
/// Arguments: JSON string in args[0] with fields:
///   - path (required): file path to write
///   - content (required): complete content to write
pub async fn handle(args: &[String]) -> Result<String, String> {
    if args.is_empty() || args[0].is_empty() {
        return Err(
            "Error: write_file requires a JSON argument with 'path' and 'content' fields"
                .to_string(),
        );
    }

    let request: Value =
        serde_json::from_str(&args[0]).map_err(|e| format!("Invalid JSON arguments: {}", e))?;

    let path_str = request["path"]
        .as_str()
        .ok_or("Error: 'path' field is required")?;

    let content = request["content"]
        .as_str()
        .ok_or("Error: 'content' field is required")?
        .to_string();

    // Expand ~ to home directory
    let path = expand_path(path_str);

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Error creating parent directories: {}", e))?;
        }
    }

    // Write the file (overwrite)
    fs::write(&path, &content)
        .map_err(|e| format!("Error writing file {}: {}", path.display(), e))?;

    let bytes_written = content.len();
    let lines_written = content.lines().count();

    Ok(serde_json::to_string_pretty(&json!({
        "status": "success",
        "path": path.display().to_string(),
        "bytes_written": bytes_written,
        "lines_written": lines_written
    }))
    .map_err(|e| e.to_string())?)
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
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_write_file_basic() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let result = handle(&[format!(
            r#"{{"path": "{}", "content": "hello world\n"}}"#,
            file_path.display()
        )])
        .await;
        assert!(result.is_ok());

        let output: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["status"], "success");
        assert_eq!(output["bytes_written"], 12);
        assert_eq!(output["lines_written"], 1);

        // Verify file contents
        let written = fs::read_to_string(&file_path).unwrap();
        assert_eq!(written, "hello world\n");
    }

    #[tokio::test]
    async fn test_write_file_creates_parents() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir
            .path()
            .join("a")
            .join("b")
            .join("c")
            .join("test.txt");

        let result = handle(&[format!(
            r#"{{"path": "{}", "content": "nested"}}"#,
            file_path.display()
        )])
        .await;
        assert!(result.is_ok());
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_write_file_overwrites() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "original content").unwrap();

        let result = handle(&[format!(
            r#"{{"path": "{}", "content": "new content"}}"#,
            file_path.display()
        )])
        .await;
        assert!(result.is_ok());

        let written = fs::read_to_string(&file_path).unwrap();
        assert_eq!(written, "new content");
    }

    #[tokio::test]
    async fn test_write_file_missing_content() {
        let result = handle(&[r#"{"path": "/tmp/test.txt"}"#.to_string()]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("'content' field is required"));
    }

    #[tokio::test]
    async fn test_write_file_empty_args() {
        let result = handle(&[]).await;
        assert!(result.is_err());
    }
}
