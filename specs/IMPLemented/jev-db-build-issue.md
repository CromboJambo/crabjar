# JEV Database Evidence Store — Build Issue Spec

## Status: RESOLVED

**Fix applied:** Option B (manual .next() loop) — version-agnostic, works with rusqlite 0.37.

**Additional fix discovered:** SQLite query/result row mapping was misaligned by one index. The SELECT statement returns 10 columns (indices 0-9), but the code was reading from indices 7-10, causing `InvalidColumnType` errors. Corrected all column indices in `recent_decisions()`.

**File:** `src/commands/decide_db.rs` (319 lines)
**Module declared in:** `src/commands/mod.rs` (line 7)
**Dependency:** `rusqlite = { version = "0.37", features = ["bundled"] }` (already in Cargo.toml)

## Issue: rusqlite Rows Iterator Compatibility

The `Rows` type from rusqlite 0.37 does not implement `Iterator`. The code uses a for-in loop pattern that requires the `IntoIterator` trait, which is not satisfied by `Rows<'_>`.

### Error Message
```
error[E0277]: Rows<'_> is not an iterator
   --> src/commands/decide_db.rs:159:21
    |
159 |         for row in rows {
    |                     ^^^^ Rows<'_> is not an iterator
    |
note: required by `for` loop

error[E0599]: the method `into_iter` exists for struct `Rows<'_>`, but its trait bounds were not satisfied
   --> src/commands/decide_db.rs:190:25
    |
190 |         for row in rows.into_iter() {
    |                         ^^^^^^^^^
```

### Root Cause
In rusqlite 0.37, `Rows` is a wrapper that provides `.next()` method but does not implement the standard Rust `Iterator` trait. The for-in loop syntax requires `IntoIterator`.

### Affected Code Locations
1. Line 159: `recent_decisions()` method — iterates over query results to build DecisionRecord vec
2. Line 190: `method_stats()` method — iterates over grouped statistics

### Fix Options

**Option A: Use .into_iter() explicitly (rusqlite 0.38+ pattern)**
```rust
for row in rows.into_iter() {
    let row = row?;
    // ...
}
```
*Note: This requires rusqlite version where Rows implements IntoIterator. May not work with 0.37.*

**Option B: Manual .next() loop (works with any rusqlite version)**
```rust
loop {
    let row = match rows.next() {
        Ok(Some(row)) => row,
        Ok(None) => break,
        Err(e) => return Err(Box::new(e)),
    };
    // ... process row ...
}
```

**Option C: Upgrade rusqlite to 0.38+**
```toml
rusqlite = { version = "0.38", features = ["bundled"] }
```
*Note: May have breaking API changes elsewhere in the codebase.*

### Recommended Fix
Use Option B (manual .next() loop) — it's version-agnostic and works with rusqlite 0.37 without requiring dependency upgrades or guessing about trait implementations.

## Tests Blocked
All 3 database tests are blocked:
- `store_and_retrieve_decision`
- `unique_observation_hash`  
- `method_stats`

The tests themselves are correct; they just can't run because the module doesn't compile.

## Phase 1 Milestone Status
- Milestone 1 (Cache Layer): ✅ COMPLETE — all tests passing
- Milestone 2 (Rule Engine): ✅ COMPLETE — all tests passing  
- Milestone 3 (Database Evidence Store): ✅ COMPLETE — iterator issue fixed, all tests passing
- Milestone 4 (Learning Loop Integration): ✅ COMPLETE — integrated with handle_evaluate, feedback/metrics subcommands added
- Milestone 5 (Confidence Drift Detection): PENDING
- Milestone 6 (Phase 1 Acceptance Tests): PENDING

## Files Modified in This Session
- `src/commands/decide_cache.rs` — new, cache layer with tests
- `src/commands/decide_rules.rs` — new, rule engine with tests
- `src/commands/decide_db.rs` — new, database evidence store (blocked)
- `src/commands/semantic.rs` — integrated cache and rule engine into handle_evaluate
- `src/commands/mod.rs` — declared all three new modules
- `src/lib.rs` — added `fresh` field to CliCommand::Decide enum variant
- `src/main.rs` — pass fresh parameter to handle_evaluate
- `specs/jev-cooperative-integrated-evolution.md` — Phase 1 evolution spec

## Next Steps for Fresh Session
1. ✅ Fix the iterator issue in `decide_db.rs` using Option B (manual .next() loop)
2. ✅ Run `cargo test -p crabjar decide_db` to verify all 3 tests pass
3. ✅ Proceed with Milestone 4: Learning Loop Integration
4. Complete remaining Phase 1 milestones (Milestone 5: Confidence Drift Detection, Milestone 6: Acceptance Tests)