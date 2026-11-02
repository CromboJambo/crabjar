---
name: comprehensive-codebase-review
description: |
  Use when performing deep architectural review of complex codebases to identify structural issues, design flaws, and long-term maintenance concerns. Trigger this when the user says "review this codebase architecture" or needs comprehensive analysis.
category: software-development
tags: [code-review, architecture, analysis]
---

# Comprehensive Codebase Review

This skill provides methodology for deep architectural review of complex codebases, identifying structural issues, design flaws, coupling problems, and long-term maintenance concerns.

## Core Workflow

1. **Map architecture**: Understand overall system structure and component relationships
2. **Analyze coupling**: Identify tight coupling and dependency issues
3. **Evaluate design patterns**: Assess appropriateness of architectural patterns used
4. **Identify technical debt**: Find areas accumulating maintenance burden
5. **Recommend improvements**: Provide actionable suggestions for architectural improvements

## Usage Instructions

### Input Requirements
* Codebase path or repository URL
* Focus areas (optional): scalability, maintainability, testability

### Example Prompts
* `/comprehensive-codebase-review analyze architecture of this project`
* `Perform deep review of codebase structure and design`
* "What are the architectural issues in this codebase?"

## Reference Files
* `references/architecture-checklist.md`: Comprehensive architectural review checklist
* `references/design-smells.md`: Catalog of common architectural design smells

## Bundled Scripts
None — this skill provides review methodology and guidance.