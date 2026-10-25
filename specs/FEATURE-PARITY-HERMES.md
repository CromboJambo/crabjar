# Feature Parity: CrabJar vs Hermes Agent

**Date:** October 25, 2026  
**Status:** Draft — for branching into individual specs  
**North Star:** Move crabjar closer to being a drop-in replacement for Hermes Agent. If none of the three conditions apply (blocked capability, measurable improvement, waiting consumer), it's theater.

## Priority 1: Blockers (Drop-in Replacement)

These capabilities are missing entirely and prevent users from switching from Hermes without losing core functionality.

### P1-1: Web Search Tooling
**Hermes equivalent:** `web_search(query)` — returns structured results  
**CrabJar gap:** No web search capability; agents cannot answer questions requiring current information  
**Branch into spec:** Yes — requires backend selection (Brave API vs DuckDuckGo vs custom), result parsing, rate limiting, error handling  
**Implementation notes:**
- Backend: evaluate Brave Search API vs DuckDuckGo Instant Answers vs custom scraper
- Output format: structured JSON matching Hermes' `{results: [{url, title, description}]}` contract
- Rate limiting: per-query throttling, backend-specific limits
- Error handling: timeout, malformed responses, empty result sets
- Integration with orchestrator's `/acp/chat` endpoint for tool calling

### P1-2: Web Content Extraction
**Hermes equivalent:** `web_extract(urls)` — returns clean markdown/text from URLs  
**CrabJar gap:** Cannot read web page content; agents cannot reference external documentation or sources  
**Branch into spec:** Yes — requires HTML parsing strategy, PDF support, error handling for dynamic content  
**Implementation notes:**
- Parser: evaluate regex-based extraction vs dedicated library (e.g., html2text)
- PDF support: pass-through to external tool or use existing PDF libraries
- Error handling: timeout, malformed HTML, empty results, JS-rendered pages
- Integration with web_search for search→extract workflows
- Output format: clean markdown/text matching Hermes contract

### P1-3: Browser Automation (Browser Use CLI)
**Hermes equivalent:** `browser_exec(code)` — Python-based browser automation via Browser Use CLI  
**CrabJar gap:** No interactive web automation; cannot automate login flows, form submissions, or dynamic page interactions  
**Branch into spec:** Yes — requires evaluation of existing tools vs custom implementation, integration with crabjar's execution pipeline  
**Implementation notes:**
- Evaluate: Playwright (via Python subprocess), Puppeteer (via Node subprocess), or direct Browser Use CLI integration
- Integration with crabjar's execution pipeline (guard → concierge → telemetry)
- Security considerations: browser sandboxing, cookie isolation, credential management
- Output format: match Hermes' structured response contract

### P1-4: Subagent Delegation Framework
**Hermes equivalent:** `delegate_task(tasks)` — spawn isolated subagents for parallel work  
**CrabJar gap:** Conductor exists but serves fleet coordination; no task delegation paradigm for agent-level parallelism  
**Branch into spec:** Yes — requires architectural decision on conductor extension vs separate layer  
**Implementation notes:**
- Architectural decision: extend conductor vs build separate delegation layer
- Task isolation: memory, context, tool access per subagent
- Result aggregation: collecting and merging subagent outputs
- Error handling: partial failures, timeout management
- Integration with guard system for delegated task authorization

### P1-5: Interactive Clarify Tooling
**Hermes equivalent:** `clarify(questions)` — choice-based user interaction with structured responses  
**CrabJar gap:** No structured interactive questioning; agents cannot gather user input during execution  
**Branch into spec:** Yes — requires TUI integration, response parsing, multi-question flow handling  
**Implementation notes:**
- Integration with existing ratatui TUI system
- Response parsing: single-select vs multi-select vs free-text
- Multi-question flow handling: sequential vs parallel question presentation
- Timeout management: user response deadlines
- Integration with execution pipeline for decision-making workflows

### P1-6: Session Management/Search
**Hermes equivalent:** `session_search(query, session_id)` — search conversation history  
**CrabJar gap:** No session history retrieval; agents cannot reference prior interactions  
**Branch into spec:** Yes — requires storage design, query indexing, integration with memory system  
**Implementation notes:**
- Storage: SQLite integration with existing memory crate architecture
- Query indexing: full-text search vs keyword matching
- Integration with execution pipeline for context-aware responses
- Privacy considerations: session data retention and cleanup

### P1-7: Process Management Tools
**Hermes equivalent:** `process_manage` — manage background processes  
**CrabJar gap:** No process management; agents cannot control long-running services or daemons  
**Branch into spec:** Yes — requires signal handling, state tracking, integration with execution pipeline  
**Implementation notes:**
- Signal handling: SIGTERM, SIGINT, SIGHUP for graceful shutdown
- State tracking: process lifecycle monitoring
- Integration with guard system for process authorization
- Error handling: orphaned processes, zombie detection

### P1-8: Cron Job Management
**Hermes equivalent:** `cronjob_manage` — schedule recurring tasks  
**CrabJar gap:** No scheduled task execution; agents cannot automate recurring workflows  
**Branch into spec:** Yes — requires scheduler design, integration with conductor for distributed scheduling  
**Implementation notes:**
- Scheduler design: in-process vs external cron daemon integration
- Integration with conductor for distributed scheduling across fleet
- Error handling: missed schedules, overlapping executions
- Security considerations: credential management for scheduled tasks

## Priority 2: Productivity (Significant Improvement)

These capabilities would significantly improve crabjar's productivity compared to Hermes but are not blockers for basic functionality.

### P2-1: Memory System Redesign
**Hermes equivalent:** MEMORY.md/USER.md persistent memory system  
**CrabJar gap:** Different design philosophy; state-docs paradigm vs explicit memory files  
**Branch into spec:** Yes — requires architectural decision on compatibility vs migration path  
**Implementation notes:**
- Architectural decision: migrate to MEMORY.md/USER.md format or document state-docs alternative clearly
- Compatibility layer: read/write both formats during transition period
- Integration with existing memory crate infrastructure

### P2-2: Home Assistant Application-Layer Tools
**Hermes equivalent:** `homeassistant` tools (HA state/services)  
**CrabJar gap:** Has MQTT infrastructure but not HA-specific tooling; agents cannot interact with Home Assistant directly  
**Branch into spec:** Yes — requires HA API integration, service discovery, state management  
**Implementation notes:**
- HA API integration: REST API client for Home Assistant instances
- Service discovery: automatic detection of HA services on network
- State management: caching and invalidation strategies
- Integration with existing host-mqtt infrastructure

### P2-3: Expand Built-in Skill Ecosystem
**Hermes equivalent:** 300+ built-in skills across domains  
**CrabJar gap:** Only 32 local skills; remote discovery infrastructure complete but breadth lacking  
**Branch into spec:** Yes — requires skill categorization, documentation standards, testing framework  
**Implementation notes:**
- Skill categorization: align with Hermes' category structure for compatibility
- Documentation standards: SKILL.md format consistency across ecosystem
- Testing framework: automated testing for skill behavior validation

## Priority 3: Nice-to-Have (Differentiation)

These capabilities would differentiate crabjar from Hermes but are not required for drop-in replacement.

### P3-1: Image Generation Tools
**Hermes equivalent:** `image_generate` tools  
**CrabJar gap:** No image generation; agents cannot create visual content  
**Branch into spec:** Yes — requires backend selection (local vs API), integration with execution pipeline  
**Implementation notes:**
- Backend evaluation: local models vs API services (Stable Diffusion, DALL-E)
- Integration with execution pipeline for tool calling
- Security considerations: credential management for API-based backends

### P3-2: Desktop Project Management
**Hermes equivalent:** `desktop_project` tools  
**CrabJar gap:** No desktop project management; agents cannot organize or manipulate project files on desktop  
**Branch into spec:** Yes — requires platform-specific implementation (Linux/Windows/macOS)  
**Implementation notes:**
- Platform-specific implementation: Linux (GNOME/KDE), Windows, macOS
- Integration with existing file system tools
- Security considerations: permission management for project directories

### P3-3: Mirror-log Attention Tracking System
**Hermes equivalent:** `mirror-log-mcp` tools (15 tools for attention logging/observation tracking)  
**CrabJar gap:** No observation/attention tracking; agents cannot track their own attention patterns or user engagement  
**Branch into spec:** Yes — requires attention model design, integration with memory system  
**Implementation notes:**
- Attention model design: what constitutes "attention" in agent workflows?
- Integration with existing memory system for persistence
- Privacy considerations: tracking data retention and cleanup

## Analysis Notes

The critical gaps are all in the **interaction layer**, not the orchestration layer. Crabjar's core execution pipeline (guard → concierge → telemetry) is mature and arguably more sophisticated than Hermes' equivalent systems. The missing capabilities are primarily user-facing tools that make agents productive out-of-the-box.

**Architectural considerations:**
- Web tooling should leverage existing HTTP infrastructure in orchestrator crate
- Browser automation requires evaluation of existing tools vs custom implementation
- Subagent delegation may extend conductor pattern or require separate layer
- Memory system redesign is architectural decision with significant compatibility implications

**Implementation strategy:**
1. Focus on Priority 1 blockers first (web tooling, subagent delegation, clarify)
2. Build compatibility layers for memory system during transition period
3. Expand skill ecosystem breadth as productivity improves
4. Nice-to-have features can be deferred until core functionality is complete

**Provenance:** This spec was generated from feature parity analysis comparing crabjar's project_map.md, AGENTS.md, CHANGELOG, and ROADMAP against Hermes Agent's documented capabilities and available skills list. The comparison identified 8 critical gaps, 3 significant gaps, and 3 minor gaps that block or reduce productivity for drop-in replacement scenarios.