//! CrabJar Skill Scanner — security scanning pipeline for remote skills.
//!
//! Provides checksum verification, pattern analysis, and risk assessment
//! for skills downloaded from GitHub or ClawHub registries.

pub mod checksum;
pub mod error;
pub mod models;
pub mod scanner;

pub use checksum::{compute_sha256, verify_checksum};
pub use error::ScannerError;
pub use models::{Recommendation, ScanFinding, ScanResult, Severity};
pub use scanner::SkillScanner;
