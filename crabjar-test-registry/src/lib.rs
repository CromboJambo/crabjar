//! # crabjar-test-registry
//!
//! Mock HTTP registry server for integration testing of CrabJar's remote skill discovery pipeline.
//! Provides controlled test scenarios without network dependencies.
//!
//! ## Features
//! - ClawHub-compatible API endpoints
//! - GitHub-compatible API endpoints
//! - Programmatic fixture loading
//! - Random port allocation
//! - Scenario-based test data

pub mod server;
pub mod fixtures;
pub mod client_helpers;

// Re-export key types for convenience
pub use server::MockRegistryServer;
pub use fixtures::{TestSkill, TestScenario};