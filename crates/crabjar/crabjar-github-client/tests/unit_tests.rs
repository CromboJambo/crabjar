//! Unit tests for crabjar-github-client models and error handling.

use crabjar_github_client::models::{RepoInfo, SearchOptions};

#[test]
fn test_repo_is_crabjar_skill() {
    let repo = RepoInfo {
        full_name: "owner/skill".to_string(),
        description: Some("A CrabJar skill".to_string()),
        stars: 10,
        forks: 2,
        tags: vec!["crabjar-skill".to_string(), "rust".to_string()],
        updated_at: None,
        clone_url: "https://github.com/owner/skill.git".to_string(),
        html_url: "https://github.com/owner/skill".to_string(),
    };

    assert!(repo.is_crabjar_skill());
}

#[test]
fn test_repo_not_crabjar_skill() {
    let repo = RepoInfo {
        full_name: "owner/random-repo".to_string(),
        description: None,
        stars: 5,
        forks: 1,
        tags: vec!["rust".to_string()],
        updated_at: None,
        clone_url: "https://github.com/owner/random-repo.git".to_string(),
        html_url: "https://github.com/owner/random-repo".to_string(),
    };

    assert!(!repo.is_crabjar_skill());
}

#[test]
fn test_trust_tier_community() {
    let repo = RepoInfo {
        full_name: "owner/community-skill".to_string(),
        description: None,
        stars: 3,
        forks: 0,
        tags: vec!["crabjar-skill".to_string()],
        updated_at: None,
        clone_url: "https://github.com/owner/community-skill.git".to_string(),
        html_url: "https://github.com/owner/community-skill".to_string(),
    };

    assert_eq!(repo.trust_tier(), "community");
}

#[test]
fn test_trust_tier_verified() {
    let repo = RepoInfo {
        full_name: "owner/verified-skill".to_string(),
        description: None,
        stars: 50,
        forks: 10,
        tags: vec![
            "crabjar-skill".to_string(),
            "crabjar-skill-verified".to_string(),
        ],
        updated_at: None,
        clone_url: "https://github.com/owner/verified-skill.git".to_string(),
        html_url: "https://github.com/owner/verified-skill".to_string(),
    };

    assert_eq!(repo.trust_tier(), "verified");
}

#[test]
fn test_trust_tier_official() {
    let repo = RepoInfo {
        full_name: "crabjar/official-skill".to_string(),
        description: None,
        stars: 100,
        forks: 25,
        tags: vec![
            "crabjar-skill".to_string(),
            "crabjar-skill-official".to_string(),
        ],
        updated_at: None,
        clone_url: "https://github.com/crabjar/official-skill.git".to_string(),
        html_url: "https://github.com/crabjar/official-skill".to_string(),
    };

    assert_eq!(repo.trust_tier(), "official");
}

#[test]
fn test_search_options_default() {
    let options = SearchOptions::default();
    assert_eq!(options.tags, vec!["crabjar-skill"]);
    assert_eq!(options.sort_by.to_string(), "stars");
    assert_eq!(options.limit, 20);
}

#[test]
fn test_search_options_custom() {
    let options = SearchOptions {
        query: "file operations".to_string(),
        tags: vec!["crabjar-skill".to_string(), "filesystem".to_string()],
        min_stars: Some(10),
        sort_by: crabjar_github_client::models::SortField::Updated,
        limit: 5,
    };

    assert_eq!(options.query, "file operations");
    assert_eq!(options.min_stars, Some(10));
    assert_eq!(options.sort_by.to_string(), "updated");
}
