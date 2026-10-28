# Code Review Pipeline — Deterministic Hybrid Architecture

**Status:** Proposed  
**Date:** 2026-10-10  
**Inspired by:** alibaba/open-code-review (proven at scale: tens of thousands of developers, millions of defects)

## Problem

Purely language-driven code review tools suffer from:
- **Incomplete coverage** — agents "cut corners" on large changesets
- **Position drift** — line numbers/file references don't match actual code locations
- **Unstable quality** — minor prompt variations cause significant quality fluctuations
- **High token consumption** — general-purpose agents use ~9x more tokens for similar precision

## Solution: Deterministic Engineering × Agent Hybrid

Combine deterministic logic (for correctness guarantees) with agent capabilities (for dynamic decisions). Each handles what it does best.

### Architecture

```
PR/Commit → [Deterministic Pre-processor] → [Agent Review Units] → [Post-processor] → Comments
              File selection/bundling           Isolated sub-agents      Position/content validation
              Rule matching per file            Context retrieval        Precision optimization
```

## Component 1: Deterministic Pre-processor

**Responsibility:** Ensure no important changes are missed; reduce information noise before LLM involvement.

### 1.1 Precise File Selection
- Parse git diff output to identify all changed files
- Filter out generated files, lockfiles, binary assets (configurable allowlist)
- Classify files by type (source, test, config, docs) for appropriate review rules
- **Guarantee:** Every relevant changed file is selected; no important change is missed

### 1.2 Smart File Bundling
- Group related files into single review units:
  - Source + corresponding tests (e.g., `foo.rs` + `tests/foo_test.rs`)
  - Translation pairs (e.g., `message_en.properties` + `message_zh.properties`)
  - Interface + implementation files
- **Monorepo awareness:** For Cargo workspaces, respect crate boundaries — never bundle files from unrelated crates together. Use `Cargo.toml` workspace member list to determine crate membership.
- Each bundle runs as an isolated sub-agent with its own context window
- **Benefits:** 
  - Divide-and-conquer strategy stays stable on very large changesets
  - Naturally supports concurrent review (parallel sub-agents)
  - Related context is always available together

### 1.3 Fine-Grained Rule Matching
- Match review rules to each file's characteristics using template-engine logic, NOT natural language
- Rules are data descriptors applied deterministically (TOML format, crabjar convention):
  ```toml
  [[rules]]
  id = "check_error_handling"
  applies_to = ["source"]
  not_applies_to = ["tests", "generated"]
  severity = "high"
  description = "Errors must be propagated or explicitly handled"
  ```
- **File type classification:** Use git diff + file extensions for initial classification. For ambiguous files (e.g., `.js` could be source or generated), use content heuristics. analyze-mcp-server is NOT required — keep the pre-processor dependency-light and fast.
- **Advantage over prompt-driven:** More stable and predictable; eliminates information noise at the source

## Component 2: Agent Review Units

**Responsibility:** Dynamic decision-making and context retrieval within constrained review units.

### 2.1 Scenario-Tuned Prompts
- Prompt templates deeply optimized for code review (not general-purpose)
- Each bundle receives a focused prompt with:
  - The diff for that specific unit
  - Relevant rules matched in pre-processing
  - Instructions for context retrieval if needed

### 2.2 Context Retrieval Tools
Agent can call tools when the diff alone is insufficient:
- `read_file(path)` — inspect full file contents beyond the diff
- `search_codebase(pattern)` — find related code across the repo
- `list_changed_files()` — see other changed files for cross-file context
- **Constraint:** Context retrieval is bounded and tracked to prevent token explosion

### 2.3 Structured Output Schema
Each review unit produces structured JSON, not free-form text:
```json
{
  "file": "path/to/file.rs",
  "line_start": 42,
  "line_end": 45,
  "severity": "high|medium|low",
  "rule_id": "check_error_handling",
  "message": "Error returned but not propagated to caller",
  "suggestion": "Return the error or handle it explicitly"
}
```

## Component 3: Post-processor (Positioning & Reflection)

**Responsibility:** Improve location accuracy and content accuracy of AI feedback before delivery.

### 3.1 External Positioning Module
- **Algorithm:** For each comment, extract the quoted code snippet from the message. Re-parse the original diff and search for that exact text. If found, report the actual line number; if not found within tolerance (±2 lines), discard the comment as hallucinated. Do NOT trust LLM-reported line numbers directly.
- **Guarantee:** Every delivered comment points to real code locations

### 3.2 Reflection Module
- **Deduplication algorithm:** Hash of `(file + rule_id + normalized_message)` where normalization = lowercase, trim whitespace, collapse multiple spaces. Duplicate hashes are removed, keeping the highest-severity instance.
- Re-evaluate each comment for content accuracy
- Remove duplicates across bundles
- Rank by severity and confidence
- Filter out low-confidence comments below threshold
- **Goal:** Higher precision (fewer false alarms), even if recall drops slightly

## Crabjar Integration Points

### Where this fits in the architecture:
1. **crabjar-conductor** receives a code review task from GitHub webhook or manual trigger
2. Conductor dispatches to appropriate worker with PR/commit metadata
3. Worker runs the deterministic pre-processor (pure Rust, no LLM)
4. Bundles are distributed across multiple sub-agents via crabjar's execution pipeline
5. Each sub-agent uses pesti-server for inference (local or remote model)
6. Results aggregate back through conductor to post-processor
7. Final comments posted to GitHub PR

### Reuse of existing components:
- **Execution Pipeline:** Already supports multi-step workflows with bounded context
- **Model Routing:** Phase-specific routing can select appropriate review models
- **Context Compression:** Manages token budget across large reviews
- **Guard System:** Validates file paths and tool calls during review

- **pesti-mcp-server:** Provides local/remote LLM inference for agent review units

### Crate/module structure:
- **New crate:** `crabjar-review` — contains pre-processor, bundling logic, rule matching, post-processor
- **Integration point:** crabjar-conductor HTTP API exposes `/review` endpoint that orchestrates the pipeline
- **Dependencies:** git2 crate (for diff parsing), pesti-mcp-server (for inference)

### State management:
- Review sessions stored in conductor's SQLite database with schema:
  ```sql
  CREATE TABLE review_sessions (
    id TEXT PRIMARY KEY,
    repo_url TEXT NOT NULL,
    pr_number INTEGER,
    commit_sha TEXT,
    status TEXT CHECK(status IN ('pending', 'processing', 'completed', 'failed')),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP
  );

  CREATE TABLE review_bundles (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES review_sessions(id),
    file_count INTEGER NOT NULL,
    files TEXT NOT NULL,  -- JSON array of paths
    status TEXT CHECK(status IN ('pending', 'processing', 'completed', 'failed')),
    result TEXT           -- JSON structured output
  );

  CREATE TABLE review_comments (
    id TEXT PRIMARY KEY,
    bundle_id TEXT REFERENCES review_bundles(id),
    file TEXT NOT NULL,
    line_start INTEGER NOT NULL,
    line_end INTEGER,
    severity TEXT CHECK(severity IN ('high', 'medium', 'low')),
    rule_id TEXT,
    message TEXT NOT NULL,
    suggestion TEXT,
    position_validated BOOLEAN DEFAULT FALSE
  );
  ```

### Incremental review:
- On new commits to an existing PR, compare against the last reviewed commit SHA
- Only re-review bundles that contain files changed since the last review
- Previous comments on unchanged bundles are preserved; only stale ones are removed

### Configuration:
- `review.toml` in repo root or conductor config directory:
  ```toml
  [review]
  model = "Qwen3.8-27B-TURBO-Fable"      # Default review model
  max_bundles_parallel = 4               # Concurrency limit
  bundle_size_limit_kb = 500             # Max diff size per bundle (500KB)
  context_retrieval_limit = 10           # Max tool calls per bundle
  position_tolerance_lines = 2           # Allowed drift for auto-correction


  [review.rules]
  path = "./review-rules.toml"           # Custom rules override defaults
  ```

### Error handling & retry strategy:
- **Bundle failure:** Retry once with reduced context; if still failing, mark as failed and continue with other bundles
- **LLM unavailable:** Fall back to smaller local model; if all models fail, return partial results with error annotations
- **Timeout:** Per-bundle timeout of 5 minutes; overall session timeout of 30 minutes
- **GitHub API failure:** Retry with exponential backoff (max 3 attempts); cache comments locally for manual posting

## Implementation Phases

### Phase 1: Deterministic Pre-processor (Week 1-2)
- [ ] Implement file selection logic with type classification
- [ ] Build smart bundling algorithm (source+test pairing, translation pairs, monorepo awareness)
- [ ] Create rule matching engine with TOML-based rule definitions
- [ ] Unit tests for each component

### Phase 2: Agent Review Units (Week 3-4)
- [ ] Design structured output schema and parsing
- [ ] Implement context retrieval tools (read_file, search_codebase)
- [ ] Build scenario-tuned prompt templates for code review
- [ ] Integration with pesti-server for inference

### Phase 3: Post-processor & GitHub Integration (Week 5-6)
- [ ] External positioning validation module (text-search based, not line-number trust)
- [ ] Reflection module for content accuracy and deduplication
- [ ] GitHub PR comment posting via crabjar-conductor HTTP API
- [ ] End-to-end testing with real PRs

### Phase 4: Benchmarking & Optimization (Week 7-8)
- [ ] Build benchmark from home-lab PRs with human annotations
- [ ] Measure precision, recall, F1, token consumption vs baseline
- [ ] Tune rule matching and reflection thresholds based on results
- [ ] Document findings in crabjar docs

### Benchmarking methodology:
- **Dataset:** Curate 20-30 recent home-lab PRs across crabjar, pesti, mirror-lab with human-annotated ground truth (real issues found vs missed)
- **Baseline comparison:** Run same PRs through general-purpose agent (Claude Code or Codex) for comparison
- **Metrics tracked per PR:**
  - Issues detected (true positives)
  - Issues missed (false negatives)  
  - False alarms generated (false positives)
  - Token consumption (input + output)
  - Wall-clock time to completion
  - Position accuracy (% of comments with correct line numbers)
- **Evaluation criteria:** Precision >85%, F1 score improvement over baseline, <20% token usage vs baseline

## Success Metrics

Target (based on open-code-review benchmarks):
- **Precision:** >85% (fewer false alarms than general-purpose agents)
- **F1 Score:** Significantly higher than baseline Claude Code/Codex reviews
- **Token Consumption:** <20% of general-purpose agent approach
- **Position Accuracy:** >99% after position validation module

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Bundling algorithm misses relationships | Start conservative; expand pairing rules based on benchmark feedback |
| Local models lack review quality | Use model routing to select stronger model for review phase |
| Context retrieval causes token explosion | Bound tool calls per bundle; track cumulative tokens |
| Reflection module filters too aggressively | Tune thresholds using benchmark dataset |
| SQLite schema migration issues | Version migrations explicitly; test on fresh and existing databases |
| GitHub webhook reliability | Support manual trigger via conductor API as fallback |

## References

- alibaba/open-code-review: https://github.com/alibaba/open-code-review
- AACR-Bench dataset: https://huggingface.co/datasets/Alibaba-Aone/aacr-bench
- Crabjar Execution Pipeline: specs/crabjar-execution-workflow.md
- Crabjar Conductor API: specs/conductor-crate.md
- Analyze MCP Server: specs/analyze-mcp-server.md
