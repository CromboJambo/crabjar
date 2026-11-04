# Adversarial Review Remediation

**Status:** Proposed  
**Source:** Read-only adversarial review of the crabjar checkout on 2026-11-03  
**Scope:** Ten concrete findings covering command execution, scheduling, configuration, architecture enforcement, and CI. This document records observed defects and acceptance criteria; implementation must verify current code because the checkout may change.

## Goal

Close the reviewed correctness and safety gaps, and make the relevant tests/CI gates detect regressions. Preserve the execution pipeline’s fail-closed authorization semantics. Do not treat telemetry or successful dispatch as proof that a command succeeded.

## Task breakdown (highest-level first)

### 1. Restore safe and truthful command execution

Address findings 1, 3, 4, and 5 as one coordinated execution-path task. The guard decision, selected execution backend, actual command result, and persisted outcome must describe the same command execution.

- [ ] **1.1 Make container mode exclusive and fail closed** (High)
  - Finding: `src/commands/exec.rs` executes through the flight recorder before calling the Podman helper; the helper can fail or be unavailable, while host execution has already occurred. Its result does not drive the returned exit code.
  - Requirement: when container mode is requested, execute only in the container. Do not silently fall back to host execution. Report container creation/start errors as failures. The response must identify the backend actually used, not merely whether a helper returned `Some`.
  - Acceptance: an integration test with a host-visible marker command proves that requesting container execution does not create the host marker; a missing/unavailable Podman backend returns failure and does not run the command on the host; returned exit code and backend metadata come from the selected executor.

- [ ] **1.2 Fail closed when GuardDb cannot be opened; require real provenance** (High)
  - Finding: `src/commands/exec.rs` substitutes a fresh in-memory GuardDb after persistent DB open failure and passes free-form `reason` as `source_event_id`.
  - Requirement: persistent GuardDb open failure must stop execution with a structured error. The command must use a real, validated provenance identifier from the authorization workflow; human-readable reason text is not provenance. Preserve approval/pending-action semantics.
  - Acceptance: tests inject an unavailable/unwritable GuardDb and prove no command executes; tests prove arbitrary reason text cannot satisfy provenance and a valid provenance record follows the intended gate path.

- [ ] **1.3 Report command outcome truthfully** (Medium)
  - Finding: `src/commands/exec.rs` returns `"success": true` regardless of nonzero final command exit status.
  - Requirement: define and document the JSON meaning of `success`. Either derive it from the final execution outcome or expose separate `dispatch_success` and `command_success` fields without conflating them. Keep the CLI output contract structured.
  - Acceptance: a successful command, a nonzero exit, and an execution error each produce unambiguous, tested JSON outcomes.

- [ ] **1.4 Persist outcomes to the correct database and return the persisted ID** (Medium)
  - Finding: outcome insertion opens `flight_db_path` rather than the GuardDb used by the gate, generates a second UUID for the response, and suppresses open/insert failures.
  - Requirement: persist the outcome in the intended database alongside related guard state (or document and test an explicit separate-store design); return the exact inserted record ID; propagate persistence errors or report an explicit partial-failure state.
  - Acceptance: a test reads back the outcome by the returned ID from the intended database and verifies action linkage, success, exit code, and output hash. A forced insert failure is not reported as a successful persisted outcome.

### 2. Restore scheduler liveness

- [ ] **2.1 Release worker capacity on every terminal path** (High)
  - Finding: `scheduler/src/worker.rs` marks workers busy in `spawn`, but `scheduler/src/loop.rs` never marks them available. Completion, join failure, cancellation, or panic can permanently consume capacity.
  - Requirement: associate each spawned job with its worker ID and release that worker on success, returned error, panic/join error, and cancellation. Avoid holding a synchronous mutex across awaits. Ensure worker-pool state cannot remain busy after a task terminates.
  - Acceptance: with a one-worker pool, repeatedly submit jobs and prove work continues after success and failure. Add cancellation/panic coverage and verify `busy_workers()` returns to zero after all jobs terminate. Due jobs must not be duplicated every tick while an earlier instance is still queued/running.

### 3. Preserve configuration validity and error visibility

- [ ] **3.1 Only create defaults for confirmed missing config files** (Medium)
  - Finding: `host/host-core/src/config.rs::load_or_default` treats every `ReadFailed` as a missing file and writes defaults.
  - Requirement: distinguish `NotFound` from permission, I/O, and other read errors. Create defaults only for `NotFound`; propagate all other errors without overwriting or masking configuration state.
  - Acceptance: tests cover missing path (default created), permission/read failure (error propagated, existing content unchanged), and malformed TOML (parse error propagated).

- [ ] **3.2 Return errors for malformed or invalid project config** (Medium)
  - Finding: `src/project_loader.rs::load_from_directory` clears config and returns `Ok(())` for TOML parse errors and missing required project names.
  - Requirement: preserve any intentional not-found behavior, but return parse and validation errors to callers. Avoid leaving loader state that implies a successful load when the requested configuration is invalid.
  - Acceptance: tests distinguish missing config from malformed TOML and missing required fields; invalid inputs produce errors and do not appear as a successful loaded project.

### 4. Make architecture-boundary enforcement complete

- [ ] **4.1 Validate every workspace crate and Cargo dependency form** (Medium)
  - Finding: `crates/crabjar-architecture/src/boundary.rs` skips workspace crates missing from the layer map, reads only top-level dependency tables, and treats dependency aliases as package names. The current completeness test checks only a minimum map size.
  - Requirement: fail the boundary check if any discovered workspace member is unmapped; inspect normal, dev, build, and target-specific dependencies; resolve dependency aliases using `package`; correctly account for workspace-inherited dependencies and workspace member paths. External crates may remain outside the layer model, but workspace members must never be silently skipped.
  - Acceptance: fixture tests prove failures for an unmapped workspace member, a forbidden target-specific dependency, and a forbidden aliased dependency. A valid workspace-inherited dependency is resolved and checked. The real workspace boundary test passes and verifies exact membership, not only map length.

### 5. Make CI claims match what it verifies

- [ ] **5.1 Restore the 500-LoC module-size gate or explicitly revise it** (High for CI health)
  - Finding: review ran `python3 scripts/module-sizes.py --threshold 500`; it exited 1 and reported 26 files over the configured threshold. Examples included `crates/crabjar-conductor/src/conductor.rs` (1,292 lines) and `src/tool_registry_cli.rs` (854 lines).
  - Requirement: split over-limit modules by concern until the configured gate passes, or make an explicit reviewed policy change with corresponding CI/docs updates. Do not weaken or bypass the check just to get a green build.
  - Acceptance: `python3 scripts/module-sizes.py --threshold 500` exits 0 and reports no violations, unless the approved policy is changed with tests/documentation in the same work.

- [ ] **5.2 Verify reproducibility by comparing independent builds** (Medium)
  - Finding: `.github/workflows/rust.yml` runs one release build and labels it a reproducible-build check; it does not compare artifacts. `-C target-cpu=native` makes output host-CPU dependent.
  - Requirement: either implement a deterministic check that performs two clean builds under the same declared environment and compares relevant artifacts, or rename/re-scope the job so it claims only what it tests. Define whether the target is same-runner byte identity or cross-runner reproducibility. Avoid `native` if the claim is portable deterministic output.
  - Acceptance: CI demonstrates a second build and artifact comparison, with documented inputs/toolchain/target; a deliberate output difference causes failure. If byte comparison is intentionally out of scope, the job and docs no longer claim to verify reproducibility.

## Suggested implementation order

1. 1.1 and 1.2 first: execution isolation and authorization integrity.
2. 1.3 and 1.4: truthful outcomes and persistence consistency.
3. 2.1 and 3.1–3.2: scheduler/config reliability.
4. 4.1: close the architecture-model blind spots.
5. 5.1–5.2: restore and align CI governance claims.

Keep each numbered high-level task in a focused change with regression tests. Re-check the cited source locations before implementation; these references describe the reviewed checkout, not a promise that line numbers remain stable.

## Review references

- `src/commands/exec.rs`, `src/commands/podman.rs`
- `scheduler/src/worker.rs`, `scheduler/src/loop.rs`
- `host/host-core/src/config.rs`, `src/project_loader.rs`
- `crates/crabjar-architecture/src/boundary.rs`
- `.github/workflows/rust.yml`, `scripts/module-sizes.py`
