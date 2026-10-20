# ADR-014: Multi-Language Topological Analysis — Lessons Learned

**Date:** 2026-10-19  
**Status:** Accepted  
**Applies to:** universal-structure crate, JEV decision layer

## Context

We completed a four-phase project to extend crabjar's code analysis capabilities from Rust-only to multi-language support with binary analysis. The project implemented:

1. Universal structural token representation (38 SpanKind variants)
2. JavaScript/TypeScript adapter using regex-based parsing
3. ELF binary analysis adapter using the `object` crate
4. Cross-language comparison infrastructure

This ADR captures the key lessons learned during implementation that should inform future work on language adapters, binary analysis, and cross-language tooling.

## Lessons Learned

### 1. Universal Token Vocabulary Must Be Superset of All Languages

The initial design assumed a 1:1 mapping between Rust constructs and universal tokens. In practice, JavaScript/TypeScript required additional token kinds (TryBlock, CatchBlock, ImportStatement) that didn't exist in the Rust-specific vocabulary. 

**Lesson:** Design the universal token set as a superset covering all target languages from the start, not an extension of one language's vocabulary.

### 2. Regex-Based Parsing Is Sufficient for Structural Analysis

The JavaScript adapter uses regex patterns rather than a full parser library. This proved adequate for structural analysis (identifying function boundaries, control flow) but has limitations with complex nested expressions.

**Lesson:** For architectural analysis, regex-based parsing is fast and maintainable. Reserve full AST parsers for semantic analysis requiring precise expression evaluation.

### 3. Binary Analysis Requires Different Symbol Classification Heuristics

ELF binaries use different symbol naming conventions than source code. The `object` crate's SymbolKind enum uses `Text` for executable code sections, not `Func` as initially assumed. 

**Lesson:** When integrating with binary analysis libraries, inspect the actual API documentation and enum values rather than assuming conventional naming. Build integration tests against real binaries early.

### 4. Cross-Language Comparison Needs Careful Metric Design

Comparing architectural properties across languages requires metrics that are meaningful in all contexts. Error handling coverage, for example, must account for different idioms (try/catch vs Result type vs exceptions).

**Lesson:** Define comparison metrics at the architectural level (patterns and properties) rather than syntactic level (specific keywords or constructs).

### 5. Budget-Aware Resolution Scales Across Languages

The budget system (max_tokens, min_node_bytes) designed for Rust analysis works equally well across JavaScript and binary analysis. This validates the design decision to make budget resolution language-agnostic.

**Lesson:** Architectural properties that are useful for one language are generally useful for all languages. Design universal systems with this in mind.

## Decisions

1. **Keep regex-based parsing for adapters:** Full parser libraries add complexity and dependencies without proportional benefit for structural analysis.
2. **Use SymbolKind::Text for function detection in binaries:** This is the correct heuristic for ELF symbol tables.
3. **Design comparison metrics at architectural level:** Avoid language-specific syntax in cross-language evaluation criteria.

## Future Work

- Add Python, Go, and C/C++ adapters using the same architecture
- Implement binary vs source diffing evaluation with real compiled programs
- Validate cross-language comparison findings against manual analysis
- Extend JEV decision layer to support multi-language criteria files