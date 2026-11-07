# Spec: Comprehensive Clippy Warning Resolution

## Overview
The crabjar workspace has significant clippy warning debt across multiple crates. This meta-spec coordinates the resolution of all warnings in a structured, phased approach using fresh sessions for each phase.

## Total Warning Inventory

| Category | Count | Auto-fixable | Manual |
|----------|-------|--------------|--------|
| crabjar-review (lib) | 114 | 79 | ~35 |
| slow-friend (lib) | 51 | 46 | ~5 |
| analyze-apk (lib) | 16 | 14 | ~2 |
| pesti-structural-tokenizer (lib) | 10 | 5 | ~5 |
| crabjar-worker (lib) | 8 | 5 | ~3 |
| analyze-firmware (lib) | 9 | 9 | 0 |
| crabjar-skill-scanner (lib) | 7 | 2 | ~5 |
| crabjar-skills (lib) | 6 | 6 | 0 |
| crabjar-gpui (bin) | 7 | 2 | ~5 |
| orchestrator (bin) | 9 | 8 | ~1 |
| **Total** | **~237** | **~176** | **~61** |

## Phase Breakdown

### Phase 1: crabjar-review (fresh session)
- **Spec**: `specs/clippy-crabjar-review.md`
- **Effort**: High (114 warnings, largest crate)
- **Risk**: Low-Medium
- **Approach**: Auto-fix 79, manual cleanup of ~35 remaining

### Phase 2: slow-friend (fresh session)
- **Spec**: `specs/clippy-slow-friend.md`
- **Effort**: Medium (51 warnings)
- **Risk**: Low-Medium
- **Approach**: Auto-fix 46, manual cleanup of ~5 remaining

### Phase 3: Remaining crates (fresh session)
- **Spec**: `specs/clippy-remaining-crates.md`
- **Effort**: Medium (72 warnings across 8 crates)
- **Risk**: Low
- **Approach**: Batch auto-fix all, then manual cleanup of ~15 remaining

### Phase 4: Workspace verification (fresh session)
- Run `cargo clippy --workspace` to confirm zero warnings
- Run full test suite to verify no regressions

## Execution Strategy

Each phase should be executed in a fresh Hermes session to avoid context pollution and ensure focused work. The specs provide detailed instructions for each phase.

**Recommended order**: Phase 1 → Phase 2 → Phase 3 → Phase 4

## Success Criteria
- Zero clippy warnings across the entire workspace
- All tests pass (`cargo test --workspace`)
- No regressions in any crate's functionality
