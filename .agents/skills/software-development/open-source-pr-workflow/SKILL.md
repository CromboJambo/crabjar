---
name: open-source-pr-workflow
description: |
  Use when submitting pull requests to large open-source projects, avoiding 'poisoning' the PR with unnecessary changes, and following project conventions. Trigger this when the user says "submit PR to OSS project" or needs guidance on OSS contribution workflow.
category: software-development
tags: [open-source, pull-requests, contribution]
---

# Open Source PR Workflow

This skill provides methodology for successfully submitting pull requests to large open-source projects, including avoiding common mistakes that 'poison' PRs and following project-specific conventions.

## Core Workflow

1. **Study project conventions**: Review CONTRIBUTING.md, existing PRs, and maintainer preferences
2. **Scope changes appropriately**: Keep PR focused on single logical change
3. **Avoid poisoning changes**: Don't include unrelated refactoring or style changes
4. **Write clear description**: Explain what+why without implementation noise
5. **Respond to feedback**: Address reviewer comments professionally and promptly

## Usage Instructions

### Input Requirements
* Target repository URL
* Change description
* Specific project conventions (if known)

### Example Prompts
* `/open-source-pr-workflow submit this fix to upstream`
* `Guide me through contributing to this OSS project`
* "How do I submit a PR that maintainers will actually merge?"

## Reference Files
* `references/oss-contribution-guide.md`: Comprehensive guide to OSS contributions
* `references/common-pr-mistakes.md`: Catalog of common PR mistakes to avoid

## Bundled Scripts
None — this skill provides workflow methodology and guidance.