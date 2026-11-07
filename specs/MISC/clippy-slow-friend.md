# Spec: Resolve slow-friend Clippy Warnings

## Overview
The `slow-friend` crate has 51 clippy warnings, the second most of any crate in the workspace. 46 are auto-fixable via `cargo clippy --fix`.

## Current State
```bash
$ cargo clippy --package slow-friend
warning: `slow-friend` (lib) generated 51 warnings (run `cargo clippy --fix --lib -p slow-friend -- ` to apply 46 suggestions)
```

## Resolution Plan

### Phase 1: Auto-fix (fresh session)
Run the auto-fixer and commit the result:
```bash
cd /home/crombo/projects/active/crabjar
cargo clippy --fix --lib -p slow-friend
# Review changes, run tests
cargo test --package slow-friend
git add -A && git commit -m "chore(slow-friend): auto-fix 46 clippy warnings"
```

### Phase 2: Manual fixes (fresh session)
After auto-fix, ~5 warnings will remain requiring manual attention. Likely candidates:
- Unused imports
- Methods marked as never used (may be public API)
- Documentation issues

### Acceptance Criteria
- `cargo clippy --package slow-friend` produces zero warnings
- All tests pass: `cargo test --package slow-friend`
- No regressions in downstream crates that depend on slow-friend

## Risk Assessment
- **Low**: Most warnings are style/formatting issues
- **Medium**: Unused method/function warnings may indicate public API that should be preserved
