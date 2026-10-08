# Crabjar Conductor Crate Specification

**Status:** Draft  
**Last Updated:** 2026-10-08  
**Author:** crabjar team (CromboJambo)

## Overview

The `crabjar-conductor` crate implements the fleet coordination layer of crabjar's three-tier orchestration architecture. It provides centralized goal management, task decomposition, worker registration, and cross-machine scheduling for home-lab agent fleets.

### Architecture Layers

1. **Local Orchestration** (existing): Single crabjar instance manages its own kanban board and tasks via SQLite persistence. Works standalone on any machine.

2. **Fleet Coordination** (this crate): Conductor service deployed on a central node (ftw3) that coordinates multiple worker agents across the lab. Receives high-level goals, decomposes them into executable tasks, routes based on worker capabilities, and aggregates results.

3. **Cross-Environment Orchestration** (future): Agents spanning local machines, cloud VMs, and edge devices with unified scheduling and state management.

### Design Principles

- **Conductor pattern**: Central coordination node that workers poll for work and report results to
- **HTTP-first API**: RESTful endpoints using the same patterns as pesti-server and vm-bridge
- **Capability-based routing**: Tasks are assigned based on worker capabilities (GPU, disk, load)
- **Goal decomposition**: Natural language goals broken into executable task DAGs using local LLM inference
- **Simple state store**: Embedded SQLite for conductor state; workers maintain local kanban boards

## Crate Structure

```
crabjar-conductor/
├── Cargo.toml
└── src/
    ├── main.rs              # HTTP server setup, config loading
    ├── conductor.rs         # Core orchestration logic (goal → tasks)
    ├── worker_registry.rs   # Track registered workers + capabilities
    ├── task_scheduler.rs    # Route tasks to appropriate workers
    ├── state_store.rs       # Abstract storage backend (SQLite)
    └── api/
        ├── goals.rs         # POST /goals, GET /goals/{id}
        ├── workers.rs       # GET /workers, POST /workers/register
        └── tasks.rs         # Internal task management endpoints
```

## Core Data Model

### Goal
High-level objective submitted by a user. Decomposed into tasks by the conductor.

```hemlock
Goal {
    id: UUID,
    description: String,
    status: "decomposed" | "running" | "completed" | "failed",
    created_at: Timestamp,
    tasks: Vec<Task>
}
```

### Task
Executable unit of work with dependency tracking.

```hemlock
Task {
    id: UUID,
    goal_id: UUID,
    worker_id: Option<UUID>,   // null = pending assignment
    command: String,
    dependencies: Vec<TaskId>,
    status: "pending" | "assigned" | "running" | "completed" | "failed",
    result: Option<String>
}
```

### Worker
Registered fleet member with capability metadata.

```hemlock
Worker {
    id: UUID,
    name: String,              // machine hostname
    ssh_key: PathBuf,          // path to SSH key for access
    capabilities: Vec<String>, // ["gpu:nvidia:3070ti", "disk:1tb"]
    status: "online" | "busy"  | "offline"
}
```

## API Endpoints

### Goals
- `POST /goals` — Submit a new goal for decomposition and execution
- `GET /goals/{id}` — Get goal status and task breakdown
- `GET /goals` — List all goals with optional status filter

### Workers
- `GET /workers` — List registered workers and their status
- `POST /workers/register` — Worker self-registration endpoint
- `POST /workers/{id}/heartbeat` — Worker liveness signal

### Tasks
- `GET /tasks/for-worker/{worker_id}` — Poll for assigned tasks (worker endpoint)
- `POST /tasks/{id}/result` — Report task execution result (worker endpoint)

## Integration Points

| Component | Purpose |
|-----------|---------|
| pesti-server | Goal decomposition via local LLM inference (port 8000) |
| ssh-bastion crate | Secure SSH access to worker nodes |
| crabjar-guard | Policy enforcement before task execution |
| vm-bridge | VM lifecycle management for ephemeral environments |

## Deployment

The conductor service deploys as a dinit-managed process on the central lab node (ftw3), following the same pattern as pesti-server and vm-bridge:

```bash
# Build
cargo build --release -p crabjar-conductor

# Deploy to ftw3
rsync target/release/crabjar-conductor crombo@ftw3:/opt/crabjar/conductor/

# Service config (/etc/dinit.d/crabjar-conductor)
type     = process
command  = cd /opt/crabjar/conductor && exec ./crabjar-conductor --port 8091
logfile  = /var/log/dinit/crabjar-conductor.log
depends-on: network.target
```

## Future Work

- Migrate state store from SQLite to Postgres for scale
- Worker-to-worker communication patterns
- Dynamic capability discovery via mDNS/Tailscale DNS
- Cross-environment orchestration (cloud + local)
