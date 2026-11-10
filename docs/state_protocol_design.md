# State Protocol Design: Context-Preserving Interprocess Handoff

**Status:** Implemented (Phases 1-3 + rehydration validation) — 2026-11-09
**Date:** 2026-11-09
**Scope:** GuardDb persistence layer, action lifecycle, interprocess handoff semantics

## Central Principle

> The sender is responsible for transmitting sufficient context.
> The receiver is responsible for validating that context.
> Neither is responsible for pretending to know what the other knows.

This is not about adding more states. It's about making the information that flows
between processes explicit, typed, and verifiable.

---

## 1. Verified Findings (from code inspection)

### 1.1 GuardDb Access Architecture

**Verified:** `GuardDb` wraps `Arc<Mutex<Connection>>` (`crates/guard/src/guard_db.rs:50`).
Any component with a file path can open it via `GuardDb::open(path)` or
`GuardDb::co_located(memory_path)`. There is no supervisor, no access control,
no ownership model.

**Verified call sites (partial — search hit 30-match limit):**
- `crates/guard/src/concierge.rs:127` — persists action requests
- `src/commands/exec.rs:155` — persists action requests before gate check
- `src/commands/guard.rs:17` — reads pending actions for CLI listing
- `src/commands/guard.rs:59,63` — approves/rejects via `update_action_status`

**Hypothesis:** The 30-match limit means there are additional call sites in tests,
benchmarks, and possibly the ACP server that were not enumerated. A complete audit
requires raising the search limit or using a different pattern.

### 1.2 Rehydration Paths (Current Safety Guarantees)

**Verified:** `read_action_requests()` (`crates/guard/src/guard_db_queries.rs:24-68`)
parses status strings with a silent fallback:

```rust
let status = match row.get::<_, String>(7)? {
    s if s == "pending" => ActionStatus::Pending,
    s if s == "trust-approved" => ActionStatus::TrustApproved,
    s if s == "denied" => ActionStatus::Denied,
    s if s == "executed" => ActionStatus::Executed,
    s if s == "interrupted" => ActionStatus::Interrupted,
    _ => ActionStatus::Pending,  // ← SILENT CORRUPTION FALLBACK (line 61)
};
```

An unknown or corrupted status string silently becomes `Pending`, re-entering the
pipeline as if it were a legitimate new request. This is the single most dangerous
rehydration path in the system.

**Verified:** There is no schema version check at open time. If the schema changes,
existing databases are used as-is with no migration or validation.

### 1.3 State Transitions and Enforcement Points

**Verified compile-time guarantees:** `GuardedAction<State>` (`crates/guard/src/action.rs:253-340`)
enforces a strict state machine at compile time:

```
Pending ──approve()──▶ TrustApproved ──execute()──▶ Executed     (terminal)
    │                        │
    └─────deny()─────────────┴──interrupt()──▶ Interrupted        (terminal)
              │
              ▼
           Denied                                                (terminal)
```

Invalid transitions are impossible in code that uses `GuardedAction<State>`.

**Verified runtime bypass:** The plain `ActionRequest` struct has `mark_approved()`,
`mark_denied()`, `mark_executed()`, `mark_interrupted()` methods (action.rs:120-143)
that can be called on ANY instance regardless of current state. These are used by
the persistence layer, which means the compile-time guarantees are completely
bypassed at the database boundary.

**Verified:** The concierge produces typestate-correct `ConciergeOutcome` values
(`crates/guard/src/concierge_types.rs:111-116`), but these are immediately unwrapped
to plain `ActionRequest` structs for persistence, losing all typestate information.

### 1.4 Where Sender Context Is Currently Lost

This is the core finding. At every handoff point, significant context is discarded:

**Handoff 1: exec.rs → GuardDb (action creation)**
- `src/commands/exec.rs:155-165`
- Sender knows: command, args, cwd, reason text, trust_layer, confidence, workspace, user intent
- Receiver stores: id, source_event_id (= own ID, circular), action_type="exec", payload (command+args as string), trust_layer, confidence, status="pending"
- **Lost:** The `reason` parameter is passed to the gate check but NEVER stored in the action row. The workspace context is not stored. The distinction between "user explicitly asked for this" and "agent decided to do this" is not captured.

**Handoff 2: Gate → Concierge (decision reasoning)**
- `crates/guard/src/concierge.rs:56-190`
- Sender knows: why it decided (risk assessment, policy matches, trust calculations)
- Receiver stores: only "proceed" / "interrupted" / "pending" in the `gate_result` TEXT column
- **Lost:** The entire reasoning chain. If a gate denies an action, there is no persistent record of WHY beyond a free-form reason string in some cases.

**Handoff 3: Concierge → GuardDb (outcome persistence)**
- `crates/guard/src/concierge.rs:127-182`
- The concierge creates a NEW `ActionRequest` with a NEW ID for each outcome path.
- **Critical finding:** The original pending row created by exec.rs is NEVER updated.
  - Exec creates action A (status=pending)
  - Concierge creates action B (new ID, status=trust-approved/denied)
  - Action A remains "pending" forever — a state leak
  - The only link between A and B is `source_event_id`, which is set to the original request ID
- **Lost:** The transition itself. There is no record that "action A was resolved by outcome B."

**Handoff 4: GuardDb → CLI/other readers (rehydration)**
- `src/commands/guard.rs:17` calls `read_action_requests()`
- Receiver gets raw rows and parses status strings with silent fallback
- **Lost:** Any validation that the state is legitimate. A tampered row becomes a valid pending action.

**Handoff 5: Execution → Outcome (completeness)**
- `src/commands/exec.rs:420-438`
- After execution, an outcome is recorded in `action_outcomes` table
- **Critical finding:** The action request's status is NEVER updated to "Executed".
  It stays in whatever state the concierge left it (typically "trust-approved").
  So `action_requests.status` and `action_outcomes` can disagree about whether
  something was actually executed.

### 1.5 Compile-Time vs Runtime vs Persistence Guarantees

| Layer | Guarantee | Enforcement | Bypassed by |
|-------|-----------|-------------|-------------|
| Compile-time | Valid state transitions | `GuardedAction<State>` typestate | Plain `ActionRequest` methods, direct DB writes |
| Runtime (in-memory) | Typestate-correct outcomes | `ConciergeOutcome` enum | Unwrapping to plain structs for persistence |
| Persistence | None | No validation on read or write | Any component with a GuardDb handle |

The compile-time guarantees are real but confined to the in-memory domain. The
moment data crosses the persistence boundary, all guarantees evaporate.

---

## 2. The Handoff Contract (Proposed Abstraction)

The six gaps identified earlier are symptoms of one missing abstraction: every
handoff between processes (or between a process and its persistence layer) needs
a typed contract that specifies:

### 2.1 Sender Context (what the sender must provide)

```rust
/// Information the sender knows and must transmit for the receiver
/// to understand and responsibly continue the work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffContext {
    /// What triggered this action (mirror-log event ID, user request, agent decision)
    pub origin: ActionOrigin,

    /// The reasoning that led to this handoff
    pub reasoning: Option<ReasoningTrace>,

    /// Who/what is sending this and what authority they hold
    pub sender: SenderIdentity,

    /// What state the action was in before this handoff
    pub previous_state: ActionStatus,

    /// The intended transition (what the sender wants to happen)
    pub intended_transition: TransitionIntent,

    /// Dependencies that must be resolved before this can proceed
    pub dependencies: Vec<DependencyRef>,

    /// Idempotency key: allows safe retries without duplicate execution
    pub idempotency_key: String,

    /// Schema version of this handoff envelope
    pub schema_version: u32,
}
```

### 2.2 Receiver Contract (what the receiver must validate)

```rust
/// Validation protocol the receiver applies to every incoming handoff.
pub trait HandoffValidator {
    /// Is this a message type we accept?
    fn accepts(&self, envelope: &HandoffEnvelope) -> bool;

    /// Does the sender have authority for this transition?
    fn validate_authority(&self, sender: &SenderIdentity, intent: &TransitionIntent) -> Result<(), ValidationFailure>;

    /// Is this transition legal from the current persisted state?
    fn validate_state_legality(&self, action_id: &str, intent: &TransitionIntent) -> Result<ActionStatus, ValidationFailure>;

    /// Is the provenance chain intact?
    fn validate_provenance(&self, context: &HandoffContext) -> Result<(), ValidationFailure>;

    /// Are all declared dependencies satisfied?
    fn validate_dependencies(&self, deps: &[DependencyRef]) -> Result<(), ValidationFailure>;

    /// Do we support this schema version?
    fn validate_schema_version(&self, version: u32) -> Result<(), ValidationFailure>;
}
```

### 2.3 The Envelope (what actually crosses the boundary)

```rust
/// A typed handoff envelope: the unit of interprocess state transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffEnvelope {
    pub context: HandoffContext,
    pub payload: HandoffPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HandoffPayload {
    /// New action request
    ActionRequest(ActionRequest),

    /// State transition on an existing action
    Transition {
        action_id: String,
        to_status: ActionStatus,
        reason: String,
    },

    /// Execution outcome
    Outcome {
        action_id: String,
        success: bool,
        exit_code: Option<i32>,
        evidence: Vec<String>,
    },
}
```

---

## 3. Proposed Types (Concrete)

### 3.1 ActionOrigin (replaces circular source_event_id)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActionOrigin {
    /// Direct user request via CLI or TUI
    UserRequest {
        session_id: String,
        reason: String,  // the actual "why" — currently lost
    },
    /// Agent-initiated action based on observations
    AgentDecision {
        agent_id: String,
        observation_ids: Vec<String>,  // mirror-log event IDs
        rationale: String,
    },
    /// System-triggered (cron, health check, etc.)
    SystemTrigger {
        trigger_type: String,
        schedule_id: Option<String>,
    },
    /// Response to a previous action's outcome
    ReactionTo {
        parent_action_id: String,
        reaction_type: String,
    },
}
```

### 3.2 SenderIdentity (authority dimension)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SenderIdentity {
    /// Human user via CLI/TUI — highest authority
    User { principal: String },
    /// The guard system itself (gate decisions)
    Guard { component: String },
    /// The concierge (outcome routing)
    Concierge,
    /// An agent with a specific trust layer
    Agent { agent_id: String, trust_layer: u32 },
    /// The JEV arbiter (belief-revision decisions)
    Arbiter { confidence: f64 },
}

impl SenderIdentity {
    /// Can this sender perform the given transition?
    pub fn can_perform(&self, transition: &TransitionIntent) -> bool {
        match (self, transition) {
            // Users can approve or deny anything
            (SenderIdentity::User { .. }, _) => true,
            // Guard can only move to pending/interrupted
            (SenderIdentity::Guard { .. }, TransitionIntent::ToPending | TransitionIntent::ToInterrupted { .. }) => true,
            // Concierge can approve (trust layer passed) or deny
            (SenderIdentity::Concierge, TransitionIntent::ToApproved | TransitionIntent::ToDenied { .. }) => true,
            // Arbiter can approve/deny with sufficient confidence
            (SenderIdentity::Arbiter { confidence }, TransitionIntent::ToApproved) if *confidence >= 0.85 => true,
            (SenderIdentity::Arbiter { confidence }, TransitionIntent::ToDenied { .. }) if *confidence >= 0.85 => true,
            _ => false,
        }
    }
}
```

### 3.3 TransitionIntent (replaces raw status strings)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "to", rename_all = "snake_case")]
pub enum TransitionIntent {
    ToPending,
    ToApproved,
    ToDenied { reason: String },
    ToExecuted { evidence: Vec<String> },
    ToInterrupted { reason: String },
}

impl TransitionIntent {
    pub fn to_status(&self) -> ActionStatus {
        match self {
            Self::ToPending => ActionStatus::Pending,
            Self::ToApproved => ActionStatus::TrustApproved,
            Self::ToDenied { .. } => ActionStatus::Denied,
            Self::ToExecuted { .. } => ActionStatus::Executed,
            Self::ToInterrupted { .. } => ActionStatus::Interrupted,
        }
    }

    /// Which statuses can legally transition to this one?
    pub fn valid_from(&self) -> Vec<ActionStatus> {
        match self {
            Self::ToPending => vec![],  // initial state only
            Self::ToApproved => vec![ActionStatus::Pending],
            Self::ToDenied { .. } => vec![ActionStatus::Pending, ActionStatus::TrustApproved],
            Self::ToExecuted { .. } => vec![ActionStatus::TrustApproved],
            Self::ToInterrupted { .. } => vec![ActionStatus::Pending, ActionStatus::TrustApproved],
        }
    }
}
```

### 3.4 ReasoningTrace (preserves the "why")

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningTrace {
    pub steps: Vec<ReasoningStep>,
    pub conclusion: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub source: String,        // "gate", "arbiter:policy_check", "user", etc.
    pub observation: String,   // what was observed
    pub inference: String,     // what was concluded
    pub confidence_delta: f64, // how this step affected overall confidence
}
```

---

## 4. Schema Changes (Proposed, Not Yet Implemented)

### 4.1 New Tables

```sql
-- Handoff envelopes: the audit trail of all state transitions
CREATE TABLE IF NOT EXISTS handoffs (
    id TEXT PRIMARY KEY,
    action_id TEXT NOT NULL REFERENCES action_requests(id),
    sender_kind TEXT NOT NULL,           -- "user", "guard", "concierge", "agent", "arbiter"
    sender_detail TEXT,                  -- JSON: principal, agent_id, confidence, etc.
    from_status TEXT NOT NULL,
    to_status TEXT NOT NULL,
    intent TEXT NOT NULL,                -- JSON: TransitionIntent
    reasoning TEXT,                      -- JSON: ReasoningTrace (optional)
    idempotency_key TEXT UNIQUE NOT NULL, -- prevents duplicate execution on retry
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX IF NOT EXISTS idx_handoffs_action_id ON handoffs(action_id);
CREATE INDEX IF NOT EXISTS idx_handoffs_idempotency ON handoffs(idempotency_key);

-- Action origins: replaces the circular source_event_id pattern
CREATE TABLE IF NOT EXISTS action_origins (
    action_id TEXT PRIMARY KEY REFERENCES action_requests(id),
    origin_kind TEXT NOT NULL,           -- "user_request", "agent_decision", "system_trigger", "reaction_to"
    origin_detail TEXT NOT NULL,         -- JSON: ActionOrigin variant data
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Schema versioning (already added provisionally)
CREATE TABLE IF NOT EXISTS schema_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
INSERT OR IGNORE INTO schema_meta (key, value) VALUES ('version', '2');
```

### 4.2 Modified Tables

```sql
-- action_requests: add origin reference, remove circular source_event_id dependency
-- (source_event_id kept for backward compatibility but deprecated)
ALTER TABLE action_requests ADD COLUMN origin_id TEXT REFERENCES action_origins(action_id);

-- action_requests: status column semantics unchanged, but now validated on read
```

### 4.3 Transactional Consistency Requirement

Every state change MUST be atomic: the action row update and the handoff record
must occur in the same SQLite transaction. If either fails, both roll back.

```rust
pub fn apply_transition(
    &self,
    envelope: &HandoffEnvelope,
) -> Result<TransitionResult, HandoffError> {
    let conn = self.conn.lock().map_err(|e| HandoffError::LockPoisoned(e.to_string()))?;
    let tx = conn.transaction()?;  // ← single transaction

    // 1. Validate idempotency (has this exact handoff been applied before?)
    let existing: Option<String> = tx.query_row(
        "SELECT id FROM handoffs WHERE idempotency_key = ?1",
        params![envelope.context.idempotency_key],
        |r| r.get(0),
    ).optional()?;
    if existing.is_some() {
        return Ok(TransitionResult::AlreadyApplied);  // safe retry
    }

    // 2. Validate transition legality against current DB state
    let current_status: String = tx.query_row(
        "SELECT status FROM action_requests WHERE id = ?1",
        params![action_id],
        |r| r.get(0),
    )?;
    let current = ActionStatus::parse(&current_status)
        .ok_or(HandoffError::UnknownStatus(current_status))?;  // ← NO SILENT FALLBACK
    if !envelope.context.intended_transition.valid_from().contains(&current) {
        return Err(HandoffError::IllegalTransition { from: current, to: ... });
    }

    // 3. Validate sender authority
    if !envelope.context.sender.can_perform(&envelope.context.intended_transition) {
        return Err(HandoffError::Unauthorized);
    }

    // 4. Apply: update action row + insert handoff record (atomic)
    tx.execute("UPDATE action_requests SET status = ?1, resolved_at = ?2 WHERE id = ?3", ...)?;
    tx.execute("INSERT INTO handoffs (...) VALUES (...)", ...)?;

    tx.commit()?;
    Ok(TransitionResult::Applied)
}
```

---

## 5. Supervisor Ownership Evaluation

**Question:** Is supervisor-owned database access necessary?

**Findings:**
- Currently, 16+ components can open GuardDb directly (verified partial list, search hit limit)
- The concierge, exec command, guard CLI, and likely tests all hold independent connections
- No component coordinates with others — concurrent writes are serialized by SQLite's mutex but not logically coordinated

**Assessment:**
Supervisor ownership is NOT automatically the right answer. The real problem is not
"who owns the DB" but "who validates transitions." A supervisor model makes sense
if:
1. There are competing writers that need arbitration (currently: mostly sequential)
2. Access control matters (currently: single-user system)
3. Audit requirements demand a single choke point (increasingly true)

**Recommendation:** Start with **validated transitions** (any component can write,
but every write must pass through `apply_transition()` which validates authority,
legality, and idempotency). Add supervisor ownership later only if the audit shows
it's needed. The handoff contract is the enforcement mechanism; the supervisor is
just one possible deployment of it.

---

## 6. Incremental Implementation Plan

### Phase 1: Stop the Bleeding (no behavior change, add validation)

**Goal:** Eliminate silent corruption without breaking existing flows.

1. Replace `_ => ActionStatus::Pending` with explicit error in `read_action_requests()`
   - Unknown status → return `Err(GuardDbError::UnknownStatus(value))`
   - Callers must handle the error (currently they don't expect one — this is a breaking change that forces awareness)
2. Add schema version check at `GuardDb::open()` time
   - Read `schema_meta.version`, compare to expected
   - Mismatch → fail with clear error, don't silently use wrong schema
3. **No new tables yet.** Just validation on existing data.

**Risk:** Low. Only affects corrupted/unknown data, which currently fails silently.

### Phase 2: Handoff Contract Types (additive, no migration)

**Status:** ✅ Complete (2026-11-09)

**Goal:** Define the types and start using them at new code boundaries.

1. Add `HandoffContext`, `HandoffEnvelope`, `TransitionIntent`, `SenderIdentity`,
   `ActionOrigin`, `ReasoningTrace` to new `handoff.rs` module ✅
2. Modify `concierge.enforce()` to construct a `HandoffEnvelope` for each outcome ✅
   - Still persist using existing methods (no schema change yet)
   - But the envelope captures what's currently lost (reasoning, origin, authority)
3. Store the envelope as JSON in a new optional column or side table
   - Additive: old readers ignore it, new readers can use it
   - Deferred to Phase 3

**Risk:** Low. Purely additive; existing persistence paths unchanged.

**Implementation notes:**
- `crates/guard/src/handoff.rs` — all types with full test coverage (7 tests)
- `GateConcierge::build_handoff_envelope()` constructs envelopes from gate decisions
- Envelopes are currently logged at debug level; persistence wiring in Phase 3
- Test count: 210 → 217 unit tests (all passing)

### Phase 3: Transition Log with Transactional Consistency

**Goal:** Every state change is recorded atomically with the state change itself.

1. Add `handoffs` table (schema migration, version 2 → 3)
2. Implement `apply_transition()` as described in §4.3
3. Migrate `update_action_status()` to use `apply_transition()` internally
   - Backward compatible: old callers pass a minimal envelope
   - New callers (concierge, arbiter) pass full context
4. Fix the state leak: when concierge creates outcome B for request A,
   update A's status in the same transaction as inserting B

**Risk:** Medium. Schema migration + behavior change (state leak fix).
Requires careful testing of all call sites.

### Phase 4: Authority Enforcement

**Goal:** Senders must prove they can perform the transition they're requesting.

1. Implement `SenderIdentity::can_perform()` checks in `apply_transition()`
2. Migrate all call sites to declare their sender identity explicitly
   - CLI approve/reject → `SenderIdentity::User { principal: "cli" }`
   - Concierge → `SenderIdentity::Concierge`
   - Arbiter → `SenderIdentity::Arbiter { confidence }`
3. Add tests for unauthorized transitions (should fail, not silently succeed)

**Risk:** Medium. Requires touching all transition call sites. Some may need
authority upgrades (e.g., if the arbiter currently does something it shouldn't).

### Phase 5: Idempotency and Retry Safety

**Goal:** Restarted workers can retry without duplicate execution.

1. Add `idempotency_key` generation to all handoff producers
   - Typically: hash of (action_id, from_status, to_status, sender, timestamp_bucket)
2. Enforce uniqueness in `apply_transition()` (already in §4.3 design)
3. Add "resume" path: a worker that crashes mid-handoff can detect it was
   already applied and skip re-execution

**Risk:** Low. Additive to Phase 3 infrastructure.

### Phase 6: Supervisor Ownership (conditional)

**Goal:** If audit shows it's needed, centralize write access.

1. Create a `GuardDbSupervisor` that owns the only writable connection
2. Other components communicate via message passing (channels, IPC, or API)
3. Supervisor applies all transitions through `apply_transition()`
4. Read access can remain direct (reads don't need coordination)

**Risk:** High. Architectural change affecting all components.
**Gate:** Only proceed if Phases 1-5 reveal actual coordination problems
that per-write validation doesn't solve.

---

## 7. What This Design Does NOT Do

- **Does not add new ActionStatus variants.** The five existing states are sufficient;
  the problem is missing context, not missing states.
- **Does not require a supervisor for every workload.** Validation is per-transition,
  not per-connection. Supervisor ownership is Phase 6, conditional on evidence.
- **Does not break existing behavior in Phases 1-2.** Migration happens incrementally.
- **Does not solve the JEV arbiter's confidence model.** That's a separate concern;
  this design just ensures the arbiter's decisions are recorded with provenance.

---

## 8. Open Questions

1. **Reasoning trace granularity:** DECIDED — self-contained first. Store full
   gate evaluation/arbiter reasoning inline in the handoff record. Measure actual
   size in practice, then identify what can be compressed or referenced against
   telemetry. Don't optimize for a size problem we haven't measured yet.

2. **Cross-process handoffs:** This design focuses on the persistence boundary
   (process → DB → process). When crabjar agents communicate directly (A2A,
   conductor API), do they use the same envelope format? Probably yes, but that's
   a separate spec.

3. **Schema migration strategy:** SQLite doesn't support `ALTER TABLE ... DROP COLUMN`.
   Deprecating `source_event_id` means keeping it forever or doing a table rebuild.
   What's the migration policy for breaking schema changes?

4. **Backward compatibility window:** How long do we support reading handoffs from
   older schema versions? The `schema_version` field in the envelope helps, but
   we need a policy (e.g., "support N-2 versions").

---

## 9. Relationship to Provisional Changes

The uncommitted changes currently in the working tree (action_transitions table,
validated update_action_status, initial transition inserts) are **provisional
sketches** that partially implement Phase 3 concepts. They should be:

- **Kept:** The concept of a transition log, authority parameter, validation on write
- **Modified:** Table name (`action_transitions` → `handoffs`), add idempotency_key,
  add reasoning field, enforce transactional atomicity
- **Discarded if design calls for it:** If the final design uses a different structure

These changes are NOT to be committed until the design is approved and Phase 1-2
are complete. They serve as implementation notes, not as the target state.
