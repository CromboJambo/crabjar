// crabjar/tool_registry/src/discovery.rs
// Four-layer tool discovery — scans skill directories, MCP manifests, and state-docs.

use crate::error::ToolRegistryError;
use tracing::debug;

/// Discover tools from a source by scanning skill directories, MCP manifests, and state-docs.
///
/// Scans four layers:
/// 1. Project-level skill directories (`.agents/skills/*/manifest.json`)
/// 2. User-level skill directories (`~/.corust-agent/skills`, `~/.agents/skills`)
/// 3. MCP server configurations (`~/.config/mcp/`, `~/.config/crabjar/mcp/`)
/// 4. State-docs registered tools
pub fn discover_tools(
    source: &str,
    project_root: &std::path::Path,
) -> Result<Vec<String>, ToolRegistryError> {
    let mut discovered = Vec::new();

    // Layer 1: Scan project-level skill directories for tool definitions
    for ancestor in project_root.ancestors() {
        let candidate = ancestor.join(".agents/skills");
        if candidate.is_dir()
            && let Ok(entries) = std::fs::read_dir(&candidate)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && let Ok(content) = std::fs::read_to_string(path.join("manifest.json"))
                    && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content)
                    && let Some(tools) = parsed["tools"].as_array()
                {
                    for tool in tools {
                        if let Some(name) = tool["name"].as_str()
                            && !discovered.contains(&name.to_string())
                        {
                            discovered.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    // Layer 2: Scan user-level skill directories
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
    for scope in [".corust-agent/skills", ".agents/skills"] {
        let candidate = std::path::Path::new(&home_dir).join(scope);
        if candidate.is_dir()
            && let Ok(entries) = std::fs::read_dir(&candidate)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && path.join("manifest.json").exists()
                    && let Ok(content) = std::fs::read_to_string(path.join("manifest.json"))
                    && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content)
                    && let Some(tools) = parsed["tools"].as_array()
                {
                    for tool in tools {
                        if let Some(name) = tool["name"].as_str()
                            && !discovered.contains(&name.to_string())
                        {
                            discovered.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    // Layer 3: Scan MCP server configurations
    for mcp_dir in [
        std::path::Path::new(&home_dir).join(".config/mcp"),
        std::path::Path::new(&home_dir).join(".config/crabjar/mcp"),
    ] {
        if mcp_dir.is_dir()
            && let Ok(entries) = std::fs::read_dir(&mcp_dir)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file()
                    && path.extension().is_some_and(|ext| ext == "json")
                    && let Ok(content) = std::fs::read_to_string(&path)
                    && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content)
                {
                    // MCP config may have a "tools" array or "command"/"args" fields
                    if let Some(tools) = parsed["tools"].as_array() {
                        for tool in tools {
                            if let Some(name) = tool["name"].as_str()
                                && !discovered.contains(&name.to_string())
                            {
                                discovered.push(name.to_string());
                            }
                        }
                    } else if let Some(cmd) = parsed["command"].as_str()
                        && let Some(args) = parsed["args"].as_array()
                    {
                        let tool_name = format!(
                            "{}-{}",
                            cmd.rsplit('/').next().unwrap_or("mcp"),
                            args.first().and_then(|a| a.as_str()).unwrap_or("server")
                        );
                        if !discovered.contains(&tool_name) {
                            discovered.push(tool_name);
                        }
                    }
                }
            }
        }
    }

    // Layer 4: Discover tools from state-docs annotations
    let state_docs_dir = project_root.join("state-docs");
    if state_docs_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&state_docs_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().is_some_and(|ext| ext == "md")
                && path
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("tool_"))
                && let Some(name) = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.trim_start_matches("tool_").to_string())
                    .filter(|n| !n.is_empty())
                && !discovered.contains(&name)
            {
                discovered.push(name);
            }
        }
    }

    debug!(
        source = %source,
        tool_count = discovered.len(),
        "Tool registry: tools discovered"
    );

    Ok(discovered)
}
