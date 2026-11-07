# EXPL-008: Epistemic Distance in Agent Evaluation

## Concept

When an agent encounters something unfamiliar, there are three possible conditions:

1. **Near**: Known relationship. Existing knowledge adequately explains the observation.
2. **Intermediate**: Partial relationship. Existing knowledge explains some aspects, but important discrepancies remain.
3. **Far**: Unmapped territory. Existing knowledge provides little to no explanation.

This concept of "epistemic distance" — how far an observation is from what the agent knows — should be explicitly tracked and used to calibrate decision confidence.

## Crabjar Context

Currently, our JEV evaluation system treats all code regions equally regardless of how familiar they are to the evaluator. A function using standard Rust patterns gets the same confidence treatment as a function using obscure FFI calls or novel algorithmic approaches.

This leads to overconfident decisions in "far" territory where the evaluator actually knows very little about what it's evaluating.

## Proposed Architecture

Extend `EvaluationTarget` and `DecisionResult` with epistemic distance tracking:

```rust
pub enum EpistemicDistance {
    Near,      // Well-understood pattern
    Intermediate,  // Partially understood
    Far,       // Novel or unfamiliar
}

pub struct DecisionResult {
    // Existing fields...
    
    pub epistemic_distance: EpistemicDistance,
    pub distance_rationale: String,  // Why this distance was assigned
}
```

How to determine epistemic distance:
1. **Pattern matching**: Does the code match known patterns in our criteria files? (Near)
2. **Semantic tag coverage**: Are all relevant semantic tags present and understood? (Near if yes, Far if missing)
3. **Language familiarity**: Is this in a language we have good structural analysis for? (Rust = Near, obscure DSL = Far)
4. **Historical evaluation data**: Have we evaluated similar code before with good outcomes?

## Benefits

1. **Calibrated confidence**: Decisions in "far" territory automatically get lower confidence scores
2. **Better routing**: Code in "far" territory can be routed to more capable evaluators or humans
3. **Learning signal**: Far evaluations that succeed become training data for future near evaluations
4. **Honest uncertainty**: The system explicitly represents when it doesn't know much

## Open Questions

- Should epistemic distance affect the decision threshold? (Higher threshold for Far?)
- Can we learn to reduce epistemic distance over time through evaluation history?
- How do we handle mixed cases where some aspects are Near and others are Far?

## Related

- EXPL-009: Mastery Feedback Loop (reducing epistemic distance through learning)
- Current implementation: `crates/universal-structure/src/evaluation.rs` (EvaluationTarget)