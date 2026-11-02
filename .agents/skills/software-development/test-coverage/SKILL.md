---
name: test-coverage
description: |
  Use when improving Rust crate test coverage by identifying low-coverage modules and generating meaningful tests that validate actual behavior. Trigger this when the user says "improve test coverage" or needs to add comprehensive tests.
category: software-development
tags: [testing, coverage, quality]
---

# Test Coverage Improvement

This skill provides methodology for improving Rust crate test coverage by systematically identifying low-coverage modules and generating meaningful tests that validate actual behavior rather than just exercising code paths.

## Core Workflow

1. **Measure current coverage**: Run coverage tools to identify gaps
2. **Prioritize by risk**: Focus on high-risk, low-coverage areas first
3. **Design behavioral tests**: Write tests that validate observable behavior
4. **Implement incrementally**: Add tests in logical batches with commits
5. **Verify improvement**: Re-measure coverage and validate test quality

## Usage Instructions

### Input Requirements
* Rust crate path
* Coverage threshold (optional)
* Focus areas (optional): specific modules or features

### Example Prompts
* `/test-coverage improve test coverage for this crate`
* `Identify low-coverage modules and add meaningful tests`
* "Help me get test coverage above 80%"

## Reference Files
* `references/coverage-tools.md`: Guide to Rust test coverage tools (tarpaulin, cargo-llvm-cov)
* `references/test-design-patterns.md`: Patterns for writing effective behavioral tests

## Bundled Scripts
None — this skill provides testing methodology and guidance.