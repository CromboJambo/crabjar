---
name: how-foo
description: |
  Explain a process to the user so they can execute it themselves. Trigger whenever
  the agent is blocked from doing something that requires user privileges (sudo, system
  config, browser auth, etc.) and needs to teach the user how to do it. Also trigger
  when the user asks "how do I..." for a task the agent can't complete alone, or when
  the agent wants to dog-food a workflow back to the user. Covers: system commands,
  external project setup, reproduction guides from reference directories, and any
  process the agent detects but cannot act on.
---

# How-Foo: Teach-the-User Guide Generator

The agent's mechanism for explaining processes it's blocked from executing. When the
agent detects a task requiring user action (sudo, system config, auth, etc.), this
skill generates a clear, executable guide the user can follow.

## When to Activate

- Agent detects a required action it **cannot** perform (sudo, user auth, browser login)
- User asks "how do I..." for a task the agent can't complete alone
- Agent needs to dog-food a workflow back to the user
- User provides a directory path with reproduction intent
- User says "turn this into a skill" for a workflow involving external setup

## Output Modes

### Mode 1: Direct Instructions (default)

For straightforward tasks, present clear steps:

```
## Actions to run

1. `sudo <command>` — what it does
2. `rsync ...` — what it does

## Notes
- Why each step is needed
- Any caveats or gotchas
```

### Mode 2: Structured Guide (for complex workflows)

For multi-step or reference-heavy tasks:

```
# Reproducing <name>

## Source
<path> — <description>

## Steps
1. <command> — <why>
2. <command> — <why>

## Key Dependencies
| Crate | Version | Purpose |

## Notes
<edge cases, platform restrictions>
```

### Mode 3: Dog-food Back

When the agent has partial knowledge but needs user input (auth, confirmation):

```
## What I detected
<what the agent found>

## What I can't do
<what requires user action>

## What you need to do
1. <specific action>
2. <specific action>
```

## Workflow

1. **Detect the gap** — what can the agent see vs what can it act on?
2. **Classify the task** — direct instruction, structured guide, or dog-food back?
3. **Generate the guide** — use the appropriate output mode
4. **Include provenance** — where the agent got its info (pacman log, git history, file scan)
5. **Flag what needs user judgment** — decisions only the user can make

## When to Skip

- The agent can do it itself without user intervention — don't generate a guide
- The user explicitly says "just do it" or "I trust you" (within agent's authority)
- The task is already covered by another skill (e.g., `post-update-audit` handles
  post-update sudo actions; use that skill instead of generating a raw guide)

## Key Principle

The guide must be **executable without ambiguity**. Every step should be a copy-paste
command with a brief explanation of what it does. Never say "run the appropriate
command" — give the actual command.

## Reference Material

- `references/repro-guide-template.md` — reproduction guide template and required fields

## Bundled Scripts

- `scripts/repro_guide.sh` — generate reproduction guide from directory
