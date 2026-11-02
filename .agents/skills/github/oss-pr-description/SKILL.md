---
name: oss-pr-description
description: |
  Use when writing PR descriptions for open-source maintainers that focus on what+why only, avoiding implementation details and conversation noise. Trigger this when the user says "write OSS PR description" or needs to submit PRs to large OSS projects.
category: github
tags: [pull-requests, open-source, documentation]
---

# OSS PR Description Writing

This skill crafts professional pull request descriptions tailored for open-source maintainers, focusing on what changed and why while avoiding implementation details and conversation noise.

## Core Workflow

1. **Analyze changes**: Review diff to understand scope of changes
2. **Determine audience**: Identify target project's maintainer style
3. **Write description**: Focus on what+why, not how
4. **Include verification**: Document testing and validation performed
5. **Format appropriately**: Match project's PR conventions

## Usage Instructions

### Input Requirements
* Diff or branch to review
* Target repository context (optional)
* Specific concerns maintainers might have (optional)

### Example Prompts
* `/oss-pr-description write description for this fix`
* `Craft a professional PR description for submission to upstream`
* "Write an OSS-style PR description focusing on what and why"

## Reference Files
* `references/oss-pr-examples.md`: Examples of well-written OSS PR descriptions
* `references/maintainer-perspectives.md`: What OSS maintainers look for in PRs

## Bundled Scripts
* `scripts/generate-pr-description.sh`: Generate PR description from diff