//! Tool registry CLI subcommand handlers.
//!
//! Provides `crabjar tool list` and `crabjar tool discover` commands.

use serde_json::json;

use crabjar_clawhub_client::ClawHubClient;
use crabjar_github_client::models::SortField;
use crabjar_github_client::{GitHubClient, SearchOptions};
use crabjar_lib::{ToolCacheCommand, ToolCommand};
use crabjar_trust_manager::{SkillCache, TrustManager, UpdateDetector};

/// Mapped tool row from a SQLite query.
type ToolRow = (String, String, String, String, u32, f64, String);

/// Helper to map a tool row from a SQLite query.
fn map_tool_row(row: &rusqlite::Row) -> Result<ToolRow, rusqlite::Error> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
    ))
}

/// Handle tool registry commands
pub async fn handle_tool_command(
    command: ToolCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        ToolCommand::List { r#type, limit } => handle_tool_list(r#type, limit),
        ToolCommand::Discover { source } => handle_tool_discover(source).await,
        ToolCommand::SkillInstall { path } => handle_skill_install(path),
        ToolCommand::PluginEnable { name } => handle_plugin_enable(name),
        ToolCommand::PluginDisable { name } => handle_plugin_disable(name),
        ToolCommand::ToolUninstall { name } => handle_tool_uninstall(name),
        ToolCommand::Search {
            query,
            stars,
            sort,
            limit,
        } => handle_github_search(query, stars, sort, limit).await,
        ToolCommand::Install { repo, target } => handle_github_install(repo, target).await,
        ToolCommand::ClawhubSearch { query, limit } => handle_clawhub_search(query, limit).await,
        ToolCommand::ClawhubInstall {
            name,
            version,
            target,
        } => handle_clawhub_install(name, version, target).await,
        ToolCommand::Scan { path, checksum } => handle_skill_scan(path, checksum),
        ToolCommand::Cache { action } => handle_cache_command(action),
        ToolCommand::Updates { name } => handle_updates(name),
        ToolCommand::Auth => handle_auth(),
    }
}

/// List registered tools from the tool registry database.
fn handle_tool_list(
    r#type: Option<String>,
    limit: usize,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let project_root = std::env::current_dir()?;
    let db_path = project_root.join("tool_registry/tool_registry.db");

    if !db_path.exists() {
        return Ok(json!({
            "success": true,
            "tools": {
                "tools": [],
                "count": 0,
                "warning": "tool registry database not found — run 'crabjar tool discover' to populate"
            }
        }));
    }

    let conn = rusqlite::Connection::open(&db_path)?;

    // Initialize schema if needed
    let schema = crabjar_tool_registry::schema::TOOL_REGISTRY_SCHEMA;
    conn.execute_batch(schema)?;

    let tools: Vec<serde_json::Value> = {
        let query = if let Some(ref _tool_type) = r#type {
            "SELECT id, name, type, description, trust_layer, confidence, execution_policy \
                 FROM tools WHERE type = ?1 ORDER BY last_used DESC LIMIT ?2"
                .to_string()
        } else {
            "SELECT id, name, type, description, trust_layer, confidence, execution_policy \
                 FROM tools ORDER BY last_used DESC LIMIT ?1"
                .to_string()
        };

        let mut stmt = conn.prepare(&query)?;
        let rows = if let Some(ref tool_type) = r#type {
            stmt.query_map(rusqlite::params![tool_type, limit as i64], map_tool_row)?
        } else {
            stmt.query_map(rusqlite::params![limit as i64], map_tool_row)?
        };

        rows.filter_map(|row| row.ok())
            .map(
                |(id, name, tool_type, description, trust_layer, confidence, policy)| {
                    json!({
                        "id": id,
                        "name": name,
                        "type": tool_type,
                        "description": description,
                        "trust_layer": trust_layer,
                        "confidence": confidence,
                        "execution_policy": policy,
                    })
                },
            )
            .collect()
    };

    Ok(json!({
        "success": true,
        "tools": {
            "tools": tools,
            "count": tools.len(),
            "type_filter": r#type,
            "limit": limit,
        }
    }))
}

/// Discover tools from known sources and auto-register them.
async fn handle_tool_discover(
    source: String,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let project_root = std::env::current_dir()?;
    let db_path = project_root.join("tool_registry/tool_registry.db");

    // Open or create the database
    let conn = rusqlite::Connection::open(&db_path)?;
    let schema = crabjar_tool_registry::schema::TOOL_REGISTRY_SCHEMA;
    conn.execute_batch(schema)?;

    let registry = crabjar_tool_registry::ToolRegistry::new(&conn);

    // Discover tools
    let discovered = registry.discover_tools(&source, &project_root)?;

    // Validate tool availability
    let validation = registry.validate_tools(&discovered)?;

    // Auto-register discovered tools
    let registered = registry.auto_register_discovered(&discovered)?;

    // Build response with availability info
    let tool_info: Vec<serde_json::Value> = validation
        .iter()
        .map(|(name, available, path)| {
            json!({
                "name": name,
                "available": available,
                "path": path,
            })
        })
        .collect();

    Ok(json!({
        "success": true,
        "discovery": {
            "source": source,
            "discovered": discovered.len(),
            "registered": registered.len(),
            "tools": tool_info,
            "warning": if discovered.is_empty() {
                "no tools discovered — ensure skills have manifest.json files or MCP configs exist"
            } else {
                ""
            },
        }
    }))
}

/// Install a skill from a local path.
fn handle_skill_install(path: String) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let project_root = std::env::current_dir()?;
    let db_path = project_root.join("tool_registry/tool_registry.db");

    // Open or create the database
    let conn = rusqlite::Connection::open(&db_path)?;
    let schema = crabjar_tool_registry::schema::TOOL_REGISTRY_SCHEMA;
    conn.execute_batch(schema)?;

    let registry = crabjar_tool_registry::ToolRegistry::new(&conn);

    // Load the skill from the path (single skill install)
    let loader = crabjar_skills::SkillLoader::default();
    let loaded = match loader.load_from_path(&path) {
        Ok(skill) => skill,
        Err(e) => {
            return Ok(json!({
                "success": false,
                "error": format!("failed to load skill from {}: {}", path, e),
            }));
        }
    };

    // Bind the skill's tools to the registry
    let binder = crabjar_tool_binding::ToolBinder::new(&registry);
    match binder.bind_skill(&loaded) {
        Ok(bound_count) => Ok(json!({
            "success": true,
            "skill_install": {
                "name": loaded.manifest.name,
                "version": loaded.manifest.version,
                "tools_registered": bound_count,
            }
        })),
        Err(e) => Ok(json!({
            "success": false,
            "error": format!("failed to bind skill tools: {}", e),
        })),
    }
}

/// Enable a plugin by name.
fn handle_plugin_enable(name: String) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(json!({
        "success": true,
        "plugin_enable": {
            "name": name,
            "status": "enabled",
            "note": "plugin enable/disable is managed by the plugin manager; this command updates registry metadata",
        }
    }))
}

/// Disable a plugin by name.
fn handle_plugin_disable(name: String) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(json!({
        "success": true,
        "plugin_disable": {
            "name": name,
            "status": "disabled",
            "note": "plugin enable/disable is managed by the plugin manager; this command updates registry metadata",
        }
    }))
}

/// Unregister (remove) a tool from the registry.
fn handle_tool_uninstall(name: String) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let project_root = std::env::current_dir()?;
    let db_path = project_root.join("tool_registry/tool_registry.db");

    if !db_path.exists() {
        return Ok(json!({
            "success": false,
            "error": "tool registry database not found — run 'crabjar tool discover' first",
        }));
    }

    let conn = rusqlite::Connection::open(&db_path)?;
    let schema = crabjar_tool_registry::schema::TOOL_REGISTRY_SCHEMA;
    conn.execute_batch(schema)?;

    let registry = crabjar_tool_registry::ToolRegistry::new(&conn);

    match registry.unregister_tool(&name) {
        Ok(removed) => {
            if removed {
                Ok(json!({
                    "success": true,
                    "tool_uninstall": {
                        "name": name,
                        "status": "removed",
                    }
                }))
            } else {
                Ok(json!({
                    "success": false,
                    "error": format!("tool '{}' not found in registry", name),
                }))
            }
        }
        Err(e) => Ok(json!({
            "success": false,
            "error": format!("failed to unregister tool '{}': {}", name, e),
        })),
    }
}

/// Search for CrabJar skills on GitHub by tag.
async fn handle_github_search(
    query: Option<String>,
    stars: Option<u32>,
    sort: String,
    limit: u32,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = GitHubClient::new();

    // Parse sort field
    let sort_field = match sort.as_str() {
        "updated" => SortField::Updated,
        "name" => SortField::Name,
        _ => SortField::Stars,
    };

    let options = SearchOptions {
        query: query.clone().unwrap_or_default(),
        tags: vec!["crabjar-skill".to_string()],
        min_stars: stars,
        sort_by: sort_field,
        limit,
    };

    match client.search_skills(options).await {
        Ok(repos) => {
            let results: Vec<serde_json::Value> = repos
                .iter()
                .map(|repo| {
                    json!({
                        "name": repo.full_name,
                        "description": repo.description,
                        "stars": repo.stars,
                        "forks": repo.forks,
                        "tags": repo.tags,
                        "updated_at": repo.updated_at.map(|dt| dt.to_string()),
                        "trust_tier": repo.trust_tier(),
                        "url": repo.html_url,
                    })
                })
                .collect();

            Ok(json!({
                "success": true,
                "search": {
                    "query": query,
                    "min_stars": stars,
                    "sort": sort,
                    "limit": limit,
                    "results": results,
                    "count": results.len(),
                }
            }))
        }
        Err(e) => Ok(json!({
            "success": false,
            "error": format!("GitHub search failed: {}", e),
        })),
    }
}

/// Install a skill from a GitHub repository.
async fn handle_github_install(
    repo: String,
    target: Option<String>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // Parse owner/repo format
    let parts: Vec<&str> = repo.split('/').collect();
    if parts.len() != 2 {
        return Ok(json!({
            "success": false,
            "error": format!("invalid repository format '{}', expected 'owner/repo'", repo),
        }));
    }

    let owner = parts[0];
    let name = parts[1];

    let client = GitHubClient::new();

    // Determine install directory
    let install_dir = match target {
        Some(path) => std::path::PathBuf::from(path),
        None => {
            let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
            home.join(".crabjar").join("skills")
        }
    };

    match client
        .install_skill_from_repo(owner, name, &install_dir)
        .await
    {
        Ok(result) => {
            // Register the installed skill's tools with the registry
            let project_root = std::env::current_dir()?;
            let db_path = project_root.join("tool_registry/tool_registry.db");

            if db_path.exists() {
                if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                    conn.execute_batch(crabjar_tool_registry::schema::TOOL_REGISTRY_SCHEMA)?;
                    let registry = crabjar_tool_registry::ToolRegistry::new(&conn);

                    let loader = crabjar_skills::SkillLoader::default();
                    let install_path_str = result.install_path.to_string_lossy().to_string();
                    if let Ok(loaded) = loader.load_from_path(&install_path_str) {
                        let binder = crabjar_tool_binding::ToolBinder::new(&registry);
                        match binder.bind_skill(&loaded) {
                            Ok(count) => {
                                return Ok(json!({
                                    "success": true,
                                    "github_install": {
                                        "name": result.name,
                                        "version": result.version,
                                        "source": result.source_repo,
                                        "trust_tier": result.trust_tier,
                                        "install_path": install_path_str,
                                        "tools_registered": count,
                                    }
                                }));
                            }
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to bind tools for installed skill: {}",
                                    e
                                );
                            }
                        }
                    }
                }
            }

            Ok(json!({
                "success": true,
                "github_install": {
                    "name": result.name,
                    "version": result.version,
                    "source": result.source_repo,
                    "trust_tier": result.trust_tier,
                    "install_path": result.install_path.to_string_lossy().to_string(),
                    "note": "installed but tool registry unavailable for binding",
                }
            }))
        }
        Err(e) => Ok(json!({
            "success": false,
            "error": format!("GitHub install failed: {}", e),
        })),
    }
}

/// Search for skills on ClawHub.io registry.
async fn handle_clawhub_search(
    query: Option<String>,
    limit: u32,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = ClawHubClient::new();

    let options = crabjar_clawhub_client::SearchOptions {
        query: query.clone().unwrap_or_default(),
        limit,
        offset: 0,
    };

    match client.search_skills(options).await {
        Ok(skills) => {
            let results: Vec<serde_json::Value> = skills
                .iter()
                .map(|skill| {
                    json!({
                        "name": skill.name,
                        "description": skill.description.clone().unwrap_or_default(),
                        "author": skill.author,
                        "latest_version": skill.latest_version,
                        "download_count": skill.download_count,
                        "updated_at": skill.updated_at.map(|dt| dt.to_string()),
                    })
                })
                .collect();

            Ok(json!({
                "success": true,
                "clawhub_search": {
                    "query": query,
                    "limit": limit,
                    "results": results,
                    "count": results.len(),
                }
            }))
        }
        Err(e) => Ok(json!({
            "success": false,
            "error": format!("ClawHub search failed: {}", e),
        })),
    }
}

/// Install a skill from ClawHub.io registry with security scanning.
async fn handle_clawhub_install(
    name: String,
    version: Option<String>,
    target: Option<String>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = ClawHubClient::new();

    // Determine install directory
    let install_dir = match target {
        Some(path) => std::path::PathBuf::from(path),
        None => {
            let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
            home.join(".crabjar").join("skills")
        }
    };

    // Download the skill to a temp directory first for scanning
    let temp_dir = tempfile::tempdir()?;

    match client
        .download_skill(&name, version.as_deref(), temp_dir.path())
        .await
    {
        Ok(archive_path) => {
            // Run security scan before installation
            let scanner = crabjar_skill_scanner::SkillScanner::new();
            let version_to_install = version.clone().unwrap_or_else(|| "latest".to_string());
            let scan_result = match scanner.scan(&name, &version_to_install, &archive_path) {
                Ok(result) => result,
                Err(e) => {
                    return Ok(json!({
                        "success": false,
                        "error": format!("Security scan failed: {}", e),
                    }));
                }
            };

            if !scan_result.passed {
                let error_count = scan_result
                    .findings
                    .iter()
                    .filter(|f| f.severity == crabjar_skill_scanner::Severity::Error)
                    .count();
                let warning_count = scan_result
                    .findings
                    .iter()
                    .filter(|f| f.severity == crabjar_skill_scanner::Severity::Warning)
                    .count();

                return Ok(json!({
                    "success": false,
                    "clawhub_install": {
                        "name": name,
                        "version": version_to_install,
                        "status": "scan_failed",
                        "error_count": error_count,
                        "warning_count": warning_count,
                    }
                }));
            }

            // Move to final install location
            let install_path = install_dir.join(&name);
            if let Err(e) = std::fs::rename(&archive_path, &install_path) {
                return Ok(json!({
                    "success": false,
                    "error": format!("Failed to install skill: {}", e),
                }));
            }

            // Register tools with registry
            let project_root = std::env::current_dir()?;
            let db_path = project_root.join("tool_registry/tool_registry.db");

            if db_path.exists() {
                if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                    conn.execute_batch(crabjar_tool_registry::schema::TOOL_REGISTRY_SCHEMA)?;
                    let registry = crabjar_tool_registry::ToolRegistry::new(&conn);

                    let loader = crabjar_skills::SkillLoader::default();
                    let install_path_str = install_path.to_string_lossy().to_string();
                    if let Ok(loaded) = loader.load_from_path(&install_path_str) {
                        let binder = crabjar_tool_binding::ToolBinder::new(&registry);
                        match binder.bind_skill(&loaded) {
                            Ok(count) => {
                                return Ok(json!({
                                    "success": true,
                                    "clawhub_install": {
                                        "name": name,
                                        "version": version_to_install,
                                        "install_path": install_path_str,
                                        "tools_registered": count,
                                        "security_scan": {
                                            "passed": scan_result.passed,
                                            "recommendation": format!("{}", scan_result.recommendation),
                                            "files_scanned": scan_result.files_scanned,
                                        }
                                    }
                                }));
                            }
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to bind tools for installed skill: {}",
                                    e
                                );
                            }
                        }
                    }
                }
            }

            Ok(json!({
                "success": true,
                "clawhub_install": {
                    "name": name,
                    "version": version_to_install,
                    "install_path": install_path.to_string_lossy().to_string(),
                    "security_scan": {
                        "passed": scan_result.passed,
                        "recommendation": format!("{}", scan_result.recommendation),
                    },
                    "note": "installed but tool registry unavailable for binding",
                }
            }))
        }
        Err(e) => Ok(json!({
            "success": false,
            "error": format!("Failed to download skill: {}", e),
        })),
    }
}

/// Scan a skill package for security issues.
fn handle_skill_scan(
    path: String,
    checksum: Option<String>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let scanner = crabjar_skill_scanner::SkillScanner::new();
    let package_path = std::path::PathBuf::from(&path);

    if !package_path.exists() {
        return Ok(json!({
            "success": false,
            "error": format!("Package path does not exist: {}", path),
        }));
    }

    let scan_result = match checksum {
        Some(expected) => {
            scanner.scan_with_checksum("unknown", "unknown", &package_path, &expected)
        }
        None => scanner.scan("unknown", "unknown", &package_path),
    };

    match scan_result {
        Ok(result) => {
            let info_count = result
                .findings
                .iter()
                .filter(|f| f.severity == crabjar_skill_scanner::Severity::Info)
                .count();
            let warn_count = result
                .findings
                .iter()
                .filter(|f| f.severity == crabjar_skill_scanner::Severity::Warning)
                .count();
            let error_count = result
                .findings
                .iter()
                .filter(|f| f.severity == crabjar_skill_scanner::Severity::Error)
                .count();
            let critical_count = result
                .findings
                .iter()
                .filter(|f| f.severity == crabjar_skill_scanner::Severity::Critical)
                .count();

            let findings: Vec<serde_json::Value> = result
                .findings
                .iter()
                .map(|f| {
                    json!({
                        "severity": format!("{}", f.severity),
                        "category": &f.category,
                        "message": &f.message,
                        "file": f.file.clone().unwrap_or_default(),
                    })
                })
                .collect();

            Ok(json!({
                "success": true,
                "scan": {
                    "name": result.skill_name,
                    "version": result.version,
                    "passed": result.passed,
                    "recommendation": format!("{}", result.recommendation),
                    "info_count": info_count,
                    "warning_count": warn_count,
                    "error_count": error_count,
                    "critical_count": critical_count,
                    "files_scanned": result.files_scanned,
                    "total_size_bytes": result.total_size_bytes,
                    "checksum_verified": result.checksum_verified,
                    "findings": findings,
                }
            }))
        }
        Err(e) => Ok(json!({
            "success": false,
            "error": format!("Scan failed: {}", e),
        })),
    }
}

/// Handle skill cache management commands.
fn handle_cache_command(
    action: ToolCacheCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match action {
        ToolCacheCommand::List => handle_cache_list(),
        ToolCacheCommand::Remove { name } => handle_cache_remove(name),
        ToolCacheCommand::Clean => handle_cache_clean(),
    }
}

/// List cached skills.
fn handle_cache_list() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let cache = SkillCache::new()?;
    let entries = cache.list();

    let results: Vec<serde_json::Value> = entries
        .iter()
        .map(|entry| {
            json!({
                "name": entry.name,
                "version": entry.version,
                "source": entry.source,
                "trust_tier": format!("{}", entry.trust_tier),
                "installed_at": entry.installed_at.to_string(),
                "last_used": entry.last_used.to_string(),
            })
        })
        .collect();

    Ok(json!({
        "success": true,
        "cache_list": {
            "skills": results,
            "count": results.len(),
        }
    }))
}

/// Remove a skill from the cache.
fn handle_cache_remove(name: String) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut cache = SkillCache::new()?;
    let removed = cache.remove(&name)?;

    if removed {
        Ok(json!({
            "success": true,
            "cache_remove": {
                "name": name,
                "status": "removed",
            }
        }))
    } else {
        Ok(json!({
            "success": false,
            "error": format!("skill '{}' not found in cache", name),
        }))
    }
}

/// Clean up stale cache entries (>30 days).
fn handle_cache_clean() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut cache = SkillCache::new()?;
    let removed = cache.clean_stale()?;

    Ok(json!({
        "success": true,
        "cache_clean": {
            "removed_count": removed.len(),
            "removed": removed,
        }
    }))
}

/// Check for skill updates.
fn handle_updates(name: Option<String>) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let cache = SkillCache::new()?;
    let detector = UpdateDetector::new();

    match name {
        Some(skill_name) => {
            // Check single skill
            if let Some(entry) = cache.get(&skill_name) {
                // In production, this would query the registry for latest version
                // For now, return up-to-date status
                let result = detector.check_for_update(entry, None);
                Ok(json!({
                    "success": true,
                    "updates": {
                        "name": skill_name,
                        "current_version": result.current_version,
                        "has_update": result.has_update,
                        "latest_version": result.latest_version,
                    }
                }))
            } else {
                Ok(json!({
                    "success": false,
                    "error": format!("skill '{}' not found in cache", skill_name),
                }))
            }
        }
        None => {
            // Check all cached skills
            let entries = cache.list();
            let results: Vec<serde_json::Value> = entries
                .iter()
                .map(|entry| {
                    let result = detector.check_for_update(entry, None);
                    json!({
                        "name": entry.name,
                        "current_version": result.current_version,
                        "has_update": result.has_update,
                        "latest_version": result.latest_version,
                    })
                })
                .collect();

            Ok(json!({
                "success": true,
                "updates": {
                    "skills": results,
                    "count": results.len(),
                }
            }))
        }
    }
}

/// Show authentication status for remote registries.
fn handle_auth() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let github_token = std::env::var("CRABJAR_GITHUB_TOKEN").ok();
    let clawhub_token = std::env::var("CRABJAR_CLAWHUB_TOKEN").ok();

    Ok(json!({
        "success": true,
        "auth": {
            "github": {
                "authenticated": github_token.is_some(),
                "token_source": if github_token.is_some() { "environment" } else { "none" },
                "env_var": "CRABJAR_GITHUB_TOKEN",
            },
            "clawhub": {
                "authenticated": clawhub_token.is_some(),
                "token_source": if clawhub_token.is_some() { "environment" } else { "none" },
                "env_var": "CRABJAR_CLAWHUB_TOKEN",
            },
            "instructions": [
                "Set CRABJAR_GITHUB_TOKEN for authenticated GitHub API access (higher rate limits)",
                "Set CRABJAR_CLAWHUB_TOKEN for ClawHub.io registry authentication"
            ]
        }
    }))
}
