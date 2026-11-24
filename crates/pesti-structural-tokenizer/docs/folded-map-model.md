# Folded Map Documentation Model

## Core Insight

Documentation drifts because it's written by humans who read code last month. What if docs were **computed** from the actual code structure, like a topographic map rendered from elevation data?

```
Code (terrain) → Structural Tokens (elevation data) → Docs (topographic map)
       ↑                                                    ↓
   Always accurate                                     Auto-generated
```

## Two-Layer Model

### Layer 1: Code Structure (Terrain)
- Captured by structural tokenizer: `FN_DECL`, `IF_EXPR`, `FOR_LOOP`, etc.
- Universal across languages — same shape, different syntax
- The "elevation data" — pure form, no meaning yet

### Layer 2: Semantic Overlay (Landmarks)
- Comments, docstrings, annotations overlaid on structure
- Domain-specific meaning attached to structural elements
- Like landmark names on a map: "this mountain is Mt. Fuji"

```rust
// Landmark annotation
/// Route matching and dispatch logic
fn match_route(path: &str) -> Option<Route> { ... }
// Structural token: FN_DECL
```

## Folded Map Metaphor

Maps fold into smaller versions at different zoom levels:

| Zoom Level | Projection | Source |
|------------|-----------|--------|
| 100% (Street) | Full source code | Original files |
| 50% (City) | Structural tokens + landmarks | Tokenizer output |
| 25% (Region) | Roadmap / project map | Aggregated token streams |
| 10% (Country) | README / overview | Computed summary |

Each level is **folded** from the one below. Changes propagate upward automatically.

## "Stay Honest" Principle

Manual docs decay:
```
Code changes → Developer forgets to update README → Docs lie
```

Folded map docs are derived:
```
Code changes → Tokenizer re-runs → Docs regenerate → Always accurate
```

## Implementation Sketch

```rust
// 1. Structural pass: tokenize all source files
let tokens = structural_tokenize(codebase);

// 2. Semantic overlay: extract comments/docstrings
let landmarks = extract_annotations(source_files);

// 3. Fold into documentation at each zoom level
let readme = fold_readme(&tokens, &landmarks);
let roadmap = fold_roadmap(&tokens);
let api_docs = fold_api_docs(&tokens, &landmarks);
```

## Benefits

1. **Accuracy**: Docs always reflect actual code state
2. **Consistency**: Same structural patterns produce same doc structure
3. **Multi-scale**: Zoom from overview to implementation detail
4. **Cross-language**: Structural tokens are language-agnostic
5. **Automated**: No manual sync between code and docs

## Open Questions

- How to detect "interesting" structures worth documenting?
- What's the right granularity for each zoom level?
- How to handle private vs public API surface?
- Can we reverse-engineer this: from folded map back to code?
