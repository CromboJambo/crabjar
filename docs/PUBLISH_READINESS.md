# Crabjar/Pesti Ecosystem Publish Readiness Spec

**Date:** 2026-10-18
**Purpose:** Assess and plan crates.io publishing for all crabjar/pesti ecosystem packages

## Current State

| Package | Local Version | crates.io Version | Status | Publish Ready? |
|---------|--------------|-------------------|--------|----------------|
| pesti-gguf | 0.2.4 | 0.2.4 | ✅ In sync | N/A |
| pesti-structural-tokenizer | 0.1.3 | 0.1.2 | ⚠️ Local ahead | Yes — just publish |
| git-sheets | 0.1.2 | 0.1.2 | ✅ In sync | N/A |
| pesti-runner | 0.1.0 | Not published | 🔧 Version wrong | No — bump to 0.1.2 first |
| pesti-embeddings | 0.1.4 | Not published | 🆕 New crate | Maybe — needs evaluation |
| nustage | 0.1.0 | Not published | 📦 Untouched | No — missing metadata |

## Publish Readiness Assessment

### pesti-structural-tokenizer (0.1.3 → publish)
**Status:** Ready to publish immediately.
- ✅ Builds clean
- ✅ Has documentation and examples
- ✅ Version bump already done locally
- **Action:** `cargo publish -p pesti-structural-tokenizer`

### pesti-runner (0.1.0 → 0.1.2 → publish)
**Status:** Not ready — version needs correction.
- ⚠️ User specified target version is 0.1.2, not current 0.1.0
- ✅ Builds clean (with warnings)
- **Action:** Bump version to 0.1.2, then publish

### pesti-embeddings (0.1.4 → evaluate)
**Status:** Substantial new crate (489 lines across 5 modules). Builds but needs evaluation.
- ✅ Builds clean
- ✅ Has description and dependencies properly declared
- ✅ Real functionality: embedder, pooling, similarity modules
- ⚠️ Very new — may not be stable enough for public release yet
- **Recommendation:** Finish embedding work, test thoroughly, then publish as 0.1.0

### nustage (0.1.0 → prepare)
**Status:** Not ready — missing essential metadata.
- ❌ No `publish = true` in Cargo.toml
- ❌ No license field
- ❌ No homepage/repository URLs
- ✅ Builds (as part of tabular-work workspace)
- **Action:** Add publish metadata, write README, then prepare for 0.1.0 release

## Version Correction Plan

### pesti-runner version bump
```bash
# In /home/crombo/projects/active/pesti
sed -i 's/version = "0.1.0"/version = "0.1.2"/' pesti-runner/Cargo.toml
cargo check -p pesti-runner  # verify build
```

## Publish Sequence

1. **Immediate:** `cargo publish -p pesti-structural-tokenizer` (local already at 0.1.3)
2. **After version bump:** `cargo publish -p pesti-runner` (after bumping to 0.1.2)
3. **After embedding work completes:** `cargo publish -p pesti-embeddings` (as 0.1.0)
4. **After metadata prep:** `cargo publish -p nustage` (as 0.1.0)

## Notes

- pesti-gguf and git-sheets are already in sync — no action needed
- crabjar main crate has `publish = true` but hasn't been published yet — separate decision
- Workspace root version (0.1.4) is used by crates that inherit via `version.workspace = true`
- pesti-runner uses explicit version (0.1.0), not inherited from workspace

## Risk Assessment

**Low risk:** pesti-structural-tokenizer publish — already tested, just a version bump
**Medium risk:** pesti-runner publish — needs version correction and testing
**Higher risk:** pesti-embeddings publish — very new crate, limited testing history