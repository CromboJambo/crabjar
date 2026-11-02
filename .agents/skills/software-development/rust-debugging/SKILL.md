---
name: rust-debugging
description: |
  Use when debugging Rust abstraction layer bugs, trait implementation issues, lifetime problems, and ownership errors. Trigger this when the user says "debug this Rust code" or needs help with Rust-specific debugging challenges.
category: software-development
tags: [rust, debugging, traits]
---

# Rust Debugging

This skill provides specialized debugging guidance for Rust-specific challenges including abstraction layer bugs, trait implementation issues, lifetime problems, ownership errors, and borrow checker confusion.

## Core Workflow

1. **Identify error category**: Classify as compile-time vs runtime, ownership vs type system
2. **Read compiler messages**: Interpret Rust's detailed error messages and suggestions
3. **Trace data flow**: Follow ownership and borrowing through function calls
4. **Test minimal reproduction**: Create smallest possible example that demonstrates issue
5. **Apply fix patterns**: Use established patterns for common Rust debugging scenarios

## Usage Instructions

### Input Requirements
* Rust code with issue (file path or snippet)
* Error messages (if any)
* Expected vs actual behavior

### Example Prompts
* `/rust-debugging fix this trait implementation error`
* `Debug lifetime issues in this Rust code`
* "Help me understand why the borrow checker is complaining"

## Reference Files
* `references/debugging-patterns.md`: Common Rust debugging patterns and solutions
* `references/error-message-guide.md`: Guide to interpreting Rust compiler errors

## Bundled Scripts
None — this skill provides debugging methodology and guidance.