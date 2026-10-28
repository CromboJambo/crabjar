# DB Consolidation: Centralize Scattered SQLite Databases

## Problem

Multiple copies of similar SQLite databases are scattered across the home directory, project directories, and dotfiles. This makes it difficult to track which is authoritative, risks stale data being used, and complicates backup/restore operations.

**Current state:**
- `guard.db`: 19 copies found (home dir, projects, dotfiles, .ssh)
- `knowledge.db`: 12 copies found
- `state-docs.db`: 5 copies found
- `mirror.db`: 1 copy in home dir (orphaned — no code reference)

## Goal

Consolidate all crabjar/Hermes/mirror-lab databases into their proper config directories:
- `.crabjar/dbs/` for guard, knowledge, and state-docs databases
- `.mirror-lab/` for mirror database
- `.hermes/` already properly organized (no action needed)

## Migration Strategy

**Phase 1 — Symlinks (non-destructive):** ✓ COMPLETE
- Create target directories: `.crabjar/dbs/`, `.mirror-lab/`
- Symlink home dir DBs to new locations
- Verify all symlinks work with SQLite read/write tests

**Phase 2 — Production verification:** 
- Use crabjar and Hermes normally for several days
- Monitor for any "file not found" or database lock errors
- Confirm all tools access DBs through symlinks correctly

**Phase 3 — Actual migration (destructive):**
- Move real DB files from home dir to new locations
- Update symlinks to point to new locations
- Test all crabjar/Hermes operations after move

**Phase 4 — Duplicate cleanup (destructive):**
- Delete all scattered project-level copies
- Delete dotfiles and .ssh duplicates
- Verify no tools reference deleted paths

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Tool writes to old path, creates new file | Low (symlinks prevent) | Medium | Symlinks redirect all writes |
| WAL/shm file conflicts | Low | High | SQLite handles symlinks correctly |
| Backup scripts miss new location | Medium | Medium | Update backup config after Phase 3 |
| Stale data used from duplicate | High (current state) | High | Cleanup in Phase 4 eliminates risk |

## Acceptance Criteria

- [ ] All crabjar commands work with DBs at new locations
- [ ] Hermes operations unaffected
- [ ] No "file not found" errors after migration
- [ ] Backup scripts updated to include new locations
- [ ] All duplicate copies deleted and verified gone
- [ ] Home directory contains no loose .db files

## Rollback Plan

If issues arise after Phase 3:
```bash
# Restore from home dir copies (if kept) or backups
cp /home/crombo/guard.db.bak /home/crombo/.crabjar/dbs/guard.db
# Update symlinks back to old locations if needed
```

## Files Affected

**Symlinked (Phase 1):**
- `/home/crombo/guard.db` → `.crabjar/dbs/guard.db`
- `/home/crombo/knowledge.db` → `.crabjar/dbs/knowledge.db`
- `/home/crombo/state-docs.db` → `.crabjar/dbs/state-docs.db`
- `/home/crombo/mirror.db` → `.mirror-lab/mirror.db`

**To be deleted (Phase 4):**
- All `guard.db` and `knowledge.db` copies in `/home/crombo/projects/`
- All DB copies in `/home/crombo/.dotfiles/`
- `/home/crombo/.ssh/guard.db` and `.ssh/knowledge.db`
