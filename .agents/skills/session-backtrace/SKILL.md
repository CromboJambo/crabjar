---
name: session-backtrace
description: |
  Reconstruct session context from available persistence layers when resuming work
  on a project or task. Use when the user mentions "pick up from," "resume," "what
  were we doing," "backtrace," "session context," or needs to recover state from
  a previous conversation.
---

# Session Backtrace

Reconstruct session context from available persistence layers when resuming work on a project or task. Use when the user mentions "pick up from," "resume," "what were we doing," "backtrace," "session context," or needs to recover state from a previous conversation.

## Persistence Layers (in priority order)

### 1. Knowledge DB (`memory/knowledge.db`)
```bash
sqlite3 <path>/knowledge.db ".tables"       # Check what tables exist
sqlite3 <path>/knowledge.db "SELECT * FROM events ORDER BY created_at DESC LIMIT 20;"
sqlite3 <path>/knowledge.db "SELECT * FROM knowledge ORDER BY created_at DESC LIMIT 20;"
```
- May have `events`, `knowledge`, or other tables
- If empty or no tables → skip to next layer

### 2. Git Commit History
```bash
cd <project> && git log --oneline --since="7 days ago" --all | head -20
cd <project> && git log --oneline -5 --format="%h %s (%ai)"
cd <project> && git show --stat HEAD~5..HEAD
```
- Focus on last 7 days for active work
- `git show --stat` reveals which files changed

### 3. State Docs (`state-docs/`)
```bash
ls state-docs/
find state-docs/ -name "*.md" -mtime -7
# Read most recent state doc
cat state-docs/<doc>.md
```
- Check modification time for recent docs
- `crabjar-state.md` and checkpoints are common
- Overlays in `state-docs/overlay/*.overlay.json`

### 4. Agent Config / Preamble Files
- `agent_config.md` — agent operational principles
- `AGENTS.md` — repository guidelines
- `project_map.md` — structural alignment reference
- `~/.dotfiles/environment_manifest.json` — system constraints (if present)

### 5. Environment Manifest
```bash
cat ~/.dotfiles/environment_manifest.json
```
- GPU/VRAM constraints, storage layout, RAM, CPU
- Critical for tasks involving inference, builds, or I/O

### 6. Crabjar State
```bash
cat state-docs/crabjar-state.md
```
- Full workspace architecture reference
- Covers all 13 workspace members, CLI commands, pipelines

## Backtrace Workflow

1. **Check knowledge DB** — `sqlite3 memory/knowledge.db ".tables"` then query tables if they exist
2. **Scan git history** — `git log --oneline --since="7 days ago"` for recent structural changes
3. **List state-docs** — `ls state-docs/` and identify most recent docs
4. **Read key state doc** — `crabjar-state.md` for project architecture, or checkpoint docs for session-specific state
5. **Check environment** — `environment_manifest.json` for constraints (VRAM, disk, etc.)
6. **Synthesize findings** — Present a concise summary of:
   - Most recent structural changes (git)
   - Available persistence data (knowledge DB tables)
   - Relevant state-docs
   - Environmental constraints
   - What's missing/stale

## Output Format

Present findings as a structured summary:

```
## Session Backtrace

**Knowledge DB**: <tables found or "empty">

**Recent Changes** (last 7 days):
- `<commit>` — `<message>`
- ...

**State Docs**:
- `<doc>` — `<last modified>` — `<brief description>`

**Environment**:
- GPU: <VRAM available>
- RAM: <available>
- Storage: <key constraints>

**Gaps**:
- What we couldn't recover
- What needs to be re-established
```

## When to Trigger

- User says "pick up from," "resume," "what were we doing," "backtrace"
- Starting work on a project after a gap
- Resuming a .dotfiles or crabjar session
- Any context where previous conversation state is lost

## Key Insight

The knowledge DB is often empty — don't assume it has data. Git history and state-docs are the reliable backtrace layers. Environment constraints (especially VRAM) are critical for inference-related tasks.

## Boundary with `env-aware`

`session-backtrace` reads environment data from existing sources (state-docs, git history, manifests) to reconstruct context. It does NOT do live system probing — that is the job of `env-aware`. If the environment data in state-docs is stale or sparse, delegate to `env-aware` for live probes.
