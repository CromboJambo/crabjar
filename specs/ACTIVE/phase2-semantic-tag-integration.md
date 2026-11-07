# Phase 2: Semantic Tag Integration — Completion Spec

**Date:** 2026-10-19  
**Applies to:** crabjar JEV decision layer + pesti-structural-tokenizer semantic analyzers

## Current State

Phase 1 delivered the structural condition evaluator. `EvaluationTarget` has a `semantic_tags` HashMap field, and the condition evaluator supports dot notation access (`target.error_handling`). However:

- Semantic tags are manually populated in tests; no integration with actual pesti analyzers
- No production criteria files leverage semantic properties
- No performance benchmarking of tagged vs. untagged evaluation
- The `to_observation()` method ignores semantic tags entirely

## Phase 2 Tasks

### Task 2.1: Integrate Pesti Semantic Analyzers

**What:** Wire pesti-structural-tokenizer's ownership, error handling, and mutability analyzers into the evaluation pipeline.

**Implementation:**
- Import analyzer functions from pesti-structural-tokenizer crate (or re-implement if API differs)
- Create `annotate_target(source: &str, target: &EvaluationTarget)` function that runs analyzers on the source text within the target's range and populates `semantic_tags`
- Integrate into `/jev/evaluate` endpoint: run annotation before evaluation

**Verification:** Evaluating a real function declaration automatically populates ownership/error/mutability tags without manual input.

### Task 2.2: Extend EvaluationTarget with Typed Semantic Fields

**What:** Add typed fields alongside the generic HashMap for better ergonomics and type safety.

**Implementation:**
```heml
pub struct EvaluationTarget {
    // ... existing fields ...
    
    // Typed semantic fields (populated by analyzers)
    pub ownership_mode: Option<OwnershipMode>,
    pub error_handling: Option<ErrorHandling>,
    pub mutability: Option<MutKind>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OwnershipMode { Owned, Borrowed, MutablyBorrowed, Unknown }
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorHandling { Propagated, Handled, Unhandled, Unknown }
#[derive(Debug, Clone, PartialEq)]
pub enum MutKind { Immutable, Mutable, Unknown }
```

**Note:** Keep `semantic_tags` HashMap for extensibility; typed fields are the primary interface.

**Verification:** Condition evaluator can access both typed fields (`target.error_handling`) and map entries (`target.semantic_tags["custom_tag"]`).

### Task 2.3: Create Semantic Criteria Files

**What:** Production criteria files that leverage semantic properties in fast-path rules.

**Criteria to create:**
1. `error_handling_adequacy.json` — evaluates whether functions have adequate error handling based on `error_handling == Propagated` vs other modes
2. `ownership_pattern_validation.json` — validates ownership patterns (e.g., borrowed references should be immutable)
3. `api_surface_documentation.json` — checks that public API functions have proper documentation

**Verification:** Each criteria file has at least one fast-path rule using semantic tags, and evaluation against test targets produces correct decisions.

### Task 2.4: Update to_observation() for Semantic Tags

**What:** Include semantic tag information in the observation string used for embedding fallback.

**Implementation:**
```heml
impl EvaluationTarget {
    pub fn to_observation(&self) -> String {
        let mut parts = vec![format!("{} '{}'", self.structure, self.name)];
        
        if let Some(oh) = &self.error_handling {
            parts.push(format!("error_handling={}", oh));
        }
        if let Some(om) = &self.ownership_mode {
            parts.push(format!("ownership={}", om));
        }
        // ... other tags
        
        parts.join(", ")
    }
}
```

**Verification:** Observation strings include semantic tag information for embedding evaluation.

### Task 2.5: Performance Benchmarking

**What:** Measure evaluation performance with vs. without semantic tags to validate the fast-path benefit.

**Implementation:**
- Create benchmark that evaluates 100 function targets against criteria files
- Measure time with structural-only conditions vs. semantic-tag conditions
- Verify that fast-path rules using semantic tags are faster than embedding fallback

**Verification:** Semantic tag evaluation is measurably faster than embedding fallback for matching conditions (expected: <1ms vs ~700ms per decision).

## Implementation Order

1. Task 2.2 — Typed semantic fields (foundation)
2. Task 2.4 — Update to_observation() (quick win)
3. Task 2.1 — Integrate analyzers (core feature)
4. Task 2.3 — Create criteria files (demonstration)
5. Task 2.5 — Benchmarking (validation)

## Success Criteria

Phase 2 is complete when:
- `EvaluationTarget` has typed semantic fields populated automatically by analyzers
- Condition evaluator supports both typed field access and map entry access
- Production criteria files use semantic properties in fast-path rules
- Observation strings include semantic tag information
- Benchmarking demonstrates fast-path performance benefit over embedding fallback
