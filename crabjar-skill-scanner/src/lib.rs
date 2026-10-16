//! CrabJar Skill Scanner — security scanning pipeline for remote skills.
//!
//! Provides checksum verification, pattern analysis, and risk assessment
//! for skills downloaded from GitHub or ClawHub registries.

pub mod error;
pub mod scanner;
pub mod checksum;
pub mod models;

pub use error::ScannerError;
pub use scanner::SkillScanner;
pub use checksum::{compute_sha256, verify_checksum};
pub use models::{ScanResult, ScanFinding, Severity, Recommendation};