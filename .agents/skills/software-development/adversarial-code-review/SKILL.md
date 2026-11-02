---
name: adversarial-code-review
description: |
  Use when performing structured code review focused on finding real logic bugs, silent failures, and edge cases through adversarial questioning of assumptions. Trigger this when the user says "adversarially review this code" or needs deep bug-finding review.
category: software-development
tags: [code-review, debugging, quality]
---

# Adversarial Code Review

This skill provides a structured methodology for finding real logic bugs, silent failures, and edge cases by adversarially questioning assumptions in code rather than just checking style or conventions.

## Core Workflow

1. **Identify assumptions**: Find implicit assumptions the code makes about inputs, state, and environment
2. **Challenge each assumption**: Ask "what if this is wrong?" for each identified assumption
3. **Trace failure modes**: For each challenged assumption, trace what happens when it's violated
4. **Document findings**: Report potential bugs with reproduction scenarios

## Usage Instructions

### Input Requirements
* Code to review (file path or diff)
* Focus areas (optional): null handling, race conditions, boundary values

### Example Prompts
* `/adversarial-code-review find logic bugs in this function`
* `Adversarially review this code for silent failures`
* "What assumptions is this code making that could be wrong?"

## Reference Files
* `references/assumption-categories.md`: Categories of common code assumptions to challenge
* `references/failure-mode-patterns.md`: Patterns of silent failures and edge cases

## Bundled Scripts
None — this skill provides review methodology and guidance.