# Crabjar Guard Persistence Bug — Storage Fixed, Approval Lookup Broken

## Status
Storage fix applied (interrupted entries now persist). Approval lookup still broken (looks in wrong table).

## Problem
When an agent action is interrupted by the guard system, `crabjar exec` generates an `interrupted_id`. The entry is now persisted to `interrupted_log`, but `crabjar guard approve --action-id <id>` fails with "not_found".

This breaks the core agent workflow: exec → interrupted → human approves → action runs.

## Root Cause Analysis
Two separate issues:

### Issue 1: Storage (FIXED)
In `src/main.rs` line 716, `GateConcierge` was created without a database connection. Without `.with_db()`, interrupted entries were in-memory only and lost when the command exited.

**Fix applied:** Added `.with_db(guard_db.clone())`. Interrupted entries now persist to `interrupted_log` table.

### Issue 2: Approval Lookup (REMAINING)
The approve command calls `update_action_status()` which queries the `action_requests` table. But interrupted entries are stored in `interrupted_log`, not `action_requests`. These are two separate tables with no connection.

This is NOT a timing issue — the entry exists immediately after exec. It's an **approval lookup mismatch**: approve looks for a record that was never created because the interrupted entry went to a different table.

## Data Model Mismatch
- `interrupted_log`: stores entries when guard interrupts an action (created by concierge.enforce())
- `action_requests`: stores entries awaiting approval/rejection (used by approve/reject commands)

When an action is interrupted, only `interrupted_log` gets a row. No corresponding `action_requests` row is created, so approve has nothing to find.

## Reproduction Steps
```bash
# 1. Run an exec command that will be interrupted
crabjar exec --container --command date --reason 'test'
# Returns: {"exec": {..., "interrupted_id": "<uuid>", "gate_result": "denied"}}

# 2. Verify entry is in database (after fix)
sqlite3 guard/guard.db 'SELECT id FROM interrupted_log ORDER BY logged_at DESC LIMIT 1;'
# Shows the UUID

# 3. Try to approve it
crabjar guard approve --action-id <uuid>
# Returns: {"guard": {"approve": {..., "status": "not_found"}}}
```

## Files Involved
- `src/main.rs` line 716 — GateConcierge initialization (fix applied)
- `guard/src/concierge.rs` lines 80-125 — enforce() method that creates interrupted entries
- `guard/src/guard_db_impl.rs` line 101 — persist_interrupted_log_entry() function
- `guard/src/guard_db_queries.rs` line 104 — update_action_status() used by approve command

## Next Steps for New Session
1. Determine correct design: should interrupted entries create action requests, or should approve look in interrupted_log?
2. Implement the fix
3. Test full exec → interrupt → approve → run flow
4. Verify with real agent loop (not just CLI commands)

## Environment
- Target: ftw3 (RTX 3070 Ti node)
- Crabjar version: v0.12.0
- Build: cargo build --release
- Deployed to: /opt/crabjar/crabjar
