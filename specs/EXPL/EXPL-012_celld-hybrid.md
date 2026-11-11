# EXPL-012: Hybrid — celld Substrate + crabjar Decision Layer

**Status**: Proposed — Experiment Branch  
**Branch**: `exp/celld-hybrid`  
**Date**: 2026-11-10  
**Motivation**: EXPL-010 (run on celld) and EXPL-011 (steal patterns) aren't mutually exclusive. The cleanest split might be: **celld answers "where does it run and how does it survive?"** while **crabjar answers "is it allowed to run and what are we doing with the results?"** This experiment tests whether that division of labor works in practice.

## Hypothesis

The two systems solve orthogonal problems:
- **celld**: distributed execution substrate — named actors, durable state, failover, replication, hibernation. Language-agnostic (JS/TS Workers), but the *infrastructure* is Rust.
- **crabjar**: decision and governance layer — guard policies, trust scoring, JEV semantic decisions, provenance tracking, risk assessment, approval flows.

Running crabjar's decision engine *on top of* celld's execution substrate gives you durable, failover-capable agent workers that are also governed by crabjar's trust model. Neither system has to implement what the other does well.

## What Success Looks Like

1. **Execution layer (celld)**: Agent workers run as Durable Objects on a 2-node fleet. State persists, failover works, tasks resume after node death.
2. **Decision layer (crabjar)**: Before any worker executes an action, it calls crabjar's guard API for approval. Trust scores update based on outcomes. JEV layers make semantic decisions about task routing and risk.
3. **Integration point**: A thin "crabjar gate" Worker that sits between the agent DO and the external world (shell, network, file system). All side-effecting calls pass through the gate.
4. **Operational simplicity**: One fleet of machines runs both celld nodes and crabjar services. dinit supervises everything. MinIO is the shared coordination substrate.

## What Would Kill It

- The two systems' operational models conflict: celld wants self-contained DOs with their own SQLite; crabjar wants centralized guard decisions and trust state. You end up with two competing "sources of truth" about what a worker can do.
- Latency: every action goes DO → gate Worker → crabjar guard API → back to DO → execute. If the guard call is slow (JEV embedding, trust computation), the agent feels sluggish.
- Debugging becomes a nightmare: a failed action could be a celld replication issue, a gate Worker bug, a guard policy mismatch, or a trust score problem. Four layers to debug instead of two.
- The "one fleet = one application" constraint (celld) conflicts with running multiple independent agent types that need different trust policies.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      Human / Conductor                          │
│              submits goals, reviews outcomes                    │
└──────────────────────────┬──────────────────────────────────────┘
                           │ HTTP POST /goals
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                    crabjar-conductor                            │
│    task decomposition, worker assignment, goal tracking          │
└──────────────────────────┬──────────────────────────────────────┘
                           │ assigns to named cell
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                    celld Fleet (2+ nodes)                       │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              Durable Object: worker-42                  │    │
│  │                                                         │    │
│  │  agent logic (JS/TS or WASM)                            │    │
│  │       │                                                 │    │
│  │       ▼ wants to execute "git push"                     │    │
│  │  ┌─────────────┐                                        │    │
│  │  │ crabjar gate│ ◄── HTTP ──► crabjar guard API         │    │
│  │  │ (thin DO)   │          (trust check, JEV decision,   │    │
│  │  └─────────────┘           provenance record)           │    │
│  │       │ approved                                             │
│  │       ▼                                                     │    │
│  │  execute action (shell, HTTP, file I/O)                     │    │
│  │       │                                                     │    │
│  │       ▼ outcome                                             │    │
│  │  report back to conductor + update trust                    │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                 │
│  State: SQLite per DO, replicated via LTX to peer + bucket       │
└─────────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                    crabjar core services                        │
│                                                                 │
│  guard API    trust store    JEV engine    provenance log        │
│  (policy)     (scores)       (decisions)   (audit trail)         │
│                                                                 │
│  Runs as standalone Rust services, supervised by dinit           │
└─────────────────────────────────────────────────────────────────┘

Shared substrate: MinIO bucket (celld ownership + LTX) 
                  + crabjar's own state.db (trust, provenance)
```

## The Gate Pattern (Key Integration Point)

The "crabjar gate" is a thin Durable Object that intercepts all side-effecting calls from agent workers:

```javascript
// Inside worker-42 DO
async function executeAction(action) {
  // Ask the gate before doing anything with side effects
  const verdict = await fetch(`http://crabjar-gate/${workerId}/check`, {
    method: "POST",
    body: JSON.stringify({ action, context: this.state }),
  }).then(r => r.json());
  
  if (verdict.approved) {
    const result = await actuallyDo(action);
    await fetch(`http://crabjar-gate/${workerId}/outcome`, {
      method: "POST",
      body: JSON.stringify({ action, result }),
    });
    return result;
  } else {
    throw new Error(`Blocked by guard: ${verdict.reason}`);
  }
}
```

The gate DO calls crabjar's guard API (Rust service) for the actual policy decision. This keeps the hot path (agent → execute) fast when policies are simple, and routes complex decisions (JEV, multi-factor trust) through the full crabjar stack.

## First Concrete Step

1. Complete EXPL-010's first step (2-node celld fleet + minimal Worker)
2. Stand up crabjar guard API as a standalone service on jambo (`crabjar guard serve`)
3. Write the gate DO: a 50-line JS Worker that proxies action requests through the guard API
4. Wire a test worker to use the gate before executing commands
5. Verify: kill the worker's node → it recovers → its trust score and pending actions are intact

## Division of Labor (Who Owns What)

| Concern | Owner | Why |
|---------|-------|-----|
| Process supervision | dinit | Both celld nodes and crabjar services are just processes to restart |
| Network trust boundary | Tailscale | celld's internal listener + crabjar service APIs both on private net |
| Object storage coordination | MinIO | celld ownership/LTX + crabjar can use it for artifact storage |
| Worker execution & state | celld | Named actors, SQLite, failover, replication — solved problem |
| Action authorization | crabjar guard | Policy engine, risk scoring, approval flows — crabjar's core value |
| Trust scoring | crabjar trust store | Longitudinal worker reputation, outcome-based updates |
| Semantic decisions | crabjar JEV | Task routing, priority, "should we retry or escalate?" |
| Provenance & audit | crabjar provenance log | Every action traced to goal → task → worker → decision chain |
| Inference | local llama-servers / pesti | Workers call models over HTTP; placement is a JEV decision |

## Open Questions

- Where does the trust store live? If it's in crabjar's state.db (SQLite on one machine), that's a single point of failure. Should it be replicated via celld LTX too?
- Can the gate DO cache guard decisions for repeated action types, or does every call need to hit the Rust service?
- What happens when the guard API is down? Fail-open (let actions through) or fail-closed (block everything)? This is a crabjar policy question, but celld's durability means the gate DO itself won't die.
- How do we handle long-running actions (training runs, large builds) that outlive a single request/response cycle? (celld workflows + step.do() might be the answer)

## Related

- EXPL-010 (run on celld) — this assumes it works
- EXPL-011 (steal patterns) — if hybrid is too complex, fall back to native patterns
- crabjar `guard/` crate — the policy engine being called through the gate
- crabjar JEV decision layer — semantic decisions about routing and risk
- celld `examples/opencode/` — proof that agent sessions work in DOs
