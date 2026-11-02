---
name: session-librarian
description: |
  Use when organizing agent sessions by prompt, including finding relevant past sessions, renaming for clarity, archiving completed work, and pruning outdated content. Trigger this when the user says "organize my sessions" or needs session management.
category: productivity
tags: [sessions, organization, cleanup]
---

# Session Librarian

This skill provides workflows for organizing agent sessions by prompt, including finding relevant past sessions, renaming for clarity, archiving completed work, and pruning outdated content to maintain a clean session library.

## Core Workflow

1. **Inventory sessions**: List all current sessions and their metadata
2. **Categorize by purpose**: Group sessions by topic or project
3. **Rename for clarity**: Update session names to be descriptive
4. **Archive completed work**: Move finished sessions to archive
5. **Prune outdated content**: Remove stale or duplicate sessions

## Usage Instructions

### Input Requirements
* Session library path (if not default)
* Organization criteria (optional): by date, topic, project

### Example Prompts
* `/session-librarian organize my agent sessions`
* `Find all sessions related to the crabjar project`
* "Archive completed work and clean up old sessions"

## Reference Files
* `references/session-organization-strategies.md`: Strategies for organizing session libraries
* `references/naming-conventions.md`: Recommended naming conventions for sessions

## Bundled Scripts
None — this skill provides organization methodology and guidance.