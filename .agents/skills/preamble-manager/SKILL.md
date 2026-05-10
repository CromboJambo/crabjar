---
name: preamble-manager
description: |
  Manage dynamic instruction loading for opencode sessions. Use whenever the user wants to add external files to opencode instructions, configure remote URL references, set global rules at ~/.config/opencode/AGENTS.md, create symlinks from dotfiles, or update opencode.json with instruction paths. Trigger when the user mentions "opencode instructions", "add rule file", "configure preamble", "dynamic updates", "dotfiles symlink", or provides a path to an instruction file.
---

# Preamble Manager

## Configuration methods

### Local opencode.json
Add file paths to `instructions` field in the project's `opencode.json`: `{"instructions": ["reference_materials/preamble-stack/*.md"]}`. Files loaded on-session start.

### Remote GitHub URL
Use raw URLs for version-controlled dynamic updates: `{"instructions": ["https://raw.githubusercontent.com/<your-repo>/main/preamble-stack/*.md"]}`. Fetch with 5s timeout.

### Global AGENTS.md
Place rules at `~/.config/opencode/AGENTS.md` — applies across all sessions. Symlink from dotfiles repo supported.

## Precedence order
1. local opencode.json instructions → 2. global ~/.config/opencode/AGENTS.md → 3. AGENTS.md files in cwd/ancestors

## Setup workflow
1. identify which files should be loaded as instructions
2. choose scope (local project vs global vs remote)
3. write paths to opencode.json or place/symlink file at correct location
4. verify by running a session and checking that files are loaded in context

## Reference Material

- `references/preamble-stack-schema.md` — opencode.json structure and AGENTS.md sections

## Bundled Scripts

- `scripts/preamble.sh` — load, config, or symlink preamble files