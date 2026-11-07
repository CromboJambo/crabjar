# Fleet Rebuild Plan - October 26, 2026

## Current State (Broken)

| Machine | Running | Issues |
|---------|---------|--------|
| ftw3 | Hermes gateway, conductor (:8091), pesti-server (:8080), llama-server (:8081), 2 workers | Services running as nohup processes, not dinit; crabjar-agent service stopped |
| lappy | Nothing | Completely empty |
| sovnode | crabjar-ssh-bastion (:2222), something on :8080 | Inconsistent state |
| artix-box | Nothing | Completely empty |

## Architecture Principle

**Build in Hermes first for the UI, build our own Rust backend whenever possible.**

This means:
1. Hermes provides the complete user-facing layer (web/TUI/gateway)
2. Crabjar provides all backend services (conductor, workers, execution pipeline, inference)
3. Clean API boundaries between them - no ad-hoc process management

## Lessons from Friday & Since

1. **Proper service management matters** - nohup processes die on reboot and are hard to monitor
2. **Consistent deployment across fleet** - all machines should have the same base setup
3. **Hermes as the control plane** - it already has the gateway, UI, and session management
4. **Rust backend for reliability** - crabjar components are more robust than Python equivalents

## Rebuild Strategy

### Phase 1: Clean Slate (All Machines)
- Stop all existing processes
- Remove ad-hoc nohup scripts
- Reset to clean state

### Phase 2: Base Infrastructure (All Machines)
- Deploy fresh crabjar binary from current HEAD
- Configure proper dinit services for each component
- Set up machine-specific configs via deploy.sh

### Phase 3: Role Assignment
- **ftw3**: Primary conductor node + GPU inference (pesti-server with Kev-9B)
- **lappy**: Worker node (formula-engine, general tasks)
- **sovnode**: SSH bastion + storage worker
- **artix-box**: Additional worker node

### Phase 4: Hermes Integration
- Configure Hermes gateway to talk to crabjar-conductor API
- Set up proper authentication and routing
- Test end-to-end task submission through UI → Hermes → Conductor → Worker

## Service Architecture

Each machine runs as proper dinit services:
```
/opt/crabjar/
├── crabjar (CLI binary)
├── conductor/ (only on ftw3)
│   ├── crabjar-conductor
│   └── conductor-state.db
├── workers/ (on worker nodes)
│   ├── formula-worker.conf
│   └── storage-worker.conf
└── .crabjar_config.toml
```

## Success Criteria

1. All services start automatically on reboot
2. Hermes UI can submit tasks to conductor
3. Conductor dispatches to appropriate workers
4. Workers execute and report back
5. No ad-hoc nohup processes or manual restarts needed
