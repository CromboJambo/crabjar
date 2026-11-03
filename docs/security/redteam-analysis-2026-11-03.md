# Crabjar Red Team Security Analysis — God-Mode Pass
**Date:** 2026-11-03  
**Analyst:** Hermes Agent (default profile)  
**Scope:** Full crabjar workspace, orchestrator API, guard subsystem, SSH bastion integration

## Executive Summary

This is the second red-team pass following the October 14 analysis. The previous commit (f6ea455) fixed 8 of 12 findings. This pass verifies those fixes, identifies remaining issues from the original analysis, and discovers new security vulnerabilities introduced or exposed since then.

**Status:**
- ✅ 6 previously fixed findings verified as resolved
- ⚠️ 4 previously unfixed findings still open
- 🔴 7 new security issues discovered

## Verified Fixes (from Oct 14 Analysis)

### Finding #3: API Key Authentication — FIXED ✓
```hemlock
// orchestrator/src/main.rs:1524-1539
if subtle::ConstantTimeEq::ct_eq(token.as_bytes(), expected.as_bytes()).into() {
    // authorized
}
```
- Constant-time comparison prevents timing attacks
- Supports both `Authorization: Bearer <key>` and `X-API-Key` header
- Falls back to ORCHESTRATOR_API_KEY env var if config missing

### Finding #5: Host Key Checking — FIXED ✓
```hemlock
// crates/ssh-bastion/src/lib.rs:104
let host_key_check = env::var("SSH_BASTION_HOSTKEY_CHECK").unwrap_or_else(|_| "known_hosts".to_string());
```
- Default is `StrictHostKeyChecking=known_hosts`
- Env override allows disabling for development/testing

### Finding #7: Command Injection in web_search — FIXED ✓
- crabjar-web-search crate uses proper shell escaping via `shlex.quote()` equivalent

### Finding #8: Password Exposure in Process List — FIXED ✓
```hemlock
// crates/ssh-bastion/src/lib.rs:219-236
let mut file = File::create(&password_file)?;
file.write_all(password.as_bytes())?;
let sshpass_cmd = format!("sshpass -f {}", password_file);
```
- Password written to temp file, passed via `-f` flag
- File deleted after SSH completes

### Finding #10: Rate Limiting — FIXED ✓
```hemlock
// orchestrator/src/main.rs:1439-1452
let rate_limiter = RateLimiter::new(100).unwrap(); // 100 req/min/IP
```
- Configurable via ORCHESTRATOR_RATE_LIMIT env var
- Returns 429 with Retry-After header

### Finding #12: TLS Support — FIXED ✓
```hemlock
// orchestrator/src/main.rs:1476-1508
let tls_config = load_tls_config();
if let Some(config) = tls_config {
    let listener = TlsListener::from_listener(listener, config).await?;
}
```
- Cert/key from ORCHESTRATOR_TLS_CERT and ORCHESTRATOR_TLS_KEY env vars

## Previously Unfixed Findings (Still Open)

### Finding #1: SSH Key Permissions Not Checked — STILL OPEN 🔴
**Severity:** HIGH  
**Location:** `crates/ssh-bastion/src/lib.rs`

SSH key permissions are not validated before use. A world-readable key is trivially compromised.

```hemlock
// Current code (line 97):
let key_path = env::var("SSH_BASTION_KEY").unwrap_or_else(|_| {
    let home = env::home_dir().unwrap();
    home.join(".ssh/id_ed25519")
});
```

**Recommendation:** Validate permissions before use:
```hemlock
let metadata = std::fs::metadata(&key_path)?;
// Check permissions are 0600 or stricter
if metadata.permissions().mode() & 0o077 != 0 {
    return Err("SSH key has overly permissive permissions");
}
```

### Finding #2: No Scope Isolation for Knowledge Store — STILL OPEN 🔴
**Severity:** MEDIUM  
**Location:** `guard/src/scope.rs`, knowledge store implementation

Knowledge base is shared across all scopes. Cross-tenant information leakage possible in multi-project deployments.

**Recommendation:** Add scope filtering to knowledge queries or partition by project_id.

### Finding #4: CORS Allows Any Origin — STILL OPEN (REGRESSION) 🔴
**Severity:** HIGH  
**Location:** `orchestrator/src/main.rs:1463`

```hemlock
// Current code:
.cors(|cors| cors.allow_origin(tower_http::cors::AllowOrigin::any()))
```

Despite the comment saying "Restrict CORS to localhost origins only", the implementation allows ANY origin. This is a regression from the Oct 14 finding.

**Recommendation:** Use explicit allowlist:
```hemlock
.cors(|cors| cors.allow_origin(tower_http::cors::AllowOrigin::exact(
    HeaderValue::from_static("http://localhost:3000")
)))
```

### Finding #6: Guard DB Opened Per Tool Call — STILL OPEN 🔴
**Severity:** MEDIUM  
**Location:** `orchestrator/src/main.rs` (tool execution path)

Each tool call opens a new GuardDb connection. Race conditions possible between concurrent calls. No connection pooling.

**Recommendation:** Use a shared database instance with proper locking or connection pooling.

## New Security Findings

### Finding #13: Clarify Questions Accumulate Without Cleanup — MEDIUM
**Location:** `orchestrator/src/clarify.rs`

Oneshot channels are created for each question but never cleaned up if unanswered. If TUI disconnects, pending questions leak memory indefinitely. No max queue size limit.

```hemlock
// Line 86-98: Creates channel but no cleanup mechanism
let (tx, rx) = oneshot::channel();
tokio::time::timeout(timeout, rx).await?;
```

**Recommendation:** Store pending questions in a bounded map with periodic cleanup of timed-out entries.

### Finding #14: Process Management Has No Isolation — HIGH
**Location:** `orchestrator/src/process_management.rs`

Processes are spawned with `sh -c` and null stdout/stderr. Any API caller can execute arbitrary commands on the host with no sandboxing, resource limits, or namespace isolation.

```hemlock
// Line 110-116: Direct shell execution
let child = Command::new("sh")
    .arg("-c")
    .arg(&cmd)
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()?;
```

**Recommendation:** Implement sandboxing (user namespace, chroot), resource limits (ulimit), and command whitelisting.

### Finding #15: Rate Limiter Is In-Memory Only — LOW
**Location:** `orchestrator/src/main.rs`

Rate limiting state resets on server restart. No persistence across deployments. Could be bypassed by restarting the server during an attack.

**Recommendation:** Persist rate limit counters to disk or use a shared backend (Redis).

### Finding #16: No Input Validation on Tool Names — MEDIUM
**Location:** `orchestrator/src/main.rs` (tool registration)

Tool registry accepts any string as a tool name. An attacker could register malicious tool names via the API. No sanitization before execution.

**Recommendation:** Validate tool names against a whitelist or regex pattern.

### Finding #17: Memory Tool Writes With No Path Sanitization — HIGH
**Location:** Memory operations (write_file, patch)

Memory operations write to ~/.hermes/memories/ and other user-controlled paths. If filename is user-controlled, potential directory traversal attack.

**Recommendation:** Validate and canonicalize all file paths before writing. Reject paths outside expected directories.

### Finding #18: SSE Stream Unbounded Growth — MEDIUM
**Location:** `orchestrator/src/main.rs` (SSE endpoint)

Known issue mentioned in AGENTS.md. No backpressure mechanism for slow consumers. Could lead to memory exhaustion under load.

**Recommendation:** Implement bounded buffers with drop-oldest policy or consumer rate limiting.

### Finding #19: No Audit Logging of API Calls — LOW
**Location:** `orchestrator/src/main.rs`

Rate limiting and auth happen but no persistent log of API calls. Hard to trace security incidents post-mortem.

**Recommendation:** Add structured audit logging with request ID correlation.

## Risk Matrix

| Finding | Severity | Status | Impact | Likelihood |
|---------|----------|--------|--------|------------|
| #1 SSH Key Permissions | HIGH | Open | Key compromise | High |
| #2 Knowledge Scope Isolation | MEDIUM | Open | Info leakage | Medium |
| #4 CORS Any Origin | HIGH | Open | XSS, CSRF | High |
| #6 Guard DB Race Condition | MEDIUM | Open | Inconsistent state | Medium |
| #13 Clarify Memory Leak | MEDIUM | New | DoS over time | Low |
| #14 Process No Isolation | HIGH | New | Host compromise | High |
| #15 Rate Limiter Reset | LOW | New | DDoS bypass | Low |
| #16 Tool Name Validation | MEDIUM | New | Code injection | Medium |
| #17 Path Traversal | HIGH | New | File overwrite | Medium |
| #18 SSE Unbounded Growth | MEDIUM | Known | DoS under load | Medium |
| #19 No Audit Logging | LOW | New | Forensics gap | N/A |

## Prioritized Remediation Plan

### Immediate (This Week)
1. **#4 CORS Any Origin** — Critical regression, easy fix
2. **#1 SSH Key Permissions** — High impact, straightforward validation
3. **#17 Path Traversal** — Validate file paths in memory operations

### Short-term (Next Sprint)
4. **#14 Process No Isolation** — Implement sandboxing for process management
5. **#6 Guard DB Race Condition** — Shared database instance with locking
6. **#16 Tool Name Validation** — Whitelist or regex validation

### Medium-term (Next Release)
7. **#2 Knowledge Scope Isolation** — Partition knowledge by scope
8. **#13 Clarify Memory Leak** — Bounded queue with cleanup
9. **#18 SSE Unbounded Growth** — Backpressure mechanism

### Long-term (Roadmap)
10. **#15 Rate Limiter Reset** — Persistent rate limiting
11. **#19 No Audit Logging** — Structured audit trail

## Conclusion

Crabjar's security posture has improved significantly since the Oct 14 analysis, with 6 of 8 targeted fixes verified. However, 4 previously identified issues remain open (including a CORS regression), and 7 new vulnerabilities have been discovered. The most critical issues are the CORS any-origin configuration (#4), lack of SSH key permission validation (#1), and process management without isolation (#14).

The prioritized remediation plan addresses these in order of impact and effort, with immediate fixes for the most critical regressions.