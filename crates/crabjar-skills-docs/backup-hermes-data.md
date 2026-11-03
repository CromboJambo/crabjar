---
name: crabjar-backup-data
description: Use when workspace state, memory, skills, or kanban data needs to be backed up before a major change, on a schedule, or after a crash recovery
version: 1.0.0
tags: [backup, ops, recovery, crabjar, telemetry]
metadata:
  crabjar:
    requires_commands: [crabjar]
    depends_on: [telemetry, state-docs]
---

## Overview

Backs up all critical data (Hermes memory, skills, kanban state, crabjar workspace state) using crabjar's telemetry for state capture and tar for packaging.

## When to Use

- Before `crabjar update` or any version upgrade
- On a scheduled cron as routine protection
- After an agent crash to capture last-known-good state
- Before destructive operations (profile reset, skill purge)

## Prerequisites

- `~/.hermes/` exists with data
- One of: `aws` CLI + S3 bucket, `rclone` configured, or a local backup path
- crabjar installed

## Procedure

**1. Capture crabjar telemetry state:**

```bash
crabjar telemetry snapshot --output /tmp/crabjar-snapshot-$(date +%Y%m%d-%H%M%S).json
```

**2. Snapshot to local file:**

```bash
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
BACKUP_FILE="/tmp/crabjar-backup-$TIMESTAMP.tar.gz"
tar -czf "$BACKUP_FILE" \
  -C "$HOME" \
  --exclude='.hermes/cache/playwright' \
  --exclude='.hermes/cache/browser' \
  .hermes crabjar-config state-docs knowledge-store guard-db telemetry-store
echo "Backup written: $BACKUP_FILE ($(du -sh "$BACKUP_FILE" | cut -f1))"
```

**3. Ship to remote — choose one:**

S3:
```bash
aws s3 cp "$BACKUP_FILE" "s3://$BACKUP_BUCKET/crabjar/$TIMESTAMP.tar.gz"
```

Rclone (Dropbox, Google Drive, Backblaze, etc.):
```bash
rclone copy "$BACKUP_FILE" "$RCLONE_REMOTE:crabjar-backups/"
```

Local only (keep last 7):
```bash
BACKUP_DIR="${BACKUP_DIR:-$HOME/crabjar-backups}"
mkdir -p "$BACKUP_DIR"
cp "$BACKUP_FILE" "$BACKUP_DIR/"
ls -t "$BACKUP_DIR"/*.tar.gz | tail -n +8 | xargs -r rm
```

**4. Log to memory:**

Save to memory: key=`last-backup`, value=`{ timestamp, destination, size }`.

## Environment variables

| Variable | Required for | Example |
|---|---|---|
| `BACKUP_BUCKET` | S3 | `my-crabjar-backups` |
| `RCLONE_REMOTE` | rclone | `dropbox` |
| `BACKUP_DIR` | local only | `/mnt/backups` |

Set at least one. If none set, backup stays in `/tmp/` — ephemeral, not useful for disaster recovery.

## Cron setup (recommended)

Daily at 3am:
```bash
crabjar cron add "0 3 * * *" "Run crabjar-backup-data"
```

## Restore

```bash
crabjar stop
tar -xzf crabjar-backup-[timestamp].tar.gz -C "$HOME"
crabjar start
crabjar kanban init
```

## Pitfalls

- `~/.hermes/` can exceed 500MB if Playwright Chromium is installed. The `--exclude` flags above skip it.
- `~/.hermes/memory/` is the most critical directory — if nothing else, back that up.
- Never restore over a running instance — stop first.
- crabjar telemetry snapshots should be included in every backup for full state recovery.

## Verification

- Tarball exists at destination
- `tar -tzf [file] | grep memory/MEMORY.md` confirms memory is included
- `tar -tzf [file] | grep telemetry` confirms telemetry data is included
- Memory shows `last-backup` key updated to today's date
