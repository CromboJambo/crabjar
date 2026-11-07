# EXPL-007: Transparency and Authority in Agent Decisions

## Concept

Two fundamental questions for any decision-making system:
1. **Transparency**: Can the participant inspect how decisions are reached?
2. **Authority**: Who can approve, reject, or change something?

These should be explicitly modeled rather than assumed. An agent might know the purpose of a task but not the full intention of the person who requested it. It should represent that distinction rather than inventing the missing intention.

## Crabjar Context

Our current architecture partially addresses both:

**Transparency:**
- The doubt block in `DecisionResult` exposes assumptions and blind spots
- Guard approval flows require human inspection for destructive actions
- Criteria files are JSON and inspectable by humans

**Authority:**
- `.crabjar_config.toml` defines execution permissions
- Guard layers control what actions can be taken without approval
- Workers have bounded autonomy based on their trust level

What's missing is explicit modeling of the transparency/authority relationship. Currently, authority is binary (approved/not approved) rather than graded based on transparency level.

## Proposed Architecture

Introduce a **Transparency-Authority Matrix** that maps decision types to required transparency levels and corresponding authority thresholds:

```toml
# In .crabjar_config.toml or guard policy files
[decision_policies]
"code_review" = { transparency: "full", authority: "worker" }
"destructive_file_ops" = { transparency: "full", authority: "human" }
"routine_task_routing" = { transparency: "summary", authority: "conductor" }
```

Transparency levels:
- `none`: Decision made internally, no explanation provided
- `summary`: High-level rationale provided
- `detailed`: Full evidence trail and reasoning exposed
- `full`: Everything including internal confidence scores and alternative options considered

Authority levels:
- `worker`: The executing agent can decide
- `conductor`: The orchestrator must approve
- `human`: A human must approve

## Benefits

1. **Configurable trust**: Users can tune how much transparency they require for different decision types
2. **Explicit trade-offs**: Makes clear that more transparency often means slower decisions
3. **Audit trail**: Every decision records its transparency level and who had authority
4. **Progressive autonomy**: As trust is built, authority can be delegated with less transparency required

## Open Questions

- Should transparency requirements be per-decision-type or per-context?
- How do we measure whether a decision was made transparently enough?
- Can transparency be retroactive (provide more detail after the fact)?

## Related

- EXPL-006: Evidence vs Authorship Separation (evidence enables transparency)
- Current implementation: `crates/guard/src/` (authority), `DecisionResult.doubt` (transparency)