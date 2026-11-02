---
name: github-issue-to-pr
description: |
  Use when carrying a GitHub issue through to a verified PR with honest CI state reporting. Trigger this when the user says "fix this issue and open a PR", "implement issue #123", or needs end-to-end issue resolution workflow.
category: github
tags: [issues, pull-requests, workflow]
---

# GitHub Issue to PR Workflow

This skill carries a GitHub issue through the complete lifecycle from identification to verified PR with honest CI state reporting and proper documentation.

## Core Workflow

1. **Analyze issue**: Understand requirements, scope, and acceptance criteria
2. **Plan implementation**: Break down into manageable tasks
3. **Create feature branch**: Branch from appropriate base
4. **Implement solution**: Code changes with tests
5. **Verify locally**: Run tests and validate functionality
6. **Open PR**: Create PR referencing the original issue
7. **Monitor CI**: Report honest CI state (passing, failing, pending)
8. **Close issue**: Reference PR in issue closure

## Usage Instructions

### Input Requirements
* Issue number or URL
* Repository context
* Implementation constraints (optional)

### Example Prompts
* `/github-issue-to-pr fix issue #42 and open verified PR`
* `Implement the feature described in this issue with proper testing`
* "Take this bug report from issue to resolved PR"

## Reference Files
* `references/issue-analysis-template.md`: Template for analyzing issues
* `references/pr-verification-checklist.md`: Checklist for PR verification

## Bundled Scripts
* `scripts/create-issue-pr.sh`: Automated issue-to-PR workflow