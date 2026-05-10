---
name: structure-auditor
description: |
  Use this skill whenever you need to verify that the actual filesystem structure matches the documented architecture in `project_map.md`. It automates the discovery process by scanning the workspace and comparing it against the "Open Book" index, identifying any moved, renamed, or missing crates or modules. Trigger this when a command fails due to a "path not found" error, or during post-refactor audits.
---

# Structure Auditor

The Structure Auditor is a diagnostic tool designed to detect "structural drift"—the discrepancy between the agent's mental model (defined in `project_map.md`) and the reality of the filesystem.

## Core Workflow

1.  **Load Reference**: Read the current `crabjar/project_map.md` to extract the list of expected paths and responsance/responsibilities.
2.  **Filesystem Scan**: Perform a recursive scan of the workspace (or specific sub-paths) using `list_directory` or `find_path`.
3.  **Discrepancy Analysis**: Compare the observed tree against the map. 
    *   **Missing**: A path exists in the map but not on disk.
    *   **Unexpected**: A directory exists on disk but is not documented in the map.
    *   **Misaligned**: A directory exists where a different one was expected (e.g., `mirror-log` inside `mirror-kernel` instead of root).
4.  **Report Generation**: Output a "Drift Report" highlighting exactly what needs to be updated in the documentation or corrected in the filesystem.

## Usage Instructions

### Input Requirements
*   The path to the `project_map.md` file (defaults to `crabjar/project_map.md`).
*   (Optional) A target root directory to scan (defaults to the workspace root).

### Example Prompt
*   `/structure-auditor verify my project map against the current filesystem`
*   `$structure-auditor audit mirror-kernel/`

## Reference Files
*   `crabjar/project_map.md`: The primary source of truth for structural alignment.
*   `crabjar/agent_config.md`: For understanding the "Discovery" and "Verification" protocols.
*   `references/project-map-format.md`: project_map format conventions and audit rules.

## Bundled Scripts
*   `scripts/fs_audit.sh`: filesystem vs project_map discrepancy analysis

## Skill Capabilities
*   **Path Resolution**: Resolves relative paths to absolute filesystem locations.
*   **Tree Diffing**: Performs a high-speed comparison between two directory trees.
implements structural integrity checks for the agent's navigation logic.