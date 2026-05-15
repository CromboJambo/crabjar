---
name: git-reflector
description: |
  Use this skill whenever a new reflection or event has been staged and needs to be formally committed to the Git decision layer. It automs the process of creating a DecisionBlob, writing it to the `mirror_decisions` directory within the Git repository, and performing a `git commit --amend` to ensure the decision is atomically linked to the filesystem state. Trigger this when the user says "commit decision", "finalize reflection", or provides a path to a staged event.
---

# Git Reflector

This skill automs the integration of unstructured reflections into the immutable Git-based decision layer. It bridges the gap between `mirror-log` (the append-only ledger) and the `git_decision_layer` (the permanent audit trail).

## Core Workflow

1.  **Identify Target**: Locate the staged event or reflection file (typically in a `staging/` directory or passed via path).
2.  **Extract Metadata**: Read the content, source, and metadata from the target file/event.
3.  **Construct DecisionBlob**: Prepare a `DecisionBlob` containing:
    *   The `selected_reflection` data.
    *   Associated `context_tags`.
    *   The `kernel_name` responsible for the decision.
    *   A human-readable `reason` (derived from the event or user input).
4.  **Execute Git Operations**:
    *   Write the JSON-encoded `DecisionBlob` to the `mirror_decisions/` directory within the repository.
    *   Run `git add <path_to_new_decision_json>`.
    *   Run `git commit --amend --no-dirty` (or a new commit if preferred) to finalize the decision in the history.
5.  **Verification**: Verify that the commit hash is captured and can be retrieved via `git rev-parse HEAD`.

## Usage Instructions

### Input Requirements
*   A path to a staged event file OR a reference to an existing event ID in the ledger.
*   (Optional) A reason string if the decision isn't self-explanatory.

### Example Prompt
*   `/git-reflector commit the staging event from ./staging/event_123.json with reason "Validated via manual review"`
*   `$git-reflector: finalize all staged reflections in the directory`

## Reference Files
*   `mirror-kernel/src/git_decision_layer/mod.rs`: For implementation details of `GitDecisionLayer` (future crate)
*   `mirror-kernel/src/lib.rs`: For `DecisionBlob` and `MirrorEvent` structures (future crate)
*   `references/decision-blob-schema.md`: For DecisionBlob field definitions and constraints

## Bundled Scripts
*   `scripts/decision_blob.sh`: Create DecisionBlob JSON from staged event
*   `scripts/git_amend.sh`: Git commit amend with DecisionBlob