# Theory: Habitat Contract Pipeline

The crabjar agent pushes attempts to the triage queue. When the queue fills,
the agent halts and waits for maintainer judgment. Guard actions requiring
destructive operations go through the pending queue.

## Current State
- Queue budget: 10
- Active attempts: 0
- Pending guard actions: 0
