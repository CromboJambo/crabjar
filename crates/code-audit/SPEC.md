# Bug Bounty Hunter — Specification for Accurate Discovery

**Version:** 0.2.0-spec  
**Date:** 2026-11-06  
**Status:** Active development spec  

## Problem Statement

The v0.1.0 implementation produces high false-positive rates because it matches on credential-related identifiers rather than detecting actual vulnerability patterns. Running against crabjar's own codebase produced 12 "HIGH" findings, all of which were false positives — the tool confused "code that manages credentials" with "hardcoded credentials."

## Design Principles

1. **Literal values over identifier names** — Flag `let key = "sk-abc123"` not `let api_key: String`
2. **Parse-aware, not regex-only** — Understand Rust syntax to distinguish declarations from assignments
3. **Context matters** — A password in a function parameter is different from a password in a string literal
4. **Precision over recall** — Fewer findings with higher confidence beats many noisy alerts
5. **Verifiable evidence** — Every finding must include the actual suspicious code, not just a file:line

## Required Improvements

### 1. Hardcoded Credentials (CWE-798) — Fix False Positives

**Current behavior:** Matches any line containing "password", "token", "secret", "key" as identifiers.

**Required behavior:** Match only actual literal secret values:

```rust
// SHOULD flag:
let password = "hunter2";
const API_KEY: &str = "sk-abc123";
let token = String::from("ghp_xxxxxxxxxxxx");

// Should NOT flag:
pub password: String,              // struct field
fn login(password: &str) { }       // function parameter
let p = read_password_from_env();  // runtime value
let err = "Failed to write password";  // error message
```

**Implementation approach:**
- Use `syn` crate to parse Rust into AST
- Walk the AST looking for literal string values assigned to variables with secret-like names
- Exclude function parameters, struct field declarations, type annotations
- Require actual string literals or byte strings, not identifiers

### 2. Complexity Analysis — Fix Empty Snippets

**Current behavior:** Reports line 1 with empty snippet for all files.

**Required behavior:**
- Report the actual function name and line range
- Include the function signature in evidence
- Only flag functions that genuinely exceed complexity thresholds (cyclomatic complexity > 10 or similar)

### 3. Add Real Vulnerability Classes

Beyond hardcoded credentials, implement detectors for:

- **SQL injection** — String concatenation in SQL queries (not parameterized)
- **Command injection** — `exec()` with user-controlled input
- **Unauthenticated endpoints** — Web routes without auth middleware
- **Missing error handling** — Bare `unwrap()` in production code paths
- **Race conditions** — Shared mutable state without synchronization

### 4. Severity Calibration

- HIGH: Actual exploitable vulnerability (injection, auth bypass)
- MEDIUM: Potential issue requiring context (complexity, missing validation)
- LOW: Style/maintainability concerns (naming, documentation)

## Testing Strategy

Create test fixtures that contain both real vulnerabilities and near-misses:

```
tests/fixtures/
  real_hardcoded.rs       // Actual hardcoded secrets — should be flagged
  credential_management.rs // Password handling code — should NOT be flagged
  complex_functions.rs    // Genuinely complex vs simple functions
  injection_examples.rs   // SQL/command injection patterns
```

Run the tool against its own test fixtures and verify:
- All real vulnerabilities are detected (recall)
- No false positives on near-miss cases (precision)

## Success Criteria

The tool is ready for production use when:

1. Running against crabjar produces 0 false positives on credential findings
2. Test fixtures achieve >90% precision and >80% recall
3. Each finding includes verifiable code evidence (not just file:line)
4. Severity ratings are calibrated to actual exploitability, not keyword matching

## Implementation Order

1. **Phase 1:** Fix hardcoded credential analyzer (AST parsing with `syn`)
2. **Phase 2:** Fix complexity analyzer reporting
3. **Phase 3:** Add SQL injection detector
4. **Phase 4:** Add command injection detector
5. **Phase 5:** Build test fixture suite and CI integration

## Phase 6: PEG Structural Analysis (Completed 2026-11-17)

Added `PegStructuralAnalyzer` using `pesti-structural`'s grammar-based parsing for
AST-level vulnerability detection alongside the existing flat token matching.

**Key implementation details:**
- Uses `parse_with_recovery()` for robust parsing of real-world code (strict parse fails on common constructs)
- Traverses both `node.children` and `node.captures` (captures hold the actual parsed content)
- Best-effort initialization: warns if grammars unavailable, doesn't break the hunter
- Rust + Python grammars loaded from pesti-structural's `grammars/` directory

**7 structural rules implemented:**
1. Unsafe blocks (Rust, CWE-1068)
2. Excessive parameters >8 (CWE-252)
3. Deep nesting >6 levels (CWE-1392)
4. Raw pointers `*mut`/`*const` (Rust, CWE-824)
5. eval/exec dynamic code execution (Python, CWE-95)
6. SQL injection via string concatenation (Python, CWE-89)
7. Shell injection via string formatting (Python, CWE-78)

**Known grammar limitations:**
- Doesn't handle `unsafe fn` (expects `visibility? fn`)
- Multi-parameter functions sometimes only capture the first param
- These are pesti-structural grammar improvements, not integration issues
