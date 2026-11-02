---
name: plan
description: |
  Use when writing a markdown plan to .hermes/plans/ directory without executing it. Trigger this when the user says "write a plan", "create an implementation plan", or needs to document approach before execution.
category: software-development
tags: [planning, documentation]
---

# Implementation Planning

This skill writes structured markdown plans to the .hermes/plans/ directory without executing them, providing clear documentation of intended approach, steps, and acceptance criteria.

## Core Workflow

1. **Understand requirements**: Clarify scope and objectives with user
2. **Research context**: Review existing code, architecture, and constraints
3. **Draft plan**: Write structured markdown with sections for overview, steps, risks, and acceptance criteria
4. **Save to plans directory**: Store in .hermes/plans/ with descriptive filename
5. **Present to user**: Share plan for review before execution

## Usage Instructions

### Input Requirements
* Task or feature description
* Scope constraints (optional)
* Timeline expectations (optional)

### Example Prompts
* `/plan write implementation plan for new authentication system`
* `Create a plan for refactoring the database layer`
* "Write a detailed plan before making changes"

## Reference Files
* `references/plan-template.md`: Standard plan document template
* `references/acceptance-criteria-guide.md`: Guide to writing clear acceptance criteria

## Bundled Scripts
None — this skill produces documentation, not executable scripts.