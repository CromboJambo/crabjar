---
skill_name: session-handoff
description: Produces a structured handoff document for a fresh agent session, detailing built work, rejected approaches, and remaining tasks to prevent regression or re-guessing.
keywords: [handoff, state management, debugging, session]
trigger: on_manual_call # Or specify other triggers if applicable (e.g., completion)
---

## Core principles

### 1. Rejected decisions matter more than built ones
A handoff that only lists "what was built" is incomplete. The next agent will re-try rejected approaches unless explicitly told what was rejected and why. Always include:
- What was attempted and rejected
- Why it was rejected (not just "it didn't work")
- What the correct path would be (reference source, not a guess)

### 2. Failures must be specific, not vague
"Doesn't match the converter's expectations" tells the next agent there's a mismatch but not where. If you know the actual vs. expected difference, state it. If you don't, say "root cause unknown — investigate before fixing." Ambiguity risks the agent guessing.

### 3. "May need updating" resolves to a task or disappears
Leaving "may need updating" just creates a distraction. Either the doc is stale and needs updating (make it a task with what's missing), or it's fine (remove it).

### 4. Priority signal is mandatory
Three remaining items with no indication of which one to start forces the next agent to infer. Given that the integration test failure is blocking a clean test suite, that's probably item 1 — but say so explicitly.

### 5. Files to review must have a purpose
"Review X" without a "because you'll need to do Y" gives the agent no orientation. Either attach the purpose ("review gguf_model_loader.rs — this is the new file, verify exports are wired correctly in lib.rs") or fold the files into the relevant remaining work items.

### 6. Contradictions in the codebase must be flagged
If a function maps type A to output B, but the implementation for type A doesn't actually produce B, flag that contradiction explicitly. The next agent will trust the mapping and waste cycles debugging.

### 7. Output must be clean — no internal reasoning
Strip all chain-of-thought, planning language, and internal notes from the output file. The handoff is a document for the next agent, not a transcript of how you got there. Never include "Thought:", "Let me...", "I should...", or similar meta-commentary.

### 8. Remove tasks resolved by checks that already passed
If a remaining work item would be verified by `cargo check`, `cargo test`, or a similar command that already passed, remove it. Listing "verify X compiles" when compilation already passed is noise, not work. A task is only legitimate remaining work if it requires new actions, not verification of existing state.

## Handoff file structure

---

````markdown
# Session handoff: <project> — <progress summary>

## What was built today

### 1. <crate/module> — ✅ complete
- Key exports and their purpose
- Test count and status
- One-sentence summary of what it does

### 2. <crate/module> — ✅ complete
...

### 3. <crate/module> — ✅ complete
...

### 4. Compilation status
- `cargo check -p <crate>` — ✅ passes / ❌ fails (error summary)

## What was NOT done (rejected)

- **<approach>**: Attempted because... Rejected because... Correct path: <reference source>.

### ⚠️ Critical: <specific code issue>
If there's a contradiction, bug, or misleading behavior in the codebase, flag it here with:
- What the code claims to do
- What it actually does
- What the next agent should do about it

## Remaining work (in priority order)

### 1. <highest priority task> (blocking — <reason>)
<Specific details: actual vs expected, root cause if known, fix direction>

### 2. <second priority task>
<Specific details>

### 3. <third priority task> (optional — resolve later)
<Specific details>

## Key implementation decisions
- <decision>: <rationale> — <what to use instead>
- <decision>: <rationale> — <what to use instead>

## What not to do
- <action>: <reason>
- <action>: <reason>

## Environment
- <constraint>: <value>
- <constraint>: <value>
```

## Output format
Write the handoff file to the specified path. If no path is given, write to `state-docs/handoff-<date>.md` in the project root.
