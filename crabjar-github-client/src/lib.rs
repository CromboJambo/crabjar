//! # crabjar-github-client
//!
//! GitHub API client for CrabJar skill discovery and installation.
//! Supports authenticated/unauthenticated requests, rate limiting,
//! repository search by tags, and skill installation from repos.

pub mod client;
pub mod error;
pub mod models;

pub use client::GitHubClient;
pub use error::GitHubError;
pub use models::{RepoInfo, SearchOptions, SkillInstallResult};
