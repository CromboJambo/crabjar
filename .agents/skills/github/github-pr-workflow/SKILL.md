---
name: github-pr-workflow
description: |
  Use when managing the complete GitHub PR lifecycle: branching, committing, opening PRs, monitoring CI, and merging. Trigger this when the user says "open a PR", "create a pull request", or needs to manage PR workflow end-to-end.
category: github
tags: [pull-requests, workflow, ci]
---

# GitHub PR Workflow

This skill manages the complete GitHub pull request lifecycle from branch creation through merge, including CI monitoring and status reporting.

## Core Workflow

1. **Create feature branch**: Branch from appropriate base (main/develop)
2. **Make changes**: Implement requested features or fixes
3. **Commit with conventional messages**: Use descriptive commit messages
4. **Push to remote**: Push branch to GitHub repository
5. **Open PR**: Create pull request with proper title, description, and labels
6. **Monitor CI**: Watch for CI status and address failures
7. **Request review** (if needed): Assign reviewers based on context
8. **Merge**: Merge when all checks pass and approvals received

## Usage Instructions

### Input Requirements
* Repository URL or path
* Base branch (default: main)
* Feature description (for PR title/description)
* Reviewer preferences (optional)

### Example Prompts
* `/github-pr-workflow open PR for new feature branch feature/my-feature`
* `Create a pull request from develop to main with title "Fix login bug"`
* "Monitor CI status on my PR and report when it's ready to merge"

## Reference Files
* `references/pr-template.md`: Recommended PR description template
* `references/commit-conventions.md`: Conventional commit message guidelines

## Bundled Scripts
* `scripts/check-pr-status.sh`: Check current PR status (CI, reviews, conflicts)