# AGENTS.md — skill-script-runner (skill-script-runner)

> Purpose: Skill script discovery and execution — find, validate, and run agent skills.

## Layer

Layer 7: skills — agent skills, may depend on all layers.

## Public API

- Skill discovery (scan directories for skill manifests)
- Skill validation (check SKILL.md structure, frontmatter)
- Skill execution (run skill scripts with proper context)

## Key Files

- `src/lib.rs` — crate entry point
- `src/discovery.rs` — skill discovery logic
- `src/execution.rs` — skill execution logic

## Dependencies

- anyhow, serde, serde_json, tokio, tracing, path-absolutize, ignore, tempfile

## Pitfalls

- Skill discovery scans for SKILL.md files — follow the standard format
- Execution must provide proper context (workspace, tools, memory)
- Validate skill structure before execution to prevent malformed skills
