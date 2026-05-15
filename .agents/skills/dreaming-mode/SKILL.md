---
name: dreaming-mode
description: |
  Use this skill whenever the user asks for a "dream", "post-mortem", "retrospective", or wants to "reflect" on recent work/errors. It enables a high-entropy, creative synthesis phase where the agent analyzes patterns, identifies structural shifts, and proposes updates to project documentation (like ${REPO_ROOT}/agent_config.md) without requiring perfect syntax or formal structure.
---

# Dreaming Mode

Dreaming Mode is a specialized state for high-level cognitive processing. It moves away from precise code execution toward pattern recognition, structural reflection, and creative synthesis.

## Core Workflow

1.  **Analyze Patterns**: Review the recent conversation history, error logs, or terminal outputs. Identify recurring themes (e.s., "structural instability", "dependency mismatch", "path confusion").
2.  **Identify Hallucinations vs. Truths**: Differentiate between valid architectural insights and transient-error-induced hallucinations. 
3.  **Synthesize Updates**: Formulate concrete, actionable updates for the agent's knowledge base (e.g., `${REPO_ROOT}/agent_config.md`, `project_map.md`).
4.  **Report back to User**: Present a structured "Dream Log" containing:
    *   **The Exploration**: What was being simulated or analyzed.
    *   **The Breakthroughs**: High-value insights or structural discoveries.
    *   **The Hallucinations**: Erroneous patterns that were identified and discarded.

## Operational Guidelines

*   **Embrace Imperfection**: In the "Dream" phase, do not worry about perfectly formatted code or exact file paths. Focus on the *concept*.
*   **High Entropy**: Allow for a broader, more associative way of thinking. Explore how a change in one crate affects others via the dependency graph.

*   **The Post-Dream Protocol**: Once a dream is complete, always conclude with a structured "Dream Log" (as shown above) to ensure the user can digest the insights without needing to understand the agent's raw thought process.

## Usage Examples

*   "Run a dream session on our recent struggle with the mirror-log move."
*   "Perform a post-mortem on the recent CI failures in the workspace."
*   "Dream about how we might restructure the ${REPO_ROOT} configuration for better scalability."

## Reference Material

- `references/dream-patterns.md` — pattern analysis guide, pattern types, output format

## Bundled Scripts

- `scripts/dream_synthesis.sh` — reflection synthesis from staged events