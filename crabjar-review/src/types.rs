use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Classification of a changed file for rule matching.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Source,
    Test,
    Config,
    Docs,
    Generated,
    Binary,
    Unknown,
}

impl FileType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FileType::Source => "source",
            FileType::Test => "test",
            FileType::Config => "config",
            FileType::Docs => "docs",
            FileType::Generated => "generated",
            FileType::Binary => "binary",
            FileType::Unknown => "unknown",
        }
    }
}

/// A single changed file with its classification and diff metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedFile {
    pub path: PathBuf,
    pub file_type: FileType,
    pub status: FileStatus,
    pub additions: u32,
    pub deletions: u32,
}

/// Git index entry status for a changed file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
}

impl FileStatus {
    pub fn from_git_status(status: git2::Delta) -> Self {
        match status {
            git2::Delta::Modified => FileStatus::Modified,
            git2::Delta::Added => FileStatus::Added,
            git2::Delta::Deleted => FileStatus::Deleted,
            _ => FileStatus::Modified,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            FileStatus::Modified => "M",
            FileStatus::Added => "A",
            FileStatus::Deleted => "D",
            FileStatus::Renamed => "R",
        }
    }
}

/// A bundle of related files to be reviewed together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewBundle {
    pub id: String,
    pub files: Vec<ChangedFile>,
    pub crate_name: Option<String>,
}

impl ReviewBundle {
    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

/// A review rule loaded from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRule {
    pub id: String,
    pub applies_to: Vec<String>,
    #[serde(default)]
    pub not_applies_to: Vec<String>,
    pub severity: Severity,
    pub description: String,
}

/// Rule severity level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    #[serde(rename = "high")]
    High,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "low")]
    Low,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
        }
    }
}

/// Configuration for the review pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewConfig {
    pub model: String,
    pub model_endpoint: String,
    pub max_bundles_parallel: u32,
    pub bundle_size_limit_kb: u32,
    pub context_retrieval_limit: u32,
    pub position_tolerance_lines: i32,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            model: "Qwen3.8-27B-TURBO-Fable".to_string(),
            model_endpoint: "http://localhost:8091".to_string(),
            max_bundles_parallel: 4,
            bundle_size_limit_kb: 500,
            context_retrieval_limit: 10,
            position_tolerance_lines: 2,
        }
    }
}
