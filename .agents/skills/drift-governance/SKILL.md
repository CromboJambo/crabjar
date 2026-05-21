---
name: drift-governance
description: |
  Use whenever the user mentions structural drift, stale documentation, checksum divergence,
  pipeline collapse, duplicate layers, bounded reversibility, worst-case perturbations,
  coasting vs resisting, or says "drift-governance", "check structural integrity",
  "audit pipeline layers", "compute perturbations", or "verify drift status". Trigger
  when project_map.md approaches >7 day stale threshold, when the user asks to detect
  filesystem divergence, when duplicate modules are found across crates, or when
  reversibility scoring needs bounded perturbation sets instead of single-point scores.
---

# Drift Governance

Governance layer for structural integrity across the workspace. Detects drift in
documentation, filesystem, pipeline layers, and action risk scoring. Combines
four mechanisms: structure audit, checksum-based drift detection, pipeline collapse
prevention, and bounded reversibility scoring.

---

## 1. Structure Audit (project_map.md staleness)

### When to trigger

`project_map.md` generated date approaching >7 day stale threshold. Any structural
change that could invalidate the documented architecture.

### Workflow

1. **Read project_map.md** — extract the documented structure and last audit date
2. **Scan filesystem** — perform recursive scan of workspace root and all crate paths
3. **Compare** — identify:
   - **Path mismatch**: documented path differs from actual (e.g. flat vs nested)
   - **Missing**: documented path not found on disk
   - **Unexpected**: disk path not documented in map
   - **Naming mismatch**: documented name vs actual name differs
4. **Update project_map.md** — write drift report section with provenance entries
5. **Run `cargo fmt --check` and `cargo clippy -- -D warnings`** — verify no regressions

### Output format

Drift report table with | Type | project_map.md | Actual | columns. Provenance entries
with UUID, Item, Set At, Reason, Source columns.

---

## 2. Querier Drift Detection (coasting vs resisting)

### When to trigger

Query state-docs and need to verify whether the returned content matches indexed checksums.
User asks "is this state-doc stable", "coasting or resisting", or "verify drift status".

### Workflow

1. **Call `drift_status()` on StateDocQuerier** — compare current file checksum vs indexed checksum
2. **Interpret result**:
   - `drift: false` = coasting (checksum matches, state-doc stable)
   - `drift: true` = resisting (checksum diverges, state-doc changed since indexing)
3. **If resisting**: flag for re-indexing or manual review
4. **If coasting**: proceed with query results as trusted

### Implementation reference

`memory/src/state_docs/querier.rs:130` — `drift_status()` method. Uses same
`compute_checksum` algorithm as `indexer.rs:149`.

---

## 3. Pipeline Collapse Prevention (concierge consolidation)

### When to trigger

Duplicate modules found across crates with identical functionality. User mentions
"pipeline collapse", "duplicate layers", "consolidate pipeline", or "remove duplicate".

### Workflow

1. **Identify duplicates** — search for identical modules across crates
2. **Determine dominant axis** — which layer is the primary enforcement function
3. **Remove secondary duplicate** — delete the duplicate file
4. **Update callers** — replace module references with the dominant layer's import
5. **Verify** — run `cargo clippy` on affected crates

### Pattern from this session

`orchestrator/src/concierge.rs` duplicate of `guard/src/concierge.rs`. Guard's
GateConcierge is the dominant axis (gate enforcement). Orchestrator handles SSE
delivery only. Removed duplicate, updated callers to use `crabjar_guard::GateConcierge`.

---

## 4. Bounded Reversibility (perturbation set)

### When to trigger

User mentions "bounded reversibility", "worst-case perturbations", "set of reachable
states", "perturbation scoring", or "compute perturbations for action". Reversibility
scoring needs bounded perturbation sets instead of single-point scores.

### Workflow

1. **Identify undo paths** — explicit rollback commands for the action
2. **Identify checksum targets** — files that need integrity verification
3. **Identify checkpoint targets** — session checkpoints for state preservation
4. **Identify flight recorder targets** — logging targets for traceability
5. **Identify data integrity targets** — data that needs integrity verification
6. **Compute PerturbationSet** — bounded set of all reachable perturbations
7. **Interpret**:
   - `bound: 1.0` = fully mitigable (all perturbations have undo paths)
   - `bound: 0.0` = fully unmitigable (no undo paths)
   - `bound: 0.5` = partially mitigable
   - `has_unmitigable: true` = requires permission
8. **Gate check** — use `gate_check_with_reversibility` with perturbation set

### Implementation reference

`guard/src/reversibility.rs` — `PerturbationSet`, `Perturbation`, `PerturbationKind`.
Replaces `ReversibilityScore` (single-point worst-case) with bounded set.

---

## Bundled Scripts

### `scripts/fs_audit.sh`

Filesystem vs project_map discrepancy analysis. Run before reading project_map.md.

```bash
bash /home/crombo/crabjar/.agents/skills/drift-governance/scripts/fs_audit.sh
```

### `scripts/perturbation_compute.sh`

Compute bounded perturbation set for a given action.

```bash
bash /home/crombo/crabjar/.agents/skills/drift-governance/scripts/perturbation_compute.sh
```

---

## Reference Files

### `references/perturbation-schema.md`

PerturbationSet schema, PerturbationKind enum, bound calculation formula.

Read when implementing or auditing reversibility scoring.

### `references/pipeline-collapse-patterns.md`

Patterns for detecting and preventing pipeline layer collapse. Duplicate module
identification, dominant axis determination, consolidation workflow.

Read when auditing pipeline layers across crates.

---

## Verification Gates

After any drift governance action:

```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
```

---

*End of skill.*