---
name: bounded-agent-execution
description: |
  Use whenever the user wants to run multi-step agent work that could bloat context,
  needs approval gating, or requires bounded result retrieval — apply DeepSeek-TUI
  patterns for sub-agent fan-out, bounded transcript management, and execution gating.
  Trigger when the user mentions parallel work, sub-agents, bounded retrieval, context
  compaction, approval gates, sandbox execution, multi-step workflows, or asks to
  "delegate work", "split tasks", "keep context lean", or "fan out parallel tasks".
---

# Bounded Agent Execution

Patterns from DeepSeek-TUI v0.8.33 for managing multi-step agent work without
context degradation. Combines sub-agent fan-out, bounded result retrieval,
session compaction guardrails, and approval policy gating.

---

## Core Principles

### Context Degradation

Long sequential sessions accumulate every message and tool result with no automatic
pruning. Session saves serialize the entire bloated array to disk. This causes
degradation and crash. To survive multi-hour work:

1. Delegate independent work early — open one focused sub-agent per task
2. Batch independent reads/searches — fire them together, summarize evidence
3. Compact aggressively — suggest compaction at 60% context usage, not 80%
4. Reassess after 3 sequential parent turns — split work into sub-agents if same
   feature still needs broad reading
5. Use bounded result retrieval for large outputs — keep parent transcript lean

### Operating Model

Keep the parent session lean. Put large-context inspection in sub-agents, parallel
side work in sub-agents, full outputs behind handles/detail pagers, and only the
decision-quality summary in the main thread. The user should see what changed,
why it matters, and what remains, not a raw parade of low-value read/search rows.

---

## Sub-Agent Fan-Out

### Launch

Open one focused sub-agent per independent task. Each gets fresh context and tool
registry and runs independently. The parent keeps working.

```bash
# Open sub-agent for task
agent_open <task-description>
```

### Fan-Out Strategy

- **Parallel investigation**: When you need to understand 3+ independent files or
  modules, spawn one read-only sub-agent per target
- **Parallel implementation**: After a plan is laid out, spawn one sub-agent per
  independent leaf task
- **Solo tasks**: A single read, a single search, a focused question — do these
  yourself. Spawning has overhead
- **Sequential work**: If step B depends on step A's output, run A yourself, then
  decide whether to spawn B based on what A found

### Completion

When a sub-agent finishes, the runtime delivers a structured completion event with:
- summary field (decision-quality synthesis)
- evidence list (concrete artifact citations)
- execution metrics (runtime, status, failures)

Read the summary first. Call bounded retrieval only when the summary is insufficient
or the child needs another assignment.

### Constraints

- Keep at most 5 sub-agents running
- After spawning agents, keep doing non-overlapping local coordination work
- Use bounded retrieval for large transcripts — not repeated reads into parent
- Do not paste full logs into the parent — store as artifacts or summarize via RLM

---

## Bounded Result Retrieval

### Handle Pattern

Large structured results (sub-agent transcripts, tool artifacts, analysis finals)
return typed handles with slice, range, count, and projection capabilities. The
model reads back only what it needs.

### When to Use

- Sub-agent transcripts exceeding 4096 tokens
- Tool outputs exceeding per-tool thresholds
- Analysis finals with multi-document corpus
- Large search results with many entries

### When NOT to Use

- A single short file you can read directly
- A simple classification on 3 items
- Interactive iterative exploration

---

## Session Compaction Guardrails

### Capacity Controller

Runtime pressure guardrails that monitor context usage and trigger compaction before
dangerous growth.

| Threshold | Action |
|---|---|
| 60% context | Suggest compaction |
| 80% context | Compaction is critical |
| > 80% context | Session degradation imminent |

### Compaction Thresholds

Based on active request input estimate, not lifetime summed usage.

| Level | Threshold | Purpose |
|---|---|---|
| L1 | 192000 tokens | Early compaction window |
| L2 | 384000 tokens | Mid-turn compaction |
| L3 | 576000 tokens | Deep compaction |
| Cycle | 768000 tokens | Hard cycle reserve |

### Workshop Routing

Tool outputs exceeding `large_output_threshold_tokens` are routed through a synthesis
sub-agent. Only the synthesis reaches the parent context; the raw text is stored
so the parent can promote later if needed.

Per-tool overrides:
- exec_shell: 2048 tokens (synthesised aggressively)
- grep_files: 2048 tokens
- web_search: 8192 tokens (web results can be large)

---

## Approval Policy Gating

### Three Tiers

| Tier | Behavior | When to use |
|---|---|---|
| on-request | Ask approval before each write/patch/shell/sub-agent | Default, untrusted workspace |
| untrusted | Additional safety rails for untrusted environments | Corporate proxies, external sandbox |
| never | Auto-approve all tools | Trusted workspace, YOLO mode |

### Efficient Approvals

When your plan includes multiple writes, present them together:
1. Show checklist with all write steps listed
2. Request approval for the batch
3. Once approved, execute all writes in one turn (parallel calls)

Don't sequence approvals one at a time. The user wants context, not interruption.

### Auto-Allow Entries

Match by command prefix, not raw string.

```
auto_allow = ["git status"]   # auto-approves: git status, git status -s
                              # does NOT auto-approve: git push, git checkout
auto_allow = ["cargo check", "npm run"]
```

---

## Network Policy

### Per-Domain Allow/Deny

Precedence: deny wins. A host listed in both allow and deny is denied.

Host-matching rules:
- Exact match: `api.deepseek.com` matches only `api.deepseek.com`
- Subdomain wildcard: entry starting with `.` (e.g. `.example.com`) matches
  subdomains but not the apex. To cover both, list both. `*.example.com` also accepted

### Defaults

When this section is absent, no policy is enforced. To opt in:

```
default = "prompt"     # allow | deny | prompt
allow = ["api.deepseek.com", "github.com", ".githubusercontent.com"]
deny = []
audit = true           # one line per call to audit log
```

---

## Sandbox Routing

### External Execution

When sandbox_backend is set to "opensandbox", all exec_shell calls are routed
through an external OpenSandbox-compatible HTTP API instead of spawning a local
process.

```
sandbox_backend = "opensandbox"
sandbox_url = "http://localhost:8080"
sandbox_api_key = "sk-opensandbox-secret"
```

The backend uses a 30-second HTTP timeout. Background, interactive, and TTY modes
are not supported with external backends — all commands run synchronously via HTTP.

### Sandbox Modes

| Mode | Scope |
|---|---|
| read-only | No workspace modifications |
| workspace-write | Modify workspace only |
| danger-full-access | Full system access |
| external-sandbox | External OpenSandbox backend |

---

## Workflow: Multi-Step Agent Work

### Step 1: Create Checklist

Lay out plan with checklist_write. Mark first task in_progress.

### Step 2: Fan-Out Independent Tasks

For tasks estimated to take 5+ steps:
1. `update_plan` — 3-6 high-level phases (status: pending)
2. `checklist_write` — concrete leaf tasks under first phase
3. Execute phase 1, updating checklist as you go
4. After each phase completes, re-read plan: does phase 2 still make sense?
5. When a phase reveals sub-problems, add to checklist or spawn investigation

### Step 3: Monitor Context

After every 3 turns, check:
- context under 60%?
- sub-agents still running?
- PRs ready to push?
- cargo check still passes?

### Step 4: Verify Before Claiming

After every tool call that produces a result you'll act on, verify before proceeding:
- File reads: confirm line numbers you're about to patch are what you think
- Shell commands: check stdout, not just exit code
- Search results: confirm the match is what you expected
- Sub-agent results: cross-check one finding against a direct read

Don't claim a change worked until you've observed evidence. Don't trust memory
over live tool output.

---

## Verification Gates

Before claiming anything is done:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

---

## Integration Points for Crabjar

### Adoption Patterns

1. **Sub-agent coordination**: Crabjar's parallel task coordination mirrors DeepSeek
   sub-agent fan-out — use bounded completion events for integration
2. **Bounded retrieval**: Crabjar knowledge bridge bounded retrieval parallels handle_read
   pattern — use slice/range projections for large knowledge store results
3. **Execution gating**: Crabjar's execution gate alignment mirrors approval policy
   tiers — enforce raw data reference before pipeline execution
4. **Network policy**: Crabjar domain allowlist alignment mirrors per-domain rules
   — use exact match + subdomain wildcard for scope gating
5. **Session persistence**: Crabjar state-docs overlay persistence parallels UUID-based
   checkpoint/resume — use sidecar JSON for agent session survival

---

*End of skill.*

</content>