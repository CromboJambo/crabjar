---
name: env-aware
description: |
  Use whenever the user mentions hardware, architecture, system constraints, or needs to perform tasks that are sensitive to the environment (e.g., OOM, disk space, CPU load). 
  Trigger this skill when the user asks "whoami", "whatami", "whereami", or "check my setup".
---

# Environment Awareness Skill

This skill provides the agent with specialized knowledge about the host machine's architecture and constraints to ensure all commands (especially heavy-duty LLM tasks) are safe and optimized.

## Core Knowledge Base

### 1. System Architecture Profile
The agent should always check `crabjar/environment_manifest.json` before proposing large-scale operations. This file contains the ground truth for:
* **CPU/RAM**: Total capacity and zram configuration.
* **Storage Topology**: Details on NVMe vs SATA drives, Btrfs usage, and mount points.
* **GPU/VRAM**: Available video memory for AI workloads.

**Note:** The manifest may exist but be sparse (placeholder notes instead of actual values). When it lacks concrete data, fall back to live system probes.

### 2. Testing Contexts (Mock Environments)
When testing `crabjar` commands or tool calls, do not use the live system configuration. Instead, use the simulated environments located in:
`crabjar/testing/configs/`

These configs allow the model to simulate behavior on different hardware tiers (e.g., a "Low RAM" tier to test OOM error handling).

**Note:** This directory may not exist yet. Create it with mock configs before testing.

## Workflow Instructions

### When asked about system identity ("whoami", "whatami"):
1. **Read the Manifest**: Read `crabjar/environment_manifest.json` if it exists.
2. **Check for Sparsity**: If the manifest contains placeholder notes (e.g., "check live system for exact capacity") instead of concrete values, proceed to live probes.
3. **Live Probes** (run in parallel):
   - `/proc/cpuinfo` — CPU type, cores, threads
   - `free -h` + `/proc/swaps` — RAM total/available, swap type/size
   - `nvidia-smi` — GPU name, VRAM total/in-use
   - `df -T` — filesystem types and usage per mount
   - `printenv | sort` — environment variables, session type (X11/Wayland), runtime dirs
4. **Summarize Capabilities**: Provide a concise summary of CPU, RAM, GPU, and primary Storage available to the agent.
5. **Identify Constraints**: Explicitly mention known constraints (e.g., "Note: We are running on Btrfs; avoid CoW for swap files").
6. **Update Manifest**: Write the live probe results back to `crabjar/environment_manifest.json` so it stays current.

### When performing heavy tasks (LLM Inference/Data Processing):
1. **Check Resource Availability**: Verify that the task's requirements fit within the `environment_manifest.json` limits (or live probe data if manifest is stale).
2. **Evaluate Risk**: If a task might trigger an OOM event, warn the user and suggest a "Reduced Context" or "CPU-only" mode.
3. **GPU VRAM Check**: If using GPU, verify available VRAM (`nvidia-smi` memory usage minus task allocation) — LM Studio and desktop compositing may already consume significant VRAM.

### When testing commands:
1. **Locate Mocks**: Look into `crabjar/testing/configs/` to find a relevant mock configuration.
2. **Create Mocks if Missing**: If the directory doesn't exist, create it with tier configs (e.g., `low_ram.json`, `gpu_only.json`, `btrfs.json`).
3. **Simulate Environment**: Inject the mock environment data into the manifest or agent context as if it were the live system, ensuring all subsequent tool calls respect these simulated constraints.
4. **Verify with Live Probes**: After testing, cross-check mock behavior against a live probe to confirm the simulation is realistic.

## Reference Files
* `crabjar/environment_manifest.json`: The source of truth for the current environment. (May be sparse — use live probes as fallback.)
* `crabjar/testing/configs/*.json`: Mock environments for testing CLI logic. (May not exist — create before testing.)
* `references/env-probe-schema.md`: probe types, output format, use cases.

## Bundled Scripts

- `scripts/env_probe.sh` — hardware/architecture probe (whoami, whatami, whereami, hardware, architecture, full)
