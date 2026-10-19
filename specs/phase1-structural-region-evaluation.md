# Phase 1: Structural Region Evaluation

**Status:** Draft  
**Date:** 2026-10-19  
**Applies to:** crabjar JEV decision layer, pesti-structural-tokenizer integration

## Goal

Enable JEV to evaluate structural regions (folded spans from pesti-structural-tokenizer) against decision criteria files, replacing free-text observation matching with structured region evaluation.

## Architecture Overview

```
pesti-structural-tokenizer          crabjar memory crate           crabjar conductor API
┌─────────────────────┐    ┌──────────────────────────┐    ┌─────────────────────────┐
│ tokenize(source)    │───▶│ EvaluationTarget         │───▶│ POST /jev/evaluate      │
│ fold(tokens)        │    │ { structure, range,      │    │   target: EvaluationTarget │
│                     │    │   semantic_tags }        │    │   criteria_file: string  │
└─────────────────────┘    └──────────────────────────┘    └─────────────────────────┘
```

## Core Abstractions

### EvaluationTarget (new type in crabjar memory crate)

Represents a structural region ready for JEV evaluation.

```heml
struct EvaluationTarget {
    // Structural identity
    structure: SpanKind,        // FnDecl, StructDecl, ImplBlock, etc.
    name: string,               // Function/struct/module name
    range: SourceRange,         // Byte range in source file
    
    // Contextual metadata
    file_path: string,
    module_path: string,        // e.g., "crabjar::memory::jev"
    
    // Semantic annotations (from Layer 2)
    ownership_mode: OwnershipMode?,     // Owned | Borrowed | MutablyBorrowed
    error_handling: ErrorHandling?,     // Propagated | Handled | Unhandled
    mutability: MutKind?,               // Immutable | Mutable
    
    // Raw structural tokens for embedding fallback
    token_sequence: string,             // Compact representation of span's tokens
}

struct SourceRange {
    start_byte: usize,
    end_byte: usize,
}
```

### Structural Criteria File Format (JSON)

Evolved from text-based criteria to support structural predicates.

```json
{
    "name": "error_handling_adequacy",
    "version": "1.0",
    "description": "Evaluate whether a function has adequate error handling",
    
    "fast_path_rules": [
        {
            "condition": "structure == FnDecl && error_handling == Propagated",
            "decision": "adequate",
            "confidence": 0.95,
            "reason": "Function uses ? operator for error propagation"
        },
        {
            "condition": "structure == FnDecl && error_handling == Unhandled",
            "decision": "inadequate",
            "confidence": 0.90,
            "reason": "Function has no error handling strategy"
        }
    ],
    
    "embedding_fallback": {
        "prompt_template": "Evaluate error handling in function {name} ({structure}) with properties: ownership={ownership_mode}, error={error_handling}. Is the error handling adequate?",
        "similarity_threshold": 0.8,
        "positive_criteria": ["adequate", "sufficient", "appropriate"],
        "negative_criteria": ["inadequate", "insufficient", "missing"]
    }
}
```

### Condition Expression Language

Fast-path rules use a simple expression language for structural predicates:

- **Equality:** `structure == FnDecl`, `error_handling == Propagated`
- **Boolean:** `has_annotation && confidence > 0.8`
- **Numeric comparison:** `complexity_score < 10`
- **Null checks:** `ownership_mode != null`

Supported operators: `==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`, `!`  
Supported values: string literals, enum variants, numeric literals, boolean literals

## Integration Points

### 1. pesti-structural-tokenizer → crabjar memory crate

**What changes:** Define `EvaluationTarget` type in crabjar's memory crate with fields that map to pesti-structural-tokenizer output types.

**No pesti changes required.** The tokenizer already produces:
- `StructuralToken` with kind, text, position
- `Span` hierarchy via `fold()`
- Semantic annotations via separate analyzers

Crabjar's memory crate wraps these into `EvaluationTarget`.

### 2. crabjar memory crate → conductor API

**What changes:** Extend JEV evaluation API to accept structured targets.

**New endpoint:** `POST /jev/evaluate-structural`
```heml
{
    "target": EvaluationTarget,
    "criteria_file": "error_handling_adequacy.json"
}
```

**Existing endpoint adapted:** `POST /jev/evaluate` accepts optional `structure` field:
```heml
{
    "observation": "fn process_payment uses ? for errors",
    "structure": { "kind": "FnDecl", "name": "process_payment" },
    "criteria_file": "error_handling_adequacy.json"
}
```

### 3. JEV Three-Tier Flow Adaptation

**Tier 1 — Rule Engine Fast-Path (modified):**
- Evaluate structural predicates instead of keyword matching
- Parse condition expressions from criteria files
- Match against `EvaluationTarget` fields directly

**Tier 2 — Cache Layer (unchanged):**
- Cache key: SHA-256 of serialized `EvaluationTarget` + criteria file hash
- TTL and eviction unchanged

**Tier 3 — Embedding Similarity (modified):**
- Serialize structural region into embeddable text using prompt template
- Template variables populated from `EvaluationTarget` fields
- Example: `{name}` → "process_payment", `{structure}` → "FnDecl"

## Implementation Tasks

### Task 1.1: Define EvaluationTarget Type

**Location:** `crabjar/memory/src/jev/types.rs` (new file)

```heml
// Core evaluation target type for structural regions
pub struct EvaluationTarget {
    pub structure: SpanKind,
    pub name: String,
    pub range: SourceRange,
    pub file_path: String,
    pub module_path: String,
    pub ownership_mode: Option<OwnershipMode>,
    pub error_handling: Option<ErrorHandling>,
    pub mutability: Option<MutKind>,
    pub token_sequence: String,
}

// Span kinds from pesti-structural-tokenizer
#[derive(Debug, Clone, PartialEq)]
pub enum SpanKind {
    FnDecl,
    StructDecl,
    EnumDecl,
    ImplBlock,
    TraitDecl,
    ModDecl,
    // ... other kinds
}

// Semantic annotation types
#[derive(Debug, Clone, PartialEq)]
pub enum OwnershipMode { Owned, Borrowed, MutablyBorrowed }
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorHandling { Propagated, Handled, Unhandled }
#[derive(Debug, Clone, PartialEq)]
pub enum MutKind { Immutable, Mutable }
```

### Task 1.2: Structural Condition Evaluator

**Location:** `crabjar/memory/src/jev/rule_engine.rs` (new module)

Implement expression parser and evaluator for structural predicates:

```heml
pub fn evaluate_condition(target: &EvaluationTarget, condition: &str) -> bool {
    // Parse "structure == FnDecl && error_handling == Propagated"
    // Evaluate against target fields
    // Return true/false
}
```

### Task 1.3: Adapt JEV Evaluation API

**Location:** `crabjar/orchestrator/src/api/jev.rs` (modify)

Add structural evaluation endpoint and adapt existing endpoint to accept optional structure field.

### Task 1.4: Embedding Serialization for Structural Targets

**Location:** `crabjar/memory/src/jev/embedding.rs` (new module)

```heml
pub fn serialize_for_embedding(target: &EvaluationTarget, template: &str) -> String {
    // Replace {name}, {structure}, {ownership_mode}, etc. with target values
    // Return embeddable text string
}
```

### Task 1.5: Test Criteria Files

**Location:** `crabjar/memory/tests/criteria/` (new directory)

Create test criteria files for:
- Error handling adequacy
- Ownership pattern validation
- API surface documentation completeness

### Task 1.6: Integration Tests

**Location:** `crabjar/orchestrator/tests/jev_structural.rs` (new file)

Test end-to-end flow:
1. Tokenize real Rust source file
2. Fold into structural spans
3. Create EvaluationTarget from span
4. Evaluate against test criteria file
5. Verify decision and confidence score

## Verification Criteria

Phase 1 is complete when:

1. `EvaluationTarget` type is defined in crabjar memory crate with all required fields
2. Structural condition evaluator can parse and evaluate predicates like `"structure == FnDecl && error_handling == Propagated"`
3. JEV evaluation API accepts structured targets and returns decisions
4. Embedding serialization works with prompt templates populated from target fields
5. Integration tests pass: tokenizing real code → folding → evaluating → getting decision

## Open Questions

1. **SpanKind enum completeness:** Should crabjar define its own SpanKind enum or re-export from pesti-structural-tokenizer? Re-exporting creates a dependency; defining locally allows independent evolution.
2. **Condition expression complexity:** Should we support function calls in conditions (e.g., `contains(error_handling, "Propagated")`), or keep to simple field comparisons?
3. **Token sequence format:** What's the best compact representation of a span's tokens for embedding fallback? Raw token text? Token kinds only? Both?

## Risks

| Risk | Mitigation |
|------|------------|
| pesti-structural-tokenizer API changes break crabjar integration | Define stable interface; use versioned dependency |
| Condition expression parser becomes too complex | Start with simple equality/comparison operators; add complexity iteratively |
| Embedding serialization produces poor quality prompts | Test with real criteria files; iterate on template format |
