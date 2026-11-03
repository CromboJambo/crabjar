---
name: crabjar-health-check
description: Use when a deployed app needs to be verified as running, after every deployment, or on-demand to confirm availability — crabjar guard-backed verification
version: 1.0.0
tags: [health, monitoring, ops, crabjar, guard]
metadata:
  crabjar:
    requires_commands: [crabjar]
    depends_on: [guard, telemetry]
---

## Overview

Three-layer health check: app endpoint → dependency check → log scan. Uses crabjar's guard for verification and telemetry for logging. Reports a full picture, not just "is it up."

## When to Use

- After every deployment (called by post-deploy workflow)
- Scheduled periodically by an ops agent cron
- On-demand after an incident to verify recovery

## Prerequisites

- Deployed app URL (from memory key `last-deployment-url`, or provided directly)
- `SUPABASE_URL` in environment (for Supabase check, if applicable)
- Any other dependency URLs/credentials

## Procedure

### 1. App health endpoint

```bash
curl -s -o /tmp/health_body.json -w "%{http_code} %{time_total}" \
  [url]/api/health
```

Validate:
- HTTP 200
- Body parses as JSON with `"status": "ok"`
- Response time < 3000ms (warn if > 1000ms)

On 404 → health endpoint missing, run bootstrap.
On timeout → retry once (cold start), then fail.

### 2. Dependency checks

Database (if applicable):
```bash
curl -s -o /dev/null -w "%{http_code}" \
  "$SUPABASE_URL/rest/v1/" \
  -H "apikey: $SUPABASE_ANON_KEY"
```

- HTTP 200 → reachable
- HTTP 401/403 → reachable but key issue
- Timeout / 5xx → dependency incident

### 3. Log scan (last N lines)

```bash
curl -s "[log-endpoint]/last?limit=50" | grep -E "Error:|FATAL|500|502"
```

Scan for:
- `Error:` or `Unhandled` — application errors
- `FATAL` — process crashes
- Response times > 5000ms — performance issues
- Status 500 or 502 — server errors

Flag any of these in the report. Do not flood — summarize ("3 errors in the last 50 requests, all related to auth").

### 4. Record via crabjar guard

```bash
crabjar guard record \
  --type health-check \
  --result [pass|fail] \
  --detail '{"endpoint":"PASS","deps":"PASS","logs":"CLEAN"}' \
  --timestamp "$(date -Iseconds)"
```

### 5. Report

```
Health check — [timestamp]
────────────────────────────────────
App endpoint:   PASS / FAIL  ([ms]ms)
Dependencies:   PASS / FAIL
Logs:           CLEAN / [n] errors detected

[If anything failed or logged errors:]
  Details: [plain-English summary]
  Action:  [what the agent will do next]
```

### 6. On any FAIL

- Save to memory: key=`health-failure-log`, append `{ timestamp, layer, detail }`
- Send notification with failure summary
- If logs show 500s: pull more logs and identify the failing route
- If dependency down: check status page, notify user, do not attempt operations

## Pitfalls

- Cold starts cause slow first requests — always retry once before failing.
- Check dependency status pages before alerting — the issue may be on their end.
- Do not report routine 200s or OPTIONS requests as errors.
- Always record the result via `crabjar guard record` for audit trail.

## Verification

Full report printed with all layers checked. Any failures logged to guard and memory.
