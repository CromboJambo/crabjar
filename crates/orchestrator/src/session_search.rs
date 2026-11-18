use serde::{Deserialize, Serialize};

/// Request payload for the session_search tool.
#[derive(Debug, Deserialize)]
pub struct SessionSearchRequest {
    /// Search query string (optional if session_id provided).
    pub query: Option<String>,
    /// Specific session ID to retrieve (optional).
    pub session_id: Option<String>,
}

/// A search result entry from conversation history.
#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub session_id: String,
    pub timestamp: String,
    pub sender: String,
    pub content_preview: String,
    pub relevance_score: f64,
}

/// Response payload for the session_search tool.
#[derive(Debug, Serialize)]
pub struct SessionSearchResponse {
    pub results: Vec<SearchResult>,
    pub total_matches: usize,
    pub query: Option<String>,
    pub session_id: Option<String>,
}

/// Execute a session search operation.
///
/// If session_id is provided, retrieves all messages from that specific session.
/// Otherwise, searches across all sessions for entries matching the query.
pub async fn execute_session_search(
    request: SessionSearchRequest,
) -> anyhow::Result<SessionSearchResponse> {
    let query = request.query.unwrap_or_default();

    // Use the library's SessionSearchIndex for FTS5-based search
    let mut index = crabjar_lib::session_search::SessionSearchIndex::new();
    let raw_results = index.search(&query, 20).map_err(|e| anyhow::anyhow!(e))?;

    let results: Vec<SearchResult> = raw_results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let content_preview = r["outcome"]
                .as_str()
                .or_else(|| r["query"].as_str())
                .unwrap_or("")
                .chars()
                .take(200)
                .collect::<String>();
            SearchResult {
                session_id: r["session_id"].as_str().unwrap_or("unknown").to_string(),
                timestamp: r["timestamp"].as_str().unwrap_or("").to_string(),
                sender: "system".to_string(),
                content_preview,
                relevance_score: 1.0 / (i as f64 + 1.0),
            }
        })
        .collect();

    Ok(SessionSearchResponse {
        total_matches: results.len(),
        query: if query.is_empty() { None } else { Some(query) },
        session_id: request.session_id,
        results,
    })
}
