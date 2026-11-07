# EXPL-009: Mastery Feedback Loop in Agent Development

## Concept

Progress increases mastery. Mastery exposes limitations. Limitations motivate capability expansion. Capability expansion creates new opportunities for mastery.

This is a feedback loop that drives development. The key insight: don't just teach an agent to get better at doing its job. Teach it to recognize when its current means of doing that job have become the limiting factor.

That's the difference between optimizing a worker and developing a craftsperson.

## Crabjar Context

Our current skill ecosystem supports learning (skills are created, updated, versioned), but we don't explicitly model the mastery feedback loop. Agents can improve at tasks, but they don't recognize when their improvement has hit a ceiling due to tool limitations or architectural constraints.

For example:
- A code review agent might get better at detecting bugs, but eventually hit the limit of what static analysis can find
- A task routing agent might optimize for speed, but eventually discover that faster routing leads to more errors
- An evaluation agent might improve confidence calibration, but eventually realize it needs better tools to evaluate novel code

## Proposed Architecture

Introduce a **Capability Ceiling Detection** mechanism:

```rust
pub struct MasteryMetrics {
    pub task_type: String,
    pub success_rate: f32,
    pub improvement_rate: f32,  // How fast is success improving?
    pub ceiling_detected: bool, // Has improvement plateaued?
    pub limiting_factor: Option<String>,  // What's causing the ceiling?
}

pub enum LimitingFactor {
    ToolCapability,     // Current tools can't do better
    KnowledgeGap,       // Agent doesn't know enough
    ProcessInefficiency, // Workflow is suboptimal
    ExternalConstraint, // Something outside agent control
}
```

How it works:
1. Track success rates over time for each task type
2. Detect when improvement rate approaches zero (ceiling)
3. Analyze what's causing the ceiling
4. Trigger capability expansion (new tools, new skills, process changes)
5. Resume mastery loop with expanded capabilities

## Benefits

1. **Self-aware agents**: Agents know when they've hit their limits
2. **Targeted improvement**: Focus on actual bottlenecks, not arbitrary optimization
3. **Continuous growth**: The system evolves to overcome its own limitations
4. **Honest capability reporting**: Can report "we're as good as we can be with current tools"

## Open Questions

- How do we measure "success" for different task types?
- What triggers capability expansion — automatic or human-approved?
- How do we avoid infinite loops of ceiling detection → expansion → new ceiling?

## Related

- EXPL-008: Epistemic Distance (knowledge gaps are a limiting factor)
- Current implementation: `crates/crabjar/crabjar-conductor/src/` (worker management), skill ecosystem