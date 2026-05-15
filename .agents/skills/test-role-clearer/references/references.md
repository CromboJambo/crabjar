# Agent vs User Role Separation Principles

## Core Principle: Detection ≠ Authorization

The ${REPO_ROOT} system follows the rule that "knowing what happened does not grant the right to change what happens." This principle is enforced across all agent components.

### Agent (${REPO_ROOT}) Characteristics:
- **Pure observer** - Detects and reports events, but has no execution capability
- **No command authority** - Cannot run commands, modify state, or execute actions  
- **Limited to detection phase** - Only analyzes existing state, never initiates changes

### User Characteristics:
- **Execution authority** - Has full command execution rights
- **State modification capability** - Can change configurations, trigger actions, and modify system state
- **Direct control** - Operates commands through the CLI interface

### Authority Boundary Examples:

1. **Configuration changes**: Only users can modify .${REPO_ROOT}_config.toml files; agent cannot initiate config updates  
2. **Command execution**: Only users can run commands like 'state list', 'build', or 'test'; agent only observes results  
3. **State modification**: Users can create, update, or delete state-docs; agent only reads existing documentation

### Real Session Examples:

- When user asks "who is executing this action?", the answer depends on whether they initiated a command or if system auto-triggered an observation
- During configuration changes, user must explicitly modify .${REPO_ROOT}_config.toml while agent observes and reports state before/after  
- In state-docs review, agent can read existing documentation but cannot edit or create new content without user command

## Test Question Format:

Each question presents three clear options with distinct roles:
- A) Agent - Pure observer role (no execution authority)
- B) User - Active executor with full command authority  
- C) Neither - System component without direct user control

The test validates understanding of documented rules rather than assumptions.