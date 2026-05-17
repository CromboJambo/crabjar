#!/usr/bin/env bash
# Clean fstab duplicates and stale entries
set -euo pipefail

echo "=== fstab audit ==="
cat /etc/fstab

echo ""
echo "=== duplicate check ==="
# Check for duplicate mount points
awk '{print $2}' /etc/fstab | sort | uniq -d

echo ""
echo "=== duplicate UUID check ==="
# Check for duplicate UUIDs
awk '{print $1}' /etc/fstab | grep UUID | sort | uniq -d

echo ""
echo "=== swapfile entries ==="
awk '/swap/{print}' /etc/fstab

echo ""
echo "=== lsblk filesystem check ==="
lsblk -f

echo ""
echo "=== recommendations ==="
if awk '{print $2}' /etc/fstab | sort | uniq -d | grep -q .; then
  echo "WARNING: duplicate mount points found — remove extras before next update"
fi
if awk '{print $1}' /etc/fstab | grep UUID | sort | uniq -d | grep -q .; then
  echo "WARNING: duplicate UUIDs found — verify with lsblk -f"
fi
