//! # crabjar-clawhub-client
//!
//! ClawHub.io registry client for CrabJar skill discovery and installation.
//! Supports skill listing, searching, downloading with version pinning,
//! and security scanning integration via crabjar-skill-scanner.

pub mod error;
pub mod models;
pub mod client;

use crabjar_skill_scanner::SkillScanner;

pub use client::ClawHubClient;
pub use error::ClawHubError;
pub use models::{SkillInfo, SkillVersion, SearchOptions};