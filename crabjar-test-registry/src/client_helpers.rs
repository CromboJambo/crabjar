//! Test client helpers for integration tests.
//!
//! Provides convenient functions for testing against the mock registry server.

/// Build a ClawHub API URL from the mock server base.
pub fn clawhub_url(base: &str, path: &str) -> String {
    format!("{}{}", base, path)
}

/// Build a GitHub API URL from the mock server base.
pub fn github_url(base: &str, path: &str) -> String {
    format!("{}{}", base, path)
}

/// Build a download URL from the mock server base.
pub fn download_url(base: &str, filename: &str) -> String {
    format!("{}/downloads/{}", base, filename)
}