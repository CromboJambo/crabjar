# Profile Skill Inheritance and Sync

## Problem

Each crabjar fleet profile (conductor, workers, supervisors) currently has access to the global skill set via `~/.hermes/skills/`. However:

1. **Earned skills are local**: When a profile creates a new skill via `skill_manage`, it's stored in that profile's local `skills/` directory and not shared with other profiles.
2. **No upward feedback**: Profiles cannot suggest improvements or new skills back to the global set for all fleet members to use.
3. **Drift risk**: Over time, different profiles will have divergent skill sets based on their individual learning.

## Goals

1. **Shared earned skills**: Skills learned by any profile should be available to all fleet profiles.
2. **Automatic sync**: When global skills are updated, all profiles see the changes immediately.
3. **Update suggestions**: Profiles can propose new skills or improvements that get promoted to the global set after review.

## Architecture

### Current State
```
~/.hermes/skills/           # Global skills (218) - all profiles read these
~/.hermes/profiles/X/skills/  # Profile-local skills - only profile X sees these
```

### Proposed State
```
~/.hermes/skills/           # Global skills - all profiles read these
~/.hermes/profiles/X/skills/  # Profile-local overrides (optional)
~/.hermes/profiles/X/skill-suggestions/  # Skills proposed for global promotion
```

## Implementation Plan

### Phase 1: Shared Skill Creation
- When a profile creates a skill via `skill_manage`, write to both local and global directories
- Add a flag or config option to control this behavior (`skills.shared_creation: true`)
- Test with one supervisor profile first

### Phase 2: Skill Sync Daemon
- Create a periodic sync job that:
  - Checks for new/updated skills in any profile's local directory
  - Promotes approved skills to the global set
  - Removes obsolete skills from global set if no profile uses them
- Run as a cron job or hermes daemon task

### Phase 3: Update Suggestions Workflow
- Profiles can submit skill suggestions via kanban tasks
- Suggestions go through review (by conductor or human)
- Approved suggestions are merged into global skill set
- All profiles automatically see updated skills on next run

## Configuration

Add to each profile's `config.yaml`:
```yaml
skills:
  shared_creation: true      # Write new skills to global directory
  sync_interval: "5m"        # How often to check for global updates
  suggest_to_global: true    # Enable suggestion workflow
```

## Testing

1. Create a skill in one profile, verify it's available in others
2. Update a global skill, verify all profiles see the change
3. Submit a skill suggestion, verify it goes through review workflow
4. Test rollback if a promoted skill causes issues

## Rollback Plan

- Keep profile-local skill directories as fallback
- Version skills to allow rollback to previous versions
- Maintain a skill promotion audit log
