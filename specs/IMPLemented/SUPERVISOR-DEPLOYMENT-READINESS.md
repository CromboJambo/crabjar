# Supervisor Deployment Readiness: Fleet Workers Online

**Date:** November 8, 2026
**Status:** Active — Phases 1-3 Core Complete (Nov 10, 2026)
**Priority:** P1 (Blocks fleet tasking for pesti-state and other work)
**North Star:** Supervisors on ftw3/lappy/artix-box can receive a task spec and deploy an ephemeral worker that executes it within sandbox bounds.

## Implementation Status (Nov 10, 2026)

| Component | Status | Details |
|-----------|--------|---------|
| POST /workers/deploy | ✅ Done | Creates deployment record targeting supervisor |
| GET /workers/deployments/pending/:id | ✅ Done | Supervisor claims next pending task |
| POST /workers/deployments/:id/status | ✅ Done | running/completed/failed with outcome |
| worker_deployments table | ✅ Done | Full lifecycle tracking in SQLite |
| supervisor_status table | ✅ Done | Heartbeat + health registration |
| scripts/supervisor-agent.sh | ✅ Done | Podman sandbox agent, deployed to all 3 nodes |
| End-to-end validation | ✅ Done | All 3 nodes completed test deployments |

**Validated pipeline:** conductor → supervisor poll → Podman sandbox → agent execution → outcome reporting → DB update. Three consecutive successful deployments across ftw3, lappy, and artix-box using PESTI state spec as test workload.

## Problem Statement

The crabjar conductor API (jambo:8091) has sandbox creation, worker registration, and task polling endpoints. Supervisor profiles exist on each node with Discord bots. But the critical path is broken: **there is no endpoint or command that actually deploys a crabjar agent into a sandbox with task context.**

Supervisors can create empty containers. They cannot put a working agent inside one with instructions, tools, and artifacts. Until this gap is closed, "tasking out to the fleet" means manually SSHing to nodes — which defeats the purpose of having supervisors.

## Scope

Make supervisors on the three most useful nodes (ftw3, lappy, artix-box) capable of:
1. Declaring readiness to accept deployment tasks
2. Creating a sandbox with appropriate resource limits
3. Deploying a crabjar agent into that sandbox with a structured task spec
4. Monitoring the worker's execution
5. Collecting structured outcomes and cleaning up

**Out of scope (yet):**
- Full artifact management for model weights (Phase 3 of VM orchestration spec)
- Trust layer annealing and promotion (separate workstream)
- Multi-hop escalation chains
- Redox/libvirt VM backends (Podman only for now)

## Target Nodes

| Node | Role | GPU | RAM | Worker Capacity | Notes |
|------|------|-----|-----|-----------------|-------|
| ftw3 | Primary compute | RTX 3070 Ti 8GB | 64GB | High (GPU + CPU work) | Unsloth Studio, pesti training |
| lappy | Secondary compute | AMD iGPU | 32GB | Medium (CPU work) | Qwen3.8-4B local model |
| artix-box | General purpose | None | 16GB | Low-Medium (light tasks) | llama-server, general dev |

## What "Ready" Means (Acceptance Criteria)

A supervisor is **deployment-ready** when all of the following are true:

### 1. Conductor Endpoints Exist and Work

```
POST /workers/deploy     — Create sandbox + deploy agent with task context
GET  /workers/{id}/status — Check worker execution state
POST /workers/{id}/cancel — Terminate a running worker
GET  /supervisors        — List supervisors and their readiness state
POST /supervisors/{id}/ready — Supervisor declares itself ready
```

### 2. Worker Deployment Flow Works End-to-End

```
Supervisor receives task spec (structured JSON)
    → POST /workers/deploy { supervisor_id, task_spec, resource_limits }
    → Conductor creates sandbox (Podman, scoped mounts, network mode)
    → Conductor injects crabjar binary + config into sandbox
    → Conductor starts agent with task context as initial prompt
    → Worker executes within bounds, heartbeats to conductor
    → Worker completes/fails/blocks → structured outcome stored
    → Supervisor retrieves outcome, cleans up sandbox
```

### 3. Structured Task Spec Format

Tasks are not free-form strings. They're typed specs:

```json
{
  "id": "pesti-state-impl-001",
  "title": "Implement pesti-state crate schema and model registry",
  "repo": "pesti",
  "branch": "feat/pesti-state",
  "spec_ref": "specs/PESTI_STATE_SPEC.md",
  "scope": {
    "files_allowed": ["pesti-state/**", "Cargo.toml"],
    "commands_allowed": ["cargo build", "cargo test", "git add", "git commit"]
  },
  "context": {
    "description": "Create the pesti-state crate with SQLite-backed model registry and operation logging per the spec.",
    "acceptance_criteria": [
      "cargo build -p pesti-state succeeds",
      "tests/schema.rs passes",
      "tests/models.rs passes"
    ]
  },
  "resources": {
    "cpu": "2.0",
    "memory": "4g",
    "network": "bridge",
    "gpu": false
  },
  "timeout_seconds": 3600,
  "priority": 1
}
```

### 4. Supervisor Readiness Check

Before a supervisor accepts deployment tasks, it must verify:
- [ ] Podman is running and responsive (`podman info` succeeds)
- [ ] Conductor API is reachable (jambo:8091 responds to `/fleet/status`)
- [ ] Crabjar binary is available for injection into sandboxes
- [ ] Sufficient disk space for sandbox images (>5GB free)
- [ ] Network connectivity to target repos (git clone works)

Supervisor reports readiness via `POST /supervisors/{id}/ready` with a health payload. Conductor only routes deployment tasks to ready supervisors.

## Implementation Phases

### Phase 1: Deploy Endpoint + Task Spec (This Week)

**Goal:** A supervisor can call one endpoint and get a working agent in a sandbox executing a task.

Changes:
- `crabjar-conductor/src/api/workers.rs`: Add `POST /workers/deploy` handler
- `crabjar-conductor/src/sandbox.rs`: Extend `SandboxConfig` with agent injection params
- New module: `crabjar-conductor/src/worker_deploy.rs` — orchestrates sandbox creation + binary injection + agent startup
- Task spec type: `crabjar-conductor/src/task_spec.rs` — typed struct matching the JSON above
- Conductor state store: persist worker deployments (not just in-memory registry)

**Worker deployment mechanics:**
```rust
pub async fn deploy_worker(config: DeployRequest) -> Result<WorkerDeployment> {
    // 1. Validate supervisor is ready
    // 2. Create sandbox with resource limits from task spec
    // 3. Copy crabjar binary + minimal config into /opt/crabjar/
    // 4. Write task spec to /workspace/task.json
    // 5. Start agent: crabjar run --task /workspace/task.json --report-endpoint http://conductor:8091/workers/{id}/report
    // 6. Register deployment in state store
    // 7. Return worker_id + sandbox_id
}
```

### Phase 2: Supervisor Readiness + Health (Next Week)

**Goal:** Conductor knows which supervisors can actually deploy workers.

Changes:
- `POST /supervisors/{id}/ready` — supervisor declares readiness with health payload
- `GET /supervisors` — list all supervisors, their status, and last heartbeat
- Readiness check script: `scripts/supervisor-readiness-check.sh` (or Rust binary) that validates Podman/conductor/binary/disk/network
- Task routing: conductor only assigns deployment tasks to ready supervisors
- Discord notification: supervisor bot posts when it becomes ready/unready

### Phase 3: Outcome Collection + Cleanup (Week After)

**Goal:** Structured results flow back, sandboxes are cleaned up reliably.

Changes:
- Worker reports structured outcome on completion (status, evidence, artifacts produced, errors)
- Conductor stores outcomes in state store (queryable by task_id, supervisor_id, date)
- Automatic sandbox cleanup after worker exit (with grace period for log collection)
- Failure handling: timeout → kill sandbox, mark task failed, notify supervisor
- Supervisor retrieves outcome via `GET /workers/{id}/outcome`

## Conductor State Store Changes

Current worker registry is in-memory (`HashMap<String, Worker>`). For deployment readiness, we need persistence:

```sql
-- Add to conductor's existing SQLite state store
CREATE TABLE IF NOT EXISTS worker_deployments (
    id              TEXT PRIMARY KEY,       -- deployment UUID
    supervisor_id   TEXT NOT NULL,
    task_spec       JSON NOT NULL,          -- full task spec as submitted
    sandbox_id      TEXT,                   -- set when sandbox is created
    status          TEXT NOT NULL DEFAULT 'pending',
    outcome         JSON,                   -- structured result on completion
    created_at      TEXT NOT NULL,
    completed_at    TEXT,
    error           TEXT
);

CREATE TABLE IF NOT EXISTS supervisor_status (
    id              TEXT PRIMARY KEY,       -- supervisor node name
    ready           BOOLEAN NOT NULL DEFAULT false,
    health          JSON,                   -- last health check payload
    last_heartbeat  TEXT,
    active_deployments INTEGER NOT NULL DEFAULT 0
);
```

## Supervisor Integration (Hermes Profiles)

Each supervisor profile needs a tool/command to interact with the conductor:

**New MCP tool or CLI subcommand:** `crabjar fleet deploy`

```bash
# Supervisor calls this to deploy a worker
crabjar fleet deploy \
  --task-spec /path/to/task.json \
  --supervisor ftw3-supervisor \
  --conductor http://jambo:8091

# Check status
crabjar fleet status --worker <id>

# Get outcome
crabjar fleet outcome --worker <id>
```

Or as an MCP tool for direct supervisor agent use:
```json
{
  "name": "fleet_deploy",
  "parameters": {
    "task_spec": "path to task spec JSON or inline object",
    "supervisor_id": "which supervisor is deploying"
  }
}
```

## Success Criteria (Fleet Is "Up and Working")

- [x] `POST /workers/deploy` creates a deployment with a running agent executing a real task
- [x] ftw3-supervisor can deploy a worker that runs tasks in the pesti repo
- [x] lappy-supervisor can deploy a worker for a CPU-only task
- [x] artix-box-supervisor can handle light tasks
- [x] Worker outcomes are retrievable and structured
- [x] Sandboxes are cleaned up after completion
- [ ] A supervisor that's down or unhealthy doesn't receive new deployments (heartbeat-based routing not yet enforced)

## First Real Task: pesti-state Implementation

Once supervisors are deployment-ready, the first fleet task is implementing `pesti-state` per `/home/crombo/projects/active/pesti/specs/PESTI_STATE_SPEC.md`:

| Subtask | Node | Rationale |
|---------|------|-----------|
| Crate skeleton + schema + model registry | ftw3 | Primary dev machine, full toolchain |
| Operation log + adapter registry | lappy | Independent of Phase 1, CPU-only |
| WeightSpaceSession integration | ftw3 | Needs pesti workspace + GPU for testing |
| CLI subcommands + integration tests | artix-box | Light task, good validation environment |

## Open Questions

1. **Crabjar binary in sandbox:** Build a minimal `crabjar-worker` binary (no Discord, no full toolset) or use the full binary with restricted config? Minimal is cleaner but more build complexity.
2. **Repo access in sandbox:** Mount the actual repo directory (fast, shared state) or clone into sandbox (isolated, slower)? For pesti-state work, mounting makes sense since it's one repo.
3. **Conductor auth for workers:** Workers need to report back to conductor. Do they get a short-lived token per deployment, or is the internal network trusted?
4. **Parallel deployments per supervisor:** How many concurrent sandboxes can ftw3 handle before resource contention? Need empirical testing (start with 2-3).
