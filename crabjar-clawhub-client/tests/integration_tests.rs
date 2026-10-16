//! Integration tests for CrabJar ClawHub Client using mock registry server

use crabjar_clawhub_client::{ClawHubClient, models::SearchOptions};
use crabjar_test_registry::{MockRegistryServer, fixtures::basic_scenario};

#[tokio::test]
async fn test_list_skills() {
    let server = MockRegistryServer::start(Some(basic_scenario())).await;
    
    let client = ClawHubClient::with_base_url(server.url(), None);
    let skills = client.list_skills(10).await.unwrap();

    assert!(!skills.is_empty());
    assert!(skills.iter().any(|s| s.name == "hello-world"));
    
    server.stop().await;
}

#[tokio::test]
async fn test_search_skills() {
    let server = MockRegistryServer::start(Some(basic_scenario())).await;
    
    let client = ClawHubClient::with_base_url(server.url(), None);
    let skills = client.search_skills(SearchOptions {
        query: "weather".to_string(),
        limit: 10,
        offset: 0,
    }).await.unwrap();

    assert!(!skills.is_empty());
    assert!(skills.iter().any(|s| s.name == "weather"));
    
    server.stop().await;
}

#[tokio::test]
async fn test_get_skill_by_name() {
    let server = MockRegistryServer::start(Some(basic_scenario())).await;
    
    let client = ClawHubClient::with_base_url(server.url(), None);
    let skill = client.get_skill("hello-world").await.unwrap();

    assert_eq!(skill.name, "hello-world");
    
    server.stop().await;
}

#[tokio::test]
async fn test_skill_not_found() {
    let server = MockRegistryServer::start(Some(basic_scenario())).await;
    
    let client = ClawHubClient::with_base_url(server.url(), None);
    let result = client.get_skill("nonexistent-skill").await;

    assert!(result.is_err());
    
    server.stop().await;
}