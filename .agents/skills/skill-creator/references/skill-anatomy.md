# Skill Anatomy Reference

## Valid skill directory locations

| Scope | Paths |
|---|---|
| User-wide | `~/.corust-agent/skills/`, `~/.agents/skills/` |
| Project | `.agents/skills/`, `.corust-agent/skills/` (relative to cwd) |

## Directory structure

```
skill-name/
├── SKILL.md          (required)
├── scripts/          (optional) — executable code
├── references/       (optional) — docs loaded into context as needed
└── assets/           (optional) — output templates, icons, data files
```

## SKILL.md requirements

- **name**: must match directory name exactly
- **description**: primary trigger — what it does AND when to use it

## Naming constraints

- lowercase, hyphens, under 64 characters
- match directory name exactly

## Progressive disclosure tiers

| Tier | Content | When loaded | Token cost |
|---|---|---|---|
| 1. Catalog | name + description | Session start | ~50–100 tokens per skill |
| 2. Instructions | Full SKILL.md body | When skill activates | < 5 000 tokens |
| 3. Resources | Scripts, references, assets | On demand | Varies |

## Ground-truth file maintenance

- Stale detection: > 7 days without modification
- Update workflow: live probes to refresh data
- Write-back: model writes refreshed data to source file
- Sparsity flag: placeholder notes vs real values → treat as stale
