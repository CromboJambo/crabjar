//! ClawHub.io registry client for skill discovery and installation.

use std::env;
use std::path::Path;

use chrono::Utc;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use tracing::{debug, info};

use crate::error::ClawHubError;
use crate::models::{SearchOptions, SkillInfo, SkillVersion};
use crabjar_skill_scanner::SkillScanner;

/// ClawHub API base URL.
const CLAWHUB_API_URL: &str = "https://clawhub.io/api/v1";

/// ClawHub registry client.
#[derive(Clone)]
pub struct ClawHubClient {
    http: Client,
    token: Option<String>,
}

impl ClawHubClient {
    /// Create a new ClawHub client.
    ///
    /// Automatically reads `CRABJAR_CLAWHUB_TOKEN` from environment if set.
    pub fn new() -> Self {
        let token = env::var("CRABJAR_CLAWHUB_TOKEN").ok();
        Self::with_token(token)
    }

    /// Create a client with an explicit token.
    pub fn with_token(token: Option<String>) -> Self {
        let http = Client::builder()
            .user_agent("crabjar-clawhub-client/0.12")
            .build()
            .expect("Failed to build HTTP client");

        Self { http, token }
    }

    /// Check if the client is authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    /// List all available skills on ClawHub.
    pub async fn list_skills(&self, limit: u32) -> Result<Vec<SkillInfo>, ClawHubError> {
        let url = format!("{}/skills?limit={}", CLAWHUB_API_URL, limit);
        debug!("Listing skills from {}", url);

        let resp = self.get(&url).await?;
        let body: Value = resp.json().await?;

        let items = body["skills"].as_array().ok_or_else(|| {
            ClawHubError::Api("Invalid list response format".to_string())
        })?;

        let mut skills = Vec::new();
        for item in items {
            if let Ok(skill) = serde_json::from_value(item.clone()) {
                skills.push(skill);
            }
        }

        info!("Listed {} skills from ClawHub", skills.len());
        Ok(skills)
    }

    /// Search for skills by query term.
    pub async fn search_skills(&self, options: SearchOptions) -> Result<Vec<SkillInfo>, ClawHubError> {
        let url = format!(
            "{}/skills/search?q={}&limit={}&offset={}",
            CLAWHUB_API_URL,
            options.query,
            options.limit,
            options.offset
        );
        debug!("Searching ClawHub for: {}", options.query);

        let resp = self.get(&url).await?;
        let body: Value = resp.json().await?;

        let items = body["skills"].as_array().ok_or_else(|| {
            ClawHubError::Api("Invalid search response format".to_string())
        })?;

        let mut skills = Vec::new();
        for item in items {
            if let Ok(skill) = serde_json::from_value(item.clone()) {
                skills.push(skill);
            }
        }

        info!("Found {} skills matching '{}'", skills.len(), options.query);
        Ok(skills)
    }

    /// Get information about a specific skill.
    pub async fn get_skill(&self, name: &str) -> Result<SkillInfo, ClawHubError> {
        let url = format!("{}/skills/{}", CLAWHUB_API_URL, name);
        debug!("Getting skill info for {}", name);

        let resp = self.get(&url).await?;
        let body: Value = resp.json().await?;

        serde_json::from_value(body).map_err(|e| ClawHubError::Api(e.to_string()))
    }

    /// Download a specific version of a skill.
    pub async fn download_skill(
        &self,
        name: &str,
        version: Option<&str>,
        target_dir: &Path,
    ) -> Result<std::path::PathBuf, ClawHubError> {
        // Get skill info first
        let skill = self.get_skill(name).await?;

        // Determine version to download
        let version_info = match version {
            Some(v) => skill.get_version(v).ok_or_else(|| {
                ClawHubError::VersionNotFound(format!("{} v{}", name, v))
            })?,
            None => skill.latest().ok_or_else(|| {
                ClawHubError::VersionNotFound(format!("{} (no latest)", name))
            })?,
        };

        info!(
            "Downloading {} v{} from {}",
            name, version_info.version, version_info.download_url
        );

        // Download the skill archive
        let resp = self.http.get(&version_info.download_url).send().await?;

        if !resp.status().is_success() {
            return Err(ClawHubError::DownloadFailed(format!(
                "HTTP {}",
                resp.status()
            )));
        }

        let bytes = resp.bytes().await?;

        // Verify checksum if provided
        if let Some(expected_checksum) = &version_info.checksum {
            let actual = sha256(&bytes);
            if actual != *expected_checksum {
                return Err(ClawHubError::DownloadFailed(format!(
                    "Checksum mismatch: expected {}, got {}",
                    expected_checksum, actual
                )));
            }
        }

        // Save to target directory
        std::fs::create_dir_all(target_dir)?;
        let archive_path = target_dir.join(format!("{}.tar.gz", name));
        std::fs::write(&archive_path, &bytes)?;

        info!("Downloaded {} to {}", name, archive_path.display());

        Ok(archive_path)
    }

    /// Scan a downloaded skill package for security issues.
    pub fn scan_skill(&self, name: &str, version: &str, package_path: &Path) -> Result<crabjar_skill_scanner::models::ScanResult, ClawHubError> {
        let scanner = SkillScanner::new();
        scanner.scan(name, version, package_path).map_err(|e| {
            ClawHubError::Api(format!("Security scan failed: {}", e))
        })
    }

    /// Scan and verify checksum of a downloaded skill package.
    pub fn scan_skill_with_checksum(
        &self,
        name: &str,
        version: &str,
        package_path: &Path,
        expected_checksum: &str,
    ) -> Result<crabjar_skill_scanner::models::ScanResult, ClawHubError> {
        let scanner = SkillScanner::new();
        scanner.scan_with_checksum(name, version, package_path, expected_checksum).map_err(|e| {
            ClawHubError::Api(format!("Security scan failed: {}", e))
        })
    }

    /// Make an authenticated GET request.
    async fn get(&self, url: &str) -> Result<reqwest::Response, ClawHubError> {
        let mut req = self.http.get(url);

        if let Some(token) = &self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let resp = req.send().await?;

        match resp.status() {
            StatusCode::OK | StatusCode::CREATED => Ok(resp),
            StatusCode::NOT_FOUND => Err(ClawHubError::SkillNotFound(url.to_string())),
            status => Err(ClawHubError::Api(format!("HTTP {}", status))),
        }
    }
}

/// Compute SHA-256 hash of data.
fn sha256(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}
