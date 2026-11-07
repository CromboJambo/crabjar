# CrabJar Internal Test Infrastructure

**Status:** In Progress
**Date:** 2026-10-16
**Author:** Hermes Agent (with user review)

## Overview

Build internal test infrastructure that proves the remote skill discovery pipeline works end-to-end without network dependencies. This unlocks integration testing for all phases (8-11) and enables security scanning, schema validation, and cache recovery tests.

## Architecture

```
crabjar-test-registry (NEW)
├── Mock HTTP server (axum)
├── Test fixtures (known-good/bad skill packages)
└── Integration test helpers

Used by:
├── crabjar-clawhub-client/tests/integration.rs
├── crabjar-github-client/tests/integration.rs
├── crabjar-skill-scanner/tests/fixtures/
└── crabjar-trust-manager/tests/cache_tests.rs
```

## Phase 1: Mock Registry Server — COMPLETED (INLINED)

**Goal:** Local HTTP server that serves controlled skill manifests.

**Status**: ✅ Mock server created and inlined into clawhub-client tests. Separate crabjar-test-registry crate was initially created but later simplified by inlining the mock server directly into `crabjar-clawhub-client/tests/mock_server.rs` — this reduced workspace complexity while maintaining full test coverage.

### Tasks

1. **Create `crabjar-test-registry` crate**
   - Axum-based HTTP server
   - Configurable endpoints for ClawHub and GitHub API shapes
   - Programmatic fixture loading

2. **Implement ClawHub-compatible endpoints**
   - `GET /api/v1/skills` — list skills
   - `GET /api/v1/skills/search?q=` — search skills
   - `GET /api/v1/skills/{name}` — get skill info
   - `GET /downloads/{name}-{version}.tar.gz` — download skill package

3. **Implement GitHub-compatible endpoints**
   - `GET /search/repositories?q=&tags=crabjar-skill` — search repos
   - `GET /repos/{owner}/{repo}` — get repo info
   - `GET /repos/{owner}/{repo}/contents/manifest.json` — get manifest

4. **Fixture management**
   - Load fixtures from JSON files in `fixtures/clawhub/` and `fixtures/github/`
   - Support scenario-based fixture sets (happy path, errors, edge cases)
   - Programmatic fixture creation for dynamic tests

### Acceptance Criteria

- Server starts on random available port
- All endpoints return valid JSON matching API contracts
- Can serve both happy-path and error scenarios
- Integration tests can start/stop server programmatically

## Phase 2: Test Fixtures for Security Scanning

**Goal:** Known-good and known-bad skill packages to validate scanner.

### Tasks

1. **Create fixture directory structure**
   - `crabjar-test-registry/fixtures/skills/good/` — valid skill packages
   - `crabjar-test-registry/fixtures/skills/bad/` — invalid/malicious packages
   - `crabjar-test-registry/fixtures/manifests/` — manifest.json variants

2. **Known-good fixtures**
   - Minimal valid skill (manifest + one tool)
   - Skill with multiple tools and dependencies
   - Skill with binary assets (under size limits)
   - Skill with network access patterns (declared in manifest)

3. **Known-bad fixtures**
   - Missing manifest.json
   - Malformed manifest.json (invalid JSON, missing required fields)
   - Binary files exceeding size limits
   - Network access without declaration
   - Filesystem writes outside skill directory
   - Environment variable access to sensitive paths
   - Checksum mismatch scenarios

4. **Version edge cases**
   - Pre-release tags (1.0.0-alpha, 1.0.0-rc.1)
   - Build metadata (1.0.0+build.123)
   - Non-standard version strings
   - Version comparison edge cases

### Acceptance Criteria

- Scanner correctly identifies all known-good fixtures as passing
- Scanner correctly identifies all known-bad fixtures with appropriate severity
- Schema validation catches malformed manifests before installation
- Version parsing handles real-world weirdness

## Phase 3: Cache Corruption Recovery Tests

**Goal:** Verify cache system detects and recovers from corruption.

### Tasks

1. **Cache corruption scenarios**
   - Truncated cache entries
   - Corrupted JSON metadata
   - Missing referenced files
   - Stale version references
   - Permission errors on cache directory

2. **Recovery behavior tests**
   - System detects corrupted entries on startup
   - Corrupted entries are cleaned up or reported
   - Valid entries remain unaffected
   - Cache rebuilds correctly after corruption

### Acceptance Criteria

- All corruption scenarios are detected
- Recovery behavior is documented and tested
- No silent failures or data loss

## Phase 4: Full Pipeline Integration Tests

**Goal:** End-to-end tests proving discovery→install→scan→cache flow.

### Tasks

1. **ClawHub pipeline test**
   - Start mock registry with test fixtures
   - Search for skill via ClawHub client
   - Download and install skill
   - Verify security scan passes/fails as expected
   - Verify cache entry created correctly
   - Test update detection with new version

2. **GitHub pipeline test**
   - Start mock registry with GitHub fixtures
   - Search for skill repo via GitHub client
   - Clone and validate manifest
   - Download and install skill
   - Verify security scan passes/fails as expected

3. **Error path tests**
   - Network timeout scenarios
   - API error responses (404, 500, rate limiting)
   - Concurrent install/update operations
   - Interrupted downloads

### Acceptance Criteria

- Full pipeline works end-to-end without network
- All error paths are tested and handled gracefully
- Tests run in < 30 seconds total
- Tests are deterministic (no flakiness)

## Design Decisions

1. **Separate crate for test infrastructure** — keeps production crates clean, enables shared fixtures across multiple crates' tests
2. **Axum for mock server** — lightweight, async, well-supported HTTP framework in Rust ecosystem
3. **Fixture-driven testing** — declarative test data enables easy addition of new scenarios
4. **Random port allocation** — avoids conflicts when running tests in parallel

## Risks

- **Port conflicts:** Multiple test runs might conflict on ports
  - Mitigation: Use random available port, clean up after each test
- **Fixture drift:** Fixtures might become outdated as API contracts change
  - Mitigation: Version fixtures alongside API changes, document expected schema
- **Test complexity:** Integration tests can be fragile and slow
  - Mitigation: Keep tests focused, use timeouts, parallelize where possible

## Timeline

- Phase 1: 2-3 days (mock registry server)
- Phase 2: 2-3 days (test fixtures)
- Phase 3: 1-2 days (cache corruption tests)
- Phase 4: 2-3 days (full pipeline integration tests)
- Total: ~7-11 days

## Success Metrics

- [ ] Mock registry server starts and serves all endpoints
- [ ] All known-good fixtures pass security scanning
- [ ] All known-bad fixtures fail with appropriate errors
- [ ] Cache corruption scenarios are detected and recovered
- [ ] Full pipeline integration tests pass end-to-end
- [ ] Tests run in < 30 seconds total
- [ ] No network dependencies in integration tests</parameter>
</function>
</tool_call>
<tool_call>
<function=terminal>
<parameter=command>
cargo new --lib crabjar-test-registry