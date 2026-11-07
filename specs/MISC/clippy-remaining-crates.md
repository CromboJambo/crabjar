# Spec: Resolve Remaining Crate Clippy Warnings

## Overview
Multiple crates have clippy warnings that should be resolved. This spec covers the remaining crates after crabjar-review and slow-friend are addressed.

## Warning Inventory by Crate

| Crate | Warnings | Auto-fixable | Priority |
|-------|----------|--------------|----------|
| analyze-apk (lib) | 16 | 14 | High |
| pesti-structural-tokenizer (lib) | 10 | 5 | High |
| crabjar-worker (lib) | 8 | 5 | Medium |
| analyze-firmware (lib) | 9 | 9 | Medium |
| crabjar-skill-scanner (lib) | 7 | 2 | Low |
| crabjar-skills (lib) | 6 | 6 | Low |
| crabjar-gpui (bin) | 7 | 2 | Low |
| orchestrator (bin) | 9 | 8 | Low |

## Resolution Plan

### Phase 1: Auto-fix all crates (fresh session)
```bash
cd /home/crombo/projects/active/crabjar
# Run auto-fix for each crate with warnings
for pkg in analyze-apk pesti-structural-tokenizer crabjar-worker analyze-firmware crabjar-skill-scanner crabjar-skills crabjar-gpui orchestrator; do
    cargo clippy --fix -p $pkg
done
# Review all changes together
git diff --stat
# Run full test suite
cargo test --workspace
git add -A && git commit -m "chore: auto-fix clippy warnings across 8 crates"
```

### Phase 2: Manual fixes (fresh session)
After auto-fix, remaining warnings will require manual attention. Expected patterns:
- Unused imports that can't be safely removed
- Methods/functions marked as never used (public API considerations)
- Documentation issues (missing backticks)
- Complex type warnings suggesting refactorings

### Phase 3: Workspace-wide verification (fresh session)
```bash
cargo clippy --workspace 2>&1 | grep "generated.*warnings"
# Should show zero or near-zero warnings across all crates
```

## Acceptance Criteria
- `cargo clippy --workspace` produces zero warnings across all crates
- All tests pass: `cargo test --workspace`
- No regressions in any crate

## Risk Assessment
- **Low**: Most warnings are style/formatting issues
- **Medium**: Unused method/function warnings may indicate public API that should be preserved with `#[allow(dead_code)]` or documented as intentional
- **High**: None expected, but complex type refactorings could introduce subtle bugs if not tested thoroughly
