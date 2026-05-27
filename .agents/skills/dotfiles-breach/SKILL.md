---
name: dotfiles-breach
description: |
  Handle secrets exposure in dotfiles repos (even private ones). Use whenever the user
  mentions secrets leaking, repo exposure, scraper risk, private repo not private, git
  leak, credential exposure, or wants to audit/secure their dotfiles before pushing.
  Also trigger when the user says "check my repo for secrets", "is my dotfiles safe",
  "scrappers can see", "agent scraping", or shares a repo URL they plan to push.
  Trigger on any concern about git repo privacy, even if the repo is "private".
---

# Dotfiles Breach Protocol

## When to activate

You are the incident response for dotfiles/secrets exposure. The user's dotfiles are
committed to a private GitHub repo, but scrapers and AI agents can access private repos
programmatically. This conversation is the working record.

## Severity Classification

| Level | Criteria | Example |
|-------|----------|---------|
| **Critical** | Live cryptographic material, private keys, authentication secrets | X25519 key pairs, SSH keys, TLS certs, RustDesk key material |
| **High** | Active credentials, connection strings with passwords, API tokens | Database URLs, obs-websocket passwords, GitHub PATs |
| **Medium** | Network topology, machine IDs, placeholder credentials | LAN IPs, remote IDs, Brave API key fields |
| **Low** | Binary auth state, session data, metadata | Pulse cookies, dconf DBs, browser localStorage |

## Remediation Workflow

### Phase 1: Detect

1. Scan all config files for secrets patterns — keys, tokens, passwords, connection strings
2. Check `.gitignore` completeness — an empty or missing `.gitignore` is a structural failure
3. Classify each finding by severity level above
4. Report findings organized by severity

### Phase 2: Rotate (Critical/High)

For **Critical** findings (live keys):
1. Generate new material on the **live machine** (never in the repo copy)
2. Replace the old material in the live config (not the committed copy)
3. Clear the old material from the committed config
4. Add a removal note explaining what was replaced

For **High** findings (credentials):
1. Clear the credential from the committed config
2. Add removal note with instructions for re-setting (env var, UI, etc.)
3. Never write placeholder values that look like real credentials

### Phase 3: Prevent

1. Write/update `.gitignore` with patterns for:
   - Key material files (`*.pem`, `*.key`, `*.p12`)
   - Generated configs (RustDesk, browser storage, session state)
   - `.env` files and secrets directories
   - Binary auth files (pulse, dconf)
2. Add a comment at the top explaining the policy
3. Verify existing files that should be ignored are not tracked

### Phase 4: Document

1. Record all changes made (what was removed, what was replaced)
2. Note any action items the user must complete (key rotation, credential re-setting)
3. Update the working record if this conversation becomes the reference

## Key Principles

- **Never write secrets to the repo** — even as placeholders, even as "examples"
- **Generate keys on the live machine** — the repo copy should have empty/removed material
- **Use env var references** — `DATABASE_URL`, `GITHUB_TOKEN`, `BRAVE_API_KEY` etc.
- **Add removal notes** — explain what was removed and how to re-set it
- **`.gitignore` is non-optional** — an empty `.gitignore` is a confirmed vulnerability

## Post-Incident Checklist

- [ ] All critical keys regenerated on live machine
- [ ] All high-severity credentials cleared from repo
- [ ] `.gitignore` updated with coverage for all secret patterns
- [ ] User has action items for re-setting credentials via env vars or UI
- [ ] Committed `.gitignore` before committing sanitized configs
- [ ] Removal notes added to all sanitized config files
