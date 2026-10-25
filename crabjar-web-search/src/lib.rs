//! Web search tooling for crabjar agents.
//!
//! Provides web search functionality via Brave Search API (primary) and
//! DuckDuckGo Instant Answers (fallback). Integrates with orchestrator tool
//! calling infrastructure via `/acp/chat` endpoint.

use std::env;

mod brave_client;
mod duckduckgo_client;
mod error;
mod models;
mod rate_limiter;

pub use error::WebSearchError;
pub use models::{SearchResult, SearchResponse};

/// Select search backend based on API key availability.
fn select_backend() -> Result<SearchBackend, WebSearchError> {
    if let Some(key) = env::var("BRAVE_SEARCH_API_KEY").ok() {
        Ok(SearchBackend::Brave(brave_client::BraveBackend::new(key)))
    } else {
        tracing::warn!("No Brave Search API key; falling back to DuckDuckGo Instant Answers");
        Ok(SearchBackend::DuckDuckGo(duckduckgo_client::DuckDuckGoBackend::new()))
    }
}

enum SearchBackend {
    Brave(brave_client::BraveBackend),
    DuckDuckGo(duckduckgo_client::DuckDuckGoBackend),
}

impl SearchBackend {
    async fn search(&self, query: &str, limit: usize) -> Result<SearchResponse, WebSearchError> {
        match self {
            SearchBackend::Brave(backend) => backend.search(query, limit).await,
            SearchBackend::DuckDuckGo(backend) => backend.search(query, limit).await,
        }
    }
}

/// Perform web search and return structured results.
///
/// # Arguments
/// * `query` - Search query string
/// * `limit` - Maximum number of results to return (default: 5)
///
/// # Returns
/// SearchResponse containing results with URLs, titles, and descriptions.
pub async fn search(query: &str, limit: Option<usize>) -> Result<SearchResponse, WebSearchError> {
    let backend = select_backend()?;
    let limit = limit.unwrap_or(5);
    backend.search(query, limit).await
}