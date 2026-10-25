//! Integration tests for crabjar-web-search crate.

use crabjar_web_search::search;

#[tokio::test]
async fn test_search_returns_results() {
    // This test requires BRAVE_SEARCH_API_KEY to be set
    if std::env::var("BRAVE_SEARCH_API_KEY").is_err() {
        println!("Skipping test: BRAVE_SEARCH_API_KEY not set");
        return;
    }

    let response = search("crabjar rust agent", Some(5)).await.unwrap();
    
    assert_eq!(response.query, "crabjar rust agent");
    assert!(!response.results.is_empty(), "Expected at least one result");
    
    for result in &response.results {
        assert!(!result.url.is_empty());
        assert!(!result.title.is_empty());
    }
}

#[tokio::test]
async fn test_search_with_default_limit() {
    // This test requires BRAVE_SEARCH_API_KEY to be set
    if std::env::var("BRAVE_SEARCH_API_KEY").is_err() {
        println!("Skipping test: BRAVE_SEARCH_API_KEY not set");
        return;
    }

    let response = search("crabjar rust agent", None).await.unwrap();
    
    assert_eq!(response.query, "crabjar rust agent");
    // Default limit is 5
    assert!(response.results.len() <= 5);
}