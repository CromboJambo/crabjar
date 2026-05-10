---
name: domain-allowlist
description: |
  Implement domain allowlist for web fetch scope gating. Use whenever the user wants to restrict web navigation and sub-resource requests to trusted domains — domain allowlist prevents unauthorized navigation and resource loading. Trigger when the user mentions domain allowlist, trusted domains, scope gating, web fetch restrictions, or wants to limit web interaction to specific domains.
---

# Domain Allowlist

## Core Concept

Domain allowlist restricts navigation and sub-resource requests to trusted domains. This prevents unauthorized navigation, resource loading from untrusted sources, and scope expansion beyond intended targets.

## Allowlist Format

```json
{
  "allowed_domains": [
    "example.com",
    "*.example.com",
    "sub.example.com"
  ],
  "blocked_domains": [
    "untrusted.com",
    "*.untrusted.com"
  ]
}
```

## Implementation

### 1. Domain Parsing

Parse domain strings:
- **Exact match**: `example.com` — only this domain
- **Wildcard**: `*.example.com` — any subdomain of example.com
- **Subdomain**: `sub.example.com` — specific subdomain only

### 2. Navigation Gate

Gate all navigation actions:
- `open` — check target domain against allowlist
- `back` — check previous domain against allowlist
- `forward` — check next domain against allowlist
- `reload` — allowed (same domain)
- `pushstate` — allowed (same domain)

### 3. Sub-Resource Gate

Gate sub-resource requests:
- Images, scripts, stylesheets
- API calls from page
- WebSocket connections
- External embeds

### 4. Violation Handling

On domain violation:
- **Blocked**: return `Interrupted` with reason
- **Logged**: record violation in interrupt log
- **Alert**: surface uncertainty before proceeding

## Configuration

- **Allowlist file**: JSON file with allowed domains
- **Allowlist source**: CLI flags, config file, env vars
- **Wildcard support**: `*.domain` patterns
- **Blocked domains**: explicit block list
- **Dynamic updates**: allowlist can be updated mid-session

## Integration with Crabjar

Crabjar's webfetch should adopt:
- Domain allowlist for scope gating
- Navigation gate before all open/back/forward actions
- Sub-resource gate for external loading
- Violation logging with reason
- Uncertainty surface before domain expansion

## References

Read `state-docs/agent-browser-state.md` section 2.6 for security features context and section 7.5 for stale detection thresholds.
