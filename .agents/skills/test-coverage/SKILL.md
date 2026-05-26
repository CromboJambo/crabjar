---
name: test-coverage
description: |
  Improve Rust crate test coverage by identifying low-coverage files and adding targeted tests.
  Trigger whenever the user asks about test coverage, coverage gaps, wants more tests, asks to
  "tackle hotspots", "add tests", "run tarpaulin", or mentions coverage metrics, line coverage,
  or test gaps — even without using the words "coverage" or "tests" explicitly (e.g. "what's the
  test situation?", "are we well tested?").
---

# Test Coverage Improvement

Identify low-coverage Rust files and add targeted tests to raise coverage while keeping tests passing and clippy clean.

## Workflow

### 1. Baseline test counts

Run `cargo test --workspace 2>&1 | grep -E "^test result:" | paste - - | column -t -s$'\t'` to get per-crate test counts. Check for crates with 0 tests or few tests relative to their scope.

### 2. Line coverage with tarpaulin

Run `cargo tarpaulin --workspace --out Html --engine llvm --timeout 120 2>&1` to get line-level coverage. Read the full output (it goes to stdout, not just the summary).

### 3. Identify hotspots

Filter tarpaulin output for workspace crates only (skip `state-docs/`, `state-docs/oxc/`, `state-docs/zeroclaw/`, `state-docs/vllm.rs/` — these are external reference docs, not project code). Sort by lowest coverage percentage.

Focus on files with coverage below 40%:
- `orchestrator/src/main.rs` — Axum server, typically lowest
- `zed-acp-server/src/lib.rs` — often has ignored tests
- `guard/src/retrieval.rs`, `guard/src/types.rs` — types and retrieval logic
- `telemetry/src/command_executor.rs`, `telemetry/src/flight_recorder.rs` — async execution paths
- `safetensors/src/safetensors_store.rs` — file parsing paths
- `memory/src/store.rs`, `memory/src/models.rs` — query paths and model types
- `tool_registry/src/tool_registry.rs` — discovery and registration

### 4. Read each low-coverage file

Read the full file to understand its structure. Look for:
- Public methods with no tests
- Conditional branches (if/else, match arms) not exercised
- Error paths (Err returns, unwrap calls)
- Type methods (Display, Default, Clone, serde)
- Edge cases (empty inputs, boundary values)

### 5. Add targeted tests

For each file, add tests covering:
- **Public method paths**: Each public method should have at least one test
- **Conditional branches**: Cover each arm of if/else and match
- **Error paths**: Test the Err/None/error-returning branches
- **Type methods**: Test Display, Default, Clone, PartialEq, serde (serialize + deserialize)
- **Edge cases**: Empty inputs, boundary values, zero limits, nonexistent lookups
- **Async methods**: Use `#[tokio::test]` for async functions
- **Removed #[ignore]**: Check for ignored tests — un-ignore and fix if they pass

Write tests in the existing `#[cfg(test)] mod tests` block. If no test module exists, create one. Use `tempfile::tempdir()` for filesystem fixtures.

### 6. Fix compilation errors

Run `cargo test --workspace 2>&1 | tail -50` to check for compilation errors. Fix iteratively:
- Missing `#[tokio::test]` on async tests
- Missing `.await` on async method calls
- Borrow/ownership errors from test changes
- Mismatched type assertions (serde output may differ from expected)
- Incorrect test setup (e.g., methods that don't store state in expected locations)

### 7. Verify

Run `cargo test --workspace 2>&1 | grep -E "test result:"` — all must show `ok. N passed; 0 failed`.
Run `cargo clippy --workspace -- -D warnings 2>&1 | tail -5` — must show `Finished` with no errors.

### 8. Summarize

Present a table showing each file's test count before/after and the total delta. Include the caveat that tarpaulin line coverage is a separate metric from unit test count.

## Notes

- Tarpaulin output includes external `state-docs/` content — always filter it out when scoring
- Some methods are inherently hard to test (network calls, real file I/O) — prioritize what's testable
- If a file has many tests already but low tarpaulin coverage, the gap may be in complex conditional logic or async paths that tarpaulin's LLVM instrumentation captures differently
- When fixing ignored tests, check if the underlying code actually stores state as the test expects — sometimes the test setup doesn't match the actual behavior (e.g., `NewSession` returning a value but not storing it in a vec)
