# Pipeline Collapse Patterns

## Detection: duplicate modules across crates

### Pattern

Identical `GateConcierge` struct with same `enforce()` method found in:
- `guard/src/concierge.rs`
- `orchestrator/src/concierge.rs`

### Symptoms

- Both crates declare `mod concierge`
- Same struct name, same method signatures
- One crate imports from the other (cross-crate dependency)
- Both try to gate enforcement (conflicting authority)

## Dominant Axis Determination

### Rule

The layer with **gate enforcement** authority is the dominant axis.
The layer with **delivery** authority (SSE, HTTP, stdio) is secondary.

### Application

Guard's GateConcierge = gate enforcement (dominant)
Orchestrator's GateConcierge = SSE delivery (secondary) → should not gate

## Consolidation Workflow

### Step 1: Identify

Search for `mod <name>` across all crates. Compare struct names and method signatures.

### Step 2: Determine Dominant

Which layer has enforcement authority? That is the dominant axis.

### Step 3: Remove Duplicate

Delete the duplicate file. Update `mod <name>` declaration in the duplicate crate.

### Step 4: Update Callers

Replace module references with dominant layer's import:
```
use crabjar_guard::GateConcierge;
```

Not:
```
mod concierge;
let mut concierge = concierge::GateConcierge::new();
```

### Step 5: Verify

Run `cargo clippy` on affected crates. Verify no unresolved module errors.

## Prevention: pipeline layer separation

### Rule

Each pipeline layer has one authority:
- Guard: gate enforcement
- Orchestrator: SSE delivery
- Telemetry: flight recording
- Concierge: pending queue persistence (in guard only)

No layer should duplicate another layer's authority.

### Warning

If two layers declare the same struct with the same enforcement method:
- This is pipeline collapse
- One must be removed
- Callers must be updated to the dominant layer
- Observability is lost when both try to gate

---

*End of patterns.*