//! DuckDuckGo Instant Answers client implementation.
//!
//! Uses DuckDuckGo's Instant Answers API as a fallback when Brave Search
//! API is not available. No API key required for basic usage.

use crate::error::WebSearchError;
use crate::models::{SearchResult, SearchResponse};
use reqwest::Client;
use urlencoding::encode;

pub struct DuckDuckGoBackend {
    client: Client,
}

impl DuckDuckGoBackend {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("crabjar-web-search/0.1")
            .build()
            .expect("Failed to build reqwest client");
        
        Self { client }
    }

    pub async fn search(&self, query: &str, _limit: usize) -> Result<SearchResponse, WebSearchError> {
        let encoded_query = encode(query);
        let url = format!(
            "https://api.duckduckgo.com/InstantAnswer.js?q={}&format=json",
            encoded_query
        );

        let response = self.client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| WebSearchError::ApiRequestFailed(format!("DuckDuckGo request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(WebSearchError::ApiRequestFailed(format!(
                "DuckDuckGo returned status {}", status
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            WebSearchError::MalformedResponse(format!("Failed to parse DuckDuckGo JSON: {}", e))
        })?;

        // Parse DuckDuckGo Instant Answers response format
        let results = parse_ddg_response(&json)?;

        Ok(SearchResponse {
            query: query.to_string(),
            results,
            total_results: json.get("results")
                .and_then(|r| r.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0),
        })
    }
}

fn parse_ddg_response(json: &serde_json::Value) -> Result<Vec<SearchResult>, WebSearchError> {
    let results = json.get("results")
        .ok_or_else(|| WebSearchError::MalformedResponse("No 'results' field in DuckDuckGo response".to_string()))?;

    let mut search_results = Vec::new();

    for result in results.as_array().ok_or_else(|| {
        WebSearchError::MalformedResponse("'results' is not an array".to_string())
    })? {
        let title = result.get("title")
            .and_then(|t| t.as_str())
            .ok_or_else(|| WebSearchError::MalformedResponse("Result missing 'title'".to_string()))?;

        let url = result.get("url")
            .and_then(|u| u.as_str())
            .ok_or_else(|| WebSearchError::MalformedResponse("Result missing 'url'".to_string()))?;

        let description = result.get("description")
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());

        search_results.push(SearchResult {
            url: url.to_string(),
            title: title.to_string(),
            description,
            snippet: None,
        });
    }

    Ok(search_results)
}