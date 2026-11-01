//! Web search tooling for crabjar agents.
//!
//! Provides web search functionality via self-hosted SearXNG (primary),
//! Brave Search API (fallback with API key), and DuckDuckGo Instant Answers
//! (final fallback). Integrates with orchestrator tool calling infrastructure.

use std::env;

mod brave_client;
mod duckduckgo_client;
mod error;
mod models;
mod rate_limiter;
mod searxng_client;

pub use error::WebSearchError;
pub use models::{SearchResult, SearchResponse};

/// Select search backend based on configuration and API key availability.
/// Priority: SearXNG (self-hosted) > Brave (API key) > DuckDuckGo (fallback)
fn select_backend() -> Result<SearchBackend, WebSearchError> {
    // Primary: Self-hosted SearXNG instance
    if let Ok(url) = env::var("SEARXNG_URL") {
        tracing::info!(url, "Using self-hosted SearXNG backend");
        return Ok(SearchBackend::Searxng(searxng_client::SearxngBackend::new()));
    }

    // Fallback 1: Brave Search API (requires API key)
    if let Some(key) = env::var("BRAVE_SEARCH_API_KEY").ok() {
        tracing::info!("Using Brave Search backend");
        return Ok(SearchBackend::Brave(brave_client::BraveBackend::new(key)));
    }

    // Fallback 2: DuckDuckGo Instant Answers (no API key needed)
    tracing::warn!("No SearXNG or Brave configured; falling back to DuckDuckGo");
    Ok(SearchBackend::DuckDuckGo(duckduckgo_client::DuckDuckGoBackend::new()))
}

enum SearchBackend {
    Searxng(searxng_client::SearxngBackend),
    Brave(brave_client::BraveBackend),
    DuckDuckGo(duckduckgo_client::DuckDuckGoBackend),
}

impl SearchBackend {
    async fn search(&self, query: &str, limit: usize) -> Result<SearchResponse, WebSearchError> {
        match self {
            SearchBackend::Searxng(backend) => backend.search(query, limit).await,
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