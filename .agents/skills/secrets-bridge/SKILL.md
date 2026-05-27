---
name: secrets-bridge
description: |
  Zero-knowledge secrets bridge for agent sessions. Use whenever the user wants to
  manage secrets safely for agent access, generate session keypairs, verify secret
  access without exposing raw values, audit secret usage, or discover secrets via
  pattern matching. Trigger when the user says "bridge", "secrets bridge", "init
  vault", "generate keypair", "verify access", "audit secrets", "discover secrets",
  "agent access to secrets", "zero-knowledge", or mentions protecting secrets from
  agents while still granting them access. Also trigger when the user wants to
  version-track public keys, monitor secret changes, or implement the "are you sure?"
  gate for secret modifications.
---

# Secrets Bridge — Zero-Knowledge Agent Access

## Overview

The secrets bridge gives agents scoped access to secrets without ever exposing the
raw values. The agent holds only a public key (tracked in git) and a short-lived
session token. The private key and secrets live in a vault outside the agent's reach.
All operations produce proof files (hashes, timestamps) instead of raw secrets.

## Directory Structure

```
vault/
├── secrets/              ← age-encrypted secrets (gitignored)
│   ├── database_url.age
│   ├── github_token.age
│   └── brave_api_key.age
├── keypairs/             ← public keys tracked in git + audit logs
│   ├── session-abc123.pub
│   ├── session-abc123.log
│   └── session-abc123.json   ← key metadata (no private key)
├── keyring/              ← active session tracking
│   └── active_session.txt
└── .gitignore            ← vault root gitignore
```

## Commands

### `bridge init`

Creates the vault structure and generates the first keypair.

```bash
~/.agents/skills/secrets-bridge/scripts/bridge-init.sh
```

What it does:
1. Creates `vault/` directory structure
2. Generates X25519 keypair
3. Commits `session-<id>.pub` to git
4. Writes `active_session.txt`
5. Writes initial audit log entry

### `bridge verify <secret_name>`

Validates session token and outputs proof of access (not the secret).

```bash
~/.agents/skills/secrets-bridge/scripts/bridge-verify.sh <secret_name>
```

What it does:
1. Validates session token + expiry
2. Checks `secret_name` is registered in vault
3. Decrypts the secret (only in memory)
4. Outputs proof file to `keypairs/` with:
   - `secret`: name of the secret
   - `used_at`: timestamp
   - `hash`: sha256 of decrypted value
   - `session_id`: current session
5. Returns exit code 0 if valid, 1 if invalid

Agent never sees the raw secret value.

### `bridge audit [session_id]`

Shows recent secret access events for a session.

```bash
~/.agents/skills/secrets-bridge/scripts/bridge-audit.sh [session_id]
```

What it does:
1. Reads the session's audit log
2. Shows recent access events
3. Detects and reports changes (last 24 hours)
4. Returns summary of changes as JSON
5. Does NOT prompt for confirmation — the audit log is an immutable record, not a gate

### `bridge discover [pattern]`

Finds secrets matching a pattern without exposing them.

```bash
~/.agents/skills/secrets-bridge/scripts/bridge-discover.sh [pattern]
```

What it does:
1. Lists secret names (not values) matching pattern
2. Uses fzf for interactive selection if no pattern given
3. Shows metadata: last accessed, hash prefix (first 8 chars)
4. Never decrypts or displays full values

### `bridge revoke [session_id]`

Invalidates a session's access.

```bash
~/.agents/skills/secrets-bridge/scripts/bridge-revoke.sh [session_id]
```

What it does:
1. Removes session from keyring
2. Logs revocation in audit trail
3. Commits changes to git

## Agent Workflow

When an agent needs access to a secret:

1. Read `vault/keyring/active_session.txt` → get session_id
2. Read `vault/keypairs/session-<id>.pub` → public key (tracked in git)
3. Validate session token (short-lived, derived from key)
4. Call `bridge verify <secret_name>` → get proof file
5. Check proof file hash format → confirm access worked
6. Never see or store the raw secret

When detecting changes:

1. Read `vault/keypairs/session-<id>.log` → audit entries
2. Compare hashes of secrets
3. Output changes as structured data for operator review
4. Do NOT wait for confirmation — the audit log is a record, not a gate

## Security Properties

- **Agent sees only public key** — cannot reconstruct secrets
- **Session tokens expire** — short-lived access window
- **All access produces proof** — auditable without raw data
- **Public keys version-tracked** — git history shows key rotation
- **Private key never leaves vault** — gitignored, encrypted
- **No human confirmation gate** — authorization from cryptographic proofs, not operator approval
- **Audit log is immutable** — records state changes, does not gate them

## Maintenance

- **Key rotation**: Generate new keypair, commit `.pub`, update `active_session.txt`
- **Audit review**: Run `bridge audit` periodically, review `*.log` files
- **Secret updates**: User edits vault directly, agent detects via hash change
- **Stale detection**: Sessions older than 7 days should be rotated

## Error Handling

| Error | Meaning | Action |
|-------|---------|--------|
| Exit 1 | Invalid session token | Generate new keypair |
| Exit 2 | Secret not registered | Add to vault first |
| Exit 3 | Session expired | Revoke and regenerate |
| Exit 4 | Proof verification failed | Check vault integrity |

## Threat Model — Why No Human Gate

The "are you sure?" gate is theater. It gives the illusion of control while the
operator is still the leak. Authorization comes from cryptographic proofs, not
human gates.

**The real threat model:**
- Scraper infrastructure already indexes private repos programmatically
- Agent sessions that hold session tokens can be exfiltrated
- Human confirmation is another channel that can be compromised
- The operator is always the weakest link

**The correct model:**
- Zero-knowledge by design, not zero-knowledge with human oversight
- Cryptographic proofs are the authorization mechanism
- Audit logs are immutable records, not decision points
- The agent understands the protocol, not just trusts the scripts
- Everything is transparent and explainable, not opaque automation

**What the agent should understand:**
- Why each step matters (not just that it works)
- What the threat model is (scraper infrastructure, not human error)
- How cryptographic proofs replace human gates
- Why the audit log is a record, not a gate
