# P2-1: Hermes-Compatible Memory System

**Date:** October 31, 2026
**Status:** Draft
**Priority:** P2 (Productivity Improvement)
**North Star:** Drop-in replacement for Hermes Agent — users migrating should find identical memory behavior.

## Problem Statement

Hermes uses a flat-file memory system (`~/.hermes/memories/MEMORY.md` and `USER.md`) with a well-defined tool interface. Crabjar's state-docs paradigm is architecturally superior but incompatible with Hermes' memory format and tool schema. Users migrating from Hermes lose their existing memory entries and must adapt to a different interaction model.

## Scope

Build a Hermes-compatible memory tool that:
1. Reads/writes `~/.hermes/memories/MEMORY.md` and `USER.md` in the exact same format
2. Implements the identical tool schema (actions: add, patch, remove; targets: memory/user)
3. Enforces character budgets (MEMORY: 2,200 chars, USER: 1,375 chars)
4. Supports batch operations via `operations` array
5. Provides substring matching for patch/remove
6. Tracks capacity and returns usage percentage

## Out of Scope

- Replacing crabjar's existing state-docs system (both coexist)
- Memory provider plugins (future P3 item)
- Session search integration (P1-6 covers this separately)

## Architecture

### New Crate: `crabjar-memory`

```
crabjar-memory/
├── src/
│   ├── lib.rs           # Public API
│   ├── store.rs         # File I/O for MEMORY.md/USER.md
│   ├── parser.rs        # Entry parsing (§ delimiter, multi-line)
│   ├── tool.rs          # Hermes-compatible tool schema handler
│   ├── capacity.rs      # Character budget tracking and enforcement
│   └── error.rs         # Error types
├── tests/
│   └── integration.rs   # End-to-end tool call tests
└── Cargo.toml
```

### Tool Schema (Hermes-Compatible)

```json
{
  "name": "memory",
  "parameters": {
    "type": "object",
    "properties": {
      "action": {
        "type": "string",
        "enum": ["add", "patch", "remove"],
        "description": "Memory operation"
      },
      "target": {
        "type": "string",
        "enum": ["memory", "user"],
        "description": "Which store to modify"
      },
      "content": {
        "type": "string",
        "description": "New entry text or replacement content"
      },
      "old_text": {
        "type": "string",
        "description": "Substring to find for patch/remove"
      },
      "operations": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "action": {"type": "string"},
            "content": {"type": "string"},
            "old_text": {"type": "string"}
          }
        },
        "description": "Batch operations (overrides single action)"
      }
    },
    "required": ["target"]
  }
}
```

### Response Format

```json
{
  "success": true,
  "message": "Added memory entry",
  "current_chars": 1578,
  "max_chars": 2200,
  "usage_percent": 72,
  "entries_count": 4
}
```

## Implementation Details

### File Format Handling

- Entries separated by `§` (U+00A7 SECTION SIGN) followed by newline
- Multi-line entries supported (text continues until next `§` or EOF)
- Whitespace normalization: trim leading/trailing whitespace from entries
- Empty entries ignored during parsing

### Capacity Management

- MEMORY.md: 2,200 character hard limit
- USER.md: 1,375 character hard limit
- Character count includes delimiters and newlines
- On overflow: return error with `closest_entries` for potential consolidation
- Batch operations check budget on final result only (not intermediate states)

### Substring Matching

- `patch`: Find unique entry containing `old_text`, replace that span with `content`
- `remove`: Find unique entry containing `old_text`, remove entire entry
- Ambiguous matches (multiple entries contain substring): return error
- No match: return error with `closest_entries` suggestions

### Error Handling

```json
{
  "success": false,
  "error": "Memory full",
  "detail": "Adding this entry would exceed the 2,200 character limit by 150 characters.",
  "closest_entries": ["Similar existing entry..."],
  "current_chars": 2150,
  "max_chars": 2200
}
```

## Integration Points

### Conductor API Endpoint

```http
POST /memory
Content-Type: application/json

{
  "action": "add",
  "target": "memory",
  "content": "New memory entry"
}
```

### CLI Subcommand

```bash
crabjar memory add --target memory "My new memory entry"
crabjar memory patch --target user --old-text "old text" --content "new text"
crabjar memory remove --target memory --old-text "entry to remove"
crabjar memory list --target memory  # Show current entries with capacity info
```

### Conductor Integration

- Memory tool registered in conductor's tool registry
- Available for use by agents via ACP chat endpoint
- Tool calls persist immediately to disk (no session boundary delay)

## Testing Strategy

1. **Unit tests**: Parser, capacity calculator, substring matching
2. **Integration tests**: Full tool call flow with real file I/O
3. **Hermes compatibility tests**: Round-trip test — write entries via crabjar, read via Hermes CLI and vice versa
4. **Edge cases**: Empty entries, special characters, UTF-8, delimiter in content

## Acceptance Criteria

- [ ] Tool schema matches Hermes exactly (same parameter names, types, enum values)
- [ ] File format identical to Hermes (can be read by Hermes without migration)
- [ ] Character budgets enforced with correct limits
- [ ] Batch operations work atomically (all succeed or all fail)
- [ ] Substring matching handles ambiguity correctly
- [ ] CLI subcommand provides equivalent functionality
- [ ] Integration tests pass against real Hermes memory files
- [ ] No regression in existing crabjar state-docs system

## Success Metrics

- Users can migrate from Hermes to crabjar without losing memory entries
- Memory tool calls from crabjar agents produce identical results to Hermes
- Character budget tracking matches Hermes behavior exactly
- All integration tests pass (including round-trip with real Hermes files)

## Effort Estimate

- **Complexity:** Medium
- **Time:** 1-2 weeks
- **Risk:** Low (isolated new crate, no changes to existing systems)
