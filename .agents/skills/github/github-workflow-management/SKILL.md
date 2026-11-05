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

## Pitfalls

- **Check for existing workflow files before adding jobs**: Before adding a new job to an existing workflow file, list `.github/workflows/` to check if a separate workflow file already handles that concern. Adding a `dependency-audit` job to rust.yml when dependency-audit.yml already exists creates duplicate runs and confusing CI output. Separate workflow files often exist for independent concerns (security audits, scheduled tasks, manual dispatch) that should not run on every PR.

- **cargo-audit has no --workspace flag**: Do not write `cargo audit --workspace` in workflows — cargo-audit does not accept a `--workspace` flag and will fail with "unexpected argument". Run just `cargo audit` (optionally with `--deny warnings`) to audit all workspace dependencies.

## Reference Files
* `references/workflow-templates.md`: Templates for common workflows
* `references/ci-best-practices.md`: CI/CD best practices and optimization tips

## Bundled Scripts
* `scripts/check-workflow-status.sh`: Check current workflow run status