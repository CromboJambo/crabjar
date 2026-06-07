# Comparity Reference — Feature Parity Patterns

## Comparison Patterns

### Pattern 1: Orthogonality Check

When the user asks "is X orthogonal enough to fork?" or "should we fork X?":

1. Map X's feature surface against crabjar's
2. Identify the **overlap zone** (features both have)
3. Identify **X's unique value** (what X does that crabjar doesn't)
4. Identify **crabjar's unique value** (what crabjar does that X doesn't)
5. Verdict:
   - **Compose** (recommended): different domains, complementary strengths
   - **Fork**: X is a direct substitute AND crabjar can do it better (rare)
   - **Ignore**: X is not relevant to crabjar's domain

### Pattern 2: Design Influence

When the user asks "learn from X's decisions" or "compare against X":

1. Fetch X's architecture docs, not just README
2. Focus on **runtime decisions** (not features):
   - Security model
   - Memory architecture
   - Config schema approach
   - Extension model
   - Binary size strategy
   - Feature flag taxonomy
3. Map each decision to crabjar's equivalent
4. Note where crabjar's approach differs and why
5. Credit where X's design influenced crabjar

### Pattern 3: Gap Analysis

When the user asks "what does X have that we don't?":

1. List X's features that crabjar lacks
2. Categorize gaps:
   - **Critical**: core functionality crabjar needs
   - **Nice-to-have**: useful but not essential
   - **Out of scope**: X does something crabjar intentionally doesn't
3. For each critical gap, propose a solution path

## Output Templates

### Parity Matrix Template

```
## [Target] vs CrabJar — Feature Parity

**Verdict**: [Compose / Fork / Ignore] — [one-line rationale]

### Feature Parity

| Target Feature | CrabJar Equivalent | Status | Notes |
|---|---|---|---|
| ... | ... | ✅ / ❌ / ⚠️ | ... |

### What [Target] Has That CrabJar Doesn't

1. **Feature** — [description] — [effort to add]
2. ...

### What CrabJar Has That [Target] Doesn't

1. **Feature** — [description]
2. ...

### Design Analysis

- **Orthogonality**: [high/medium/low] — [explanation]
- **Fork viability**: [yes/no] — [rationale]
- **Influence worth**: [what to learn]

### Recommendation

[Compose / Adopt / Fork / Ignore] — [why]
```

### Attribution Template

```markdown
**Acknowledgments**

[Target] docs inspired:
- [Feature] — [specific influence]
- [Feature] — [specific influence]

[Target]'s contribution: [what they do]
Crabjar's contribution: [what we do differently]
```

## Common Target Patterns

### Rust projects (like ZeroClaw)

Fetch: `Cargo.toml`, `README.md`, `AGENTS.md`, `docs/book/src/`, `ROADMAP.md`

### TypeScript projects (like opencode)

Fetch: `package.json`, `README.md`, `AGENTS.md`, `src/` structure, `CLAUDE.md`

### Mixed/monorepo projects

Fetch: workspace root manifest, each sub-project's README, architecture docs

## When to Skip

- Target is a SaaS (no code to analyze)
- Target's code is behind auth wall
- Target is a UI-only project (no runtime to compare)
- Target is too large to analyze in one pass (focus on core runtime only)
