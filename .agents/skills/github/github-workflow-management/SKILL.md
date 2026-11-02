---
name: github-workflow-management
description: |
  Use when managing GitHub Actions workflows, CI/CD pipeline configuration, and workflow troubleshooting. Trigger this when the user says "fix CI", "update workflow", or needs to manage GitHub Actions.
category: github
tags: [actions, ci-cd, workflows]
---

# GitHub Workflow Management

This skill provides comprehensive management of GitHub Actions workflows including creation, modification, troubleshooting, and optimization of CI/CD pipelines.

## Core Workflow

1. **Analyze existing workflows**: Review current workflow configuration
2. **Identify improvements**: Find optimization opportunities
3. **Modify workflows**: Update workflow files with best practices
4. **Test changes**: Verify workflow functionality
5. **Monitor execution**: Track workflow runs and failures

## Usage Instructions

### Input Requirements
* Repository URL or path
* Workflow operation (create, modify, troubleshoot)
* Specific workflow concerns (speed, reliability, coverage)

### Example Prompts
* `/github-workflow-management optimize CI pipeline for faster builds`
* `Fix failing GitHub Actions workflow on main branch`
* "Create a new workflow for automated testing on PRs"

## Reference Files
* `references/workflow-templates.md`: Templates for common workflows
* `references/ci-best-practices.md`: CI/CD best practices and optimization tips

## Bundled Scripts
* `scripts/check-workflow-status.sh`: Check current workflow run status