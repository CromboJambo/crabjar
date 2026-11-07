# EXPL-006: Evidence vs Authorship Separation

## Concept

Preserve the evidence. Preserve the authorship. Never confuse the two.

In agent systems, decisions are made based on evidence (data, observations, criteria evaluations) by an author (the evaluator). These should be tracked separately to enable auditing, understanding, and trust calibration.

## Crabjar Context

Currently, our JEV decision system produces `DecisionResult` objects that contain:
- The decision outcome (option_id, confidence)
- The matched conditions
- A doubt block with assumptions and blind spots

What's missing is explicit separation of:
1. **Evidence**: What data was available at decision time? What criteria were evaluated? What observations were made?
2. **Authorship**: Who/what made the decision? Was it the conductor's fast-path rule engine, or the embedding-based JEV fallback?

## Proposed Architecture

Extend `DecisionResult` with explicit provenance fields:

```rust
pub struct DecisionResult {
    // Existing fields...
    
    // New: Evidence trail
    pub evidence: Vec<EvidenceItem>,
    
    // New: Authorship record
    pub authored_by: DecisionAuthor,
}

pub enum DecisionAuthor {
    RuleEngine { criteria_file: String, matched_option: String },
    EmbeddingFallback { model: String, threshold: f32 },
    HumanOverride { user_id: String, timestamp: i64 },
}

pub struct EvidenceItem {
    pub source: String,      // "criteria_file", "semantic_tag", "observation"
    pub content: String,     // The actual evidence
    pub relevance: f32,      // How relevant this was to the decision (0-1)
}
```

## Benefits

1. **Auditability**: Can trace exactly what led to a decision
2. **Trust calibration**: Users can see if decisions were rule-based (high confidence) or embedding-based (probabilistic)
3. **Debugging**: When a decision is wrong, can inspect the evidence trail to find the root cause
4. **Learning**: The system can learn which types of evidence lead to better decisions

## Open Questions

- How much detail should be captured in the evidence trail? Full source text vs summaries?
- Should evidence items be versioned along with the decision?
- How does this interact with the existing doubt block? Are they complementary or overlapping?

## Related

- EXPL-007: Transparency and Authority (complementary concept)
- Current implementation: `crates/crabjar/crabjar-conductor/src/decision/evaluate.rs`