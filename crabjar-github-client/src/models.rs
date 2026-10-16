//! Data models for GitHub API interactions.

use serde::{Deserialize, Serialize};
use chrono::DateTime;
use std::path::PathBuf;

/// Repository information returned by GitHub API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    pub full_name: String,
    pub description: Option<String>,
    pub stars: u32,
    pub forks: u32,
    pub tags: Vec<String>,
    pub updated_at: Option<DateTime<chrono::Utc>>,
    pub clone_url: String,
    pub html_url: String,
}

impl RepoInfo {
    /// Check if this repo has the crabjar-skill tag.
    pub fn is_crabjar_skill(&self) -> bool {
        self.tags.iter().any(|t| t == "crabjar-skill")
    }

    /// Get trust tier based on tags.
    pub fn trust_tier(&self) -> &'static str {
        if self.tags.iter().any(|t| t == "crabjar-skill-official") {
            "official"
        } else if self.tags.iter().any(|t| t == "crabjar-skill-verified") {
            "verified"
        } else {
            "community"
        }
    }
}

/// Options for searching repositories.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub query: String,
    pub tags: Vec<String>,
    pub min_stars: Option<u32>,
    pub sort_by: SortField,
    pub limit: u32,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            query: String::new(),
            tags: vec!["crabjar-skill".to_string()],
            min_stars: None,
            sort_by: SortField::Stars,
            limit: 20,
        }
    }
}

/// Sort field for search results.
#[derive(Debug, Clone, Default)]
pub enum SortField {
    #[default]
    Stars,
    Updated,
    Name,
}

impl std::fmt::Display for SortField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortField::Stars => write!(f, "stars"),
            SortField::Updated => write!(f, "updated"),
            SortField::Name => write!(f, "name"),
        }
    }
}

/// Result of a skill installation.
#[derive(Debug)]
pub struct SkillInstallResult {
    pub name: String,
    pub version: String,
    pub install_path: PathBuf,
    pub source_repo: String,
    pub trust_tier: String,
}

impl std::fmt::Display for SkillInstallResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Installed skill '{}' v{} from {} ({})",
            self.name, self.version, self.source_repo, self.trust_tier
        )
    }
}

/// Raw GitHub search result item.
#[derive(Debug, Deserialize)]
pub struct RawRepo {
    full_name: String,
    description: Option<String>,
    stargazers_count: u32,
    forks_count: u32,
    updated_at: Option<DateTime<chrono::Utc>>,
    clone_url: String,
    html_url: String,
    topics: Vec<String>,
}

impl From<RawRepo> for RepoInfo {
    fn from(raw: RawRepo) -> Self {
        Self {
            full_name: raw.full_name,
            description: raw.description,
            stars: raw.stargazers_count,
            forks: raw.forks_count,
            tags: raw.topics,
            updated_at: raw.updated_at,
            clone_url: raw.clone_url,
            html_url: raw.html_url,
        }
    }
}
