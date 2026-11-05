//! SearXNG JSON API client for crabjar web search.
//!
//! Uses a self-hosted SearXNG instance as the primary search backend.
//! The instance URL is configured via the SEARXNG_URL environment variable.

use reqwest::Client;
use serde_json::Value;

use super::error::WebSearchError;
use super::models::{SearchResponse, SearchResult};

const DEFAULT_SEARXNG_URL: &str = "http://localhost:8080";

pub struct SearxngBackend {
    client: Client,
    base_url: String,
}

impl SearxngBackend {
    pub fn new() -> Self {
        let base_url =
            std::env::var("SEARXNG_URL").unwrap_or_else(|_| DEFAULT_SEARXNG_URL.to_string());

        // Remove trailing slash for consistent URL construction
        let base_url = base_url.trim_end_matches('/').to_string();

        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("crabjar-web-search/1.0")
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url,
        }
    }

    pub async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<SearchResponse, WebSearchError> {
        let url = format!(
            "{}/search?q={}&format=json&limit={}",
            self.base_url,
            urlencoding::encode(query),
            limit
        );

        let response = self.client.get(&url).send().await.map_err(|e| {
            WebSearchError::ApiRequestFailed(format!("SearXNG request failed: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(WebSearchError::ApiRequestFailed(format!(
                "SearXNG returned status {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let json: Value = response.json().await.map_err(|e| {
            WebSearchError::MalformedResponse(format!("Failed to parse SearXNG JSON: {}", e))
        })?;

        self.parse_results(json, query, limit)
    }

    fn parse_results(
        &self,
        json: Value,
        query: &str,
        limit: usize,
    ) -> Result<SearchResponse, WebSearchError> {
        let results_array = json
            .get("results")
            .and_then(|r| r.as_array())
            .ok_or_else(|| {
                WebSearchError::MalformedResponse(
                    "Missing 'results' array in SearXNG response".to_string(),
                )
            })?;

        let mut results = Vec::new();
        for (i, item) in results_array.iter().enumerate() {
            if i >= limit {
                break;
            }

            let title = item
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();

            let url = item
                .get("url")
                .and_then(|u| u.as_str())
                .unwrap_or("")
                .to_string();

            let content = item.get("content").and_then(|c| c.as_str()).unwrap_or("");

            // Truncate long descriptions
            let description = if content.len() > 300 {
                format!("{}...", &content[..300])
            } else {
                content.to_string()
            };

            results.push(SearchResult {
                url,
                title,
                description: Some(description),
                snippet: None,
            });
        }

        let total = results.len();
        Ok(SearchResponse {
            query: query.to_string(),
            results,
            total_results: total,
        })
    }
}
