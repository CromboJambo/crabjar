---
name: post-update-audit
description: |
  Investigate system state after pacman/paru updates. Check pacman logs for upgrade timeline, boot logs for post-reboot behavior, failed services, and fstab/EFI boot order issues. Use whenever the user mentions updates took long, strange boot behavior, fstab duplicates, EFI boot order, PXE network boot, or says "post-update-audit", "check after update", "audit update", or "what happened during the update". Also trigger when the user shares a long update run or reboot cycle with odd behavior.
---

# Post-Update Audit

## Workflow

### 1. Check pacman log timeline

Read `/var/log/pacman.log`. Focus on:
- kernel update timestamps and mkinitcpio rebuild duration
- full system upgrade start/end timestamps
- paru individual upgrades
- orphan removal attempts

Run:
```
tail -100 /var/log/pacman.log
rg "transaction completed" /var/log/pacman.log | grep recent timestamps
```

### 2. Check boot logs

Run `journalctl --boot=0` for current boot and `journalctl --boot=-1` for pre-reboot boot. Filter:
- kernel messages (e820/BIOS/ACPI — skip these, they're verbose noise)
- powerdevil udev events (drm device changes during kernel update)
- systemd-fstab-generator errors (duplicate entries)
- kwin_wayland DRM device failures (nvidia modules not loaded yet)
- D-Bus reload events (hooks triggered mid-session)
- tailscaled connection timeouts (post-reboot network state)

### 3. Check failed services

Run `diagnose_system(action='failed_services')` to confirm no failed systemd units.

### 4. Check fstab duplicates

Read `/etc/fstab`. Look for:
- duplicate swapfile entries
- duplicate mount points
- stale UUID references

If duplicate swapfile found, remove extras:
```
sudo sed -i 'N,N+1d' /etc/fstab  # remove lines N and N+1
```

### 5. Check EFI boot order

Run `efibootmgr` to list entries. Look for:
- PXE IPv4/IPv6 entries (network boot targets)
- non-arch entries in boot order
- timeout settings

### 6. EFI boot order management

PXE entries exist on Asus TUF Z690 firmware (Intel I225-V controller). They persist in EFI NVRAM, not on disk.

To remove PXE entries:
```
sudo efibootmgr -b <entry-num> -B
```

To add them back later:
```
sudo efibootmgr -b <entry-num> -B
```

BIOS settings (UEFI Network Boot / WOL) and EFI boot order are separate. Disabling network boot in BIOS doesn't remove EFI entries — you need efibootmgr. Resetting BIOS to defaults re-enables network boot capability but doesn't recreate entries.

### 7. Summarize findings

Report:
- upgrade duration (start to end timestamps)
- kernel update + mkinitcpio rebuild time
- systemd-fstab-generator errors
- kwin_wayland DRM failures (temporary, resolved post-reboot)
- current boot status (failed services count)
- fstab issues found
- EFI boot order entries and recommendations

## Common patterns

- kernel update triggers mkinitcpio (2 images: default + fallback) + depmod — adds 2-3 minutes
- nvidia-open-dkms depmod runs during kernel update — DRM devices unavailable until reboot
- systemd-fstab-generator fails on duplicate entries — fix fstab before next update
- D-Bus reload mid-session — hooks triggered, normal behavior
- tailscaled connection timeouts post-reboot — network state settling, normal

## Command categorization

Commands requiring sudo are **user-run only** — the agent should never attempt to execute them. Present them as actionable items for the user to run directly.

- `sudo sed -i 'N,N+1d' /etc/fstab` — user-run
- `sudo efibootmgr -b <num> -B` — user-run
- `sudo pacman -Sy` — user-run
- `lsblk -f`, `efibootmgr`, `journalctl`, `tail`, `rg`, `cat /etc/fstab` — agent-run (read-only)

Present sudo commands in a "Actions to run" section, not embedded in workflow steps. Batch them when they're related:
```
sudo sed -i 'N,N+1d' /etc/fstab && sudo efibootmgr -b <num> -B
```

Single commands when they're independent or require user decision:
```
sudo efibootmgr -b <num> -B  # remove PXE — decide whether you need network boot
```

### 1.5. Stale detection

Fstab and EFI boot order files are persistent — no stale threshold. System health checks and databases have stale thresholds (> 24h).

### 2.5. Live probe fallback

If `diagnose_system` or `arch_ops_server` tools are unavailable, use:
```
journalctl --boot=0 --no-pager | grep -i "failed\|error\|critical"
lsblk -f
arch_ops_server_check_database_freshness
```

## References

- `references/efibootmgr-schema.md` — EFI boot entry structure, efibootmgr commands
- `references/fstab-conventions.md` — fstab format, common issues
