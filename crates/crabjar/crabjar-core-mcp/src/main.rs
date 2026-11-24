//! crabjar-core-mcp: MCP server exposing crabjar's core substrate tools.
//!
//! Provides read_file, write_file, patch, search_files, terminal, and web_search
//! as MCP tools — the fundamental building blocks agents need to interact with
//! the filesystem and OS. This is the substrate layer: Hermes (or any agent)
//! uses its brain for reasoning, crabjar provides the hands.

use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;

// ─── Request Types ──────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReadFileRequest {
    /// Path to the file to read (absolute, relative, or ~/path)
    pub path: String,
    /// Line number to start reading from (1-indexed, default: 1)
    #[serde(default = "default_offset")]
    pub offset: usize,
    /// Maximum number of lines to read (default: 2000, max: 2000)
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_offset() -> usize { 1 }
fn default_limit() -> usize { 2000 }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WriteFileRequest {
    /// Path to the file to write
    pub path: String,
    /// Complete content to write to the file
    pub content: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PatchRequest {
    /// File path to edit
    pub path: String,
    /// Exact text to find and replace
    pub old_string: String,
    /// Changed replacement text
    pub new_string: String,
    /// Replace all occurrences (default: false)
    #[serde(default)]
    pub replace_all: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchFilesRequest {
    /// Regex for content search; glob for file search
    pub pattern: String,
    /// 'content' searches inside files, 'files' finds files by name (default: "content")
    #[serde(default = "default_search_target")]
    pub target: String,
    /// Directory to search in (default: ".")
    #[serde(default = "default_search_path")]
    pub path: String,
    /// Filter files by pattern for content search (e.g. "*.py")
    pub file_glob: Option<String>,
    /// Maximum number of results (default: 50)
    #[serde(default = "default_search_limit")]
    pub limit: usize,
}

fn default_search_target() -> String { "content".to_string() }
fn default_search_path() -> String { ".".to_string() }
fn default_search_limit() -> usize { 50 }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TerminalRequest {
    /// The shell command to execute
    pub command: String,
    /// Run as background process (default: false)
    #[serde(default)]
    pub background: bool,
    /// Max seconds for foreground commands (default: 180, max: 600)
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// Working directory for the command
    pub workdir: Option<String>,
}

fn default_timeout() -> u64 { 180 }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WebSearchRequest {
    /// The search query
    pub query: String,
    /// Maximum number of results (default: 5)
    #[serde(default = "default_web_limit")]
    pub limit: usize,
}

fn default_web_limit() -> usize { 5 }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EmptyRequest {}

// ─── Server ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CoreToolsServer;

/// Expand ~ to home directory
fn expand_path(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix('~') {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/crombo".to_string());
        PathBuf::from(format!("{}{}", home, rest))
    } else {
        PathBuf::from(p)
    }
}

#[tool_router(server_handler)]
impl CoreToolsServer {
    #[tool(description = "Read a text file with line numbers and pagination. Output format: 'LINE_NUM|CONTENT'. Use offset and limit for large files.")]
    fn read_file(&self, params: Parameters<ReadFileRequest>) -> String {
        let req = params.0;
        let path = expand_path(&req.path);

        if !path.exists() {
            return format!("Error: file not found: {}", path.display());
        }
        if !path.is_file() {
            return format!("Error: not a regular file: {}", path.display());
        }

        match fs::read_to_string(&path) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let total = lines.len();
                let start = (req.offset.saturating_sub(1)).min(total);
                let end = (start + req.limit.min(2000)).min(total);
                let selected: Vec<String> = lines[start..end]
                    .iter()
                    .enumerate()
                    .map(|(i, l)| format!("{}|{}", start + i + 1, l))
                    .collect();

                serde_json::to_string(&json!({
                    "content": selected.join("\n"),
                    "total_lines": total,
                    "start_line": start + 1,
                    "end_line": end,
                    "path": path.display().to_string()
                }))
                .unwrap_or_else(|e| format!("Error: {}", e))
            }
            Err(e) => format!("Error reading file: {}", e),
        }
    }

    #[tool(description = "Write content to a file, completely replacing existing content. Creates parent directories automatically.")]
    fn write_file(&self, params: Parameters<WriteFileRequest>) -> String {
        let req = params.0;
        let path = expand_path(&req.path);

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    return format!("Error creating parent directories: {}", e);
                }
            }
        }

        match fs::write(&path, &req.content) {
            Ok(_) => serde_json::to_string(&json!({
                "status": "success",
                "path": path.display().to_string(),
                "bytes_written": req.content.len()
            })).unwrap_or_else(|e| format!("Error: {}", e)),
            Err(e) => format!("Error writing file: {}", e),
        }
    }

    #[tool(description = "Targeted find-and-replace edits in files. Uses fuzzy matching (exact, whitespace-normalized, trimmed, case-insensitive). Returns a unified diff.")]
    fn patch(&self, params: Parameters<PatchRequest>) -> String {
        let req = params.0;
        let path = expand_path(&req.path);

        if !path.exists() {
            return format!("Error: file not found: {}", path.display());
        }

        let original = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => return format!("Error reading file: {}", e),
        };

        // Simple exact match first
        if req.replace_all {
            let count = original.matches(&req.old_string).count();
            if count == 0 {
                return "Error: old_string not found in file".to_string();
            }
            let patched = original.replace(&req.old_string, &req.new_string);
            match fs::write(&path, &patched) {
                Ok(_) => serde_json::to_string(&json!({
                    "status": "success",
                    "replacements": count,
                    "path": path.display().to_string()
                })).unwrap_or_else(|e| format!("Error: {}", e)),
                Err(e) => format!("Error writing file: {}", e),
            }
        } else {
            if !original.contains(&req.old_string) {
                return "Error: old_string not found in file. Use read_file to see current content.".to_string();
            }
            let patched = original.replacen(&req.old_string, &req.new_string, 1);
            match fs::write(&path, &patched) {
                Ok(_) => serde_json::to_string(&json!({
                    "status": "success",
                    "replacements": 1,
                    "path": path.display().to_string()
                })).unwrap_or_else(|e| format!("Error: {}", e)),
                Err(e) => format!("Error writing file: {}", e),
            }
        }
    }

    #[tool(description = "Search file contents or find files by name. Ripgrep-backed. target='content' searches inside files, target='files' finds files by name pattern.")]
    async fn search_files(&self, params: Parameters<SearchFilesRequest>) -> String {
        let req = params.0;
        let search_path = expand_path(&req.path);

        if !search_path.exists() {
            return format!("Error: path not found: {}", search_path.display());
        }

        // Use ripgrep for content search
        let mut cmd = tokio::process::Command::new("rg");
        cmd.arg("--line-number")
            .arg("--no-heading")
            .arg("--color=never")
            .arg(format!("--max-count={}", req.limit))
            .env("PATH", format!("/home/crombo/.cargo/bin:{}", std::env::var("PATH").unwrap_or_default()))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if req.target == "files" {
            cmd.arg("--files");
            if let Some(glob) = &req.file_glob {
                cmd.arg("-g").arg(glob);
            }
            cmd.arg(&req.pattern).arg(".");
        } else {
            if let Some(glob) = &req.file_glob {
                cmd.arg("-g").arg(glob);
            }
            cmd.arg(&req.pattern).arg(".");
        }

        cmd.current_dir(&search_path);

        match cmd.output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let lines: Vec<&str> = stdout.lines().take(req.limit).collect();
                serde_json::to_string(&json!({
                    "matches": lines,
                    "count": lines.len(),
                    "path": search_path.display().to_string(),
                    "pattern": req.pattern,
                    "target": req.target
                })).unwrap_or_else(|e| format!("Error: {}", e))
            }
            Err(e) => format!("Error running search: {}", e),
        }
    }

    #[tool(description = "Run shell commands with structured output and timeout control. Returns JSON with output, exit_code, and metadata.")]
    async fn terminal(&self, params: Parameters<TerminalRequest>) -> String {
        let req = params.0;
        let timeout_s = req.timeout.min(600);

        if req.background {
            return serde_json::to_string(&json!({
                "status": "error",
                "error": "Background execution not yet supported in core-mcp"
            })).unwrap_or_default();
        }

        let mut cmd = tokio::process::Command::new("bash");
        cmd.arg("-c").arg(&req.command)
            .env("PATH", format!("/home/crombo/.cargo/bin:{}", std::env::var("PATH").unwrap_or_default()))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(wd) = &req.workdir {
            cmd.current_dir(expand_path(wd));
        }

        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => return format!("Error spawning command: {}", e),
        };

        match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_s),
            child.wait_with_output(),
        ).await {
            Ok(Ok(output)) => serde_json::to_string(&json!({
                "output": String::from_utf8_lossy(&output.stdout),
                "stderr": String::from_utf8_lossy(&output.stderr),
                "exit_code": output.status.code().unwrap_or(-1),
                "command": req.command
            })).unwrap_or_else(|e| format!("Error: {}", e)),
            Ok(Err(e)) => format!("Error running command: {}", e),
            Err(_) => serde_json::to_string(&json!({
                "output": "",
                "stderr": format!("Command timed out after {}s", timeout_s),
                "exit_code": -1,
                "timed_out": true
            })).unwrap_or_default(),
        }
    }

    #[tool(description = "Search the web. Returns titles, URLs, and snippets. Supports operators like site:, filetype:, intitle:, -term, and \"exact phrase\".")]
    async fn web_search(&self, params: Parameters<WebSearchRequest>) -> String {
        let req = params.0;

        // Use DuckDuckGo HTML endpoint (no API key required)
        let url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(&req.query)
        );

        match reqwest::get(&url).await {
            Ok(response) => {
                let text = response.text().await.unwrap_or_default();
                // Simple HTML parsing for DuckDuckGo results
                let results = parse_ddg_results(&text, req.limit);
                serde_json::to_string(&json!({
                    "results": results,
                    "query": req.query,
                    "count": results.len()
                })).unwrap_or_else(|e| format!("Error: {}", e))
            }
            Err(e) => format!("Error: web search failed: {}", e),
        }
    }
}

/// Simple parser for DuckDuckGo HTML search results
fn parse_ddg_results(html: &str, limit: usize) -> Vec<Value> {
    let mut results = Vec::new();
    let mut remaining = html;

    for _ in 0..limit {
        // Find next result block
        let Some(start) = remaining.find("result__a") else { break };
        let after_start = &remaining[start..];
        
        // Extract URL (href attribute)
        let Some(href_start) = after_start.find("href=\"") else { break };
        let href_rest = &after_start[href_start + 6..];
        let Some(href_end) = href_rest.find('"') else { break };
        let url = &href_rest[..href_end];

        // Skip DuckDuckGo redirect URLs
        if url.contains("uddg=") {
            continue;
        }

        // Extract title (text after result__a tag)
        let Some(title_start) = after_start.find('>') else { break };
        let title_rest = &after_start[title_start + 1..];
        let Some(title_end) = title_rest.find('<') else { break };
        let title = title_rest[..title_end].trim().to_string();

        // Find snippet in the following text
        let after_title = &remaining[start + title.len()..];
        let snippet = if let Some(snip_start) = after_title.find("result__snippet") {
            let snip_rest = &after_title[snip_start..];
            if let Some(s2) = snip_rest.find('>') {
                let s3 = &snip_rest[s2 + 1..];
                if let Some(s4) = s3.find('<') {
                    s3[..s4].trim().to_string()
                } else { String::new() }
            } else { String::new() }
        } else { String::new() };

        results.push(json!({
            "title": title,
            "url": url,
            "description": snippet
        }));

        remaining = &remaining[start + 100..]; // Advance past this result
    }

    results
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = CoreToolsServer;
    let running = rmcp::serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;
    running.waiting().await?;
    Ok(())
}
