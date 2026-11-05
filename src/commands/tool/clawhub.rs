//! ClawHub search, install, and scan command handlers.

use crabjar_clawhub_client::ClawHubClient;
use serde_json::json;

/// Search for skills on ClawHub.io registry.
pub async fn handle_clawhub_search(
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
pub async fn handle_clawhub_install(
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

            if db_path.exists()
                && let Ok(conn) = rusqlite::Connection::open(&db_path)
            {
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
                            eprintln!("Warning: Failed to bind tools for installed skill: {}", e);
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
pub fn handle_skill_scan(
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
