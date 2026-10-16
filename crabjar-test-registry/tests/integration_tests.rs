//! Integration tests for the mock registry server.
//!
//! These tests spin up a real mock server and make HTTP requests to it
//! to verify all endpoints work correctly.

use crabjar_test_registry::fixtures::{basic_scenario, GitHubRepo};
use crabjar_test_registry::MockRegistryServer;

#[tokio::test]
async fn test_list_skills() {
    let scenario = basic_scenario();
    let server = MockRegistryServer::start(Some(scenario)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/skills", server.url()))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["total"], 3);
    assert_eq!(body["skills"].as_array().unwrap().len(), 3);

    server.stop().await;
}

#[tokio::test]
async fn test_search_skills() {
    let scenario = basic_scenario();
    let server = MockRegistryServer::start(Some(scenario)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/skills/search?q=hello", server.url()))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["total"], 1);
    assert_eq!(body["skills"][0]["name"], "hello-world");

    server.stop().await;
}

#[tokio::test]
async fn test_get_skill() {
    let scenario = basic_scenario();
    let server = MockRegistryServer::start(Some(scenario)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/skills/hello-world", server.url()))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["name"], "hello-world");
    assert_eq!(body["latest_version"], "1.0.0");

    server.stop().await;
}

#[tokio::test]
async fn test_get_skill_not_found() {
    let scenario = basic_scenario();
    let server = MockRegistryServer::start(Some(scenario)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/skills/nonexistent", server.url()))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 404);

    server.stop().await;
}

#[tokio::test]
async fn test_download_file() {
    let scenario = basic_scenario();
    let server = MockRegistryServer::start(Some(scenario)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/downloads/hello-world.tar.gz", server.url()))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert!(body.contains("hello-world"));

    server.stop().await;
}

#[tokio::test]
async fn test_github_search_repos() {
    let mut scenario = basic_scenario();
    scenario.add_github_repo(GitHubRepo {
        owner: "test-org".to_string(),
        name: "test-skill".to_string(),
        description: "A test skill repo".to_string(),
        stars: 10,
        manifest: Some(r#"{"name":"test-skill","version":"1.0.0"}"#.to_string()),
    });

    let server = MockRegistryServer::start(Some(scenario)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/search/repositories?q=test-skill", server.url()))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["total_count"], 1);
    assert_eq!(body["items"][0]["full_name"], "test-org/test-skill");

    server.stop().await;
}

#[tokio::test]
async fn test_github_get_manifest() {
    let mut scenario = basic_scenario();
    scenario.add_github_repo(GitHubRepo {
        owner: "test-org".to_string(),
        name: "manifest-skill".to_string(),
        description: "A skill with manifest".to_string(),
        stars: 5,
        manifest: Some(r#"{"name":"manifest-skill","version":"2.0.0"}"#.to_string()),
    });

    let server = MockRegistryServer::start(Some(scenario)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/repos/test-org/manifest-skill/contents/manifest.json", server.url()))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["content"], r#"{"name":"manifest-skill","version":"2.0.0"}"#);

    server.stop().await;
}

#[tokio::test]
async fn test_dynamic_scenario_update() {
    let server = MockRegistryServer::start(None).await;

    // Initially no skills
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/skills", server.url()))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["total"], 0);

    // Update scenario with skills
    let scenario = basic_scenario();
    server.set_scenario(Some(scenario)).await;

    // Now should have skills
    let response = client
        .get(format!("{}/api/v1/skills", server.url()))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["total"], 3);

    server.stop().await;
}