---
name: how-foo
description: |
  Use whenever the user asks to document how to reproduce a workflow from a reference
  directory, or mentions "how-foo", "reproduce this", "repro guide", or provides a
  directory path with reproduction intent. Also trigger when the user says "turn this
  into a skill" for a workflow that involves reproducing external project setup.
---

# How-Foo: Reproduction Guide Generator

## Trigger

User phrases: "how-foo", "reproduce this", "repro guide", "document how to reproduce",
"turn this workflow into a skill", or a directory path with reproduction intent.

## Workflow

1. **Read the target directory** — discover its structure, Cargo.toml, README, and key files.
2. **Extract reproduction data** — source repo URL, build commands, install commands, dependencies, MSRV, release profile, notable features.
3. **Write REPRO.md** — output a reproduction guide to the target directory.

## Output Format

```
# Reproducing the <name> <path> workflow

## Source
The directory <path> contains <project-description>.

## Reproduction Steps
1. Clone/install/build commands
2. Usage commands

## Key Dependencies (Cargo.toml)
| Crate | Version | Purpose |

## MSRV
<version>

## Release Profile
<toml block>

## Notes
<edge cases, platform restrictions, branch info>
```

## When to Skip

- Directory is a local project with no upstream repo (no clone step needed).
- Directory contains no Cargo.toml or README (insufficient data for a repro guide).
- User explicitly says "don't write a file" or "just tell me".

## Reference Material

- `references/repro-guide-template.md` — reproduction guide template and required fields

## Bundled Scripts

- `scripts/repro_guide.sh` — generate reproduction guide from directory
