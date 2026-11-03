//! Unit tests for CrabJar ClawHub Client

use chrono::Utc;
use crabjar_clawhub_client::models::{SearchOptions, SkillInfo, SkillVersion};

#[test]
fn test_skill_info_latest() {
    let skill = SkillInfo {
        name: "test-skill".to_string(),
        description: Some("Test skill".to_string()),
        author: "author".to_string(),
        versions: vec![
            SkillVersion {
                version: "1.0.0".to_string(),
                download_url: "https://example.com/1.0.0.tar.gz".to_string(),
                checksum: None,
                published_at: Some(Utc::now()),
            },
            SkillVersion {
                version: "2.0.0".to_string(),
                download_url: "https://example.com/2.0.0.tar.gz".to_string(),
                checksum: None,
                published_at: Some(Utc::now()),
            },
        ],
        latest_version: "2.0.0".to_string(),
        download_count: 100,
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
    };

    assert_eq!(skill.latest().unwrap().version, "2.0.0");
}

#[test]
fn test_skill_info_get_version() {
    let skill = SkillInfo {
        name: "test-skill".to_string(),
        description: Some("Test skill".to_string()),
        author: "author".to_string(),
        versions: vec![SkillVersion {
            version: "1.0.0".to_string(),
            download_url: "https://example.com/1.0.0.tar.gz".to_string(),
            checksum: None,
            published_at: Some(Utc::now()),
        }],
        latest_version: "1.0.0".to_string(),
        download_count: 50,
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
    };

    assert!(skill.get_version("1.0.0").is_some());
    assert!(skill.get_version("9.9.9").is_none());
}

#[test]
fn test_search_options_default() {
    let opts = SearchOptions::default();
    assert_eq!(opts.limit, 20);
    assert_eq!(opts.offset, 0);
    assert!(opts.query.is_empty());
}

#[test]
fn test_search_options_custom() {
    let opts = SearchOptions {
        query: "weather".to_string(),
        limit: 50,
        offset: 10,
    };
    assert_eq!(opts.query, "weather");
    assert_eq!(opts.limit, 50);
    assert_eq!(opts.offset, 10);
}
