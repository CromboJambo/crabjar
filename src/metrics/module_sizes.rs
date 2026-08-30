/// Module size metrics (500 LoC rule).
///
/// Scans all .rs files in the workspace (excluding target/) and reports
/// per-file and per-crate LoC totals, plus any violations.
use serde_json::json;

/// Run module size metrics.
pub fn run_module_sizes() -> serde_json::Value {
    let output = match std::process::Command::new("find")
        .args([".", "-name", "*.rs", "-not", "-path", "*/target/*"])
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            return json!({
                "success": false,
                "error": format!("Failed to run find: {}", e),
                "usage": ["crabjar metrics modules"],
            });
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    if lines.is_empty() {
        return json!({
            "success": false,
            "error": "No .rs files found",
            "usage": ["crabjar metrics modules"],
        });
    }

    let mut total_lines: usize = 0;
    let mut max_file: String = String::new();
    let mut max_lines: usize = 0;
    let mut violations: Vec<String> = Vec::new();
    let mut crate_sizes: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();

    for line in &lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(wc_output) = std::process::Command::new("wc").args(["-l", line]).output() {
            let wc_stdout = String::from_utf8_lossy(&wc_output.stdout);
            if let Some(first_word) = wc_stdout.split_whitespace().next()
                && let Ok(locl) = first_word.parse::<usize>()
            {
                total_lines += locl;
                if locl > max_lines {
                    max_lines = locl;
                    max_file = line.to_string();
                }
                if locl > 500 {
                    violations.push(format!("{}: {} LoC", line, locl));
                }

                // Extract crate name from path
                let crate_name = if let Some(pos) = line.find("/src/") {
                    line[..pos].to_string()
                } else {
                    "root".to_string()
                };

                if let Some(obj) = crate_sizes.get_mut(&crate_name) {
                    if let Some(total) = obj.get("total_lines").and_then(|v| v.as_u64()) {
                        obj["total_lines"] = json!((total as usize + locl) as u64);
                    }
                    if let Some(count) = obj.get("file_count").and_then(|v| v.as_u64()) {
                        obj["file_count"] = json!((count as usize + 1) as u64);
                    }
                } else {
                    let mut m = serde_json::Map::new();
                    m.insert("total_lines".to_string(), json!(locl as u64));
                    m.insert("file_count".to_string(), json!(1u64));
                    crate_sizes.insert(crate_name, serde_json::Value::Object(m));
                }
            }
        }
    }

    let under_500 = if violations.is_empty() {
        "all files under 500 LoC".to_string()
    } else {
        format!("{} violations found", violations.len())
    };

    json!({
        "success": true,
        "metrics": {
            "modules": {
                "total_files": lines.len(),
                "total_lines": total_lines,
                "max_file": max_file,
                "max_lines": max_lines,
                "violations": violations,
                "under_500": under_500,
            },
            "crate_sizes": crate_sizes,
        },
        "doubt": {
            "assumptions": [
                "find . -name '*.rs' captures all Rust source files",
                "wc -l counts lines correctly (trailing newline dependent)",
            ],
            "blind_spots": [
                "Does not exclude generated files (build.rs output, proc-macro output)",
                "Does not count blank lines vs code lines separately",
                "File paths are relative to CWD",
            ],
            "last_validation": chrono::Utc::now().to_rfc3339(),
            "stale_after": chrono::Utc::now()
                .checked_add_signed(chrono::Duration::hours(24))
                .unwrap_or_default()
                .to_rfc3339(),
        },
    })
}
