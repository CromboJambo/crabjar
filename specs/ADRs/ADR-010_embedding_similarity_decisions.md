# ADR-010: Embedding Similarity for Semantic Decisions

**Status:** Accepted  
**Date:** 2026-10-19  
**Author:** crabjar team (CromboJambo)  
**Context:** JEV decision layer architecture

## Problem

Static rule-based decision making (regex, pattern matching, threshold comparisons) fails on operational triage because:
- Commands and observations vary in wording but share intent ("service down" vs "not responding" vs "unreachable")
- New failure modes emerge that aren't covered by existing rules
- Rule maintenance cost grows linearly with fleet complexity

## Decision

Use embedding similarity against curated decision criteria files for semantic decision-making. An observation is embedded, compared via cosine similarity to option embeddings from a criteria file, and the highest-similarity option above a confidence threshold is selected.

## Rationale

1. **Semantic understanding**: "homeassistant not responding on port 8123" matches "service is down or unresponsive" even though neither contains the other's words
2. **Low maintenance**: Adding new decision contexts requires writing criteria files, not code changes
3. **Confidence calibration**: Similarity scores provide natural confidence metrics (0.0-1.0)
4. **Offline operation**: Hash-based embedding backend works without network; llama-server backend provides higher accuracy when available
5. **Proven in production**: 18 JEV milestone implementations validated this approach across domains (service triage, deployment gates, cost optimization, security hardening, etc.)

## Implementation Pattern

```hemlock
// Criteria file: JSON with options and descriptions
{
  "options": [
    { "id": "restart_service", "description": "Service is down or unresponsive" },
    { "id": "disk_full", "description": "Disk space exhaustion" }
  ],
  "threshold": 0.75
}

// Evaluation: embed observation, compare to option embeddings
let obs_embedding = backend.embed(observation);
let best_option = argmax(cosine_similarity(obs_embedding, option_embeddings));
if (best_score > threshold) { return decisive(best_option); }
else { return uncertain("route to human"); }
```

## Threshold Convention

- Default: 0.75 (good balance of precision/recall for operational decisions)
- Range: 0.50-0.95 (below 0.5 is noise, above 0.95 requires near-identical wording)
- Domain-specific adjustments documented in criteria file metadata

## Doubt Block Requirement

All JEV decisions include a `doubt` block with assumptions, blind spots, last validation timestamp, and staleness threshold. This acknowledges the heuristic nature of embedding similarity and provides auditability.

## Alternatives Considered

1. **Static rules (current guard approach)**: Too brittle for semantic variation
2. **Local LLM inference**: Higher accuracy but 5-10x latency; overkill for routine triage
3. **JEV API (TypeSafe AI)**: Good accuracy but external dependency, per-call cost, privacy concerns

## Consequences

- Decision quality depends on criteria file wording quality
- Embedding backend choice affects accuracy vs. latency trade-off
- Threshold calibration requires historical outcome data for optimization
- New decision contexts require curated criteria files (not automatic)