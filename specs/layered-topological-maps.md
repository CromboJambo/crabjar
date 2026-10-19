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

### Phase 1: Structural Region Evaluation (Weeks 1-2) ✓ COMPLETE

**Goal:** Enable JEV to evaluate structural regions instead of free-text observations.

Tasks:
- [x] Define `EvaluationTarget` type in crabjar memory crate → `crabjar-conductor/src/decision/structural.rs`
- [x] Adapt JEV three-tier flow for structured targets → `evaluate_structural_conditions()` + embedding fallback in `api.rs`
- [x] Create test criteria files for error handling and ownership patterns → 3 production JSON files in `specs/criteria/`
- [x] Integration test with pesti-structural-tokenizer output → `build_target()` function analyzes real source

**Verification:** JEV can evaluate a folded span against a criteria file and return a structured decision. ✓

**Implementation notes:**
- SpanKind enum covers 25 structural token kinds mapped from pesti-structural-tokenizer
- EvaluationTarget includes both typed semantic fields (Phase 2) and legacy HashMap for backward compatibility
- API endpoint: POST /jev/evaluate with target serialization supporting tokens or source+range

### Phase 2: Semantic Tag Integration (Weeks 3-4) ✓ COMPLETE (minus benchmark)

**Goal:** Overlay semantic annotations onto structural evaluation targets.

Tasks:
- [x] Integrate ownership/error/mutability analyzers into evaluation pipeline → `build_target()` detects patterns from source text
- [x] Extend `EvaluationTarget` with semantic tag fields → typed enums: ErrorHandling, OwnershipMode, Mutability
- [x] Create criteria files that leverage semantic tags in fast-path rules → error-handling-adequacy.json uses `error_handling == Propagated`
- [ ] Benchmark evaluation performance with semantic tags vs. without

**Verification:** Decision criteria can reference semantic properties (e.g., "error_mode == Propagated") in fast-path rules. ✓

**Implementation notes:**
- Error handling detection: unwrap() → Panicked, .map_err(/with_context → Recovered, return Err/? → Propagated
- Ownership detection: Rc/Arc → RefCounted, Box → Boxed
- to_observation() includes typed tags first, then legacy string tags for backward compatibility
- All 41 conductor crate tests pass including Phase 2 integration tests

### Phase 3: Post-Processor & Reflection Integration ✓ COMPLETE (pivot from original spec)

**Original spec goal:** Generate navigable documentation from systematic evaluation.

**Actual implementation:** Built deterministic post-processing layer that validates JEV decisions against structural rules, plus feedback collection for criteria improvement over time. This is a natural prerequisite for reliable audit trails.

Tasks:
- [x] Post-processor validates JEV decisions against structural rules → `decision/postprocessor.rs` with validate_decision()
- [x] Confidence adjustment based on validation results (-0.15 to -0.05 for issues)
- [x] Feedback collection endpoint for criteria improvement → POST /jev/feedback
- [x] Reflection analysis identifies low-accuracy evaluation options (<70% over 5+ decisions) → reflect_on_feedback()
- [x] API integration with automatic post-processing on /jev/evaluate

**Validation rules implemented:**
- Functions must have error handling analysis (−0.1 confidence if missing)
- Structs should have ownership analysis (−0.05 confidence if missing)
- High-confidence decisions need structural tag support (−0.15 confidence if missing)

**Original Phase 3 tasks deferred to Phase 4:**
- [ ] Build audit trail recording system for JEV decisions across codebases
- [ ] Extend state-doc generation to incorporate structural analysis results
- [ ] Implement architectural overview generation from folded spans
- [ ] Create CLI workflow: `crabjar analyze <path> --criteria <file>`

### Phase 4: Multi-Crate Workspace Analysis ✓ COMPLETE (validated against crabjar workspace)

**Goal:** Scale to entire workspaces with inter-crate dependency analysis.

Tasks:
- [x] Extend structural map to represent cross-crate dependencies → `discover_crates()` + `build_dependency_graph()` in `decision/workspace.rs`
- [x] Evaluate workspace-level architectural properties → max depth, circular deps, orphaned crates, coupling/cohesion scores
- [x] Generate workspace-wide audit trail and overview → `analyze_workspace()` with per-crate evaluation results
- [x] Optimize for large codebases using budget-aware collapsing → `AnalysisBudget` struct with max_crates and time limits

**Verification:** Analysis of crabjar workspace produces coherent architectural overview across all crates. ✓

**Validation results (crabjar workspace, 5-crate budget):**
- Discovered 5 crates with correct dependency graph
- Max dependency depth: 1
- No circular dependencies detected
- All 5 crate evaluations matched structural conditions (no embedding fallbacks)
- Average confidence: 1.0

**Implementation notes:**
- `discover_crates()` parses root Cargo.toml for workspace members and individual crate manifests
- Dependency graph built by matching declared dependencies to discovered crate names
- Architectural properties calculated: depth, cycles, orphans, coupling, cohesion
- Budget-aware evaluation prioritizes most depended-upon crates when budget is limited

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
