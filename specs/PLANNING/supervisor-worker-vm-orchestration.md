# Supervisor-Worker VM Orchestration Architecture

**Status:** Draft  
**Created:** 2026-11-02  
**Related:** sandbox crate, guard trust layers, worker-promotion-causal-model.md, crabjar-conductor

## Overview

A system where supervisors provision ephemeral worker environments rehydrated from artifacts (model weights, baked-in data). Workers are not persistent identities — they're stateless executors instantiated per task with scoped access. The supervisor provides the "nurture" (environment, tools, context); the worker brings its "nature" (model weights, training). The fleet is promotion-seeking: it grows by creating needs that require higher-tier supervision.

## Core Principles

### Nature vs Nurture
- **Nature:** Worker's model weights, training data, baked-in capabilities — immutable, provided as artifacts
- **Nurture:** Supervisor-provided environment — sandbox config, tool access, task context, mounts, network

### Ephemeral Workers
- Workers don't persist between tasks
- Each task instantiates a fresh worker from artifacts
- State lives in supervisor/conductor, not in workers
- Workers report outcomes, then terminate

### Promotion-Seeking Fleet
- The fleet's purpose is to create needs that require supervisors
- Lower-tier agents handle simple tasks; complex ones escalate
- As fleet grows, more supervision capacity is needed → more promotion opportunities
- Systemic growth drives individual advancement

## Architecture

```
Supervisor Profile (persistent identity)
    │
    ├── Task arrives via conductor API
    │       │
    │       ▼
    │   Artifact Resolution (which model weights / baked-in data?)
    │       │
    │       ▼
    │   Sandbox Provisioning (Podman container with scoped access)
    │       │
    │       ▼
    │   Worker Rehydration: inject artifacts + task context
    │       │
    │       ▼
    │   Ephemeral Worker Instance
    │       ├── Executes within sandbox bounds
    │       ├── Tool calls pass through guard gate
    │       └── Reports structured outcomes
    │
    ├── Outcome Evaluation (supervisor assesses results)
    │       │
    │       ▼
    │   Evidence Collection for promotion decisions
    │
    └── Fleet Planning: provision more supervisor capacity as needed
```

## Trust Levels → Sandbox Configuration

| Trust Layer | Worker Tier | Network | Mounts | Resources | Persistence |
|-------------|-------------|---------|--------|-----------|-------------|
| Layer 1 (Quarantine) | Task Worker | None (--network none) | /workspace only | Limited CPU/mem | Ephemeral |
| Layer 2 (Working) | Gig Worker | Custom bridge | /workspace + tool cache | Standard | Container lifetime |
| Layer 3 (Annealed) | Dinit Service | Full network | Host directories | Host resources | Persistent volume |
| Layer 4+ (Shared) | Fleet Agent | All networks | Full filesystem | Unbounded | Persistent identity |

## Supervisor Responsibilities

1. **Task Intake:** Receive tasks via conductor API (`POST /tasks`)
2. **Artifact Resolution:** Determine which model weights/artifacts the task requires
3. **Sandbox Provisioning:** Create container with appropriate configuration:
   - Network isolation based on trust layer
   - Filesystem mounts scoped to task needs
   - Resource limits matching worker tier
4. **Worker Rehydration:** Inject artifacts + task context into sandbox
5. **Monitoring:** Track worker execution via conductor heartbeat
6. **Outcome Evaluation:** Assess structured results from worker
7. **Evidence Collection:** Feed outcomes to promotion engine
8. **Fleet Planning:** Provision additional supervisor/worker capacity as demand grows

## Worker Responsibilities (Ephemeral)

1. **Task Execution:** Operate within sandbox bounds using provided artifacts
2. **Tool Calls:** All tool calls pass through guard gate with provenance tracking
3. **Heartbeat:** Report liveness via conductor API (`POST /heartbeat`)
4. **Outcome Reporting:** Submit structured results before termination:
   ```json
   {
     "task_id": "...",
     "status": "completed|failed|blocked",
     "evidence": [...],
     "artifacts_produced": [...],
     "trust_signals": {
       "tool_calls_made": 12,
       "guard_approvals": 3,
       "errors_encountered": 0,
       "artifact_usage": ["model-v2-weights", "tool-cache"]
     }
   }
   ```

## Artifact Management

Workers are rehydrated from artifacts stored by supervisors:

```hemlock
// Supervisor stores artifact
conductor.store_artifact({
    id: "worker-model-v2",
    type: "model_weights",
    path: "/artifacts/models/v2.gguf",
    metadata: { version: 2, capabilities: ["code-review", "planning"] }
});

// Supervisor provisions worker with artifact
conductor.provision_worker({
    task_id: "...",
    artifacts: ["worker-model-v2"],
    trust_layer: 2,
    sandbox_config: { network: "bridge", mounts: ["/workspace"] }
});
```

## API Design (Conductor Endpoints)

### Sandbox Management
- `POST /sandboxes` — Provision sandbox for task
  ```json
  {
    "task_id": "...",
    "trust_layer": 1,
    "config": {
      "network_mode": "none",
      "mounts": ["/tmp/task:/workspace"],
      "resource_limits": "0.5"
    }
  }
  ```
- `GET /sandboxes/{id}` — Sandbox status
- `POST /sandboxes/{id}/exec` — Execute command in sandbox (supervisor override)
- `DELETE /sandboxes/{id}` — Destroy sandbox

### Worker Deployment & Rehydration
- `POST /workers/deploy` — Deploy crabjar agent to sandbox with artifacts
  ```json
  {
    "sandbox_id": "...",
    "artifacts": ["worker-model-v2"],
    "task_context": {...},
    "profile": "worker-default"
  }
  ```

### Artifact Management
- `POST /artifacts` — Store artifact for worker rehydration
- `GET /artifacts/{id}` — Retrieve artifact metadata
- `DELETE /artifacts/{id}` — Remove artifact

## Integration with Existing Components

### Guard Crate (Trust Layers)
- Trust layer assessment determines sandbox configuration
- Execution gate inside sandbox validates tool calls
- Annealing pipeline updates trust based on outcomes

### Sandbox Crate (Podman Backend)
- `PodmanSandbox::create_container()` — provision worker node
- `PodmanSandbox::exec_in_container()` — supervisor commands / rehydration
- `PodmanSandbox::destroy_container()` — cleanup after task

### Conductor (Orchestration)
- Task routing considers sandbox availability and artifact requirements
- Promotion API receives evidence from worker outcomes
- Heartbeat tracking for worker liveness
- Fleet planning: monitor demand vs capacity, provision supervisors as needed

## Implementation Phases

### Phase 1: Basic VM Worker Provisioning
- Extend conductor API with sandbox endpoints
- Integrate `PodmanSandbox` into conductor's task dispatch
- Deploy crabjar agent to containers with basic config
- Supervisor provisions → worker executes → outcome reported

### Phase 2: Trust-Gated Access Levels
- Map guard trust layers to sandbox configurations
- Implement network isolation, mount scoping, resource limits
- Guard execution gate inside sandbox validates tool calls
- Annealing updates trust based on task outcomes

### Phase 3: Artifact Management & Rehydration
- Conductor artifact storage API
- Worker deployment with artifact injection
- Model weights / baked-in data versioning
- Supervisor manages artifact lifecycle

### Phase 4: Evidence-Based Promotion
- Worker reports structured evidence with outcomes
- Supervisor evaluates evidence for promotion decisions
- Context window expansion on promotion (per causal model spec)
- Opportunity perception metrics tracked per worker

### Phase 5: Fleet Planning & Growth
- Monitor task demand vs fleet capacity
- Auto-provision supervisor instances as needed
- Promotion-seeking behavior: escalate complex tasks to create supervision needs
- Systemic growth drives individual advancement

## Success Criteria

- [ ] Supervisor can provision sandbox for task via conductor API
- [ ] Crabjar agent deploys and executes within sandbox bounds
- [ ] Trust layer determines network/mount/resource configuration
- [ ] Worker outcomes feed back to supervisor for evaluation
- [ ] Promotion decisions based on structured evidence from tasks
- [ ] Workers rehydrate from artifacts (model weights, baked-in data)
- [ ] Fleet planning provisions supervisor capacity as demand grows

## Open Questions

1. Should sandboxes use Podman containers exclusively, or support libvirt VMs (Redox) as alternative?
2. How should tool call provenance be tracked across sandbox boundaries?
3. What's the appropriate heartbeat interval for worker liveness tracking?
4. Should supervisors have override capability to exec into running sandboxes?
5. How do we version and manage artifact lifecycle (model weights, baked-in data)?
