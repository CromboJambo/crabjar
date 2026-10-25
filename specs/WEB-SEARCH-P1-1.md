# Implementation Spec: P1-1 Web Search Tooling

**Date:** October 25, 2026  
**Status:** Draft  
**Related:** FEATURE-PARITY-HERMES.md (P1-1)  
**North Star:** Move crabjar closer to being a drop-in replacement for Hermes Agent. Web search is essential for agents to answer questions requiring current information — without it, agents are limited to training data knowledge only.

## Problem

Crabjar has no web search capability. Agents cannot answer questions requiring current information or reference external sources. This blocks the core use case of "ask an agent a question about something that happened after its training data."

Hermes Agent provides `web_search(query)` which returns structured results with URLs, titles, and descriptions. Crabjar needs equivalent functionality to be a drop-in replacement.

## Decision

Implement web search as a new crate `crabjar-web-search/` that integrates with the orchestrator's tool calling infrastructure via `/acp/chat`. Use Brave Search API as primary backend with DuckDuckGo Instant Answers as fallback for environments without API keys.

## Architecture

### Crate: crabjar-web-search/

New workspace member providing web search functionality.

```
crabjar-web-search/
├── Cargo.toml
├── src/
│   ├── lib.rs            # Public API surface
│   ├── brave_client.rs   # Brave Search API client
│   ├── duckduckgo_client.rs  # DuckDuckGo Instant Answers client
│   ├── models.rs         # SearchResult, SearchResponse types
│   ├── error.rs          # WebSearchError enum
│   └── rate_limiter.rs   # Per-backend rate limiting logic
├── tests/
│   ├── integration_tests.rs  # End-to-end search tests
│   └── mock_responses/       # Test fixtures for API responses
└── AGENTS.md             # Crate-specific documentation
```

### Integration Points

1. **Orchestrator Tool Calling:** Register web_search as a tool callable via `/acp/chat` endpoint
2. **CLI Command:** `crabjar search <query>` for direct CLI usage
3. **Guard System:** Integration with execution pipeline for authorization and telemetry

## Backend Selection

### Primary: Brave Search API

**Why Brave Search:**
- Structured JSON responses (no HTML parsing needed)
- Reliable uptime and SLA
- Free tier available (100 queries/month)
- Well-documented API with SDKs
- Supports pagination, filtering, and result types

**API Endpoint:** `https://api.search.brave.com/res/search/web/`  
**Authentication:** Bearer token via environment variable `BRAVE_SEARCH_API_KEY`

### Fallback: DuckDuckGo Instant Answers

**Why DuckDuckGo:**
- No API key required for basic usage
- Good coverage of factual questions
- Free and open source alternatives exist

**API Endpoint:** `https://api.duckduckgo.com/`  
**Authentication:** None (rate limiting applies)

**Limitations:** Less structured than Brave, limited result types, rate limiting without API key

### Backend Selection Logic

```rust
fn select_backend() -> SearchBackend {
    if let Some(key) = env("BRAVE_SEARCH_API_KEY") {
        return BraveBackend::new(key);
    }
    warn!("No Brave Search API key; falling back to DuckDuckGo Instant Answers");
    DuckDuckGoBackend::new();
}
```

## Implementation Details

### Public API Surface (lib.rs)

```rust
pub async fn search(query: &str, limit: Option<usize>) -> Result<SearchResponse, WebSearchError>;

pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub total_results: usize,
}

pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub snippet: Option<String>,
}
```

### Brave Search Client (brave_client.rs)

```rust
impl BraveBackend {
    fn new(api_key: String) -> Self;
    
    async fn search(&self, query: &str, limit: usize) -> Result<SearchResponse, WebSearchError>;
}
```

Implementation notes:
- Use `reqwest` crate for HTTP requests
- Set appropriate headers (Authorization, Accept)
- Parse JSON response with serde
- Handle pagination via offset parameter
- Implement exponential backoff for rate limiting errors

### DuckDuckGo Client (duckduckgo_client.rs)

```rust
impl DuckDuckGoBackend {
    fn new() -> Self;
    
    async fn search(&self, query: &str, limit: usize) -> Result<SearchResponse, WebSearchError>;
}
```

Implementation notes:
- Use `reqwest` crate for HTTP requests
- Parse HTML/JSON response (mix of formats depending on query type)
- Handle rate limiting via sleep between requests
- Implement basic error handling for malformed responses

### Rate Limiting Logic (rate_limiter.rs)

Implement per-backend rate limiting to prevent API abuse:

```rust
pub struct RateLimiter {
    backend: SearchBackend,
    max_requests_per_minute: usize,
}

impl RateLimiter {
    fn new(backend: SearchBackend) -> Self;
    
    async fn search(&self, query: &str, limit: Option<usize>) -> Result<SearchResponse, WebSearchError>;
}
```

Rate limits:
- Brave: 100 queries/hour (free tier), configurable via environment variable
- DuckDuckGo: 1 request/second without API key

### Error Handling (error.rs)

Define comprehensive error types using `thiserror`:

```rust
#[derive(Debug, Error)]
pub enum WebSearchError {
    #[error("Backend selection failed")]
    BackendSelectionFailed,
    
    #[error("API request failed: {0}")]
    ApiRequestFailed(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    
    #[error("Malformed response from backend")]
    MalformedResponse,
    
    #[error("Empty result set for query '{query}'")]
    EmptyResults { query: String },
    
    #[error("Timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Authentication failed for backend '{backend}'")]
    AuthFailed { backend: String },
}
```

## Integration with Orchestrator

### Tool Registration

Register web_search as a callable tool in the orchestrator's `/acp/chat` endpoint handler. The tool schema should match Hermes' `web_search` contract:

```json
{
    "name": "web_search",
    "description": "Search the web for information. Returns structured results with URLs, titles, and descriptions.",
    "parameters": {
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "The search query to look up on the web."
            },
            "limit": {
                "type": "integer",
                "description": "Maximum number of results to return. Defaults to 5.",
                "minimum": 1,
                "maximum": 20
            }
        },
        "required": ["query"]
    }
}
```

### CLI Command Registration

Add `crabjar search <query>` command to the main CLI binary. This provides direct CLI access without going through orchestrator's tool calling infrastructure.

## Security Considerations

### API Key Management

- Store API keys in environment variables (`BRAVE_SEARCH_API_KEY`, `DUCKDUCKGO_API_KEY`)
- Support loading keys from `.crabjar_config.toml` via existing config infrastructure
- Never commit API keys to repository; use secrets management patterns from crabjar-skills

### Output Sanitization

- Sanitize all URLs in search results before returning to prevent XSS attacks
- Validate URL schemes (http://, https:// only)
- Limit result length to prevent buffer overflow issues

### Rate Limiting Security

- Implement per-user rate limiting to prevent API key abuse in multi-user deployments
- Log rate limit violations for audit trail

## Testing Strategy

### Unit Tests

```rust
#[tokio::test]
async fn test_brave_backend_search() {
    // Mock Brave Search API responses
    let backend = BraveBackend::new("test_key");
    let response = backend.search("crabjar rust agent", 5).await.unwrap();
    assert!(!response.results.is_empty());
}

#[tokio::test]
async fn test_duckduckgo_backend_search() {
    // Mock DuckDuckGo Instant Answers responses
    let backend = DuckDuckGoBackend::new();
    let response = backend.search("crabjar rust agent", 5).await.unwrap();
    assert!(!response.results.is_empty());
}

#[tokio::test]
async fn test_rate_limiting() {
    // Test rate limiting logic with mock backend
}

#[tokio::test]
async fn test_error_handling() {
    // Test various error conditions
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_end_to_end_search() {
    // Real Brave Search API call (requires BRAVE_SEARCH_API_KEY env var)
    let response = search("crabjar rust agent", Some(5)).await.unwrap();
    assert_eq!(response.query, "crabjar rust agent");
    assert!(!response.results.is_empty());
}
```

### Mock Response Fixtures

Create test fixtures in `tests/mock_responses/` for API response mocking:

- `brave_search_response.json` — Mock Brave Search API response
- `duckduckgo_search_response.html` — Mock DuckDuckGo Instant Answers response

## Documentation Requirements

### CLI Documentation

Update crabjar's main README.md and AGENTS.md to document the new web search capability:

```bash
# Web search examples
crabjar search "latest Rust release"
crabjar search "Hermes Agent documentation" --limit 3
```

### Developer Documentation

Create `docs/web-search/` directory with developer documentation:

- Architecture diagrams
- API key configuration instructions
- Error handling patterns
- Rate limiting strategies
- Testing procedures

## Success Criteria

1. **Functional:** Agents can perform web searches via orchestrator tool calling and CLI commands
2. **Reliable:** Search functionality works consistently across different environments and network conditions
3. **Secure:** API keys are managed securely; output is sanitized against XSS attacks
4. **Performant:** Rate limiting prevents API abuse while maintaining responsiveness
5. **Compatible:** Output format matches Hermes' `web_search` contract for drop-in replacement

## Rollout Plan

1. Implement core search functionality with Brave Search backend
2. Add DuckDuckGo fallback for environments without API keys
3. Integrate with orchestrator tool calling infrastructure
4. Add CLI command registration
5. Comprehensive testing suite (unit, integration, E2E)
6. Documentation updates (CLI docs, developer docs)
7. Release as part of crabjar 0.13.0

## Future Enhancements

- Support for multiple search backends simultaneously (parallel search)
- Caching layer for frequently searched queries
- Integration with web content extraction tools (P1-2) for search→extract workflows
- Advanced search operators (site:, filetype:, intitle:) support
- Custom search result formatting options