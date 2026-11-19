# code-audit

Structural code vulnerability analyzer using pesti tokens.

## Analyzers

- **StaticCode** — Flat token/pattern matching (pesti-structural-tokenizer)
- **PegStructural** — Grammar-based structural analysis (pesti-structural PEG grammars)

The PEG analyzer uses `pesti-structural` to parse source code into ASTs via language-specific
`.peg` grammars, then traverses the tree for structural vulnerability patterns:

| Rule | Language | Severity | Description |
|------|----------|----------|-------------|
| Unsafe block | Rust | High | Code in `unsafe {}` blocks (CWE-1068) |
| Excessive params | Rust/Python | Medium | Functions with >8 parameters (CWE-252) |
| Deep nesting | Rust/Python | Medium | Blocks nested >6 levels (CWE-1392) |
| Raw pointers | Rust | High | `*mut` / `*const` usage (CWE-824) |
| eval/exec | Python | Critical | Dynamic code execution (CWE-95) |
| SQL injection | Python | Critical | String concatenation in queries (CWE-89) |
| Shell injection | Python | Critical | String formatting in shell commands (CWE-78) |

## Usage

```bash
# CLI
code-audit <path> --type code

# Via crabjar-conductor API
POST /api/audit {"target": "/path/to/code", "type": "code"}

# Via MCP (crabjar-code-audit-mcp)
tool: audit_code { path, target_type }
```

## Dependencies

- `pesti-structural-tokenizer` — flat token stream for StaticCode analyzer
- `pesti-structural` — PEG grammar parsing for PegStructural analyzer

Both are loaded from the local pesti workspace via path dependencies.
