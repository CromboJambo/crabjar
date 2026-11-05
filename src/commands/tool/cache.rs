//! Skill cache management command handlers.

use crabjar_trust_manager::{SkillCache, UpdateDetector};
use serde_json::json;

/// Handle skill cache management commands.
pub fn handle_cache_command(
    action: crabjar_lib::ToolCacheCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match action {
        crabjar_lib::ToolCacheCommand::List => handle_cache_list(),
        crabjar_lib::ToolCacheCommand::Remove { name } => handle_cache_remove(name),
        crabjar_lib::ToolCacheCommand::Clean => handle_cache_clean(),
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
pub fn handle_updates(
    name: Option<String>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
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
pub fn handle_auth() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
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
