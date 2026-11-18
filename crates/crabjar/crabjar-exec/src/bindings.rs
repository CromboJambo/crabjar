//! Tool bindings for the Rhai execution engine.
//!
//! Registers crabjar tools as callable functions within scripts.

use crate::error::{ExecError, ExecResult};
use crate::ExecutionContext;
use rhai::{Engine, EvalAltResult};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Register all built-in tool functions with the Rhai engine.
pub fn register_all(engine: &mut Engine, context: ExecutionContext) {
    register_file_tools(engine, context.clone());
    register_shell_tools(engine, context);
}

/// Register file operation tools.
fn register_file_tools(engine: &mut Engine, context: ExecutionContext) {
    let ctx = Arc::new(Mutex::new(context));

    // read_file(path) -> JSON with content and line numbers
    {
        let ctx = Arc::clone(&ctx);
        engine.register_fn(
            "read_file",
            move |path: String| -> Result<String, Box<EvalAltResult>> {
                let ctx = ctx.blocking_lock();
                read_file_impl(&ctx, &path, 1, 2000)
            },
        );
    }

    // write_file(path, content) -> JSON confirmation
    {
        let ctx = Arc::clone(&ctx);
        engine.register_fn(
            "write_file",
            move |path: String, content: String| -> Result<String, Box<EvalAltResult>> {
                let ctx = ctx.blocking_lock();
                let full_path = resolve_path(&ctx.workdir, &path);

                if let Some(parent) = full_path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create directory: {}", e))?;
                }

                std::fs::write(&full_path, &content)
                    .map_err(|e| format!("Failed to write file: {}", e))?;

                Ok(json!({
                    "path": full_path.display().to_string(),
                    "bytes_written": content.len(),
                    "status": "ok"
                })
                .to_string())
            },
        );
    }

    // search_files(pattern) -> JSON with matches
    {
        let ctx = Arc::clone(&ctx);
        engine.register_fn(
            "search_files",
            move |pattern: String| -> Result<String, Box<EvalAltResult>> {
                let ctx = ctx.blocking_lock();
                search_files_impl(&ctx, &pattern, ".", "")
            },
        );
    }
}

/// Register shell/command tools.
fn register_shell_tools(engine: &mut Engine, context: ExecutionContext) {
    let ctx = Arc::new(Mutex::new(context));

    // terminal(command) -> JSON with output and exit code
    engine.register_fn(
        "terminal",
        move |command: String| -> Result<String, Box<EvalAltResult>> {
            let mut ctx = ctx.blocking_lock();
            if let Err(e) = increment_tool_call(&mut ctx) {
                return Err(format!("{}", e).into());
            }

            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(&command)
                .output()
                .map_err(|e| format!("Failed to execute command: {}", e))?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(-1);

            // Truncate very large outputs
            let (stdout_trunc, stdout_file) = truncate_output(&stdout, 50_000);
            let (stderr_trunc, stderr_file) = truncate_output(&stderr, 20_000);

            let mut result = json!({
                "output": stdout_trunc,
                "stderr": stderr_trunc,
                "exit_code": exit_code
            });

            if let Some(path) = stdout_file {
                result["stdout_file"] = json!(path);
            }
            if let Some(path) = stderr_file {
                result["stderr_file"] = json!(path);
            }

            Ok(result.to_string())
        },
    );
}

/// Implementation of read_file with offset and limit.
fn read_file_impl(
    ctx: &ExecutionContext,
    path: &str,
    offset: i32,
    limit: i32,
) -> Result<String, Box<EvalAltResult>> {
    let full_path = resolve_path(&ctx.workdir, path);

    if !full_path.exists() {
        return Err(format!("File not found: {}", full_path.display()).into());
    }

    let content = std::fs::read_to_string(&full_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let lines: Vec<&str> = content.lines().collect();
    let start = offset.max(1) as usize - 1;
    let end = (start + limit as usize).min(lines.len());

    if start >= lines.len() {
        return Err(format!(
            "Offset {} exceeds file length ({})",
            offset,
            lines.len()
        )
        .into());
    }

    let numbered: Vec<String> = lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{}|{}", start + i + 1, line))
        .collect();

    Ok(json!({
        "content": numbered.join("\n"),
        "total_lines": lines.len(),
        "start_line": start + 1,
        "end_line": end,
        "truncated": end < lines.len()
    })
    .to_string())
}

/// Implementation of search_files.
fn search_files_impl(
    ctx: &ExecutionContext,
    pattern: &str,
    path: &str,
    file_glob: &str,
) -> Result<String, Box<EvalAltResult>> {
    let search_path = resolve_path(&ctx.workdir, path);

    if !search_path.exists() {
        return Err(format!("Search path not found: {}", search_path.display()).into());
    }

    let mut matches = Vec::new();
    search_dir(&search_path, pattern, file_glob, 50, &mut matches)?;

    Ok(json!({
        "matches": matches,
        "count": matches.len(),
        "truncated": matches.len() >= 50
    })
    .to_string())
}

/// Resolve a potentially relative path against the working directory.
fn resolve_path(workdir: &PathBuf, path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        workdir.join(p)
    }
}

/// Recursively search a directory for files matching a pattern.
fn search_dir(
    dir: &PathBuf,
    pattern: &str,
    file_glob: &str,
    limit: usize,
    matches: &mut Vec<Value>,
) -> Result<(), String> {
    if matches.len() >= limit {
        return Ok(());
    }

    let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;

    for entry in entries.flatten() {
        if matches.len() >= limit {
            break;
        }

        let path = entry.path();

        // Skip hidden directories and target/
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') || name == "target" {
                continue;
            }
        }

        if path.is_dir() {
            search_dir(&path, pattern, file_glob, limit, matches)?;
        } else if path.is_file() {
            // Check file glob filter
            if !file_glob.is_empty() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let expected = file_glob.trim_start_matches('*');
                    if !ext.contains(expected) {
                        continue;
                    }
                }
            }

            // Read and search content
            if let Ok(content) = std::fs::read_to_string(&path) {
                for (line_num, line) in content.lines().enumerate() {
                    if matches.len() >= limit {
                        break;
                    }
                    if line.contains(pattern) {
                        matches.push(json!({
                            "file": path.display().to_string(),
                            "line": line_num + 1,
                            "text": line.trim()
                        }));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Truncate large output and save full text to a temp file if needed.
fn truncate_output(text: &str, max_chars: usize) -> (String, Option<String>) {
    if text.len() <= max_chars {
        return (text.to_string(), None);
    }

    // Save full output to temp file
    let tmp_dir = std::env::temp_dir().join("crabjar-exec");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let filename = format!("output_{}.txt", nanos);
    let path = tmp_dir.join(&filename);

    if let Ok(()) = std::fs::write(&path, text) {
        let head_len = max_chars / 2;
        let tail_start = text.len().saturating_sub(head_len);
        let head = &text[..head_len];
        let tail = &text[tail_start..];
        let truncated = format!(
            "{}\n\n[... {} characters truncated ...]\n\n{}",
            head,
            text.len() - max_chars,
            tail
        );
        (truncated, Some(path.display().to_string()))
    } else {
        (text[..max_chars].to_string(), None)
    }
}

/// Increment the tool call counter and check limits.
fn increment_tool_call(ctx: &mut ExecutionContext) -> Result<(), ExecError> {
    // Since we hold a blocking lock on ctx, we can safely mutate the count
    let count = ctx.tool_call_count.try_lock();
    match count {
        Ok(mut guard) => {
            *guard += 1;
            if *guard > ctx.max_tool_calls {
                return Err(ExecError::ToolCallLimitExceeded(ctx.max_tool_calls));
            }
            Ok(())
        }
        Err(_) => {
            // If we can't lock (shouldn't happen since we hold the parent lock),
            // just proceed without counting
            Ok(())
        }
    }
}
