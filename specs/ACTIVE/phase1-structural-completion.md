# Phase 1 Completion: Structural Region Evaluation

**Status:** In Progress  
**Date:** 2026-10-19  
**Builds on:** `specs/phase1-structural-region-evaluation.md`  
**Previous work:** Commit `f4db809` (type definitions + API endpoint)

## What's Missing

Adversarial review of commit `f4db809` found that only scaffolding was built. The core Phase 1 feature — structural condition evaluation — doesn't exist. This spec defines the remaining work needed to complete Phase 1.

## Remaining Tasks

### Task 1: Move EvaluationTarget to Memory Crate

**Location:** `crabjar/memory/src/jev/types.rs` (new)

Move the type definitions from `crabjar-conductor/src/decision/structural.rs` to the memory crate per the original spec. Update conductor to import from memory.

**Rationale:** The glass abstraction boundary requires shared types at the interface, not inside consumer crates.

### Task 2: Implement Structural Condition Evaluator

**Location:** `crabjar/memory/src/jev/rule_engine.rs` (new)

The core missing feature. Parse and evaluate structural predicates against EvaluationTarget fields.

```heml
// Example condition from criteria file
"structure == FnDecl && error_handling == Propagated"

// Evaluate against target
let result = evaluate_condition(&target, condition);  // true/false
```

**Implementation approach:** Simple recursive descent parser for the expression language defined in the original spec:
- Equality: `field == value`
- Comparison: `<`, `>`, `<=`, `>=`
- Boolean: `&&`, `||`, `!`
- Null checks: `!= null`

Supported field names: `structure`, `name`, `ownership_mode`, `error_handling`, `mutability`  
Supported values: enum variants (FnDecl, Propagated, etc.), string literals, numeric literals

### Task 3: Adapt JEV Three-Tier Flow for Structural Targets

**Location:** `crabjar-conductor/src/decision/structural.rs` (modify)

Replace the current "convert to observation string" approach with the three-tier flow:

1. **Tier 1 — Rule Engine Fast-Path:** Load criteria file, evaluate fast_path_rules against target using condition evaluator. If match found, return decision immediately (~microseconds).
2. **Tier 2 — Cache Layer:** Check cache for target+criteria hash. If hit, return cached decision.
3. **Tier 3 — Embedding Fallback:** Serialize target using prompt template from criteria file, send to embedding backend for similarity matching.

### Task 4: Template-Based Embedding Serialization

**Location:** `crabjar/memory/src/jev/embedding.rs` (new)

```heml
pub fn serialize_for_embedding(target: &EvaluationTarget, template: &str) -> String {
    // Replace {name}, {structure}, {ownership_mode}, etc. with target values
}
```

### Task 5: Test Criteria Files

**Location:** `crabjar/memory/tests/criteria/` (new directory)

Create test criteria files demonstrating the new structural evaluation capability:
- `error_handling_adequacy.json` — evaluates function error handling patterns
- `ownership_validation.json` — validates ownership modes in struct fields

### Task 6: Integration Tests

**Location:** `crabjar-conductor/tests/jev_structural.rs` (new)

End-to-end test flow:
1. Create EvaluationTarget from synthetic structural data
2. Evaluate against test criteria file via condition evaluator
3. Verify fast-path decisions return correct option/confidence
4. Test embedding fallback path with template serialization

## Verification Criteria

Phase 1 is complete when:
1. ✅ EvaluationTarget defined in memory crate (not conductor)
2. ✅ Condition evaluator parses and evaluates structural predicates
3. ✅ JEV evaluation API returns decisions based on structural properties (not just observation strings)
4. ✅ Embedding fallback uses prompt templates populated from target fields
5. ✅ Integration tests pass with real criteria files

## Architecture Diagram

```
POST /jev/evaluate
    │
    ▼
EvaluateStructuralRequest { target, criteria_path }
    │
    ▼
Load criteria file → fast_path_rules[] + embedding_fallback{}
    │
    ▼
For each rule in fast_path_rules:
    evaluate_condition(target, rule.condition)
        ├── true → return rule.decision (fast path, ~µs)
        └── false → next rule
    │
    ▼ (no rules matched)
Cache lookup: hash(target + criteria)
    ├── hit → return cached decision
    └── miss → continue
    │
    ▼
serialize_for_embedding(target, criteria.embedding_fallback.prompt_template)
    │
    ▼
JEV embedding evaluation → similarity matching
    │
    ▼
Cache result → return decision
```
