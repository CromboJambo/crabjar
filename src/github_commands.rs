//! GitHub skill search and install handlers.

use serde_json::json;

use crabjar_github_client::models::SortField;
use crabjar_github_client::{GitHubClient, SearchOptions};

/// Search for CrabJar skills on GitHub by tag.
pub async fn handle_github_search(
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
pub async fn handle_github_install(
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

            if db_path.exists()
                && let Ok(conn) = rusqlite::Connection::open(&db_path) {
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
