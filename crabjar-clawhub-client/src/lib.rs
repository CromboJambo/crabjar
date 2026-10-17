//! # crabjar-clawhub-client
//!
//! ClawHub.io registry client for CrabJar skill discovery and installation.
//! Supports skill listing, searching, downloading with version pinning,
//! and security scanning integration via crabjar-skill-scanner.

pub mod client;
pub mod error;
pub mod models;

use crabjar_skill_scanner::SkillScanner;

pub use client::ClawHubClient;
pub use error::ClawHubError;
pub use models::{SearchOptions, SkillInfo, SkillVersion};
