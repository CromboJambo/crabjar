//! # crabjar-clawhub-client
//!
//! ClawHub.io registry client for CrabJar skill discovery and installation.
//! Supports skill listing, searching, downloading with version pinning,
//! and security scanning integration.

pub mod error;
pub mod models;
pub mod client;
pub mod scanner;

pub use client::ClawHubClient;
pub use error::ClawHubError;
pub use models::{SkillInfo, SkillVersion, SearchOptions};
pub use scanner::{SecurityScanResult, SecurityScanner};
