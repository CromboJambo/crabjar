# Dream Pattern Analysis Guide

## Analysis steps

1. **Analyze patterns**: identify recurring errors or structural shifts
2. **Update knowledge**: synthesize into agent_config.md or project_map.md
3. **Summarize changes**: concise bullet list of proposed updates

## Pattern types

- **Recurring errors**: same tool call failure, same path missing
- **Structural shifts**: crate moved, directory renamed, dependency changed
- **Config drift**: thresholds updated, baselines changed
- **Knowledge gaps**: missing state-docs, stale references

## Output format

- **Patterns Identified**: list of recurring patterns
- **Structural Shifts**: list of architectural changes
- **Proposed Updates**: list of updates to agent_config.md or project_map.md

## Constraints

- No deeply nested references in output
- Keep synthesis concise (< 50 lines)
- Every update must surface what assumptions were made
