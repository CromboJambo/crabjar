# Preamble Stack Schema

## opencode.json structure

| Field | Type | Purpose |
|---|---|---|
| `instruction_paths` | ARRAY of TEXT | paths to preamble files loaded at session start |
| `global_rules_path` | TEXT | path to ~/.config/opencode/AGENTS.md |
| `dynamic_updates` | BOOLEAN | whether preamble files are refreshed periodically |

## AGENTS.md structure

| Section | Purpose |
|---|---|
| Global Agent Rules | cross-project constraints (communication, core constraints, discovery protocol) |
| Rust Conventions | naming, error handling, lint rules |
| Testing Guidelines | framework, paths, naming |
| Commit Style | imperative, sentence-case, subject length |
| Repository Guidelines | project-specific overrides |

## Preamble types

- **Global rules**: ~/.config/opencode/AGENTS.md — applies to all sessions
- **Project rules**: committed to repo in AGENTS.md — project-specific overrides
- **Remote URLs**: configured in opencode.json — fetch at session start
- **Dotfile symlinks**: symlinked from user dotfiles to AGENTS.md

## Stale detection

- Preamble files stale after > 7 days without modification
- Update workflow: read source, check for changes, write-back if updated
- Sparsity flag: placeholder notes vs real values → treat as stale
