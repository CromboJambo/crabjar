# Profile Skill Update Suggestions

## Problem

Profiles should be able to suggest new skills or improvements to existing skills that benefit the entire fleet, not just the individual profile. Currently, there's no mechanism for upward feedback from profiles to the global skill set.

## Goals

1. Profiles can propose new skills or improvements
2. Suggestions go through a review process (automated or human)
3. Approved suggestions are merged into the global skill set
4. All profiles automatically see updated skills

## Architecture

### Suggestion Submission
```bash
# Profile creates a suggestion via kanban task
hermes kanban create \
    --board crabjar-skill-suggestions \
    --assignee crabjar-conductor \
    --body "Proposed skill: example-skill\nDescription: This skill does X..." \
    "New skill proposal: example-skill"
```

### Suggestion Workflow
1. **Submit**: Any profile creates a kanban task on `crabjar-skill-suggestions` board
2. **Review**: Conductor (or human) reviews the suggestion
3. **Approve/Reject**: Task is marked done with outcome metadata
4. **Promote**: Approved skills are added to global skill set via sync script
5. **Notify**: All profiles see the new/updated skill on next sync

### Suggestion Format
Each suggestion should include:
- Skill name
- Description of what it does
- Use case (when to use it)
- Example invocation
- Any dependencies or prerequisites

## Implementation

### Phase 1: Kanban-based Suggestions
- Create `crabjar-skill-suggestions` kanban board
- Conductor profile monitors and reviews suggestions
- Manual promotion via sync script after review

### Phase 2: Automated Review
- Use pesti-server to evaluate suggestion quality
- Auto-approve suggestions that meet quality thresholds
- Flag low-quality suggestions for human review

### Phase 3: Skill Versioning
- Track skill versions across profiles
- Allow rollback if a promoted skill causes issues
- Maintain audit log of skill promotions

## Testing

1. Submit a test skill suggestion from one profile
2. Verify conductor receives and can review it
3. Approve the suggestion and verify it's promoted to global
4. Verify all profiles see the new skill after sync
