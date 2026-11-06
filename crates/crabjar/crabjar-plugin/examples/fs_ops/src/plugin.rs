//! Filesystem operations plugin for CrabJar.
//! Demonstrates dynamic component loading with real fs operations.
//! Exports: handle_tool_call(tool_name, args_json) -> result_json
//!          list_tools() -> tools_json

use std::fs;
use std::path::Path;

#[unsafe(no_mangle)]
pub fn list_tools() -> String {
    let tools = vec![
        "read_file",
        "write_file",
        "list_dir",
        "exists",
        "remove_file",
    ];
    format!(
        r#"{{"tools":[{}]}}"#,
        tools
            .iter()
            .map(|t| format!("\"{}\"", t))
            .collect::<Vec<_>>()
            .join(",")
    )
}

#[unsafe(no_mangle)]
pub fn handle_tool_call(tool_name: &str, args_json: &str) -> String {
    let args: serde_json::Value = match serde_json::from_str(args_json) {
        Ok(v) => v,
        Err(e) => {
            return format!(
                r#"{{"success":false,"error":"Failed to parse args: {}"}}"#,
                e
            );
        }
    };

    match tool_name {
        "read_file" => handle_read_file(&args),
        "write_file" => handle_write_file(&args),
        "list_dir" => handle_list_dir(&args),
        "exists" => handle_exists(&args),
        "remove_file" => handle_remove_file(&args),
        _ => format!(
            r#"{{"success":false,"error":"Unknown tool: {}"}}"#,
            tool_name
        ),
    }
}

fn handle_read_file(args: &serde_json::Value) -> String {
    let path = match args["path"].as_str() {
        Some(p) => p,
        None => return r#"{"success":false,"error":"Missing 'path' argument"}"#.to_string(),
    };

    match fs::read_to_string(path) {
        Ok(content) => format!(
            r#"{{"success":true,"content":"{}"}}"#,
            content.replace("\"", "\\\"").replace("\n", "\\n")
        ),
        Err(e) => format!(
            r#"{{"success":false,"error":"Failed to read {}: {}"}}"#,
            path, e
        ),
    }
}

fn handle_write_file(args: &serde_json::Value) -> String {
    let path = match args["path"].as_str() {
        Some(p) => p,
        None => return r#"{"success":false,"error":"Missing 'path' argument"}"#.to_string(),
    };
    let content = match args["content"].as_str() {
        Some(c) => c,
        None => return r#"{"success":false,"error":"Missing 'content' argument"}"#.to_string(),
    };

    match fs::write(path, content) {
        Ok(_) => format!(r#"{{"success":true,"path":"{}"}}"#, path),
        Err(e) => format!(
            r#"{{"success":false,"error":"Failed to write {}: {}"}}"#,
            path, e
        ),
    }
}

fn handle_list_dir(args: &serde_json::Value) -> String {
    let path = match args["path"].as_str() {
        Some(p) => p,
        None => return r#"{"success":false,"error":"Missing 'path' argument"}"#.to_string(),
    };

    match fs::read_dir(path) {
        Ok(entries) => {
            let names: Vec<String> = entries
                .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().to_string()))
                .collect();
            format!(
                r#"{{"success":true,"entries":[{}]}}"#,
                names
                    .iter()
                    .map(|n| format!("\"{}\"", n))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        Err(e) => format!(
            r#"{{"success":false,"error":"Failed to list {}: {}"}}"#,
            path, e
        ),
    }
}

fn handle_exists(args: &serde_json::Value) -> String {
    let path = match args["path"].as_str() {
        Some(p) => p,
        None => return r#"{"success":false,"error":"Missing 'path' argument"}"#.to_string(),
    };

    let result = Path::new(path).exists();
    format!(r#"{{"success":true,"exists":{}}}"#, result)
}

fn handle_remove_file(args: &serde_json::Value) -> String {
    let path = match args["path"].as_str() {
        Some(p) => p,
        None => return r#"{"success":false,"error":"Missing 'path' argument"}"#.to_string(),
    };

    match fs::remove_file(path) {
        Ok(_) => format!(r#"{{"success":true,"path":"{}"}}"#, path),
        Err(e) => format!(
            r#"{{"success":false,"error":"Failed to remove {}: {}"}}"#,
            path, e
        ),
    }
}
