# State Doc Format Reference

## Required sections

1. **Overview** — project name, kind, core value, version/state
2. **Architecture** — workspace layout, core components, core pipeline
3. **Build & Test** — commands for build, test, lint
4. **Code Quality & Style** — rules, guidelines, style patterns
5. **Additional sections** — as needed (security, config, next-gen proposals)
6. **Crabjar Context** — architecture alignment, integration points
7. **Confidence Assessment** — what captured, what missed, assumptions, stale after
8. **Key Takeaways** — numbered plain statements

## Crabjar Context sections

### Architecture Alignment
Map each component to one of:
- Pure observer (no runtime execution)
- Append-only (no deletion, no modification)
- Gated (execution requires gate)
- Decision records (produces reflections, not actions)

### Integration Points
Identify patterns from this project that crabjar could adopt:
- config loading patterns
- state-docs format conventions
- knowledge bridge mechanisms
- CLI command surface

## Confidence Assessment

Every state doc must include:
- **What captured**: list of source data items examined
- **What missed**: list of inaccessible or unverified data
- **Assumptions**: list of assumptions made from partial data
- **Stale after**: conditions that invalidate this review

## When to skip

- Empty directory with no meaningful content
- User already has a state doc for this project (update instead)
- Source data too sparse (<5 files, no README, no config) — produce minimal overview
