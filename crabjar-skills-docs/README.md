# CrabJar Skills

CrabJar-specific skills that leverage CrabJar's infrastructure (state-docs, knowledge store, guard, telemetry).

## Installed Skills

| Skill | Description | Depends On |
|---|---|---|
| `crabjar-kanban-task` | Kanban task tracking with state-docs persistence | state-docs |
| `crabjar-create-skill` | Skill creation with knowledge-store indexing | skill-reference-store |
| `crabjar-backup-data` | Workspace state backup via telemetry | telemetry, state-docs |
| `crabjar-health-check` | Three-layer health verification via guard | guard, telemetry |
| `crabjar-product-brief` | Product brief generation from requirements | knowledge-store |
| `crabjar-clarify-requirements` | Structured requirements gathering | knowledge-store |
| `crabjar-choose-engine` | Engine routing (Hermes / Claude Code / Codex) | guard |

## Installation

```bash
bash crabjar-skills/install.sh
```

This copies all skills to `~/.hermes/skills/` where Hermes will discover them.

## Integration with Oh My Hermes

These skills complement the [Oh My Hermes](https://github.com/Salomondiei08/oh-my-hermes) skill pack:

- **Oh My Hermes** provides platform-specific skills (Vercel, Supabase, Sentry, etc.)
- **CrabJar Skills** provides infrastructure-backed skills (state-docs, knowledge store, guard)
- Together they form a complete agent capability layer

## Design Principles

1. **CrabJar-backed**: Each skill uses crabjar's native infrastructure, not external APIs
2. **Idempotent**: Safe to run multiple times
3. **Self-documenting**: Each skill includes prerequisites, procedure, pitfalls, and verification
4. **Composable**: Skills can be combined (e.g., clarify-requirements → product-brief → kanban-task)
