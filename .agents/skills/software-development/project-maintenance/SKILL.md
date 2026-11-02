---
name: project-maintenance
description: |
  Use when cleaning up clutter and historical noise in a repository, removing unused code, outdated documentation, and stale dependencies. Trigger this when the user says "clean up this repo" or needs maintenance work.
category: software-development
tags: [maintenance, cleanup, refactoring]
---

# Project Maintenance

This skill provides methodology for cleaning up clutter and historical noise in repositories, including removing unused code, outdated documentation, stale dependencies, and improving overall project hygiene.

## Core Workflow

1. **Audit repository**: Identify unused files, deprecated code, and outdated content
2. **Analyze dependencies**: Check for unused or outdated package dependencies
3. **Review documentation**: Find outdated READMEs, docs, and comments
4. **Clean incrementally**: Remove clutter in logical batches with commits
5. **Verify functionality**: Ensure cleanup doesn't break existing functionality

## Usage Instructions

### Input Requirements
* Repository path or URL
* Focus areas (optional): dependencies, documentation, dead code

### Example Prompts
* `/project-maintenance clean up this repository`
* `Remove unused files and outdated docs from this project`
* "Do maintenance cleanup on this codebase"

## Reference Files
* `references/maintenance-checklist.md`: Comprehensive maintenance task checklist
* `references/cleanup-strategies.md`: Strategies for safe repository cleanup

## Bundled Scripts
None — this skill provides maintenance methodology and guidance.