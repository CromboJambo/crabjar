---
name: rust-triage
description: |
  Use when performing systematic triage of Rust projects to distinguish build errors, type errors, logic bugs, and dependency issues. Trigger this when the user says "triage this Rust project" or needs structured debugging approach for Rust.
category: software-development
tags: [rust, debugging, triage]
---

# Rust Project Triage

This skill provides systematic triage methodology for Rust projects, helping distinguish between build errors, type errors, logic bugs, and dependency issues through structured investigation.

## Core Workflow

1. **Categorize error**: Determine if issue is build-time, compile-time, or runtime
2. **Check dependencies**: Verify cargo dependencies and version conflicts
3. **Analyze compiler output**: Interpret Rust compiler errors and warnings
4. **Test incrementally**: Isolate problematic code through targeted testing
5. **Recommend fixes**: Provide specific remediation steps based on error category

## Usage Instructions

### Input Requirements
* Rust project path
* Error messages or symptoms
* Recent changes (if applicable)

### Example Prompts
* `/rust-triage diagnose build failure in this Rust project`
* `Triage type errors in this Rust codebase`
* "Help me systematically debug this Rust compilation issue"

## Reference Files
* `references/error-categories.md`: Classification of common Rust error types
* `references/triage-flowchart.md`: Decision tree for Rust debugging

## Bundled Scripts
None — this skill provides triage methodology and guidance.