//! Brave Search API client implementation.

use crate::error::WebSearchError;
use crate::models::{SearchResult, SearchResponse};
use reqwest::Client;
use urlencoding::encode;

pub struct BraveBackend {
    api_key: String,
    client: Client,
}

impl BraveBackend {
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .user_agent("crabjar-web-search/0.1")
            .build()
            .expect("Failed to build reqwest client");
        
        Self { api_key, client }
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<SearchResponse, WebSearchError> {
        let encoded_query = encode(query);
        let url = format!(
            "https://api.search.brave.com/res/search/web/?q={}&count={}",
            encoded_query, limit
        );

        let response = self.client
            .get(&url)
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| WebSearchError::ApiRequestFailed(format!("Brave Search request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.map_err(|e| {
                WebSearchError::ApiRequestFailed(format!("Failed to read Brave Search response: {}", e))
            })?;
            return Err(WebSearchError::ApiRequestFailed(format!(
                "Brave Search returned status {}: {}", status, body
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            WebSearchError::MalformedResponse(format!("Failed to parse Brave Search JSON: {}", e))
        })?;

        // Parse Brave Search API response format
        let results = parse_brave_response(&json)?;

        Ok(SearchResponse {
            query: query.to_string(),
            results,
            total_results: json.get("results")
                .and_then(|r| r.get("total"))
                .and_then(|t| t.as_i64())
                .unwrap_or(0) as usize,
        })
    }
}

fn parse_brave_response(json: &serde_json::Value) -> Result<Vec<SearchResult>, WebSearchError> {
    let results = json.get("results")
        .ok_or_else(|| WebSearchError::MalformedResponse("No 'results' field in Brave Search response".to_string()))?;

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