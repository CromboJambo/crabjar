# Crabjar Security Red Team Analysis

**Date:** 2026-10-14  
**Scope:** Full codebase security audit (execution pipeline, SSH bastion, orchestrator, guard gate, network exposure)  
**Methodology:** Adversarial code review — assume attacker has network access to the deployment environment

---

## Executive Summary

Crabjar has a well-designed security architecture on paper (trust layers, provenance requirements, scope isolation), but several implementation gaps create exploitable attack surfaces. The most critical issues are in the SSH bastion and orchestrator authentication layers.

**Risk Rating:** HIGH — multiple medium-severity vulnerabilities that chain together for significant impact.

---

## Critical Findings

### 1. SSH Bastion: Password Exposed in Process List (CVSS ~7.5)

**Location:** `crates/ssh-bastion/src/lib.rs:168-170`

```rust
let mut cmd = Command::new("sshpass");
cmd.args(["-p", password, "-o", "StrictHostKeyChecking=accept-new"]);
```

**Vulnerability:** `sshpass -p 'password'` passes the password as a command-line argument, which is visible to any local user via `/proc/<pid>/cmdline` or `ps aux`. On a shared system (like your lab machines), this leaks credentials.

**Exploit Scenario:** Attacker with local access to any lab machine runs `ps aux | grep sshpass` during an exec operation and captures the SSH password for all bastion targets.

**Fix:** Use `sshpass -f /dev/fd/3` with a file descriptor, or write to a temp file with restrictive permissions:
```rust
let passfile = NamedTempFile::new()?;
passfile.write_all(password.as_bytes())?;
cmd.args(["-f", passfile.path().to_str().unwrap(), ...]);
// Keep passfile alive for duration of command
```

### 2. SSH Bastion: Accepts Any Password (CVSS ~8.0)

**Location:** `crates/ssh-bastion/src/handler.rs:49-51`

```rust
fn auth_password(&mut self, name: &str, pass: &str) -> bool {
    // For now, accept any password
    true
}
```

**Vulnerability:** The SSH server accepts any username and any password. There is no credential validation.

**Exploit Scenario:** Attacker connects to the bastion's SSH port with arbitrary credentials (`ssh user@bastion -p 2222`), gets a shell, and can execute commands on all targets through the `exec <target> <command>` interface.

**Fix:** Implement actual password verification against a credential store or Bitwarden vault:
```rust
fn auth_password(&mut self, name: &str, pass: &str) -> bool {
    match self.credentials.verify(name, pass) {
        Ok(true) => true,
        _ => false,
    }
}
```

### 3. Orchestrator: No Authentication Layer (CVSS ~7.0)

**Location:** `orchestrator/src/main.rs` — all endpoint handlers

**Vulnerability:** The orchestrator HTTP API has no authentication middleware. Any client that can reach the TCP port can call all endpoints (`/api/tasks`, `/api/exec`, `/api/knowledge/write`, etc.).

**Evidence:**
- No auth middleware in route definitions (lines 370-465)
- No Bearer token validation on any endpoint
- CORS is permissive (line 1340: `CorsLayer::permissive()`)

**Exploit Scenario:** Attacker on the same network sends HTTP requests to the orchestrator port, creates/executes tasks, reads knowledge base, or writes malicious state.

**Fix:** Add authentication middleware:
```rust
let app = Router::new()
    .route("/api/tasks", post(create_task).get(list_tasks))
    // ... other routes
    .layer(Extension(ApiKeyAuth::new("your-api-key")));
```

### 4. Orchestrator: Permissive CORS (CVSS ~5.0)

**Location:** `orchestrator/src/main.rs:1340`

```rust
.layer(CorsLayer::permissive())
```

**Vulnerability:** Any web page from any origin can make cross-origin requests to the orchestrator API. Combined with finding #3, this allows a malicious website visited by the user to interact with the crabjar API.

**Exploit Scenario:** User visits attacker-controlled website → JavaScript makes fetch requests to `http://localhost:8080/api/exec` → executes arbitrary commands through crabjar.

**Fix:** Restrict CORS to known origins or remove if not needed for browser access:
```rust
.layer(CorsLayer::new().allow_origin("http://localhost:3000".parse::<Origin>().unwrap()))
```

### 5. Command Risk Assessment: Weak Matching (CVSS ~4.0)

**Location:** `guard/src/gate.rs:430-450`

```rust
fn assess_command_risk(&self, command: &str, args: &[String]) -> CommandRisk {
    let basename = command.split('/').next_back().unwrap_or(command);
    for risk_cmd in &self.risk_config.high_risk {
        if basename.eq_ignore_ascii_case(risk_cmd) { ... }
        let full_cmd = format!("{} {}", basename, args.join(" "));
        if full_cmd.eq_ignore_ascii_case(risk_cmd) { ... }
    }
```

**Vulnerability:** Risk assessment only checks exact command names and full command strings. It doesn't detect:
- Aliases (`rm -rf` vs `trash`)
- Script wrappers (`./my-script.sh` that runs dangerous commands)
- Arguments that change semantics (`cp` is low-risk, but `cp /dev/sda /tmp/file` is catastrophic)

**Exploit Scenario:** Attacker creates a script named `safe-backup.sh` that actually formats a disk. The guard lets it through because the basename isn't in the high-risk list.

**Fix:** Add argument-aware risk assessment and consider using a more sophisticated command parser.

### 6. CrossScopeAuth: Auto-Approval Without Review (CVSS ~4.5)

**Location:** `guard/src/scope.rs:171-184`

```rust
pub fn auto_for_scopes(actor: &Scope, target: &Scope) -> Option<Self> {
    if actor == target { return None; }
    if actor.can_access(target) { return None; }
    Some(Self::new(
        target.clone(),
        actor.clone(),
        "auto-cross-scope",
        "system-default-policy",
    ))
}
```

**Vulnerability:** When called by the orchestrator (which it is, see `orchestrator/src/main.rs:384-385`), this auto-approves cross-scope operations without human review. The entire purpose of scope isolation is defeated.

**Evidence from orchestrator:**
```rust
let cross_scope_auth = crabjar_guard::CrossScopeAuth::auto_for_scopes(&actor_scope, &target_scope);
```

This is called for EVERY exec operation, regardless of whether the scopes actually differ or need approval.

**Fix:** Only auto-approve when scopes are identical; require explicit approval for genuine cross-scope operations:
```rust
if actor != target && !actor.can_access(target) {
    // Require human approval or policy evaluation
    return GateResult::Pending;
}
```

### 7. Provenance Check: Existence Only, No Freshness (CVSS ~3.5)

**Location:** `guard/src/gate.rs:364-374`

```rust
fn verify_provenance(&self, id: &str) -> Result<bool, GuardDbError> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM action_requests WHERE source_event_id = ?1)",
        ...
    );
}
```

**Vulnerability:** The provenance check only verifies that a `source_event_id` exists in the database, not that it's fresh or relevant. An old event ID could be replayed to authorize new actions.

**Exploit Scenario:** Attacker captures a valid `source_event_id` from logs and replays it for unauthorized commands.

**Fix:** Add timestamp validation:
```sql
SELECT EXISTS(SELECT 1 FROM action_requests WHERE source_event_id = ?1 AND created_at > datetime('now', '-5 minutes'))
```

---

## Medium Findings

### 8. Bitwarden CLI: Password in Command Line

**Location:** `src/bitwarden/cli.rs:18-20`

```rust
pub fn login(email: &str, master_password: &str, server: Option<&str>) -> BitwardenResult<()> {
    cmd.arg("login").arg(email).arg(master_password);
}
```

Same issue as finding #1 — master password visible in process list.

### 9. Orchestrator API Token: Optional and Not Enforced

**Location:** `orchestrator/src/lm_studio_client/types.rs:71`

The LM Studio client supports an optional `api_token`, but the orchestrator itself doesn't require authentication for its own endpoints.

### 10. No Rate Limiting on Orchestrator Endpoints

Any endpoint can be hammered with requests, potentially exhausting resources or flooding the task queue.

---

## Low Findings

### 11. SSH Host Key Checking: `accept-new`

**Location:** `crates/ssh-bastion/src/lib.rs:169`

```rust
cmd.args(["-o", "StrictHostKeyChecking=accept-new"]);
```

Accepts new host keys without verification, enabling MITM attacks on first connection. Consider `yes` for automation or manual review for new hosts.

### 12. No TLS on Orchestrator HTTP API

The orchestrator uses plain HTTP, not HTTPS. Traffic is unencrypted and susceptible to sniffing on the local network.

---

## Architecture Observations (Positive)

- **Trust layer model** is well-designed with proper separation of concerns
- **Provenance requirement** (detection ≠ authorization) is a strong security principle
- **Scope isolation** concept is correct, even if implementation has gaps
- **Command risk assessment** provides a good baseline for auto-execution decisions
- **Dry-run mode** allows safe testing of execution pipelines

---

## Recommendations by Priority

### Immediate (Critical Path)
1. Fix SSH bastion password handling (findings #1, #2) — use file descriptors and real credential validation
2. Add authentication to orchestrator API (finding #3) — even a simple API key middleware
3. Restrict CORS (finding #4)

### Short-term (This Week)
4. Fix CrossScopeAuth auto-approval logic (finding #6)
5. Add provenance freshness check (finding #7)
6. Improve command risk assessment (finding #5)

### Medium-term (This Month)
7. Add TLS to orchestrator API
8. Implement rate limiting
9. Add comprehensive security test suite covering all findings

---

## Attack Chain Example

An attacker with network access to your lab could:

1. Visit a malicious website → JavaScript exploits permissive CORS (#4)
2. Calls orchestrator `/api/exec` without auth (#3)
3. Orchestrator auto-approves cross-scope operation (#6)
4. Command executes via SSH bastion with password in process list (#1)
5. Attacker captures SSH credentials by running `ps aux` on any lab machine
6. Attacker now has persistent access to all bastion targets

Each individual vulnerability is medium-severity, but chained together they create a critical attack path.

---

## Testing Recommendations

Add integration tests that:
- Verify SSH passwords are not visible in process list
- Test orchestrator auth middleware rejects unauthenticated requests
- Validate CrossScopeAuth requires explicit approval for different scopes
- Check provenance freshness with expired event IDs
- Test command risk assessment with various argument combinations
