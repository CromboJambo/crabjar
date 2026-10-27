# Knowledge Compression Triad

**Status:** Draft  
**Created:** 2026-10-27  
**Related:** Evidence-based promotion spec, JEV decision layer, skill system

## Problem Statement

Crabjar stores knowledge in multiple forms (skills, JEV rules, execution traces) but lacks a unified model for how knowledge is compressed, validated, and traced back to its source. This creates three problems:

1. **No quality signal:** Compressed knowledge (rules, procedures) has no attached evidence of when it's correct vs. when it fails.
2. **Opaque abstractions:** Users can't inspect the underlying reasoning that produced a compressed rule or skill.
3. **Brittle generalization:** When abstractions fail in novel situations, there's no systematic way to return to raw experience and re-derive better rules.

The system needs a formal triad for all knowledge: compressed form + supporting evidence + detailed source record.

### Relationship to Existing JEV Rule Promotion

Crabjar already has a partial implementation of this pattern in the JEV decision layer (`src/commands/semantic.rs`, ADR-011). Decisions that consistently yield the same outcome can be "promoted" to deterministic rules, eliminating embedding calls. This spec generalizes that pattern:

- **Existing:** Rule promotion for JEV decisions only, with implicit evidence (outcome consistency)
- **This spec:** Formal triad for ALL knowledge types (rules, procedures, models, skills), with explicit evidence tracking and source traceability

The existing JEV rule promotion becomes a special case of the broader knowledge compression framework. No changes needed to JEV's current implementation — it already follows the pattern this spec formalizes.

## The Triad Model

Every piece of knowledge in crabjar should have three components:

### 1. Compressed Knowledge
Reusable, generalizable form:
- Rules (JEV decision rules)
- Procedures (skills with step-by-step instructions)
- Models (predictive patterns learned from experience)
- Known limitations (explicit boundaries where the compression fails)

### 2. Supporting Evidence
Validation data attached to the compressed form:
- Examples where it worked correctly
- Counterexamples where it failed or was incorrect
- Performance metrics (accuracy, precision, recall for decision rules)
- Provenance (who/what generated it, when, from what source)

### 3. Detailed Specialist Record
The raw experience that produced the compression:
- Original execution traces
- Contextual information not captured in compression
- Alternative approaches considered and rejected
- Reasoning process for why this compression was chosen

## API Design

### POST /knowledge/compress
Compress raw observations into reusable knowledge.

**Request:**
```json
{
  "observations": [
    {
      "observation_id": "obs_123",
      "context": "string",
      "outcome": "success|failure",
      "details": {}
    }
  ],
  "compression_type": "rule|procedure|model",
  "evidence_threshold": 0.85
}
```

**Response:**
```json
{
  "knowledge_id": "kn_abc123",
  "compressed_form": {
    "type": "rule",
    "content": "If X and Y, then Z",
    "confidence": 0.92
  },
  "evidence_summary": {
    "supporting_examples": 45,
    "counterexamples": 3,
    "accuracy": 0.9375
  },
  "source_observations": ["obs_123", "obs_124", ...]
}
```

### GET /knowledge/{id}/evidence
Retrieve supporting evidence for a piece of knowledge.

**Response:**
```json
{
  "knowledge_id": "kn_abc123",
  "supporting_examples": [
    {
      "example_id": "ex_789",
      "context": "string",
      "outcome": "success",
      "relevance_score": 0.95
    }
  ],
  "counterexamples": [
    {
      "example_id": "ex_790",
      "context": "string",
      "outcome": "failure",
      "reason": "string explaining why it failed"
    }
  ]
}
```

### GET /knowledge/{id}/source
Retrieve the detailed specialist record.

**Response:**
```json
{
  "knowledge_id": "kn_abc123",
  "source_observations": [
    {
      "observation_id": "obs_123",
      "full_context": {},
      "raw_output": "string",
      "alternative_approaches": ["approach A", "approach B"],
      "reasoning": "Why this approach was chosen"
    }
  ]
}
```

## Integration with Existing Systems

### JEV Decision Layer
JEV already has partial triad support:
- Compressed form: decision rules
- Supporting evidence: outcome tracking
- Source record: observation history

This spec formalizes and extends the pattern to all knowledge types.

### Skill System
Skills currently lack the evidence and source components. This spec adds:
- Evidence of when skills work vs. fail
- Source observations that led to skill creation
- Known limitations explicitly documented

### Execution Traces
Raw execution traces become the "detailed specialist record" for any knowledge derived from them. The compression step transforms traces into reusable rules/procedures with attached evidence.

## Phase Breakdown

### Phase 1: Knowledge Schema Extension
**Goal:** Add triad fields to existing knowledge storage.

- Extend JEV rule schema with evidence and source fields
- Create SQLite schema for `knowledge_compression` table
- Migrate existing rules to new schema (backward compatible)

**Acceptance Criteria:**
- All knowledge types have fields for compressed form, evidence, and source
- Existing queries continue to work without modification

### Phase 2: Compression Engine
**Goal:** Build the compression step that transforms raw observations into triad-form knowledge.

- Implement observation clustering and pattern detection
- Generate compressed forms with confidence scores
- Attach supporting evidence during compression
- Link back to source observations

**Acceptance Criteria:**
- Raw observations can be submitted for compression
- Compressed output includes all three triad components
- Confidence scores are calibrated (0.95 confidence = 95% accuracy on test set)

### Phase 3: Evidence Management
**Goal:** Maintain and update evidence as new observations arrive.

- Periodic evidence refresh for existing knowledge
- Counterexample detection and incorporation
- Confidence score updates based on new evidence
- Automatic flagging of stale or inaccurate knowledge

**Acceptance Criteria:**
- Knowledge confidence scores update as new evidence arrives
- Stale knowledge is flagged for review or regeneration
- Users can query knowledge by confidence level

## Success Metrics

- [ ] Phase 1: Schema migration complete, all knowledge types have triad fields
- [ ] Phase 2: Compression engine producing valid triad-form knowledge
- [ ] Phase 3: Evidence management updating knowledge quality over time
- [ ] End-to-end: Raw observations → compressed knowledge with evidence and source traceability

## Open Questions

1. How should compression handle conflicting observations (some support, some contradict)?
2. What's the appropriate granularity for counterexamples — individual cases or aggregated patterns?
3. Should source records be retained indefinitely or pruned after a certain time?
4. How does this integrate with the evidence-based promotion system (shared infrastructure)?

## Reversibility Notes

- Schema changes are additive (new columns, not modifications)
- Compression engine is a separate module that can be disabled
- Evidence management runs as periodic background task, not critical path
- Existing knowledge continues to work without triad components (graceful degradation)
