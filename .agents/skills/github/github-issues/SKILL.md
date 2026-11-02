---
name: github-issues
description: |
  Use when creating, triaging, labeling, and assigning GitHub issues via gh CLI or REST API. Trigger this when the user says "create an issue", "triage issues", or needs to manage issue workflow.
category: github
tags: [issues, triage, labels]
---

# GitHub Issue Management

This skill provides comprehensive issue management workflows including creation, triage, labeling, assignment, and lifecycle management.

## Core Workflow

1. **Create issues**: Generate well-structured issue templates with all required fields
2. **Triage incoming issues**: Categorize by type, priority, and assignee
3. **Apply labels**: Use consistent labeling scheme for filtering and reporting
4. **Assign ownership**: Route issues to appropriate team members
5. **Track progress**: Monitor issue status and lifecycle

## Usage Instructions

### Input Requirements
* Repository URL or path
* Issue type (bug, feature, enhancement, documentation)
* Priority level (optional)
* Assignee information (optional)

### Example Prompts
* `/github-issues create bug report for login failure on mobile`
* `Triage all open issues and assign appropriate labels`
* "Create an issue template for this repository"

## Reference Files
* `references/issue-templates.md`: Templates for different issue types
* `references/label-scheme.md`: Recommended labeling conventions

## Bundled Scripts
* `scripts/triage-issues.sh`: Automated issue triage workflow