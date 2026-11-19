# LM Studio → Unsloth Studio Migration

**Date:** November 18, 2026
**Status:** Complete (Nov 18, 2026)
**Priority:** P1 (Infrastructure Alignment)
**North Star:** Drop-in replacement for Hermes Agent — crabjar must work with the user's actual LLM infrastructure.

## Problem Statement

Crabjar's orchestrator was built around LM Studio as the local LLM backend. The user's home-lab fleet has migrated to **Unsloth Studio** (ftw3:8081, lappy:8082) with llama-server on other nodes. All references to "LM Studio" in crabjar are now misleading — the code talks to OpenAI-compatible endpoints that happen to be served by Unsloth Studio, not LM Studio.

The new `crabjar-vision` crate already uses `UNSLOTH_STUDIO_*` env vars with default port 8081. The existing `lm_studio_client` module in the orchestrator still uses `LM_STUDIO_*` naming and defaults to port 1234 (LM Studio's default).

## Scope

Migrate all LM Studio references to Unsloth Studio:

1. **Module rename**: `crates/orchestrator/src/lm_studio_client/` → `unsloth_studio_client/`
2. **Type renames**: `LmStudioClient` → `UnslothStudioClient`, `LmStudioConfig` → `UnslothStudioConfig`, etc.
3. **Env var migration**: `LM_STUDIO_*` → `UNSLOTH_STUDIO_*` (with backward-compatible fallback)
4. **Default URL change**: `http://localhost:1234/v1` → `http://localhost:8081/v1`
5. **Documentation updates**: All comments, doc strings, and references

## Out of Scope

- Changing the actual API protocol (both LM Studio and Unsloth Studio use OpenAI-compatible endpoints)
- Modifying endpoint detection logic (native/openai/anthropic/mistralrs) — this is backend-agnostic
- Changes to `crabjar-vision` (already migrated in commit `275191a`)

## Current State (Files Affected)

**16 files with LM Studio references:**

| File | Type of Reference |
|------|-------------------|
| `crates/orchestrator/src/lm_studio_client/mod.rs` | Module declaration, re-exports |
| `crates/orchestrator/src/lm_studio_client/client.rs` | `LmStudioClient`, `LmStudioConfig`, `LmStudioEndpoint` |
| `crates/orchestrator/src/lm_studio_client/types.rs` | Response types (likely backend-agnostic) |
| `crates/orchestrator/src/lm_studio_client/error.rs` | `LmStudioError` enum |
| `crates/orchestrator/src/lm_studio_client/session.rs` | Session management |
| `crates/orchestrator/src/lm_studio_client/prompt_validator.rs` | Prompt validation |
| `crates/orchestrator/src/lm_studio_client/prompt_types.rs` | Prompt types |
| `crates/orchestrator/src/lm_studio_client/tests.rs` | Tests using `LM_STUDIO_*` env vars |
| `crates/orchestrator/src/main.rs` | Client instantiation |
| `crates/orchestrator/src/inference_servers.rs` | Server discovery |
| `crates/orchestrator/src/backend/mod.rs` | Backend abstraction |
| `crates/orchestrator/src/backend/metrics.rs` | Metrics collection |
| `src/lib.rs` | Public API re-exports |
| `src/cli_commands.rs` | CLI commands |
| `src/main.rs` | CLI entry point |
| `src/commands/backend.rs` | Backend commands |

## Migration Strategy

### Phase 1: Env Var Backward Compatibility (Low Risk)

Add `UNSLOTH_STUDIO_*` env vars as primary with `LM_STUDIO_*` as fallback:

```rust
// New config loading order:
// 1. UNSLOTH_STUDIO_URL (primary)
// 2. LM_STUDIO_URL (backward compat, deprecated)
// 3. Default: http://localhost:8081/v1

pub fn from_env() -> Self {
    let base_url = std::env::var("UNSLOTH_STUDIO_URL")
        .or_else(|_| std::env::var("LM_STUDIO_URL"))
        .unwrap_or_else(|_| "http://localhost:8081/v1".to_string());
    // ... same pattern for MODEL, ENDPOINT, CONTEXT_LENGTH, TEMPERATURE, MAX_OUTPUT_TOKENS
}
```

**Env vars to migrate:**
- `LM_STUDIO_URL` → `UNSLOTH_STUDIO_URL` (default: `http://localhost:8081/v1`)
- `LM_STUDIO_MODEL` → `UNSLOTH_STUDIO_MODEL` (no default change)
- `LM_STUDIO_ENDPOINT` → `UNSLOTH_STUDIO_ENDPOINT` (values unchanged: native/openai/anthropic/mistralrs)
- `LM_STUDIO_CONTEXT_LENGTH` → `UNSLOTH_STUDIO_CONTEXT_LENGTH`
- `LM_STUDIO_TEMPERATURE` → `UNSLOTH_STUDIO_TEMPERATURE`
- `LM_STUDIO_MAX_OUTPUT_TOKENS` → `UNSLOTH_STUDIO_MAX_OUTPUT_TOKENS`

### Phase 2: Module and Type Renames (Medium Risk)

Rename the module directory and all public types:

```
crates/orchestrator/src/lm_studio_client/  →  crates/orchestrator/src/unsloth_studio_client/
LmStudioClient      →  UnslothStudioClient
LmStudioConfig      →  UnslothStudioConfig
LmStudioEndpoint    →  UnslothStudioEndpoint
LmStudioError       →  UnslothStudioError
```

Use `#[deprecated]` attributes on old names during transition if needed, or do a hard rename in one commit.

### Phase 3: Documentation and Comments (Low Risk)

Update all doc comments and references from "LM Studio" to "Unsloth Studio":
- Module docs (`//! The unified LM Studio client...`)
- Struct docs
- CLI help text
- Configuration documentation

## Implementation Order

1. **Phase 1 first** — env var changes are additive (backward compatible), can be tested immediately against live Unsloth Studio on ftw3:8081
2. **Phase 2 second** — mechanical rename, verify with `cargo check --workspace` and full test suite
3. **Phase 3 last** — cosmetic but important for clarity

## Testing Strategy

1. **Unit tests**: Update all `LM_STUDIO_*` env var references in tests to `UNSLOTH_STUDIO_*`
2. **Integration test**: Verify client connects to live Unsloth Studio on ftw3:8081 (requires network access)
3. **Backward compat test**: Verify `LM_STUDIO_*` env vars still work when `UNSLOTH_STUDIO_*` are not set
4. **Full workspace check**: `cargo check --workspace` + `cargo test --workspace`

## Acceptance Criteria

- [x] No references to "LM Studio" or `LM_STUDIO_*` in primary code paths (backward compat fallbacks allowed)
- [x] Default endpoint is `http://localhost:8081/v1` (Unsloth Studio default port)
- [x] All 16 affected files updated
- [x] Full test suite passes (`cargo test --workspace`) — orchestrator: 115/115, crabjar lib: 43/43
- [ ] Client verified working against live Unsloth Studio on ftw3:8081 (pending network test)
- [x] Backward compatibility: `LM_STUDIO_*` env vars still work via `.or_else()` fallbacks

## Success Metrics

- Crabjar orchestrator works out-of-the-box with user's Unsloth Studio fleet (ftw3:8081, lappy:8082)
- No confusion between LM Studio and Unsloth Studio in code, docs, or CLI output
- Zero regression in existing functionality

## Effort Estimate

- **Complexity:** Low-Medium (mostly mechanical renames + env var changes)
- **Time:** 1-2 hours
- **Risk:** Low (backward-compatible env vars; type renames are compile-time checked)

## Related Commits

- `275191a` — crabjar-vision crate already uses `UNSLOTH_STUDIO_*` env vars with port 8081 default
- `60f252f` — FEATURE-PARITY-HERMES.md (context for why this matters)
