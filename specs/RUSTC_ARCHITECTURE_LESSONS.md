# CRABJAR ARCHITECTURE SPEC: LEARNING FROM RUSTC

## Cross-Cutting Introspective Analysis — How rustc's Design Informs crabjar's Evolution

**Status**: In Progress — Step 1 (typed ConciergeOutcome) and Step 2 (JEV Arbiter core + exec wiring) complete; remaining: GuardDb judgment recording, JEV embedding pass, async background runner, sequential-vs-parallel reconciliation  
**Date**: 2026-11-09  
**Updated**: 2026-11-09 — Step 2 integration verified end-to-end (commit `d5415bd`)  
**Inspiration**: Structural analysis of rust-lang/rust (4.1M LOC, 389 crates)  
**Scope**: Architecture patterns, language feature utilization, and concrete migration paths

---

## Executive Summary

rustc is the most sophisticated piece of Rust software ever written — a 4-million-line compiler that compiles itself. Its architecture embodies decades of lessons about how to build large, maintainable, extensible systems in Rust. crabjar, as an agent orchestration framework aiming to be a drop-in replacement for Hermes Agent, faces analogous challenges: managing complexity, enabling incremental updates, and supporting pluggable components.

This spec identifies **five architectural patterns** from rustc that directly apply to crabjar, assesses where crabjar already uses them (partially), and proposes concrete migration paths to fully utilize Rust's language features.

### The Five Patterns

| Pattern | rustc Implementation | crabjar Current State | Migration Priority |
|---------|---------------------|----------------------|-------------------|
| 1. Query System as Architecture | 232 memoized queries in declarative DSL | Procedural orchestrator (1,381-line main.rs) | **High** |
| 2. Observed Dependency Tracking | Dep graph with green/red invalidation | Age-based staleness tiers (7/14/30 days) | **Medium** |
| 3. Logical vs Procedural Solving | Trait solver: proof search (12.7k LOC) vs candidate selection (51.6k LOC) | Guard: if/else policy chains | **High** |
| 4. Generic Core + Specific Delegates | Search graph parameterized over `Interner` + `Delegate` | Monolithic orchestrator with hardcoded backends | **Medium** |
| 5. Declarative DSLs via Macros | `rustc_queries!`, `define_dep_nodes!` | TOML config files (no macros) | **Low** (but high leverage) |

### Implementation Progress (2026-11-09)

**Step 1: Typed `ConciergeOutcome` (commit `9cd9809`)** — Replaced string-based gate results with an exhaustive `ConciergeOutcome` enum (`Approved`, `Denied`, `Queued`). This makes the concierge's decision space explicit and forces all callers to handle every outcome.

**Step 2: JEV Arbiter (commits `6664be8`, `d5415bd`)** — Implemented a multi-pass arbiter that evaluates pending actions before escalating to human review:

- `JevArbiter::with_standard_passes()` runs 5 evaluation passes:
  - `CommandRiskPass`: pattern-matches against risk lists (high/medium/low)
  - `TrustLayerPass`: evaluates trust layer position
  - `ConfidencePass`: assesses action confidence score
  - `ProvenancePass`: checks for source event/memory node linkage
  - `HistoryPass`: compares against past decisions (stub, pending GuardDb integration)
- Outcomes: `Approved` (confidence ≥ 0.7), `Denied` (≤ -0.7), or `Escalated` (threshold not reached) with structured evidence trail
- Wired into `src/commands/exec.rs`: when `GateConcierge` produces `Queued`, the arbiter runs synchronously; approved actions proceed, non-approved fall through to human review

**Verified end-to-end**: Running `crabjar exec --command "git" --args '["status"]'` at trust layer 3 produced:
```
JEV Arbiter: running 5 passes for action "git status"
  command_risk: -0.2 (medium-risk command)
  trust_layer: +0.1 (trust layer 3)
  confidence: +0.24 (action confidence 0.90)
  provenance: -0.3 (no provenance)
  history: 0.0 (no historical data)
JEV Arbiter: final confidence -0.160, escalating to human review
```

**Remaining work**:
- GuardDb judgment recording for arbitration outcomes (beyond gate persistence)
- JEV embedding pass (semantic similarity against past decisions)
- Async background runner for the arbiter (to avoid blocking the exec pipeline)
- Sequential-vs-parallel pass reconciliation
- Workspace-wide re-verification

---

## Pattern 1: Query System as Architecture

### The rustc Pattern

rustc's core innovation is treating the entire compilation pipeline as a **graph of memoized queries**. Each pass (parsing, type checking, borrow checking, MIR generation, codegen) is a query with:
- A **key** (usually `DefId` — the unique identifier for an item)
- A **value** (the result of that pass)
- **Dependencies** (observed during execution, not declared)
- **Invalidation semantics** (content-hash based)

This enables:
- **Incremental compilation**: Only recompute what changed
- **Parallelism**: Independent queries run concurrently
- **Debuggability**: Every intermediate result is inspectable
- **Extensibility**: New passes are new queries, no pipeline modification needed

### crabjar's Current State

The orchestrator (`crates/orchestrator/src/main.rs`, 1,381 lines) is a procedural state machine:

```rust
// Current pattern (simplified):
async fn run_session() {
    let user_input = read_input().await;
    let context = build_context(user_input).await;      // Step 1
    let decision = make_decision(context).await;         // Step 2
    let actions = plan_actions(decision).await;          // Step 3
    for action in actions {
        execute(action).await;                           // Step 4
    }
    let outcome = evaluate_outcome(actions).await;       // Step 5
    update_memory(outcome).await;                        // Step 6
}
```

Problems:
- No memoization: Re-running a session recomputes everything
- No incremental updates: Changing one piece of context invalidates the entire pipeline
- No parallelism: Steps are sequential even when independent
- Hard to extend: Adding a new step requires modifying the main loop

### Proposed Migration: The crabjar Query System

Adopt rustc's query pattern for the agent pipeline. Define each stage as a memoized query:

```rust
// crates/orchestrator/src/queries.rs
use crabjar_query::{query, QueryRegistry};

/// The query registry — all agent pipeline stages are queries.
pub struct AgentQueryRegistry;

impl QueryRegistry for AgentQueryRegistry {
    type Key = SessionKey;
    
    rustc_with_all_queries! { define_queries! }
}

// Individual queries:
query parse_user_input(session: SessionKey) -> ParsedInput {
    desc { "parsing input for session `{}`", session.id() }
    cache_on_disk
}

query build_context(session: SessionKey) -> AgentContext {
    desc { "building context for session `{}`", session.id() }
    depends_on(parse_user_input)
}

query make_decision(session: SessionKey) -> Decision {
    desc { "making decision for session `{}`", session.id() }
    depends_on(build_context)
    arena_cache  // Results are large, keep in memory
}

query plan_actions(session: SessionKey) -> Vec<Action> {
    desc { "planning actions for session `{}`", session.id() }
    depends_on(make_decision)
}

query execute_action(action_id: ActionId) -> ExecutionResult {
    desc { "executing action `{}`", action_id }
    feedable  // Results can be injected externally (e.g., from human approval)
}

query evaluate_outcome(session: SessionKey) -> Outcome {
    desc { "evaluating outcome for session `{}`", session.id() }
    depends_on(plan_actions)
}

query update_memory(session: SessionKey) -> MemoryUpdate {
    desc { "updating memory for session `{}`", session.id() }
    depends_on(evaluate_outcome)
    eval_always  // Never cache — side effect
}
```

**Benefits**:
- **Incremental sessions**: If only the user input changes, only `parse_user_input` and downstream queries recompute. Context building, decision making, etc. are cached.
- **Parallel execution**: `execute_action` queries for independent actions can run concurrently.
- **Debuggability**: Every intermediate result (parsed input, context, decision, plan) is inspectable via the query cache.
- **Extensibility**: Adding a new stage (e.g., "review_actions") is adding one query, not modifying the main loop.

**Implementation Notes**:
- The `crabjar-query` crate would be a lightweight reimplementation of rustc's query infrastructure, adapted for async Rust and agent workloads.
- Keys would be `SessionKey` (session ID + step identifier) instead of `DefId`.
- Cache backend: SQLite (already in use) with content-hash invalidation.
- This is a **new crate**, not a rewrite of the orchestrator. The existing procedural code becomes the query implementations.

---

## Pattern 2: Observed Dependency Tracking

### The rustc Pattern

rustc's dep graph tracks **what queries actually read during execution**, not what they claim to depend on. When query A reads query B's result, an edge A→B is recorded automatically via thread-local state.

This enables:
- **Precise invalidation**: Only queries that actually used a changed value are invalidated
- **No drift**: Dependencies can't become stale because they're observed, not declared
- **Cycle detection**: The graph naturally reveals circular dependencies

### crabjar's Current State

State-docs use **age-based staleness tiers**:

```rust
// crates/memory/src/state_docs/models.rs
pub enum StalenessStatus {
    Fresh,     // < 7 days
    Stale,     // 7-14 days
    Expired,   // 14-30 days
    Moldy,     // > 30 days
}
```

Problems:
- **Imprecise**: A doc that hasn't changed in 8 days is marked "Stale" even if nothing depends on it
- **Eager invalidation**: All docs older than 7 days are flagged, even if their content is still accurate
- **No dependency awareness**: Changing one doc doesn't automatically invalidate docs that depend on it

### Proposed Migration: Content-Hash Invalidation with Observed Dependencies

Adopt rustc's dep graph pattern for state-docs:

```rust
// crates/memory/src/state_docs/dep_graph.rs
use std::collections::HashMap;
use sha2::{Sha256, Digest};

/// A node in the state-doc dependency graph.
#[derive(Debug, Clone)]
pub struct DocNode {
    pub doc_id: DocId,
    pub content_hash: [u8; 32],  // SHA-256 of content
    pub dependencies: Vec<DocId>,  // Docs this one reads from
    pub dependents: Vec<DocId>,    // Docs that read from this one
}

/// The dependency graph for state-docs.
pub struct StateDocDepGraph {
    nodes: HashMap<DocId, DocNode>,
}

impl StateDocDepGraph {
    /// Record that `reader` accessed `source` during evaluation.
    pub fn record_access(&mut self, reader: DocId, source: DocId) {
        let reader_node = self.nodes.entry(reader).or_insert_with(|| DocNode {
            doc_id: reader,
            content_hash: [0; 32],
            dependencies: vec![],
            dependents: vec![],
        });
        let source_node = self.nodes.entry(source).or_insert_with(|| DocNode {
            doc_id: source,
            content_hash: [0; 32],
            dependencies: vec![],
            dependents: vec![],
        });
        
        if !reader_node.dependencies.contains(&source) {
            reader_node.dependencies.push(source);
            source_node.dependents.push(reader);
        }
    }
    
    /// Check if a doc is still valid (green) given the current graph state.
    pub fn is_valid(&self, doc_id: DocId) -> bool {
        let node = &self.nodes[&doc_id];
        // A doc is valid if its content hash matches AND all dependencies are valid
        node.content_hash == self.current_content_hash(doc_id)
            && node.dependencies.iter().all(|dep| self.is_valid(*dep))
    }
    
    /// Mark a doc as changed and return all docs that need re-evaluation.
    pub fn invalidate(&mut self, doc_id: DocId) -> Vec<DocId> {
        // BFS through dependents to find all affected docs
        let mut affected = vec![];
        let mut queue = vec![doc_id];
        while let Some(current) = queue.pop() {
            if let Some(node) = self.nodes.get(&current) {
                for dependent in &node.dependents {
                    if !affected.contains(dependent) {
                        affected.push(*dependent);
                        queue.push(*dependent);
                    }
                }
            }
        }
        affected
    }
}
```

**Benefits**:
- **Precise invalidation**: Only docs that actually depend on a changed doc are invalidated
- **Content-based, not age-based**: A doc is valid as long as its content (and its dependencies' content) hasn't changed
- **Automatic propagation**: Changing one doc automatically flags all downstream dependents

**Implementation Notes**:
- This replaces the `StalenessStatus` enum with a graph-based validity check
- The age-based tiers can remain as a secondary signal ("this doc is old, consider refreshing it") but not as the primary invalidation mechanism
- Thread-local tracking of accesses during doc evaluation (similar to rustc's `dep_tracking.rs`)

---

## Pattern 3: Logical vs Procedural Solving

### The rustc Pattern

The trait solver migration is the clearest example of "reframe the problem" yielding massive code reduction:

| Aspect | Old Solver (Procedural) | New Solver (Logical) |
|--------|------------------------|---------------------|
| Approach | Generate candidates, pick one | Search for a proof |
| LOC | 51,600 | 12,700 (4x smaller) |
| Cycle handling | Special-cased heuristics | Systematic fixpoint iteration |
| Soundness | Proven by code review | Proven by algorithm design |

The key insight: **trait solving is a logical problem** (does this type satisfy this trait?), not a procedural one (which candidate should I pick?). By reframing it as proof search with optimistic assumptions and fixpoint iteration, the new solver is both smaller and more sound.

### crabjar's Current State

The guard system (`crates/guard/src/policy.rs`, 692 lines) uses **procedural if/else chains**:

```rust
// Current pattern (simplified):
pub fn evaluate(&self, action: &Action, context: &GateContext) -> GateResult {
    // Check 1: Dangerous commands
    if self.config.dangerous_commands.contains(&action.command) {
        return GateResult::Denied("dangerous command");
    }
    
    // Check 2: Confidence floor
    if context.confidence < self.config.min_confidence {
        return GateResult::Pending("low confidence");
    }
    
    // Check 3: Trust layer
    if context.trust_layer < self.config.min_trust_layer {
        return GateResult::Denied("insufficient trust");
    }
    
    // ... 20 more checks
    
    GateResult::Allowed
}
```

Problems:
- **Hard to extend**: Adding a new check requires modifying the function
- **No composition**: Checks can't be combined or reused across contexts
- **Opaque reasoning**: You can't see *why* an action was denied without reading the code
- **No proof tree**: The decision process is a black box

### Proposed Migration: Constraint-Based Policy Solving

Reframe policy evaluation as **constraint satisfaction** rather than sequential checks:

```rust
// crates/guard/src/solver.rs
use std::collections::HashMap;

/// A policy constraint — a logical statement about when an action is allowed.
pub trait PolicyConstraint {
    /// Evaluate this constraint against the action and context.
    /// Returns:
    /// - Ok(true): Constraint satisfied
    /// - Ok(false): Constraint violated (action denied)
    /// - Err(Ambiguous): Can't determine (need more info or human input)
    fn evaluate(&self, action: &Action, context: &GateContext) -> Result<bool, Ambiguity>;
    
    /// Get a human-readable description of this constraint.
    fn describe(&self) -> String;
}

/// A policy solver that evaluates constraints as a logical problem.
pub struct PolicySolver {
    constraints: Vec<Box<dyn PolicyConstraint>>,
}

impl PolicySolver {
    pub fn evaluate(&self, action: &Action, context: &GateContext) -> GateResult {
        let mut proof_tree = ProofTree::new(action);
        
        for constraint in &self.constraints {
            match constraint.evaluate(action, context) {
                Ok(true) => {
                    proof_tree.add_step(constraint.describe(), StepOutcome::Satisfied);
                }
                Ok(false) => {
                    proof_tree.add_step(constraint.describe(), StepOutcome::Violated);
                    return GateResult::DeniedWithProof(proof_tree);
                }
                Err(ambiguity) => {
                    proof_tree.add_step(constraint.describe(), StepOutcome::Ambiguous(ambiguity));
                    // Continue evaluating other constraints — maybe they resolve the ambiguity
                }
            }
        }
        
        GateResult::AllowedWithProof(proof_tree)
    }
}

/// Individual constraints as composable units:
pub struct DangerousCommandConstraint {
    pub commands: Vec<String>,
}

impl PolicyConstraint for DangerousCommandConstraint {
    fn evaluate(&self, action: &Action, _context: &GateContext) -> Result<bool, Ambiguity> {
        Ok(!self.commands.contains(&action.command))
    }
    
    fn describe(&self) -> String {
        format!("command '{}' not in dangerous list", action.command)
    }
}

pub struct ConfidenceFloorConstraint {
    pub min_confidence: f32,
}

impl PolicyConstraint for ConfidenceFloorConstraint {
    fn evaluate(&self, _action: &Action, context: &GateContext) -> Result<bool, Ambiguity> {
        if context.confidence >= self.min_confidence {
            Ok(true)
        } else {
            Err(Ambiguity::InsufficientConfidence {
                actual: context.confidence,
                required: self.min_confidence,
            })
        }
    }
    
    fn describe(&self) -> String {
        format!("confidence {} >= {}", context.confidence, self.min_confidence)
    }
}
```

**Benefits**:
- **Composable**: New constraints are new types implementing `PolicyConstraint`, no modification to the solver
- **Transparent reasoning**: The `ProofTree` shows exactly which constraints were checked and why
- **Ambiguity as first-class**: Constraints can return "I don't know" instead of forcing a binary decision
- **Smaller codebase**: Each constraint is focused (20-50 lines) vs. the current 692-line monolith

**Implementation Notes**:
- This is a **refactor of the existing guard system**, not a rewrite
- The `PolicyConstraint` trait is the key abstraction — it's the crabjar equivalent of rustc's `Delegate` trait
- Constraints can be loaded from TOML config (existing pattern) or defined in code
- The `ProofTree` enables better error messages and audit trails

---

## Pattern 4: Generic Core + Specific Delegates

### The rustc Pattern

The search graph (`rustc_type_ir::search_graph`, 1,503 lines) is completely generic — it knows nothing about Rust traits. It's parameterized over:
- `Cx` — the context (interner)
- `Input` — the goal being solved
- `Result` — the solution
- `Delegate` — trait providing goal-specific logic

This separation enables:
- **Reusability**: The same search graph powers trait solving, and could power other logical problems
- **Testability**: The generic core can be tested with mock delegates
- **Extensibility**: New goal types are new delegate implementations, no changes to the search graph

### crabjar's Current State

The orchestrator has **hardcoded backends**:

```rust
// Current pattern (simplified):
async fn make_inference_request(prompt: &str) -> Result<String> {
    if config.use_lm_studio {
        lm_studio_client.complete(prompt).await
    } else if config.use_openai {
        openai_client.complete(prompt).await
    } else {
        local_model.complete(prompt).await
    }
}
```

Problems:
- **Tightly coupled**: Adding a new backend requires modifying the orchestrator
- **Hard to test**: Can't easily mock the inference layer
- **No abstraction**: The orchestrator knows about specific backends (LM Studio, OpenAI, local)

### Proposed Migration: The crabjar Inference Delegate

Adopt rustc's delegate pattern for the inference layer:

```rust
// crates/orchestrator/src/inference.rs
use async_trait::async_trait;

/// The inference delegate — abstracts over all LLM backends.
#[async_trait]
pub trait InferenceDelegate: Send + Sync {
    /// Generate a completion for the given prompt.
    async fn complete(&self, prompt: &Prompt) -> Result<Completion, InferenceError>;
    
    /// Get metadata about this backend (model name, capabilities, etc.).
    fn metadata(&self) -> BackendMetadata;
    
    /// Check if this backend is available.
    async fn is_available(&self) -> bool;
}

/// The orchestrator uses the delegate, not specific backends:
pub struct Orchestrator<D: InferenceDelegate> {
    delegate: D,
    // ... other fields
}

impl<D: InferenceDelegate> Orchestrator<D> {
    async fn make_decision(&self, context: &AgentContext) -> Result<Decision> {
        let prompt = build_prompt(context);
        let completion = self.delegate.complete(&prompt).await?;
        parse_decision(completion)
    }
}

// Concrete implementations:
pub struct LmStudioDelegate { /* ... */ }
impl InferenceDelegate for LmStudioDelegate { /* ... */ }

pub struct OpenAiDelegate { /* ... */ }
impl InferenceDelegate for OpenAiDelegate { /* ... */ }

pub struct LocalModelDelegate { /* ... */ }
impl InferenceDelegate for LocalModelDelegate { /* ... */ }

// Mock for testing:
#[cfg(test)]
pub struct MockDelegate {
    pub responses: Vec<Completion>,
}
impl InferenceDelegate for MockDelegate { /* ... */ }
```

**Benefits**:
- **Pluggable backends**: New backends are new `InferenceDelegate` implementations, no orchestrator changes
- **Testable**: The orchestrator can be tested with `MockDelegate`
- **Generic core**: The orchestration logic is the same regardless of backend

**Implementation Notes**:
- This is a **refactor of the existing inference layer**, not a rewrite
- The `InferenceDelegate` trait is the crabjar equivalent of rustc's `Delegate` trait
- Existing backends (LM Studio, OpenAI) become delegate implementations
- The orchestrator becomes generic over the delegate type

---

## Pattern 5: Declarative DSLs via Macros

### The rustc Pattern

rustc uses macros to generate large amounts of boilerplate code from declarative specifications:

```rust
// Define 232 queries in one macro invocation:
rustc_queries! {
    query parse_input(session: SessionKey) -> ParsedInput { ... }
    query build_context(session: SessionKey) -> AgentContext { ... }
    // ... 230 more
}

// Automatically generate DepKind variants for all queries:
crate::queries::rustc_with_all_queries! { define_dep_nodes! }
```

This enables:
- **Consistency**: All queries have the same structure and behavior
- **Maintainability**: Adding a new query is one line, not 50 lines of boilerplate
- **Type safety**: The macro generates type-safe code, no stringly-typed APIs

### crabjar's Current State

crabjar uses **TOML config files** for declarative configuration:

```toml
# .crabjar_config.toml
[tool_execution]
enabled = true

[policy]
min_confidence = 0.8
dangerous_commands = ["rm -rf", "sudo"]
```

This is good, but it's **data-driven**, not **code-generating**. The config is interpreted at runtime, not compiled into type-safe code.

### Proposed Migration: A crabjar Query DSL

Introduce a lightweight macro system for defining queries and constraints:

```rust
// crates/crabjar-query/src/lib.rs
macro_rules! define_queries {
    ($($query:ident($key:ident : $key_ty:ty) -> $value_ty:ty $( { $($meta:tt)* } )?);* $(;)?) => {
        // Generate a QueryRegistry with all the queries
        pub struct QueryRegistry;
        
        impl crate::QueryRegistry for QueryRegistry {
            $(
                fn run_$query(&self, key: $key_ty) -> crate::QueryResult<$value_ty> {
                    // Call the user-defined implementation
                    super::$query(key)
                }
            )*
        }
    };
}

// Usage in the orchestrator:
define_queries! {
    parse_input(session: SessionKey) -> ParsedInput { cache_on_disk };
    build_context(session: SessionKey) -> AgentContext { depends_on(parse_input) };
    make_decision(session: SessionKey) -> Decision { arena_cache };
}
```

**Benefits**:
- **Type-safe queries**: The macro generates type-safe code, no runtime interpretation
- **Automatic registry**: All queries are automatically registered in the `QueryRegistry`
- **Declarative metadata**: Cache policies and dependencies are declared inline

**Implementation Notes**:
- This is a **new crate** (`crabjar-query`) that provides the macro infrastructure
- Start small: One macro for query definitions, one for constraint definitions
- Don't try to replicate rustc's full macro system — keep it simple and focused

---

## Rust Language Features crabjar Should Fully Utilize

Beyond the architectural patterns, there are specific Rust language features that crabjar underutilizes:

### 1. Traits (Currently: 14 across the workspace)

rustc has **thousands** of traits. crabjar has 14. This is a huge gap.

**Where to add traits**:
- `InferenceDelegate` (Pattern 4 above)
- `PolicyConstraint` (Pattern 3 above)
- `QueryProvider` (Pattern 1 above)
- `StateDocSource` — abstract over where state-docs come from (filesystem, API, etc.)
- `ExecutionBackend` — abstract over execution environments (local, VM, container, remote)

**Target**: 50+ traits within 6 months. Not for the sake of it — each trait should represent a genuine abstraction boundary.

### 2. Macros (Currently: 0)

crabjar has **zero** macros. rustc has hundreds.

**Where to add macros**:
- `define_queries!` — declarative query definitions (Pattern 5 above)
- `define_constraints!` — declarative policy constraints
- `derive_query_key!` — automatic key derivation for queries
- `impl_delegate!` — boilerplate reduction for delegate implementations

**Target**: 5-10 focused macros that eliminate genuine boilerplate. Don't add macros for the sake of it.

### 3. Async/Await Patterns

crabjar uses async/await, but mostly in a "fire and forget" pattern. rustc's query system shows how to use async more effectively:

- **Parallel query evaluation**: Independent queries should run concurrently
- **Cancellation propagation**: If a session is cancelled, all in-flight queries should be cancelled
- **Timeout handling**: Queries should have configurable timeouts with graceful degradation

### 4. Error Handling (thiserror)

crabjar uses `thiserror` for library errors (good), but the orchestrator has ad-hoc error handling. Adopt rustc's pattern of **structured error types** that carry context:

```rust
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("query '{name}' failed: {source}")]
    ExecutionFailed { name: String, source: Box<dyn Error> },
    
    #[error("cycle detected in query graph: {path}")]
    CycleDetected { path: Vec<String> },
    
    #[error("query '{name}' timed out after {timeout:?}")]
    TimedOut { name: String, timeout: Duration },
}
```

### 5. Compile-Time Guarantees

rustc uses the type system to prevent entire classes of bugs. crabjar can do the same:

- **Type-safe query keys**: `SessionKey`, `ActionId`, `DocId` as newtypes instead of `String`
- **Phantom types for state machines**: Use zero-sized types to represent session states, making invalid transitions impossible
- **Const generics for configuration**: Use const generics for fixed-size buffers and limits

---

## Migration Roadmap

### Phase 1: Foundation (Weeks 1-4)
- [ ] Create `crabjar-query` crate with basic query infrastructure
- [ ] Define `InferenceDelegate` trait and refactor existing backends
- [ ] Define `PolicyConstraint` trait and refactor guard policy system
- [ ] Add type-safe newtypes for `SessionKey`, `ActionId`, `DocId`

### Phase 2: Query System (Weeks 5-8)
- [ ] Implement memoization layer for queries (SQLite backend)
- [ ] Migrate orchestrator pipeline to query-based architecture
- [ ] Add parallel query evaluation for independent actions
- [ ] Implement cancellation propagation

### Phase 3: Dependency Tracking (Weeks 9-12)
- [ ] Implement `StateDocDepGraph` with observed dependencies
- [ ] Replace age-based staleness with content-hash invalidation
- [ ] Add incremental re-evaluation for state-docs
- [ ] Integrate dep graph with query system

### Phase 4: DSL and Macros (Weeks 13-16)
- [ ] Implement `define_queries!` macro
- [ ] Implement `define_constraints!` macro
- [ ] Add automatic registry generation
- [ ] Migrate existing queries/constraints to declarative form

### Phase 5: Polish (Weeks 17-20)
- [ ] Add proof tree visualization for policy decisions
- [ ] Implement query debugging tools (inspect intermediate results)
- [ ] Add benchmarks comparing procedural vs query-based performance
- [ ] Write documentation and migration guide

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Query system adds latency | Medium | High | Benchmark early; cache aggressively; only memoize expensive queries |
| Macro complexity | Low | Medium | Keep macros simple; start with 1-2, add more as needed |
| Refactoring breaks existing behavior | Medium | High | Incremental migration; keep old code path available during transition |
| Over-engineering | Medium | Medium | Each pattern must solve a real problem; don't adopt for the sake of it |

---

## Conclusion

rustc's architecture is a masterclass in how to build large, maintainable systems in Rust. The five patterns identified here — query system as architecture, observed dependency tracking, logical vs procedural solving, generic core + specific delegates, and declarative DSLs — are not just compiler tricks; they're general-purpose software engineering principles that apply directly to crabjar's needs.

The migration is **incremental** and **optional**. Each pattern can be adopted independently, and the existing codebase continues to work throughout the transition. The goal is not to rewrite crabjar as a compiler, but to learn from how rustc solves problems that crabjar also faces: managing complexity, enabling incremental updates, and supporting pluggable components.

The payoff is a crabjar that is:
- **Faster**: Memoization and parallelism reduce redundant work
- **More reliable**: Content-hash invalidation and proof trees catch errors early
- **More extensible**: Traits and delegates make it easy to add new backends, constraints, and pipeline stages
- **More debuggable**: Query caches and proof trees make the system's behavior transparent

This is what "fully utilizing Rust" means: not just using the language's features, but adopting the architectural patterns that make those features shine.

---

## Appendix A: How rustc Handles Complexity — Empirical Analysis

*Generated 2026-11-09 via structural analysis of rust-lang/rust (shallow clone) and crabjar (working tree).*

### A.1 Scale Comparison

| Metric | rustc (compiler/) | crabjar (crates/ + src/) |
|--------|-------------------|--------------------------|
| Total Rust files | 2,243 | ~150 |
| Total lines | 891,000 | 107,000 |
| Crates | 389 (compiler) | ~20 |
| Avg file size | 398 lines | 166 lines |

### A.2 Function Complexity

| Metric | rustc (rustc_middle + next_trait_solver) | crabjar (orchestrator + guard) |
|--------|------------------------------------------|-------------------------------|
| Files analyzed | 144 | 64 |
| Total functions | 3,607 | 703 |
| Avg function size | **11.9 lines** | **15.4 lines** |
| Branch density | 3.2% of lines | 2.2% of lines |
| Branches per function | 0.7 | 0.5 |

**Key insight**: rustc's functions are *smaller* and *more branched* than crabjar's. This reflects a design philosophy where complex logic is decomposed into many small, focused decision points rather than fewer, larger procedural blocks.

### A.3 Language Feature Utilization

| Feature | rustc (compiler/) | crabjar (crates/ + src/) | Ratio |
|---------|-------------------|--------------------------|-------|
| Trait definitions | 394 | 98 | crabjar: **higher density** (9 vs 4 per 10k LOC) |
| Macro definitions (`macro_rules!`) | **487** | **10** | rustc: **49x more** |
| Async functions | 128 | **796** | crabjar: **6x more** (inherently async domain) |
| `await` expressions | 54 | **1,230** | crabjar: **23x more** |
| Named lifetime parameters (`'a`) | **297** | **6** | rustc: **50x more** |
| PhantomData usage | 313 | 90 | rustc: **3.5x more** |
| Where clauses | 1,864 | 1,075 | Similar density |
| Type aliases (`type X =`) | 262 | 70 | rustc: **3.7x more** |

### A.4 The Four Complexity-Handling Techniques

#### Technique 1: Declarative DSLs via Proc Macros (rustc: 487 macros vs crabjar: 10)

rustc's most powerful complexity-management tool is the **proc macro**. Instead of writing boilerplate code, developers define *what* they want declaratively, and macros generate the implementation.

**Example: The Query DSL**

```rust
// In rustc_middle/src/queries.rs — ONE declarative block generates 232 query implementations
rustc_queries! {
    query derive_macro_expansion(key: (LocalExpnId, &'tcx TokenStream)) 
        -> Result<&'tcx TokenStream, ()> {
        desc { "expanding a derive (proc) macro" }
        cache_on_disk
    }
    
    query trigger_delayed_bug(key: DefId) {
        desc { "triggering a delayed bug for testing incremental" }
    }
    
    query registered_attr_tools(_: ()) -> &'tcx ty::RegisteredTools {
        arena_cache
        desc { "compute registered attribute tools for crate" }
    }
}
```

Each query definition generates:
- A method on `TyCtxt` for calling the query
- Cache management (in-memory + on-disk)
- Dependency tracking registration
- Cycle detection hooks
- Error handling boilerplate

**crabjar opportunity**: Define tools, guard policies, and state transitions declaratively. Generate handlers, validators, and transition logic via macros. This would reduce the 234-line `execute_process_management` function to a declarative spec + generated code.

#### Technique 2: Named Lifetimes for Memory Safety (rustc: 297 vs crabjar: 6)

rustc uses the `'tcx` lifetime pervasively to tie all query results to the compiler context's lifetime. This ensures that:
- Query results can't outlive the data they reference
- No manual cleanup is needed (arena allocation + lifetime guarantees)
- The borrow checker enforces correctness at compile time

**Example: TyCtxt as a thin wrapper**

```rust
pub struct TyCtxt<'tcx> {
    gcx: &'tcx GlobalCtxt<'tcx>,
}

impl<'tcx> Deref for TyCtxt<'tcx> {
    type Target = &'tcx GlobalCtxt<'tcx>;
    fn deref(&self) -> &Self::Target { &self.gcx }
}
```

All query methods return references tied to `'tcx`. This means:
- `tcx.type_of(def_id)` returns `&'tcx Ty<'tcx>` — can't outlive the compiler session
- No cloning, no Arcs, no manual lifetime management
- The type system *proves* memory safety

**crabjar opportunity**: Use named lifetimes for:
- `'session` — session-scoped data (can't outlive the agent session)
- `'task` — task-scoped resources (can't outlive the task)
- `'guard` — guard evaluation context (can't outlive the evaluation)

This would eliminate many `Arc<T>` wrappers and make resource lifetimes explicit in the type system.

#### Technique 3: Typestate Patterns via PhantomData (rustc: 313 vs crabjar: 90)

rustc uses `PhantomData` to encode state in types, making invalid states **unrepresentable** at compile time.

**Example from rustc_attr_parsing**:
```rust
pub struct Single<T>(T);           // Exactly one argument
pub struct WithoutArgs<T>(T);      // Zero arguments
```

These types make it impossible to accidentally pass the wrong number of arguments — the compiler catches it.

**crabjar opportunity**: Encode guard and task states as types:
```rust
pub struct GuardState<Pending> { _marker: PhantomData<Pending> }
pub struct Approved<Pending> { pending: Pending, approval_id: Uuid }
pub struct Denied<Pending> { pending: Pending, reason: String }

// Invalid transitions are compile errors:
// - Can't execute a Denied guard
// - Can't approve an already-Approved guard
// - Can't query the approval_id of a Pending guard
```

This would eliminate runtime state checks and make the guard system's invariants *provable*.

#### Technique 4: Generic Core + Specific Delegates

rustc's trait solver uses a **generic search graph** parameterized over an `Interner` (type representation) and a `Delegate` (domain-specific logic). The same search algorithm works for:
- Trait resolution
- Type normalization
- Coercion checking
- Auto-trait derivation

**Example: SearchGraph struct**

```rust
pub struct SearchGraph<D: Delegate<Cx = X>, X: Cx = <D as Delegate>::Cx> {
    root_depth: AvailableDepth,
    stack: Stack<X>,
    provisional_cache: HashMap<X::Input, Vec<ProvisionalCacheEntry<X>>>,
    _marker: PhantomData<D>,
}
```

The search graph knows *how* to search (DFS with fixpoint iteration), but the delegate knows *what* to search for (trait goals, normalization steps, etc.). This separation enables:
- Code reuse across different solving strategies
- Easy addition of new solving domains
- Independent testing of the search algorithm and domain logic

**crabjar opportunity**: The orchestrator's decision-making could use a similar pattern:
- Generic `DecisionGraph<D>` parameterized over a `DecisionDelegate`
- Different delegates for tool selection, guard evaluation, task routing
- The graph handles search/caching/invalidation; the delegate handles domain logic

### A.5 Most Complex Functions — Refactoring Candidates

**crabjar's most complex functions** (candidates for decomposition):

| Lines | File | Function | Suggested Decomposition |
|-------|------|----------|------------------------|
| 234 | orchestrator/process_management.rs | `execute_process_management` | Split into: parse_request, validate_target, spawn_process, monitor_output, handle_result |
| 221 | guard/gate/mod.rs | `check` | Policy evaluation pipeline: load_policy → evaluate_rules → apply_overrides → produce_decision |
| 208 | guard/concierge.rs | `enforce` | Enforcement decision tree: check_permissions → verify_scope → execute_or_deny → log_outcome |

**rustc's most complex functions** (for comparison — all in specialized solving logic):

| Lines | File | Function |
|-------|------|----------|
| 221 | next_trait_solver/solve/normalizes_to.rs | `consider_impl_candidate` |
| 131 | next_trait_solver/solve/trait_goals.rs | `consider_builtin_upcast_to_principal` |
| 128 | next_trait_solver/solve/assembly/mod.rs | `assemble_alias_bound_candidates_recur` |

rustc's largest functions are concentrated in the trait solver (the most algorithmically complex part of the compiler). crabjar's largest functions are in the orchestrator and guard — suggesting these are where complexity management should focus.

### A.6 Summary: What Makes rustc Manageable at 4M LOC

1. **Declarative over procedural**: Define what, not how. Macros generate the how.
2. **Small, focused functions**: Average 11.9 lines. Each function does one thing.
3. **Type system as documentation**: Named lifetimes, typestate patterns, and newtypes make invariants explicit.
4. **Generic cores with specific delegates**: Algorithmic logic is separated from domain logic.
5. **Automatic infrastructure**: Dependency tracking, caching, cycle detection are generated, not hand-written.

crabjar already excels at #2 (small functions) and has good trait density. The biggest gaps are in #1 (macros), #3 (named lifetimes, typestate), and #4 (generic cores). These are the areas where "fully utilizing Rust" would have the highest impact on crabjar's complexity management.
