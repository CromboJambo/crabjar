# CrabJar Remote Skill Discovery & Security Scanning

**Status:** Draft — planning phase
**Date:** 2026-10-16
**Author:** Hermes Agent (with user review)

## Overview

Extend CrabJar's skill system to discover, install, and verify skills from remote registries:
- **GitHub** — community skills as tagged repositories
- **ClawHub** — curated registry with security scanning pipeline

This enables `crabjar tool search <query>`, `crabjar tool install <skill>`, and automated trust verification.

## Architecture

```
crabjar-agent
├── Tool Registry (existing)
├── Plugin Manager (existing)
├── Skill System (existing)
│   ├── Skill Loader
│   ├── Skill Context
│   └── Tool Binder
└── Remote Discovery Layer (NEW)
    ├── GitHub Client — search, clone, tag detection
    ├── ClawHub Client — API integration for registry operations
    ├── Security Scanner — code analysis, checksum verification
    └── Trust Manager — community/verified/official tier management
```

## Phase 8: GitHub Skill Discovery

**Goal:** Search and install skills from GitHub repositories.

### Tasks

1. **Create `crabjar-github-client` crate**
   - HTTP client for GitHub API (use `reqwest`)
   - Authentication via environment variable (`GITHUB_TOKEN`) or SSH key
   - Rate limiting handling (60/hour unauthenticated, 5000/hour authenticated)

2. **Implement skill search on GitHub**
   - Search repositories by tag: `crabjar-skill`
   - Filter by star count, recent activity
   - Parse repository description for skill metadata
   - Support query parameters: `--stars`, `--sort-by`, `--language`

3. **Skill installation from GitHub**
   - Clone repository to temporary directory
   - Validate manifest.json exists in root or `skill/` subdirectory
   - Run security scanner on downloaded code
   - Install to local skill registry after scan passes

4. **Tag-based discovery protocol**
   - Skills must be tagged with `crabjar-skill` on GitHub
   - Optional tags: `crabjar-skill-verified`, `crabjar-skill-official`
   - Repository README must include skill metadata block

### Acceptance Criteria

- Can search for skills by name/description on GitHub
- Can install a skill from a GitHub repository URL
- Validates manifest.json exists before installation
- Handles authentication (token/SSH) transparently
- Unit tests for GitHub API interaction (mocked)

## Phase 9: ClawHub Integration

**Goal:** Integrate with ClawHub registry for curated skill discovery.

### Tasks

1. **Create `crabjar-clawhub-client` crate**
   - HTTP client for ClawHub API (`https://clawhub.com/api`)
   - Implement search, install, list, info endpoints
   - Handle pagination and rate limiting

2. **Implement ClawHub skill search**
   - Search by name, description, tags
   - Filter by trust tier (community, verified, official)
   - Sort by popularity, recency, rating

3. **Skill installation from ClawHub**
   - Download skill package (tarball or zip)
   - Verify checksum against registry metadata
   - Run security scanner on downloaded code
   - Install to local skill registry after scan passes

4. **Trust tier integration**
   - Map ClawHub trust tiers to CrabJar's execution policies
   - Community → low_risk default, requires user confirmation for medium/high
   - Verified → trusted by default
   - Official → highest trust, auto-approve execution policies

### Acceptance Criteria

- Can search for skills on ClawHub by name/description/tags
- Can install a skill from ClawHub registry
- Verifies checksums against registry metadata
- Maps ClawHub trust tiers to CrabJar execution policies
- Unit tests for ClawHub API interaction (mocked)

## Phase 10: Security Scanning Pipeline

**Goal:** Scan downloaded skills for malicious code before installation.

### Tasks

1. **Create `crabjar-skill-scanner` crate**
   - Code analysis engine for skill packages
   - Support multiple scanning backends (pluggable)
   - Generate scan reports with severity levels

2. **Implement scanning rules**
   - File size limits (max 10MB per skill, max 100 files)
   - Binary detection (flag non-source files > 1KB)
   - Network access patterns (detect curl/wget/HTTP calls in scripts)
   - Filesystem write patterns (detect writes outside skill directory)
   - Environment variable access (detect HOME, SSH keys, etc.)
   - Dependency analysis (check for known vulnerable packages)

3. **Checksum verification**
   - SHA-256 checksums for all downloaded files
   - Compare against registry-provided checksums
   - Reject installation on checksum mismatch

4. **Scan report generation**
   - JSON report with findings
   - Severity levels: info, warning, error, critical
   - Summary statistics (files scanned, issues found)
   - Recommendation: allow, warn, or reject

### Acceptance Criteria

- Can scan a skill package for security issues
- Detects binary files, network access, filesystem writes
- Verifies checksums against registry metadata
- Generates structured scan reports
- Unit tests for scanning rules and report generation

## Phase 11: Trust Management & Cache

**Goal:** Manage trust tiers and cache downloaded skills.

### Tasks

1. **Trust tier system**
   - Community: user-submitted, minimal review
   - Verified: passed security scan, reviewed by CrabJar maintainers
   - Official: maintained by CrabJar team or trusted partners
   - Store trust tier in tool registry alongside skill metadata

2. **Skill cache management**
   - Cache downloaded skills in `~/.crabjar/skill-cache/`
   - Version-aware caching (different versions cached separately)
   - Automatic cleanup of stale cache entries (> 30 days)
   - Manual cache management via CLI (`crabjar tool cache clean`)

3. **Update detection**
   - Check for newer versions of installed skills
   - Notify user when updates are available
   - Support bulk updates (`crabjar tool update-all`)

### Acceptance Criteria

- Can assign and query trust tiers for skills
- Caches downloaded skills with version awareness
- Detects and notifies about skill updates
- Supports manual cache management via CLI
- Unit tests for cache management and update detection

## Design Decisions

1. **Pluggable scanning backends** — allow custom scanners to be added without modifying core code
2. **Checksum-first verification** — verify integrity before running security scans
3. **Trust tier mapping** — translate external registry trust levels to CrabJar's execution policies
4. **Cache-aware updates** — only download newer versions, reuse cached files
5. **Transparent authentication** — use environment variables for tokens/keys, no interactive prompts

## Risks

- **API rate limiting:** GitHub and ClawHub have rate limits; need efficient caching and retry logic
  - Mitigation: Implement exponential backoff and local caching of search results
- **False positives in scanning:** Security scanner may flag legitimate skills
  - Mitigation: Allow user to override scan results with `--force` flag
- **Registry downtime:** External registries may be unavailable
  - Mitigation: Cache last known good state, support offline installation from cache
- **Checksum collisions:** SHA-256 is theoretically breakable
  - Mitigation: Use SHA-256 for now, plan migration to SHA-3 in future

## Timeline

- Phase 8: 2-3 weeks (GitHub client, search, install)
- Phase 9: 2-3 weeks (ClawHub integration, trust tier mapping)
- Phase 10: 2-3 weeks (security scanning pipeline)
- Phase 11: 1-2 weeks (trust management, cache)
- Total: ~7-11 weeks for complete implementation

## Success Metrics

- [x] Can search for skills on GitHub by name/description
- [x] Can install a skill from a GitHub repository URL
- [x] Can search for skills on ClawHub by name/description/tags
- [x] Can install a skill from ClawHub registry
- [x] Security scanner detects binary files, network access, filesystem writes
- [ ] Checksum verification rejects corrupted downloads
- [ ] Trust tiers mapped to CrabJar execution policies
- [ ] Skill cache manages versions and stale entries
- [ ] Update detection notifies about newer skill versions
- [ ] Integration tests pass across all phases