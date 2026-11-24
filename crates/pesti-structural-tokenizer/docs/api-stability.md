# pesti-structural-tokenizer API Stability

This document formalizes the public API surface of `pesti-structural-tokenizer` and establishes the breaking change policy for consumers, particularly crabjar's JEV (Just Enough Verification) decision-making system.

## Stable API Surface (v0.1.x)

The following types and functions are considered stable and will not change in minor or patch releases:

### Core Tokenization API

```rust
pub struct StructuralTokenizer {
    // Internal state — not exposed
}

impl StructuralTokenizer {
    pub fn new() -> Self;
    pub fn tokenize(&self, src: &str) -> Result<Vec<StructuralToken>, TokenizeError>;
    pub fn tokenize_with_budget(&self, src: &str, budget: Budget) -> Result<Vec<Emission>, TokenizeError>;
    pub fn tokenize_with_semantics(&self, src: &str, budget: Budget) -> Result<Vec<SemanticEmission>, TokenizeError>;
}
```

### Semantic Analyzer API (v0.1.4+)

The `pesti_structural_tokenizer::semantic` module provides four semantic analyzers:

```rust
// Error handling analysis
pub fn analyze_errors(source: &str) -> HashMap<usize, ErrorAnnotation>;

// Mutability analysis
pub fn analyze_mutability(source: &str) -> HashMap<usize, MutabilityAnnotation>;

// Ownership mode analysis (v0.1.4+)
pub fn analyze_ownership(source: &str) -> HashMap<usize, OwnershipAnnotation>;

// Complexity analysis (v0.1.4+)
pub fn analyze_complexity(source: &str) -> HashMap<usize, ComplexityAnnotation>;
```

Each analyzer returns a map from node position to an annotation struct with fields documented in the module docs.

### Semantic Tag Encoding

```rust
pub struct SemanticTag {
    // Encodes ownership (2 bits), error handling (3 bits), mutability (1 bit), confidence (2 bits)
}

impl SemanticTag {
    pub fn encode(ownership: Option<OwnershipMode>, error_handling: Option<ErrorHandling>, mutable: bool, confidence: u8) -> Self;
    pub fn ownership(&self) -> Option<OwnershipMode>;
    pub fn error_handling(&self) -> Option<ErrorHandling>;
    pub fn mutable(&self) -> bool;
    pub fn confidence(&self) -> u8;
}
```

### Budget Configuration

```rust
pub struct Budget {
    pub max_tokens: usize,      // Default: 1000
    pub min_body_stmts: usize,  // Default: 5
}
```

## Breaking Change Policy

### Versioning Scheme

This crate follows semantic versioning (semver):

- **Major version (X.0.0):** Breaking changes to the stable API surface
- **Minor version (0.X.0):** New features, backward-compatible additions
- **Patch version (0.0.X):** Bug fixes, documentation updates

### What Constitutes a Breaking Change

The following changes require a major version bump:

1. Removing or renaming public types, functions, or fields
2. Changing the signature of a public function (parameter types, return types)
3. Changing the encoding scheme of `SemanticTag` bits
4. Changing the semantics of analyzer outputs (e.g., what constitutes "Propagated" vs "Recovered")
5. Changing the tokenization output for identical input

### What Does NOT Constitute a Breaking Change

The following changes can occur in minor or patch releases:

1. Adding new public types, functions, or fields
2. Adding new `TokenKind` variants (existing tokens unchanged)
3. Performance improvements that don't change output
4. Documentation updates
5. Internal refactoring that doesn't affect the public API

### Deprecation Process

Before removing a deprecated API in a major version:

1. Mark the API as deprecated using Rust's `#[deprecated]` attribute
2. Document the replacement API
3. Release at least one minor version with the deprecation warning
4. Remove the API in the next major version

## Migration Guide

### v0.1.x → v0.2.x (Hypothetical)

If a breaking change is needed, consumers should:

1. Review the changelog for migration notes
2. Update their code to use the new API
3. Run the test suite to verify behavior hasn't changed
4. Check that semantic tag encoding/decoding still works correctly

### Crabjar Integration Notes

Crabjar's JEV pipeline depends on consistent behavior of:

- `analyze_errors()` — error handling classification must remain stable
- `analyze_mutability()` — mutability patterns must remain stable
- `analyze_ownership()` (v0.1.4+) — ownership modes must remain stable
- `analyze_complexity()` (v0.1.4+) — complexity metrics must remain stable

Any changes to these analyzers' behavior should be coordinated with the crabjar team before release.

## Contact

For questions about API stability or breaking change notifications, contact the pesti team via the pesti repo issues. Crabjar integration concerns should be directed to the crabjar team.