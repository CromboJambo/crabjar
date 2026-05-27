# Secrets Bridge Protocol — Full Specification

## Overview

The secrets bridge implements a zero-knowledge proof system for agent access to secrets.
The agent never sees raw secret values but can prove access through cryptographic hashes
and audit trails.

## Threat Model

**Attacker:**
- Has access to git repo (even private)
- Can read all tracked files
- Can run agent in session context
- Cannot access vault/ directory (gitignored)

**Defender:**
- Secrets encrypted with age (X25519)
- Private key never leaves vault
- Agent holds only public key + session token
- All access produces auditable proof

## Key Management

### Key Generation
```
openssl genpkey -algorithm X25519 -out session-<id>.priv
openssl pkey -in session-<id>.priv -pubout -out session-<id>.pub
```

### Key Tracking
- `session-<id>.pub` committed to git (public, versioned)
- `session-<id>.priv` stays in vault (gitignored)
- `session-<id>.json` contains metadata (hash, expiry, status)
- `session-<id>.log` contains audit trail

### Session Lifecycle
1. **Active**: Valid token, within expiry window
2. **Expired**: Past expiry time, needs regeneration
3. **Revoked**: Explicitly invalidated by user

## Secret Storage

### Encryption
```bash
# Encrypt
age -p <public_key> -o secret.age secret.txt

# Decrypt (only with private key)
age -d -i session-<id>.priv secret.age
```

### Format
```
Age: <encrypted_content>
```

### Hashing
```bash
# Generate proof hash
echo "$DECRYPTED" | openssl dgst -sha256
```

## Proof System

### Proof File Format
```json
{
  "secret": "database_url",
  "used_at": "2026-05-26T18:00:00Z",
  "hash": "sha256_of_decrypted_value",
  "session_id": "session-abc123",
  "public_key_hash": "sha256_of_public_key",
  "verified": true
}
```

### Verification
1. Agent receives proof file
2. Checks `hash` format (64 hex chars)
3. Checks `verified` is true
4. Checks `session_id` matches active session
5. Never sees raw secret value

## Audit Trail

### Log Format
```
TIMESTAMP | ACTION | SECRET | HASH | STATUS
```

### Actions
- `verify`: Secret access
- `revoke`: Session revocation
- `audit`: Audit review
- `init`: Vault initialization

### Monitoring
- Check `*.log` files for changes
- Compare secret hashes over time
- Prompt user on unauthorized changes

## Session Token

### Generation
Short-lived token derived from session keypair:
```bash
# Token is session_id + public_key_hash signed with private key
echo -n "$SESSION_ID:$PUBLIC_KEY_HASH" | openssl dgst -sha256 -hmac "$(cat session-<id>.priv)"
```

### Validation
1. Check token format (64 hex chars)
2. Verify against public key
3. Check expiry time
4. Check session status

## "Are You Sure?" Gate

### Trigger Conditions
- Secret file modified in last 24 hours
- New secret added to vault
- Session key rotated
- Audit log shows unusual patterns

### Prompt Format
```
=== Recent Changes Detected ===
Secrets modified in last 24 hours:
database_url
github_token

Are you sure these changes are authorized? [y/N]:
```

### Logging
```
TIMESTAMP | audit | confirmed/rejected | user | authorized/unauthorized
```

## Pattern Discovery

### fzf Integration
```bash
find vault/secrets/ -name "*.age" | \
    fzf --delimiter='|' --format='plain' --preview="..."
```

### rg Pattern Matching
```bash
find vault/secrets/ -name "*.age" | \
    rg "pattern" | \
    xargs -I {} basename {} .age
```

### Output Format
```json
{
  "name": "database_url",
  "hash_prefix": "a1b2c3d4",
  "last_accessed": "2026-05-26T17:00:00Z"
}
```

## Error Codes

| Code | Meaning | Fix |
|------|---------|-----|
| 0 | Success | — |
| 1 | Invalid session | Run bridge-init.sh |
| 2 | Secret not registered | Add to vault/secrets/ |
| 3 | Session expired | Revoke and regenerate |
| 4 | Decryption failed | Check private key |

## Maintenance

### Key Rotation
1. Run `bridge-init.sh` (creates new keypair)
2. Re-encrypt secrets with new public key
3. Update `active_session.txt`
4. Commit new `.pub` to git

### Audit Review
1. Run `bridge-audit.sh`
2. Review `*.log` files
3. Check for unauthorized changes
4. Confirm or reject changes

### Vault Backup
```bash
# Backup vault (encrypted)
tar -czf vault-backup.tar.gz vault/
# Encrypt backup
age -p <backup_key> -o vault-backup.tar.gz.age vault-backup.tar.gz
```

## Security Notes

- **Never** commit private keys or decrypted secrets
- **Always** validate session expiry before use
- **Always** log access attempts (success and failure)
- **Rotate** keys every 7 days or after suspected compromise
- **Monitor** audit logs for unusual patterns
- **Verify** proof hashes match expected format
