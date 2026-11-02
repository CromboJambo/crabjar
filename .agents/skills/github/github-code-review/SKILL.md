---
name: github-code-review
description: |
  Use when reviewing pull requests, examining diffs, and providing inline comments via gh CLI or GitHub REST API. Trigger this when the user says "review this PR", "check the diff", or needs to provide code review feedback.
category: github
tags: [code-review, pull-requests, feedback]
---

# GitHub Code Review

This skill provides structured code review workflows for examining pull requests, analyzing diffs, and providing actionable feedback through inline comments.

## Core Workflow

1. **Identify PR**: Get PR number or URL from user
2. **Fetch diff**: Retrieve complete diff using gh CLI or REST API
3. **Analyze changes**: Review code quality, logic, and potential issues
4. **Generate comments**: Create structured feedback with line references
5. **Submit review**: Post inline comments and overall review status

## Usage Instructions

### Input Requirements
* PR number or URL
* Review focus (optional): general, security, performance, style
* Severity level for findings (optional)

### Example Prompts
* `/github-code-review review PR #42 with focus on security`
* `Examine the diff on this branch and provide feedback`
* "Post a code review comment on line 150 about error handling"

## Reference Files
* `references/review-checklist.md`: Comprehensive code review checklist
* `references/comment-templates.md`: Templates for common review comments

## Bundled Scripts
* `scripts/submit-review.sh`: Submit structured review with inline comments