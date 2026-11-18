//! First-class `search_files` tool — ripgrep-backed content and filename search.
//! Matches Hermes Agent API contract for drop-in parity.

use serde_json::{json, Value};
use std::path::PathBuf;

/// Handle the `search_files` tool call.
///
/// Arguments: JSON string in args[0] with fields:
///   - pattern (required): regex for content search or glob for file search
///   - target (optional, default "content"): "content" or "files"
///   - path (optional, default "."): directory to search in
///   - file_glob (optional): filter files by pattern (content search only)
///   - limit (optional, default 50): max results
///   - offset (optional, default 0): skip first N results
pub async fn handle(args: &[String]) -> Result<String, String> {
    if args.is_empty() || args[0].is_empty() {
        return Err("Error: search_files requires a JSON argument with 'pattern' field".to_string());
    }

    let request: Value = serde_json::from_str(&args[0])
        .map_err(|e| format!("Invalid JSON arguments: {}", e))?;

    let pattern = request["pattern"]
        .as_str()
        .ok_or("Error: 'pattern' field is required")?;

    let target = request["target"].as_str().unwrap_or("content");
    let search_path = expand_path(request["path"].as_str().unwrap_or("."));
    let file_glob = request["file_glob"].as_str();
    let limit = request["limit"].as_u64().unwrap_or(50) as usize;
    let offset = request["offset"].as_u64().unwrap_or(0) as usize;

    if !search_path.exists() {
        return Err(format!("Error: search path does not exist: {}", search_path.display()));
    }

    match target {
        "content" => search_content(pattern, &search_path, file_glob, limit, offset).await,
        "files" => search_files_by_name(pattern, &search_path, limit, offset).await,
        _ => Err(format!("Error: unknown target '{}'. Use 'content' or 'files'.", target)),
    }
}

/// Search file contents using ripgrep.
async fn search_content(
    pattern: &str,
    path: &PathBuf,
    file_glob: Option<&str>,
    limit: usize,
    offset: usize,
) -> Result<String, String> {
    let mut cmd = tokio::process::Command::new("rg");
    cmd.arg("--line-number")
        .arg("--no-heading")
        .arg("--color=never")
        .arg("-e")
        .arg(pattern)
        .arg(path);

    if let Some(glob) = file_glob {
        cmd.arg("--glob").arg(glob);
    }

    // Limit output to avoid huge results
    cmd.arg("--max-count").arg((limit + offset).to_string());

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Error running ripgrep: {}", e))?;

    if !output.status.success() && output.status.code() != Some(1) {
        // Exit code 1 means no matches (not an error)
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
            return Err(format!("ripgrep error: {}", stderr));
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // Skip offset, take limit
    let total_matches = lines.len();
    let results: Vec<Value> = lines[offset..total_matches.min(offset + limit)]
        .iter()
        .filter_map(|line| {
            // Format: path:line_number:content
            let parts: Vec<&str> = line.splitn(3, ':').collect();
            if parts.len() >= 3 {
                Some(json!({
                    "path": parts[0],
                    "line": parts[1].parse::<u64>().unwrap_or(0),
                    "content": parts[2]
                }))
            } else {
                None
            }
        })
        .collect();

    Ok(serde_json::to_string_pretty(&json!({
        "matches": results,
        "total_matches": total_matches,
        "returned": results.len(),
        "offset": offset,
        "limit": limit,
        "has_more": total_matches > offset + limit
    })).map_err(|e| e.to_string())?)
}

/// Search for files by name using find or rg --files.
async fn search_files_by_name(
    pattern: &str,
    path: &PathBuf,
    limit: usize,
    offset: usize,
) -> Result<String, String> {
    // Use find with -name for glob patterns
    let mut cmd = tokio::process::Command::new("find");
    cmd.arg(path)
        .arg("-type")
        .arg("f")
        .arg("-name")
        .arg(pattern);

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Error running find: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("find error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();

    let total_files = files.len();
    let results: Vec<Value> = files[offset..total_files.min(offset + limit)]
        .iter()
        .map(|f| json!({"path": f}))
        .collect();

    Ok(serde_json::to_string_pretty(&json!({
        "files": results,
        "total_files": total_files,
        "returned": results.len(),
        "offset": offset,
        "limit": limit,
        "has_more": total_files > offset + limit
    })).map_err(|e| e.to_string())?)
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
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_search_content_basic() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let mut f = fs::File::create(&file_path).unwrap();
        writeln!(f, "hello world").unwrap();
        writeln!(f, "goodbye world").unwrap();

        let result = search_content(
            "world",
            &temp_dir.path().to_path_buf(),
            None,
            10,
            0,
        ).await;
        assert!(result.is_ok());

        let output: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["total_matches"], 2);
    }

    #[tokio::test]
    async fn test_search_files_by_name() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("other.md"), "content").unwrap();

        let result = search_files_by_name(
            "*.txt",
            &temp_dir.path().to_path_buf(),
            10,
            0,
        ).await;
        assert!(result.is_ok());

        let output: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["total_files"], 1);
    }

    #[tokio::test]
    async fn test_search_missing_pattern() {
        let result = handle(&[r#"{}"#.to_string()]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("'pattern' field is required"));
    }

    #[tokio::test]
    async fn test_search_invalid_target() {
        let result = handle(&[r#"{"pattern": "test", "target": "invalid"}"#.to_string()]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown target"));
    }

    #[tokio::test]
    async fn test_search_empty_args() {
        let result = handle(&[]).await;
        assert!(result.is_err());
    }
}
