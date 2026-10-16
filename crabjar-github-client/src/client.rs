//! GitHub API client with authentication, rate limiting, and skill discovery.

use std::env;

use chrono::Utc;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use tracing::{debug, info, warn};

use crate::error::GitHubError;
use crate::models::{RepoInfo, SearchOptions, SkillInstallResult};

/// GitHub API base URL.
const GITHUB_API_URL: &str = "https://api.github.com";

/// GitHub client for skill discovery and installation.
#[derive(Clone)]
pub struct GitHubClient {
    http: Client,
    token: Option<String>,
}

impl GitHubClient {
    /// Create a new GitHub client.
    ///
    /// Automatically reads `GITHUB_TOKEN` from environment if set.
    pub fn new() -> Self {
        let token = env::var("GITHUB_TOKEN").ok();
        Self::with_token(token)
    }

    /// Create a client with an explicit token.
    pub fn with_token(token: Option<String>) -> Self {
        let http = Client::builder()
            .user_agent("crabjar-github-client/0.12")
            .build()
            .expect("Failed to build HTTP client");

        Self { http, token }
    }

    /// Check if the client is authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    /// Search for CrabJar skills on GitHub by tag.
    ///
    /// Uses the `crabjar-skill` topic filter and optionally additional query terms.
    pub async fn search_skills(&self, options: SearchOptions) -> Result<Vec<RepoInfo>, GitHubError> {
        let mut query_parts = Vec::new();

        // Always filter by crabjar-skill tag
        if !options.tags.iter().any(|t| t == "crabjar-skill") {
            query_parts.push("topic:crabjar-skill".to_string());
        } else {
            for tag in &options.tags {
                query_parts.push(format!("topic:{}", tag));
            }
        }

        if !options.query.is_empty() {
            query_parts.push(options.query.clone());
        }

        let query = query_parts.join(" ");
        let sort = options.sort_by.to_string();

        debug!("Searching GitHub for skills: {} (sort: {}, limit: {})", query, sort, options.limit);

        // URL-encode the query parameter manually
        let encoded_query = query.replace(" ", "+");
        let url = format!(
            "{}/search/repositories?q={}&sort={}&per_page={}",
            GITHUB_API_URL, encoded_query, sort, options.limit
        );

        let resp = self.get(&url).await?;
        let body: Value = resp.json().await?;

        // Parse search results
        let items = body["items"].as_array().ok_or_else(|| {
            GitHubError::Api {
                code: 0,
                message: "Invalid search response format".to_string(),
            }
        })?;

        let mut repos: Vec<RepoInfo> = Vec::new();
        for item in items {
            if let Ok(repo) = serde_json::from_value(item.clone()) {
                repos.push(repo);
            }
        }

        // Filter by minimum stars if specified
        if let Some(min_stars) = options.min_stars {
            repos.retain(|r| r.stars >= min_stars);
        }

        info!(
            "Found {} CrabJar skills on GitHub (query: {})",
            repos.len(),
            query
        );

        Ok(repos)
    }

    /// Get information about a specific repository.
    pub async fn get_repo(&self, owner: &str, name: &str) -> Result<RepoInfo, GitHubError> {
        let url = format!("{}/repos/{}/{}", GITHUB_API_URL, owner, name);
        let resp = self.get(&url).await?;

        let raw: crate::models::RawRepo = resp.json().await?;
        Ok(raw.into())
    }

    /// Install a skill from a GitHub repository.
    ///
    /// Clones the repo to a temp directory, validates the manifest exists,
    /// and returns the installation result.
    pub async fn install_skill_from_repo(
        &self,
        owner: &str,
        name: &str,
        install_dir: &std::path::Path,
    ) -> Result<SkillInstallResult, GitHubError> {
        let full_name = format!("{}/{}", owner, name);
        info!("Installing skill from {}", full_name);

        // Get repo info first
        let repo_info = self.get_repo(owner, name).await?;

        // Clone to temp directory
        let temp_dir = tempfile::Builder::new()
            .prefix("crabjar-skill-")
            .tempdir()
            .map_err(|e| GitHubError::CloneFailed(e.to_string()))?;

        self.clone_repo(&repo_info.clone_url, temp_dir.path()).await?;

        // Find manifest.json in root or skill/ subdirectory
        let manifest_path = self.find_manifest(temp_dir.path())?;
        info!("Found skill manifest at {}", manifest_path.display());

        // Read manifest to get version
        let manifest_content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| GitHubError::ManifestNotFound(e.to_string()))?;
        let manifest: serde_json::Value = serde_json::from_str(&manifest_content)
            .map_err(|e| GitHubError::ManifestNotFound(e.to_string()))?;

        let version = manifest["version"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        let skill_name = manifest["name"]
            .as_str()
            .unwrap_or(name)
            .to_string();

        // Copy to install directory (simplified - would do actual install logic here)
        let target_dir = install_dir.join(&skill_name);
        std::fs::create_dir_all(&target_dir).map_err(|e| GitHubError::CloneFailed(e.to_string()))?;

        Ok(SkillInstallResult {
            name: skill_name,
            version,
            install_path: target_dir,
            source_repo: full_name,
            trust_tier: repo_info.trust_tier().to_string(),
        })
    }

    /// Make an authenticated GET request.
    async fn get(&self, url: &str) -> Result<reqwest::Response, GitHubError> {
        let mut req = self.http.get(url);

        if let Some(token) = &self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        // Add Accept header for JSON
        req = req.header("Accept", "application/vnd.github.v3+json");

        let resp = req.send().await?;

        // Check rate limit headers
        self.check_rate_limit(&resp).await?;

        match resp.status() {
            StatusCode::OK | StatusCode::CREATED => Ok(resp),
            StatusCode::NOT_FOUND => Err(GitHubError::RepoNotFound(url.to_string())),
            status => Err(GitHubError::Api {
                code: status.as_u16(),
                message: format!("HTTP {}", status),
            }),
        }
    }

    /// Check rate limit headers in response.
    async fn check_rate_limit(&self, resp: &reqwest::Response) -> Result<(), GitHubError> {
        if let Some(retry_after) = resp.headers().get("X-RateLimit-Reset") {
            // Parse reset time and check if we're rate limited
            if let Ok(reset_time) = retry_after.to_str() {
                if let Ok(unix_ts) = reset_time.parse::<i64>() {
                    let now = Utc::now().timestamp();
                    if unix_ts > now {
                        warn!("GitHub rate limit approaching; resets in {}s", unix_ts - now);
                    }
                }
            }
        }

        // Check X-RateLimit-Remaining
        if let Some(remaining) = resp.headers().get("X-RateLimit-Remaining") {
            if let Ok(val) = remaining.to_str() {
                if val == "0" {
                    return Err(GitHubError::RateLimited { retry_after: 3600 });
                }
            }
        }

        Ok(())
    }

    /// Clone a repository to the specified directory.
    async fn clone_repo(
        &self,
        url: &str,
        target: &std::path::Path,
    ) -> Result<(), GitHubError> {
        info!("Cloning {} to {}", url, target.display());

        let output = tokio::process::Command::new("git")
            .args(["clone", "--depth", "1", url, "."])
            .current_dir(target)
            .output()
            .await
            .map_err(|e| GitHubError::CloneFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitHubError::CloneFailed(stderr.to_string()));
        }

        Ok(())
    }

    /// Find manifest.json in the repository root or skill/ subdirectory.
    fn find_manifest(
        &self,
        repo_dir: &std::path::Path,
    ) -> Result<std::path::PathBuf, GitHubError> {
        // Check root first
        let root_manifest = repo_dir.join("manifest.json");
        if root_manifest.exists() {
            return Ok(root_manifest);
        }

        // Check skill/ subdirectory
        let skill_manifest = repo_dir.join("skill").join("manifest.json");
        if skill_manifest.exists() {
            return Ok(skill_manifest);
        }

        Err(GitHubError::ManifestNotFound(repo_dir.display().to_string()))
    }
}
