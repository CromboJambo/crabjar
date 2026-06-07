---
name: skill-creator
description: Create, write, or improve a skill for corust-agent. Use this skill whenever the user wants to make a new skill, edit an existing skill's SKILL.md, improve a skill's description, add bundled scripts or references to a skill, figure out where to install a skill, or understand how skills work in corust. Also trigger when the user asks "how do I add a skill", "can you turn this workflow into a skill", "what should go in SKILL.md", or "my skill isn't triggering correctly".
keywords: [skill, creator, documentation, setup]
trigger: on_manual_call # Triggered when the user explicitly asks to create or modify skills
---

# Skill Creator

Skills are modular instruction sets that extend corust-agent with specialized knowledge, workflows, and bundled resources. This guide covers creating and improving them.

## How skills work in corust

### Activation

Skills activate in three ways:

1. **Slash command**: Type `/skill-name` in the chat — corust maps it to the skill automatically
2. **Text mention**: Write `$skill-name` in a message
3. **Markdown link**: Insert `[@skill-name](skill:///absolute/path/to/SKILL.md)` inline

The model also reads the skill catalog at session start and self-selects relevant skills based on their descriptions.

### Discovery: where to put skills

corust scans these directories (project-level takes precedence over user-level):

| Scope | Paths scanned |
|-------|--------------|
| Project | `.corust-agent/skills/`, `.agents/skills/` (relative to cwd, ancestors up to home) |
| User | `~/.corust-agent/skills/`, `~/.agents/skills/` |

Use `~/.corust-agent/skills/` for personal skills available across all projects. Use `.agents/skills/` (committed to the repo) for project-specific skills that teammates share.

### Progressive disclosure — three tiers

| Tier | Content | When loaded | Token cost |
|------|---------|-------------|------------|
| 1. Catalog | `name` + `description` | Session start | ~50–100 tokens per skill |
| 2. Instructions | Full `SKILL.md` body | When skill activates | < 5 000 tokens |
| 3. Resources | Scripts, references, assets | On demand (model reads as needed) | Varies |

This means: keep the `description` precise (it's always in context), keep the body under 500 lines (loaded on activation), and put large reference material in `references/` (loaded only when needed).

---

## Skill anatomy

```
skill-name/
├── SKILL.md          (required)
├── scripts/          (optional) — executable code
├── references/       (optional) — docs loaded into context as needed
└── assets/           (optional) — output templates, icons, data files
```

### SKILL.md structure

```yaml
---
name: skill-name          # must match the directory name
description: |            # the primary trigger — what it does AND when to use it
  ...
---

# Skill Title

Instructions for the model...
```

Only `name` and `description` are required in frontmatter. No other fields.

### `scripts/` — bundled executables

Include scripts when the same code would be rewritten on every invocation. The model can execute them without loading them into context, saving tokens and ensuring consistency.

### `references/` — documentation loaded on demand

Large domain knowledge, schemas, API docs, or detailed guides. Keep `SKILL.md` lean; reference these files with clear guidance on when to read them. For files over 100 lines, add a table of contents at the top.

### `assets/` — output files

Templates, icons, boilerplate directories. Not loaded into context — the model copies or modifies them as output.

---

## Creation process

### 1. Understand the skill

Before writing, clarify:
- What should this skill enable?
- What does a user say to trigger it? Give 2–3 concrete example prompts.
- Are there recurring scripts, references, or templates worth bundling?
- Does the skill depend on external data files (manifests, configs)? Check them for sparsity — placeholder notes vs concrete values. If sparse, the workflow must include live probes as fallback.
- Does the skill depend on directories that may not exist? Plan creation steps for missing resource dirs.

### 2. Write SKILL.md

**Description** is the most important field. It controls triggering. Write it to include:
- What the skill does
- Specific contexts and user phrases that should activate it
- Edge cases where it should (and shouldn't) trigger

Err toward being a little "pushy" in the description — models tend to undertrigger skills. Instead of `"Use when working with PDFs"`, prefer `"Use whenever the user mentions PDFs, asks to extract or edit a document, or shares a .pdf file path — even if they don't say 'PDF skill' explicitly"`.

**Body** — write instructions for the model, not the user. Use the imperative. Include:
- The core workflow
- How to use any bundled resources
- Pointers to `references/` files with guidance on when to read each

### 2.5. Verify source data (if the skill depends on external files)

If the skill references manifests, configs, or data files:
1. **Read them first**. Check for sparsity — placeholder notes ("check live system for exact capacity") vs concrete values.
2. **If sparse**: add a fallback workflow step in SKILL.md that specifies which live probes to run (parallel, not sequential).
3. **If missing**: add a creation step — the model should create the file/directory before using it.
4. **After updating**: write the new data back to the source file so it stays current.

### 3. Create bundled resources (if needed)

- **Scripts**: Write, then test by running them. Only include scripts that will be reused.
- **References**: Move detailed documentation here. Keep `SKILL.md` focused on procedure.
- **Assets**: Copy templates or boilerplate here for the model to use as output.

Do not create `README.md`, `CHANGELOG.md`, or other auxiliary documentation. The skill contains only what an AI agent needs to do the job.

### 4. Test the skill

Install the skill directory and trigger it:

```
/skill-name test this out with a sample prompt
```

Check that:
- The slash command resolves correctly
- The model reads `SKILL.md` and follows the instructions
- Bundled resources are accessible (model can read scripts/references by path)
- The description correctly triggers in contexts you expect
- If the skill uses mock environments, cross-check mock behavior against a live probe to confirm realism

### 5. Iterate

After testing, refine based on what the model actually did versus what you intended:
- If triggering is inconsistent → strengthen the `description`
- If the model ignores a step → make the instruction more explicit or explain the *why*
- If the model rewrites a script every time → add it to `scripts/` and reference it in the body
- If `SKILL.md` is getting long → split detailed content into `references/` files and add pointers
- If the skill depends on a manifest or config that was sparse → update it with live data before finalizing
- If a resource directory didn't exist → create it with appropriate content before finalizing

## Reference Material

- `references/skill-anatomy.md` — skill directory structure, naming constraints, progressive disclosure tiers

## Bundled Scripts

- `scripts/skill_create.sh` — create new skill directory with SKILL.md skeleton
- `scripts/skill_validate.sh` — validate skill directory structure and constraints

---

## Writing guidelines

- **Imperative form**: "Read the schema before querying", not "You should read..."
- **Explain why**: Models respond better to reasons than mandates. Prefer "X matters because Y" over `ALWAYS do X`.
- **Avoid over-constraining**: Don't lock in one approach when multiple valid approaches exist. Use low freedom (specific scripts) only for fragile, repetitive operations.
- **No deeply nested references**: All reference files should link directly from `SKILL.md`, not from each other.
- **Skill naming**: lowercase, hyphens, under 64 characters. Match directory name exactly.

---

## Quick reference: valid skill directory locations

```
# User-wide (recommended for personal skills):
~/.corust-agent/skills/my-skill/SKILL.md

# Cross-client (visible to other agents that support .agents/skills):
~/.agents/skills/my-skill/SKILL.md

# Project-specific (commit to repo for team sharing):
.agents/skills/my-skill/SKILL.md
.corust-agent/skills/my-skill/SKILL.md
```

## Persistence

`.agents/skills/` files committed to the repo persist across sessions. Zed updates its skill catalog on restart — restart zed to persist `.agent` skills to the active catalog.

## Ground-truth file maintenance

Skills that depend on external data files (manifests, configs, schemas) should include a maintenance step:
- **Stale detection**: add a threshold (e.g., "> 7 days") for when the file is considered stale
- **Update workflow**: specify which probes or commands to run to refresh the data
- **Write-back**: the model should write the refreshed data back to the source file, not just read it
- **Sparsity flag**: if the file contains placeholder notes instead of real values, treat it as stale and run the update workflow immediately

