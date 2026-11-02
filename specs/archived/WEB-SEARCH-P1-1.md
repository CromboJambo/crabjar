# Web Search Tooling

**Status:** Implemented  
**Priority:** P1-1 (Drop-in replacement blocker)  
**Target:** crabjar-web-search crate  
**Estimated effort:** 3-5 days  

## Overview

Provide web search functionality via self-hosted SearXNG instance, with Brave Search API and DuckDuckGo as fallbacks. Integrates with orchestrator tool calling infrastructure so agents can perform web searches during execution.

## Architecture Decision: Self-Hosted SearXNG

**Primary backend:** Self-hosted SearXNG instance (deployed on ftw3 via podman)  
**Fallback 1:** Brave Search API (requires API key)  
**Fallback 2:** DuckDuckGo Instant Answers (no API key needed)  

### Why SearXNG?
- No external API keys required for primary operation
- Full control over search behavior and privacy
- Multiple upstream engines (Google, Bing, Brave, DuckDuckGo, etc.)
- JSON API with structured results
- Already have podman-compose setup from tuirowser project

### Deployment
SearXNG instance deployed on ftw3 at http://ftw3:8080 using podman. Configuration stored in `~/searxng-settings.yml` with custom settings enabling JSON format and multiple search engines.

## API Design

```hemlock
// Tool definition for orchestrator
define tool web_search {
    description: "Search the web for information",
    parameters: {
        query: { type: "string", required: true, description: "Search query" },
        limit: { type: "integer", default: 5, description: "Max results (1-20)" }
    }
}

// Response format
{
    "success": true,
    "query": "rust programming language",
    "results": [
        {
            "url": "https://example.com",
            "title": "Example Title",
            "description": "Brief description of the page content"
        }
    ],
    "total_results": 3,
    "backend": "searxng"
}
```

## Implementation Plan

### Phase 1: Crate Setup (Day 1)
- [x] Create `crabjar-web-search` crate in workspace
- [x] Add dependencies: reqwest, serde, tokio, thiserror, urlencoding
- [x] Define SearchResponse and SearchResult types
- [x] Implement error handling with WebSearchError enum

### Phase 2: SearXNG Backend (Day 1-2)
- [x] Implement SearxngBackend struct with HTTP client
- [x] Implement search() method using SearXNG JSON API
- [x] Parse results into SearchResult structs
- [x] Handle rate limiting and error responses

### Phase 3: Fallback Backends (Day 2-3)
- [x] Implement BraveBackend with API key authentication
- [x] Implement DuckDuckGoBackend for no-key fallback
- [x] Backend selection logic based on configuration

### Phase 4: Integration & Testing (Day 3-5)
- [ ] Register web_search tool in orchestrator
- [ ] Add to conductor's tool manifest
- [ ] Write integration tests with mock HTTP server
- [ ] Test error handling and edge cases
- [ ] Update CHANGELOG.md

## Configuration

### Environment Variables
- `SEARXNG_URL` - URL of self-hosted SearXNG instance (default: http://localhost:8080)
- `BRAVE_SEARCH_API_KEY` - Brave Search API key for fallback
- `WEB_SEARCH_TIMEOUT` - Request timeout in seconds (default: 30)

### Conductor Configuration
```toml
[tools.web_search]
enabled = true
backend = "searxng"  # or "brave", "duckduckgo"
limit = 5
timeout_seconds = 30
```

## Error Handling

- **Rate limiting:** Exponential backoff with jitter
- **Network errors:** Retry up to 3 times with increasing delay
- **Empty results:** Return success with empty array, not error
- **Timeout:** Configurable per-request timeout

## Testing Strategy

1. Unit tests for each backend's search method
2. Integration tests with mock HTTP server (wiremock)
3. End-to-end test against real SearXNG instance
4. Error handling tests (timeout, network failure, rate limit)

## Open Questions

- Should we cache search results to reduce API calls? (TBD)
- How to handle pagination for large result sets? (Limit to first page for now)
- Should agents be able to specify search engines? (No, keep it simple)

## Success Criteria

- Agents can call `web_search` tool and receive structured results
- Self-hosted SearXNG instance works as primary backend
- Graceful fallback to Brave/DuckDuckGo when SearXNG unavailable
- Search results are accurate and relevant
- Error handling covers all failure modes
- Integration tests pass consistently

## References

- [SearXNG Documentation](https://docs.searxng.org/)
- [Brave Search API](https://brave.com/search/api/)
- [DuckDuckGo Instant Answers API](https://api.duckduckgo.com/)