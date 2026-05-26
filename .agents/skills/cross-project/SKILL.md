---
name: cross-project
description: |
  Manage references to external projects in opencode sessions. Use whenever the user wants to
  allow viewing another project's files, add another project's AGENTS.md or config to instructions,
  reference external project context, configure cross-project access, or set up gated/sandboxed
  modifications to external projects. Trigger when the user mentions "other projects", "external
  projects", "add project reference", "view another project", "cross-project", "bring in context
  from", "link another workspace", "sandbox changes", or "gated access".
---

# Cross-Project Reference Manager

Add external project files to opencode's instruction set so the agent can reference them
during the current session. Supports read-only preamble loading and gated write access
for version-controlled changes.

## Pre-configured Projects

| Project | Path | Key Files |
|---------|------|-----------|
| LinuxFromCrates | `/home/crombo/LinuxFromCrates/` | `AGENTS.md`, `agent_config.md`, `lfc.toml`, `Justfile` |
| .dotfiles | `/home/crombo/.dotfiles/` | `AGENTS.md`, `environment_manifest.json`, `state-docs/`, `symlinks/` |

## Access Control

Agent access is declared in `~/.dotfiles/manifest/graph.toml`. All mutations go to the owner dir (`~/.dotfiles/.config/`). Symlinks in `~/.config/` are immutable borrows — you never write through them.

- `symlink-enforce.sh` — validates the graph (`cargo check` equivalent)
- `symlink-apply.sh --grant NAME` — adds a new symlink (`cargo install` equivalent)
- `symlink-apply.sh --revoke NAME` — removes a symlink (`cargo uninstall` equivalent)

## Methods

### 1. Add to opencode.json `instruction_paths`

Edit `opencode.json` and add the external project's rule/config files:

```json
{
  "instruction_paths": [
    "/home/crombo/LinuxFromCrates/AGENTS.md",
    "/home/crombo/LinuxFromCrates/agent_config.md",
    "/home/crombo/.dotfiles/AGENTS.md",
    "/home/crombo/.dotfiles/environment_manifest.json"
  ]
}
```

Files are loaded at session start. Paths can be absolute or relative.

### 2. Symlink from dotfiles repo

Add an entry to `~/.dotfiles/manifest/graph.toml` and run `symlink-apply.sh`:

```toml
[[entries]]
name = "lfc-rules"
source = "/home/crombo/LinuxFromCrates/AGENTS.md"
dest = "/home/crombo/.config/opencode/lfc-rules.md"
type = "immutable"
notes = "LinuxFromCrates agent rules"
```

```bash
~/.dotfiles/symlinks/tools/symlink-apply.sh  # creates the symlink
```

Then add the symlink path to `instruction_paths`.

### 3. Global rules file

For rules that should apply to all sessions across all projects:

```json
{
  "global_rules_path": "/home/crombo/.config/opencode/cross-project-rules.md"
}
```

### 4. Remote GitHub URL

For version-controlled external context:

```json
{
  "instruction_paths": [
    "https://raw.githubusercontent.com/<user>/<repo>/main/AGENTS.md"
  ]
}
```

## Gated Write Access

External projects can be modified through a gated workflow:

1. **Sandbox mode**: Changes are staged in a temp directory or branch, not applied directly
2. **Review gate**: User reviews proposed changes before application
3. **Apply gate**: Approved changes are committed to the external project's VCS
4. **Userland merge**: User manually applies or merges changes to their system

### Sandbox configuration

```json
{
  "sandbox_mode": "workspace-write",
  "approval_required": true,
  "apply_to": ["/home/crombo/LinuxFromCrates/", "/home/crombo/.dotfiles/"]
}
```

### Gated workflow

1. Agent proposes changes in a temp dir or feature branch
2. Agent presents diff to user for review
3. On approval, agent commits to the external project's VCS
4. Agent presents a summary of what changed and where to apply
5. User manually applies changes (symlinks, config reload, etc.)

## Workflow

1. Identify which files from each external project you want referenced (AGENTS.md, agent_config.md, project_map.md, etc.)
2. Choose scope: local to this opencode session (instruction_paths) or global (global_rules_path)
3. Add paths to the correct location
4. Restart opencode or reload config for changes to take effect

## Verification

After adding paths, start a new session and check that the external files appear in the loaded preamble context.

## Reference

- `references/preamble-stack-schema.md` — opencode.json structure and AGENTS.md sections
