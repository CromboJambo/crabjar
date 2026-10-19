# Layered Topological Maps Architecture

**Status:** Draft  
**Date:** 2026-10-19  
**Author:** CromboJambo (with Hermes Agent)  
**Applies to:** crabjar, pesti-structural-tokenizer, JEV decision layer

## Vision

Build a multi-resolution understanding stack for codebases where structural analysis, semantic annotation, and decision-making compose into navigable topological maps. Each layer adds semantic density to the scaffolding below it, enabling systematic architectural analysis rather than text pattern matching.

The system transforms code review from "read and judge" to "query structured regions against decision criteria" — making evaluation reproducible, auditable, and scalable across large codebases.

## Architecture Overview

```
Layer 4: State-Doc Generation (what we know)
   ↑ generates navigable documentation from analysis results
Layer 3: Decision/Attention Layer (what matters now)
   ↑ JEV evaluates structural regions against criteria files
Layer 2: Semantic Annotation Map (what they do)
   ↑ ownership, error handling, mutability properties overlaid on structure
Layer 1: Structural Token Map (what exists)
   ↑ FnDecl, StructDecl, ImplBlock tokens with position info and span hierarchy
```

## Layer Specifications

### Layer 1: Structural Token Map

**Input:** Rust source files  
**Output:** Hierarchical structural token stream with fold/span topology

**Core abstractions:**
- `StructuralToken` — token kind (FnDecl, StructDecl, etc.) + text + source position
- `Span { kind, open_token, body, close_token }` — hierarchical span representing code blocks
- `fold(tokens)` — transforms flat token stream into nested span tree
- `Budget { max_tokens, min_node_bytes }` — resolution control for zoom levels

**Existing implementation:** pesti-structural-tokenizer crate (v0.2+)  
**Key capabilities:** Tokenization, fold/span hierarchy, budget-aware collapsing, CLI wrapper (`struct-tok`)

### Layer 2: Semantic Annotation Map

**Input:** Source text + structural token positions  
**Output:** Position-keyed annotation maps with property tags

**Core abstractions:**
- `SemanticTag` — 8-bit bitmask encoding properties (ownership mode, error handling, mutability)
- `Annotation { position, tag, confidence }` — property mapped to source location
- Analyzer pipeline: OwnershipAnalyzer → ErrorAnalyzer → MutabilityAnalyzer

**Existing implementation:** pesti-structural-tokenizer semantic annotation layer (v0.2+)  
**Key capabilities:** Sidecar analysis without vocabulary inflation, conservative "unknown" reporting

### Layer 3: Decision/Attention Layer

**Input:** Structural regions + semantic annotations + decision criteria files  
**Output:** Structured decisions with confidence scores and doubt blocks

**Core abstractions:**
- `EvaluationTarget` — a structural region (function, module, crate) with its semantic tags
- `DecisionCriteria` — JSON file defining what to evaluate (replaces free-text observation matching)
- `JEV.evaluate(target, criteria)` — three-tier evaluation: rule fast-path → cache → embedding similarity

**Existing implementation:** crabjar JEV decision layer (conductor API)  
**Integration gap:** Currently evaluates free-text observations; needs adaptation for structured targets

### Layer 4: State-Doc Generation

**Input:** Analysis results from layers 1-3  
**Output:** Navigable state documents with architectural overviews, audit trails, and actionable insights

**Core abstractions:**
- `ArchitecturalOverview` — high-level map of codebase structure and properties
- `AuditTrail` — JEV decisions applied across the codebase with outcomes
- `KnowledgeTriage` — categorized findings (actionable, informational, stale)

**Existing implementation:** crabjar memory/state-docs system  
**Integration gap:** Currently generates docs from git/file metadata; needs structural analysis integration

## Integration Architecture

### The Evaluation Pipeline

```heml
// 1. Tokenize and fold into structural map
let tokens = tokenizer.tokenize(source);
let topology = fold(tokens);

// 2. Overlay semantic annotations
let annotations = analyze_semantics(source, tokens);

// 3. Query for regions of interest
let functions = topology.find_all_spans(SpanKind::FnDecl);

// 4. Evaluate each region against decision criteria
for fn in functions {
    let target = EvaluationTarget {
        structure: fn.kind,
        range: fn.range,
        semantic_tags: annotations.in_range(fn.range)
    };
    let decision = jev.evaluate(target, criteria_file);
    audit_trail.record(fn.name, decision);
}

// 5. Generate state doc from audit trail
let overview = generate_state_doc(audit_trail, topology);
```

### Decision Criteria as Structural Queries

Decision criteria files evolve from text pattern matching to structural queries:

```heml
// Old: Free-text observation evaluation
jev.evaluate("fn process_payment uses ? for errors", "error_handling.json");

// New: Structured region evaluation
let target = {
    structure: FnDecl,
    name: "process_payment",
    has_error_annotation: true,
    error_mode: Propagated  // from semantic tags
};
jev.evaluate(target, "error_handling_adequacy.json");
```

Criteria files define structural predicates rather than text patterns:

```json
{
    "name": "error_handling_adequacy",
    "version": "1.0",
    "fast_path_rules": [
        {
            "condition": "structure == FnDecl && has_error_annotation == true && error_mode == Propagated",
            "decision": "adequate",
            "confidence": 0.95
        }
    ],
    "embedding_fallback": {
        "prompt_template": "Evaluate error handling in function {name} with structure {structure} and properties {tags}",
        "similarity_threshold": 0.8
    }
}
```

## Use Cases

### 1. Systematic Code Review

Evaluate every function against a battery of decision criteria:
- Error handling adequacy
- Ownership/mutability patterns
- API surface documentation
- Complexity metrics

Output: Audit trail with per-function decisions and architectural overview highlighting problem areas.

### 2. Architectural Drift Detection

Track structural changes over time by comparing topological maps:
- New functions added to modules
- Changed ownership patterns
- Growing complexity in specific regions

Output: Drift report showing which architectural properties have changed since last analysis.

### 3. Targeted Refactoring Guidance

Use JEV decisions to prioritize refactoring targets:
- Functions with inadequate error handling
- Regions with complex ownership patterns
- High-complexity functions that could be decomposed

Output: Ranked list of refactoring candidates with justification from decision layer.

### 4. Onboarding Documentation

Generate navigable state docs for new developers:
- Architectural overview from structural map
- Key design patterns identified by semantic analysis
- Known issues and technical debt flagged by JEV evaluation

Output: Multi-resolution documentation that zooms from crate overview to function-level details.

## Implementation Phases

### Phase 1: Structural Region Evaluation (Weeks 1-2)

**Goal:** Enable JEV to evaluate structural regions instead of free-text observations.

Tasks:
- Define `EvaluationTarget` type in crabjar memory crate
- Adapt JEV three-tier flow for structured targets
- Create test criteria files for error handling and ownership patterns
- Integration test with pesti-structural-tokenizer output

**Verification:** JEV can evaluate a folded span against a criteria file and return a structured decision.

### Phase 2: Semantic Tag Integration (Weeks 3-4)

**Goal:** Overlay semantic annotations onto structural evaluation targets.

Tasks:
- Integrate ownership/error/mutability analyzers into evaluation pipeline
- Extend `EvaluationTarget` with semantic tag fields
- Create criteria files that leverage semantic tags in fast-path rules
- Benchmark evaluation performance with semantic tags vs. without

**Verification:** Decision criteria can reference semantic properties (e.g., "error_mode == Propagated") in fast-path rules.

### Phase 3: Audit Trail and State-Doc Generation (Weeks 5-6)

**Goal:** Generate navigable documentation from systematic evaluation.

Tasks:
- Build audit trail recording system for JEV decisions across codebases
- Extend state-doc generation to incorporate structural analysis results
- Implement architectural overview generation from folded spans
- Create CLI workflow: `crabjar analyze <path> --criteria <file>`

**Verification:** Running analysis on a real crate produces a navigable state doc with per-function decisions and architectural overview.

### Phase 4: Multi-Crate Workspace Analysis (Weeks 7-8)

**Goal:** Scale to entire workspaces with inter-crate dependency analysis.

Tasks:
- Extend structural map to represent cross-crate dependencies
- Evaluate workspace-level architectural properties
- Generate workspace-wide audit trail and overview
- Optimize for large codebases using budget-aware collapsing

**Verification:** Analysis of crabjar workspace produces coherent architectural overview across all crates.

## Open Design Questions

1. **Criteria file format:** JSON vs. TOML vs. custom DSL? JSON aligns with existing JEV infrastructure; TOML is more readable for humans.
2. **Semantic tag vocabulary:** Should the 8-bit SemanticTag bitmask be extended, or should new properties use separate annotation channels?
3. **Evaluation caching:** How should structural region evaluations be cached? By source hash + position range?
4. **Inter-crate analysis:** Should workspace-level analysis require building dependency graphs, or can it work from filesystem layout alone?
5. **Performance targets:** What's acceptable evaluation time for a full crate analysis? Current JEV is ~679µs per decision; a 100-function crate would be ~68s without optimization.

## Relationship to Existing Systems

- **pesti-structural-tokenizer:** Provides layers 1 and 2 (structural tokens + semantic annotations). No changes required; integration happens in crabjar.
- **JEV decision layer:** Provides layer 3 evaluation infrastructure. Requires adaptation for structured targets (Phase 1).
- **crabjar memory/state-docs:** Provides layer 4 generation infrastructure. Requires extension to incorporate structural analysis (Phase 3).
- **cargo-graph skill:** Complementary tool for dependency analysis; could integrate in Phase 4.

## Success Criteria

The system is successful when:

1. A developer can run `crabjar analyze <path>` and receive a navigable architectural overview with actionable insights within acceptable time (<5 minutes for a medium crate).
2. JEV decisions over structural regions are reproducible — same code + same criteria always produces the same decision (modulo embedding API nondeterminism, which is cached).
3. The audit trail provides sufficient provenance to understand why each decision was made (which rule matched, or which embedding similarity was computed).
4. Analysis scales from single files to entire workspaces using budget-aware resolution control.

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| JEV evaluation too slow at scale | Analysis takes hours for large crates | Use rule fast-path aggressively; cache results; parallelize across functions |
| Semantic analysis misses edge cases | Inaccurate decisions based on wrong tags | Conservative "unknown" reporting; embedding fallback catches missed patterns |
| Criteria files become brittle | Decisions break when code structure changes | Version criteria files; include compatibility notes in state docs |
| Integration complexity overwhelms value | System is too hard to use for the insights gained | Focus on high-value use cases first (code review, drift detection); iterate based on feedback |
