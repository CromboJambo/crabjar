# Unified Agent Environment Specification

## Vision

Standardize the Unix environment across all lab machines so any agent framework (crabjar, hermes-agent, future systems) can run with consistent configuration, remote observability, and a clear migration path from shell scripts to Rust implementations.

## Architecture: Two-Layer User Model

### Layer 1: Human User (`crombo`)
- Personal SSH keys, configs, dotfiles
- Development environment (Rust toolchain, git, etc.)
- Direct access to all agent environments
- Manages deployment and upgrades

### Layer 2: Agent User (`crabjar-agent`)
- Dedicated Unix user for agent execution
- Isolated home directory with standard structure
- Configurable via dotfiles and environment variables
- Observable via VNC/KDE Connect for debugging
- Gradually hardened with Rust-based tooling

## Standard Agent Home Directory Structure

```
/home/crabjar-agent/
├── .profile              # Environment setup (sourced on login)
├── .bashrc               # Shell configuration
├── .ssh/                 # SSH keys for agent-to-agent communication
│   ├── id_lab            # Shared lab SSH key
│   └── known_hosts       # Trusted host keys
├── workspace/            # Working directory for tasks and artifacts
├── inbox/                # Tasks dropped by supervisor (JSON files)
├── outbox/               # Agent writes results/reports here
├── log/                  # Execution logs
│   ├── completed_*.json  # Completed task records
│   └── errors.log        # Error tracking
├── config/               # Agent configuration files
│   ├── agent.json        # Main agent config (model, behavior)
│   └── environment.json  # Environment-specific settings
└── cache/                # Temporary/cache data
```

## Remote Observability Stack

### Phase 1: VNC + KDE Connect (Current)
- Each agent machine runs a VNC server on the agent user's desktop session
- Access via unsloth or KDE Connect for screen sharing
- Enables visual debugging of agent behavior
- Low security, high accessibility

### Phase 2: Rust-Based Remote Desktop Protocol
- Implement lightweight custom protocol in Rust
- Replaces VNC with more efficient, secure channel
- Integrates with crabjar's existing SSH infrastructure

### Phase 3: Integrated Agent Dashboard
- Web-based dashboard for fleet-wide agent monitoring
- Real-time task tracking, logs, and metrics
- Built on crabjar's conductor API

## Migration Path: Shell → Rust

### Current State (Shell)
- `agent-loop.sh` — Bash script that polls inbox and processes tasks
- Shell-based configuration management
- Direct filesystem operations for state management

### Target State (Rust)
- `crabjar-agent` binary replaces shell loop
- Rust-based configuration system with validation
- SQLite-backed state management (already in place)
- Proper signal handling, logging, and error recovery

### Migration Strategy
1. **Parallel operation**: Run Rust agent alongside shell loop, compare behavior
2. **Feature parity**: Ensure Rust implementation handles all edge cases shell handled
3. **Gradual replacement**: Switch production traffic to Rust, keep shell as fallback
4. **Cleanup**: Remove shell scripts after validation period

## Environment Configuration

### Shared Configs (via dotfiles repo)
- SSH configuration (`~/.ssh/config`)
- Shell aliases and functions
- Editor settings (for agent-generated files)
- Git configuration

### Machine-Specific Configs
- `config/environment.json` — GPU availability, network interfaces, local services
- `config/agent.json` — Model selection, behavior parameters
- Environment variables via `.profile`

## Security Hardening Roadmap

### Phase 1: Basic Isolation (Current)
- Dedicated agent user with limited privileges
- Separate home directory structure
- SSH key authentication for agent-to-agent communication

### Phase 2: Enhanced Security
- AppArmor/SELinux profiles for agent processes
- Network namespace isolation
- Filesystem read-only where possible

### Phase 3: Rust-Based Security Layer
- Custom capability system in Rust
- Formal verification of security properties
- Automated threat detection and response

## Implementation Plan

### Immediate (Week 1)
- [ ] Standardize `crabjar-agent` user setup across all lab machines
- [ ] Create shared dotfiles repo for agent environment configuration
- [ ] Set up VNC access on ftw3 as proof of concept

### Short-term (Weeks 2-4)
- [ ] Migrate `agent-loop.sh` functionality to Rust binary
- [ ] Implement basic remote observability via crabjar conductor API
- [ ] Document environment setup and configuration procedures

### Medium-term (Months 2-3)
- [ ] Replace shell-based configuration with Rust config system
- [ ] Implement secure remote desktop protocol in Rust
- [ ] Deploy standardized environment to all lab machines

## Compatibility Matrix

| Agent Framework | Environment Requirements | Status |
|----------------|--------------------------|--------|
| crabjar | Standard agent user + inbox/outbox | ✅ Current |
| hermes-agent | Similar structure, different config format | 🔄 Adapting |
| Future agents | Must conform to standard home directory layout | 📋 Specified |

## Notes

- This spec intentionally leaves room for future agent frameworks — the environment should be generic enough that any Unix-based agent can operate within it
- The Rust takeover is strategic and gradual — each shell component is replaced when its Rust equivalent is proven more reliable/maintainable
- Remote observability prioritizes accessibility over security initially, with hardening coming later as the system matures