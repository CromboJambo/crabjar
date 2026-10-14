# SSH Bastion Integration — Implementation Plan

## Scope Philosophy

**Build the smallest thing that works end-to-end first.** Get `crabjar exec --target jambo "hostname"` working before adding fan-out, interactive shells, or advanced guard policies. Each phase must be independently testable and usable.

---

## Phase 1: Core Remote Execution (MVP)

**Goal:** Execute a single command on a single remote machine via crabjar CLI.

### In Scope
- New crate: `crates/ssh-bastion` with russh dependency
- Hardcoded target config (jambo only, for now)
- Basic SSH connection + command execution
- Output capture (stdout/stderr/exit status)
- CLI flag: `--target <name>` on `crabjar exec`

### Out of Scope
- Secrets bridge integration (use plaintext key path initially)
- Guard evaluation for remote exec (rely on local guard only)
- Multi-target support
- Telemetry for remote execution

### Deliverable
```bash
crabjar exec --target jambo "hostname"
# Returns: jambo.local
```

### Implementation Steps
1. Create `crates/ssh-bastion` with Cargo.toml (russh dep)
2. Implement `execute_remote(target, command, key_path)` function
3. Add `--target` flag to crabjar CLI's exec subcommand
4. Wire up target config loading from `.crabjar_config.toml`
5. Test against jambo with existing SSH key

### Success Criteria
- Command executes on jambo and returns correct output
- Exit status propagated correctly
- Errors handled (connection refused, auth failed)

---

## Phase 2: Secrets Bridge Integration

**Goal:** SSH keys managed through vault, not plaintext paths.

### In Scope
- Integrate secrets-bridge crate for key decryption
- Store encrypted SSH key in `vault/secrets/ssh_keys/jambo.age`
- Session-based key access (no persistent plaintext)

### Out of Scope
- Host key verification (use known_hosts initially)
- Key rotation automation

### Deliverable
```bash
# Key stored encrypted, decrypted at runtime
crabjar exec --target jambo "whoami"
# Returns: crombo
```

### Implementation Steps
1. Add secrets-bridge dependency to ssh-bastion crate
2. Modify target config to reference vault paths
3. Implement key decryption at connection time
4. Update test workflow to use encrypted keys

---

## Phase 3: Guard Integration & Multi-Target

**Goal:** Trust evaluation for remote exec + support multiple machines.

### In Scope
- Remote execution policy in guard evaluation
- Target registry (jambo, ftw3, pixel9a)
- Per-target SSH config
- Basic approval flow for destructive commands

### Out of Scope
- Fan-out execution (parallel across targets)
- Network topology detection

### Deliverable
```bash
crabjar exec --target jambo "systemctl restart nginx"  # Requires approval
crabjar exec --target ftw3 "cargo build -p crabjar"    # Allowed by policy
```

### Implementation Steps
1. Add remote execution policy to guard crate
2. Implement target registry in `.crabjar_config.toml`
3. Add approval flow for high-risk remote commands
4. Test against multiple machines

---

## Phase 4: Telemetry & Polish

**Goal:** Complete audit trail and production readiness.

### In Scope
- Flight recorder events for all remote execution
- Network topology detection (LAN vs Tailscale)
- Error handling improvements
- Documentation and examples

### Out of Scope
- Interactive shell sessions
- File transfer operations

---

## Dependencies & Risks

### Key Dependencies
- russh crate (SSH client/server)
- secrets-bridge crate (already exists in crabjar)
- guard crate (already exists, needs remote policy extension)

### Known Risks
1. **russh exec race condition** — EOF without ExitStatus. Mitigation: implement the workaround from ssh-bastion-russh skill.
2. **Network connectivity** — Tailscale vs LAN addressing. Mitigation: test against both network types early.
3. **Key format compatibility** — russh may not support all SSH key formats. Mitigation: test with ed25519 first (most compatible).

### Testing Strategy
- Phase 1: Manual testing against jambo only
- Phase 2: Add automated tests with mock SSH server
- Phase 3: Integration tests across multiple machines
- Phase 4: Full regression suite including network failure scenarios

---

## Implementation Order

```
Phase 1 (Core) → Phase 2 (Secrets) → Phase 3 (Guard + Multi-Target) → Phase 4 (Telemetry)
     │                │                      │                          │
     ▼                ▼                      ▼                          ▼
  Working      Secure keys          Trust evaluation            Production ready
  end-to-end   management           + multi-machine             with audit trail
```

Each phase builds on the previous but is independently deployable and testable.