# Spec: Resolve crabjar-review Clippy Warnings

## Overview
The `crabjar-review` crate has 114 clippy warnings, the most of any crate in the workspace. 79 are auto-fixable via `cargo clippy --fix`.

## Current State
```bash
$ cargo clippy --package crabjar-review
warning: `crabjar-review` (lib) generated 114 warnings (run `cargo clippy --fix --lib -p crabjar-review -- ` to apply 79 suggestions)
```

## Resolution Plan

### Phase 1: Auto-fix (fresh session)
Run the auto-fixer and commit the result:
```bash
cd /home/crombo/projects/active/crabjar
cargo clippy --fix --lib -p crabjar-review
# Review changes, run tests
cargo test --package crabjar-review
git add -A && git commit -m "chore(crabjar-review): auto-fix 79 clippy warnings"
```

### Phase 2: Manual fixes (fresh session)
After auto-fix, ~35 warnings will remain requiring manual attention. Common patterns to expect:
- Unused imports that the auto-fixer can't safely remove
- Methods/functions marked as never used (may be public API)
- Documentation issues (missing backticks)
- Complex type warnings

### Acceptance Criteria
- `cargo clippy --package crabjar-review` produces zero warnings
- All tests pass: `cargo test --package crabjar-review`
- No regressions in downstream crates that depend on crabjar-review

## Risk Assessment
- **Low**: Most warnings are style/formatting issues
- **Medium**: Unused method/function warnings may indicate public API that should be preserved with `#[allow(dead_code)]` or documented as intentional
