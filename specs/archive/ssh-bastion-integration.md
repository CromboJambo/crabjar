# Crabjar SSH Bastion Integration — Multi-Machine Remote Execution

## Overview

Integrate crabjar's execution pipeline with an SSH bastion for secure multi-machine remote execution across LAN and Tailscale networks. This enables crabjar to execute commands on home-lab machines (jambo, ftw3, etc.) with proper trust evaluation, secret management, and audit trails.

## Architecture

```
crabjar exec --target jambo "systemctl restart service"
        │
        ▼
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   Guard     │────▶│ Secrets      │────▶│ SSH Bastion │
│ Evaluation  │     │ Bridge       │     │ (russh)     │
│ (trust +    │     │ (SSH keys,   │     │             │
│  scope)     │     │  host keys)  │     └─────┬───────┘
└─────────────┘     └──────────────┘           │
        │                                      ▼
        └──────────────────────────────▶ Remote Machine (jambo/ftw3/etc.)
```

## Integration Points

### 1. Guard System — Remote Execution Policy

Add remote execution policy to guard evaluation:

```toml
# .crabjar_config.toml
[execution.remote]
allowed_targets = ["jambo", "ftw3", "pixel9a"]
require_approval = true  # For destructive commands on remote machines
max_concurrent = 3       # Limit parallel remote sessions
timeout_seconds = 300    # Per-command timeout
```

Guard evaluates:
- Is the target in `allowed_targets`?
- Does the command require approval (based on risk classification)?
- Are we within concurrent session limits?

### 2. Secrets Bridge — SSH Key Management

Store SSH keys and host keys in vault:

```
vault/secrets/
├── ssh_keys/
│   ├── jambo_ed25519.age      # Private key for jambo
│   ├── ftw3_ed25519.age       # Private key for ftw3
│   └── pixel9a_rsa.age        # Private key for Pixel 9a (Termux)
├── ssh_host_keys/
│   ├── jambo.pub.age          # Expected host key
│   └── ftw3.pub.age           # Expected host key
```

Session workflow:
1. `crabjar exec --target jambo` triggers guard evaluation
2. Guard passes → secrets bridge provides SSH key for jambo
3. Bastion establishes connection with proper authentication
4. Command executes, output captured
5. Session closed, audit trail written

### 3. SSH Bastion Crate — crabjar-ssh-bastion

New crate implementing the bastion logic using russh:

```rust
// crates/ssh-bastion/src/lib.rs
pub struct RemoteExecRequest {
    pub target: String,        // Machine name (jambo, ftw3, etc.)
    pub command: String,       // Command to execute
    pub ssh_key: Vec<u8>,      // Decrypted private key
    pub host_key: Option<Vec<u8>>,  // Expected host key for verification
    pub timeout: Duration,
}

pub struct RemoteExecResult {
    pub exit_status: u32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
    pub target: String,
}

pub async fn execute_remote(request: RemoteExecRequest) -> Result<RemoteExecResult, BastionError> {
    // Establish SSH connection using russh
    // Execute command
    // Capture output and exit status
    // Handle the exec race condition (EOF without ExitStatus)
}
```

Key implementation details from ssh-bastion-russh skill:
- Use dedicated `exec_request` callback for command execution
- Handle EOF without ExitStatus by sending default exit status to client
- Establish outbound connection lazily when exec request arrives
- Support multi-target routing via configuration

### 4. Telemetry — Remote Execution Audit

Flight recorder captures all remote execution events:

```json
{
    "event": "remote_exec",
    "timestamp": "2026-10-14T18:30:00Z",
    "target": "jambo",
    "command": "systemctl restart nginx",
    "exit_status": 0,
    "duration_ms": 1250,
    "guard_decision": "allowed",
    "trust_layer": "verified",
    "session_id": "sess-abc123"
}
```

## CLI Interface

```bash
# Execute on remote machine
crabjar exec --target jambo "systemctl status nginx"

# With explicit timeout
crabjar exec --target ftw3 --timeout 60 "cargo build -p crabjar"

# List available targets
crabjar targets list

# Add new target (stores SSH config in vault)
crabjar targets add pixel9a --host 192.168.50.218 --user crombo --key ~/.ssh/id_pixel

# Execute on multiple machines (fan-out)
crabjar exec --targets jambo,ftw3 "df -h /"
```

## Target Configuration

Machine registry stored in `.crabjar_config.toml`:

```toml
[targets.jambo]
host = "192.168.50.218"      # LAN IP or Tailscale name
user = "crombo"
port = 22
key_path = "vault/secrets/ssh_keys/jambo_ed25519.age"
network = "lan"             # lan | tailscale

[targets.ftw3]
host = "ftw3.tailnet.xyz"   # Tailscale hostname
user = "crombo"
port = 22
key_path = "vault/secrets/ssh_keys/ftw3_ed25519.age"
network = "tailscale"

[targets.pixel9a]
host = "pixel9a.tailnet.xyz"
user = "crombo"
port = 8022                 # Termux sshd port
key_path = "vault/secrets/ssh_keys/pixel9a_rsa.age"
network = "tailscale"
```

## Network Topology Support

### LAN (192.168.50.x)
- Direct SSH connections
- Lower latency, higher bandwidth
- Used for jambo and local machines

### Tailscale
- Magic DNS resolution (ftw3.tailnet.xyz)
- Works across networks
- Used for ftw3 and Pixel 9a
- Requires tailscale daemon running on bastion host

### Hybrid
- Crabjar automatically detects network type from target config
- Retries with alternate address if primary fails
- Logs which network path was used

## Error Handling

| Error | Meaning | Recovery |
|-------|---------|----------|
| `TargetNotFound` | Machine not in registry | Add target via `crabjar targets add` |
| `ConnectionRefused` | SSH service not running | Check machine status, network connectivity |
| `AuthFailed` | Wrong key or host key mismatch | Rotate keys in vault, verify host key |
| `Timeout` | Command exceeded timeout | Increase timeout, check machine load |
| `GuardDenied` | Trust evaluation failed | Review guard policy, trust layers |

## Security Model

1. **Key Management**: SSH keys encrypted with age, stored in vault
2. **Host Key Verification**: Expected host keys stored and verified on connection
3. **Session Isolation**: Each remote exec gets its own SSH session
4. **Audit Trail**: All remote execution logged with guard decisions
5. **Least Privilege**: Targets only have access to machines they're configured for

## Implementation Phases

### Phase 1: Core Remote Execution (Week 1-2)
- Create `crabjar-ssh-bastion` crate
- Implement single-target command execution
- Integrate with secrets bridge for key management
- Basic CLI interface (`--target` flag)

### Phase 2: Guard Integration (Week 3)
- Add remote execution policy to guard evaluation
- Trust layer integration
- Approval flows for destructive remote commands

### Phase 3: Multi-Machine & Fan-Out (Week 4)
- Target registry and management CLI
- Parallel execution across multiple machines
- Network topology detection and retry logic

### Phase 4: Advanced Features (Week 5+)
- Interactive shell sessions (`crabjar shell --target jambo`)
- File transfer operations
- Session persistence for long-running remote tasks

## Testing Strategy

1. **Unit Tests**: SSH connection establishment, command execution, output capture
2. **Integration Tests**: End-to-end exec on real lab machines (jambo first)
3. **Network Tests**: LAN vs Tailscale connectivity, timeout handling
4. **Security Tests**: Key rotation, host key verification, session isolation

## Dependencies

- `russh` — SSH client/server library
- `age` — Encryption for vault secrets
- Existing crabjar crates: guard, secrets-bridge, telemetry

## Success Criteria

- Can execute commands on jambo and ftw3 from crabjar CLI
- Guard system properly evaluates remote execution requests
- SSH keys managed through secrets bridge (not exposed in plaintext)
- Audit trail captures all remote execution events
- Handles both LAN and Tailscale network topologies