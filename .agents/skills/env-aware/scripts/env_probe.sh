#!/usr/bin/env bash
# hardware/architecture probe for env-aware
set -euo pipefail

probe="${1:-full}"

case "$probe" in
    whoami)
        printf '{"user": "%s"}\n' "$(whoami)"
        ;;
    whatami)
        printf '{"hostname": "%s"}\n' "$(hostname 2>/dev/null || echo 'unknown')"
        ;;
    whereami)
        printf '{"cwd": "%s"}\n' "$(pwd)"
        ;;
    hardware)
        cpu="$(grep 'model name' /proc/cpuinfo | head -1 | sed 's/.*: //' || echo 'not available')"
        cores="$(grep 'processor' /proc/cpuinfo | wc -l || echo 'not available')"
        mem_total="$(grep 'MemTotal' /proc/meminfo | sed 's/.*: //' || echo 'not available')"
        disk="$(df -h / | tail -1 | awk '{print $4}' || echo 'not available')"
        printf '{"cpu": "%s", "cores": %s, "mem_total": "%s", "disk_available": "%s"}\n' "$cpu" "$cores" "$mem_total" "$disk"
        ;;
    architecture)
        arch="$(uname -m || echo 'not available')"
        os="$(uname -s || echo 'not available')"
        kernel="$(uname -r || echo 'not available')"
        printf '{"arch": "%s", "os": "%s", "kernel": "%s"}\n' "$arch" "$os" "$kernel"
        ;;
    full)
        user="$(whoami)"
        hostname="$(hostname 2>/dev/null || echo 'unknown')"
        cwd="$(pwd)"
        cpu="$(grep 'model name' /proc/cpuinfo | head -1 | sed 's/.*: //' || echo 'not available')"
        cores="$(grep 'processor' /proc/cpuinfo | wc -l || echo 'not available')"
        mem_total="$(grep 'MemTotal' /proc/meminfo | sed 's/.*: //' || echo 'not available')"
        disk="$(df -h / | tail -1 | awk '{print $4}' || echo 'not available')"
        arch="$(uname -m || echo 'not available')"
        os="$(uname -s || echo 'not available')"
        kernel="$(uname -r || echo 'not available')"
        printf '{"user": "%s", "hostname": "%s", "cwd": "%s", "cpu": "%s", "cores": %s, "mem_total": "%s", "disk_available": "%s", "arch": "%s", "os": "%s", "kernel": "%s"}\n' "$user" "$hostname" "$cwd" "$cpu" "$cores" "$mem_total" "$disk" "$arch" "$os" "$kernel"
        ;;
    *)
        printf '{"error": "unknown probe", "usage": "env_probe.sh [whoami|whatami|whereami|hardware|architecture|full"}\n'
        exit 1
        ;;
esac
