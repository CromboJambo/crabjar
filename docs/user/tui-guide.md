# Crabjar TUI Guide

The crabjar Terminal User Interface (TUI) provides an interactive way to manage agent workflows, monitor execution, and interact with your deployment.

## Starting the TUI

```bash
crabjar tui
```

This launches the full-featured TUI in your terminal. The interface is built on ratatui and works in any modern terminal emulator.

## Interface Layout

```
┌─────────────────────────────────────────────────────────────┐
│ Header: [Status] [Clock] [User/Profile]                     │
├─────────────────────┬───────────────────────────────────────┤
│ Left Panel          │ Main Content Area                     │
│ - Navigation        │ - Task list                           │
│ - State docs        │ - Execution output                    │
│ - Knowledge store   │ - Interactive prompts                 │
│                     │                                       │
├─────────────────────┴───────────────────────────────────────┤
│ Footer: [Input field] [Help] [Quit]                          │
└─────────────────────────────────────────────────────────────┘
```

## Keyboard Shortcuts

### Navigation
- `j` / `k` or `↑` / `↓`: Move selection up/down
- `h` / `l` or `←` / `→`: Switch panels (left/right)
- `gg`: Jump to top of list
- `G`: Jump to bottom of list

### Actions
- `Enter`: Select/execute action on highlighted item
- `Esc`: Go back / close current view
- `q`: Quit TUI
- `?`: Show help overlay

### Task Management
- `n`: New task
- `a`: Approve selected pending action
- `r`: Reject selected pending action
- `s`: Show task details/status

### State Documentation
- `d`: List state docs
- `D`: View/edit specific state doc
- `A`: Add annotation to current doc

### Knowledge Store
- `k`: Search knowledge store
- `K`: Verify knowledge freshness

### Refresh & Updates
- `R`: Refresh all data
- `Ctrl+R`: Force full reload from disk

## Task Management Workflow

### Viewing Tasks

The main view shows all active and pending tasks:

```
Tasks (5)
───────────────────────────────────────
> [Running] Deploy service to staging    2m ago
  [Pending] Approve database migration   5m ago
  [Blocked] Wait for CI pipeline         10m ago
  [Done] Update documentation            30m ago
```

### Approving Actions

When a task requires approval:

1. Navigate to the pending action in the queue
2. Press `a` to approve or `r` to reject
3. Provide reason for rejection if applicable

### Monitoring Execution

Selected running tasks show live output in the main content area, including:
- Command being executed
- Standard output/error streams
- Progress indicators
- Timing information

## State Documentation Management

### Browsing State Docs

Press `d` to list all state docs. Select one to view its contents:

```
State Docs (12)
───────────────────────────────────────
> deployment-config.md    Last updated: 2h ago
  trust-layer.md          Last updated: 1d ago
  api-endpoints.md        Last updated: 3d ago
```

### Viewing & Editing

Press `D` on a selected doc to open it in the built-in editor or view mode. Changes are saved when you exit with `Esc`.

### Checking Staleness

The TUI automatically highlights stale state docs based on age thresholds:
- **Fresh** (< 7 days): Normal display
- **Stale** (7-14 days): Yellow highlight + warning icon
- **Expired** (14-30 days): Orange highlight + "needs update" badge
- **Moldy** (> 30 days): Red highlight + "outdated" badge

## Knowledge Store Interaction

### Searching Knowledge

Press `k` to search the knowledge store:

```
Search Knowledge Store
───────────────────────────────────────
Query: [deployment procedures]
───────────────────────────────────────
> How to deploy microservices (api, deployment)
  Database migration best practices (db, deployment)
  Rollback procedures for failed deploys (ops)
```

### Verifying Knowledge Freshness

Press `K` on a selected knowledge item to check its freshness and see when it was last validated.

## Execution Gate Approvals

When actions require approval based on trust layer or risk assessment:

1. The pending queue appears in the left panel with notification badge
2. Select an action to see details:
   - Proposed command
   - Risk assessment score
   - Trust layer required
   - Justification for approval requirement
3. Approve (`a`) or reject (`r`) as appropriate

## Configuration Through TUI

Some configuration options can be adjusted directly in the TUI:

- **Log level**: Settings → Logging → Level (debug/info/warn/error)
- **Theme**: Settings → Appearance → Theme (light/dark/custom)
- **Auto-refresh interval**: Settings → Refresh → Interval (5s/10s/30s/off)

## Troubleshooting

### TUI Not Rendering Properly

Ensure your terminal supports UTF-8 and has sufficient dimensions (minimum 80x24). Try:

```bash
export TERM=xterm-256color
crabjar tui
```

### Slow Performance

If the TUI feels sluggish, check:
- Number of active tasks (too many concurrent operations)
- Database size (consider running `crabjar knowledge sync` to prune old entries)
- Network latency (if connected to remote conductor)

### Connection Issues

If the TUI can't connect to the conductor service:
1. Verify conductor is running: `curl http://localhost:8090/api/v1/fleet/status`
2. Check network connectivity between your machine and conductor host
3. Verify configuration file has correct conductor URL