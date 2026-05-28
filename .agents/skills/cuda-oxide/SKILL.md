---
name: cuda-oxide
description: |
  Analyze, review, and compare cuda-oxide GPU kernels and compilation patterns.
  Use whenever the user mentions cuda-oxide, GPU kernels in Rust, PTX generation,
  Pliron IR, NVIDIA GPU tooling, or wants to investigate cuda-oxide patterns for
  integration into crabjar. Also trigger when the user asks about Rust-to-GPU
  compilation, single-source compilation, or compares cuda-oxide against alternatives
  (cudarc, half, gpu-alloc, rust-gpu).
---

# cuda-oxide Skill

Analyze cuda-oxide's architecture, review GPU kernels, and identify integration
patterns for crabjar.

## When to Activate

- User mentions `cuda-oxide`, `cargo oxide`, `PTX`, or `Pliron`
- User wants to review or write `#[kernel]` functions
- User asks to compare cuda-oxide against other Rust GPU projects
- User wants to understand cuda-oxide's compilation pipeline
- User is building GPU-related tooling and needs cuda-oxide reference

## State Doc Reference

The full project state doc is at `state-docs/cuda-oxide-state.md`. Read it for:
- Complete workspace layout (15 crates)
- Compilation pipeline details (MIR → dialect-mir → dialect-llvm → LLVM IR → PTX)
- 46 examples with descriptions
- Toolchain requirements
- All 10 integration patterns

## Core Compilation Pipeline

```
Rust source → rustc MIR → dialect-mir → dialect-llvm → LLVM IR → llc (LLVM 21+) → PTX
```

Key crates:
- `rustc-codegen-cuda` — custom rustc backend
- `mir-importer` — MIR → dialect-mir translation
- `mir-lower` — dialect-mir → dialect-llvm lowering
- `dialect-mir` / `dialect-llvm` / `dialect-nvvm` — Pliron dialect hierarchy
- `cuda-core` — RAII wrappers (CudaContext, DeviceBuffer)
- `cuda-async` — lazy async execution (DeviceOperation)
- `cuda-device` — device intrinsics (thread, warp, shared mem, barriers, TMA)
- `cuda-host` — module loading, LTOIR loader
- `cuda-macros` — proc macros (#[cuda_module], #[kernel])

## Key Patterns to Identify

1. **Single-source compilation**: host + device in one file, `cargo oxide build`
2. **Pliron dialect hierarchy**: Rust-native IR framework (no Python/MLIR C++)
3. **LTOIR embedding**: device-side LTO for Blackwell+ (sm_100a)
4. **Async lazy execution**: `DeviceOperation` with `.sync()` or `.await`
5. **Closure capture**: move closures scalarized + passed as kernel params; non-move via HMM
6. **Cross-crate kernels**: library crates define kernels, bundled via `#[cuda_module]`
7. **Example-driven docs**: 46 examples (vecadd, gemm_sol, async_mlp, tcgen05)

## crabjar Integration Analysis

When evaluating cuda-oxide patterns for crabjar adoption, consider:

1. **Pliron-inspired IR pipeline**: Maps to crabjar's knowledge pipeline (index → sync → query → verify). Rust-native approach relevant for WASM constraints.
2. **Doctor pre-flight validation**: `cargo oxide doctor` → crabjar could adopt `crabjar doctor` for guard/db/telemetry state validation.
3. **LTOIR embedding**: `#[cuda_module]` embeds device artifacts into host binary — relevant for safetensors model weight embedding.
4. **Async lazy execution**: `cuda-async` pattern maps to crabjar's pending queue → gate → execute flow.
5. **Example-driven documentation**: 46 examples are more actionable than API reference — crabjar state-docs could adopt this.
6. **deny.toml auditing**: License/security checks per crate — crabjar already has this pattern.
7. **Dev container reproducibility**: `.devcontainer/` for reproducible environments.

## Review Workflow

1. Read `state-docs/cuda-oxide-state.md` for project baseline
2. Fetch target URL (GitHub repo, specific file, or example) if not already cached
3. Compare against state doc for changes (new crates, API changes, status shifts)
4. Identify specific integration patterns applicable to crabjar
5. Output structured analysis with doubt block (assumptions, blind_spots, stale_after)
6. If user wants to build: propose concrete implementation steps

## Pipeline Output Clarification

`cargo oxide pipeline` outputs MIR/LLVM IR, **not PTX**. For PTX, look for
`.ptx` files in the build artifacts or use `--emit=ptx` if supported.
This distinction matters — the pipeline visualization shows the translation
chain, not the final GPU code.

## crabjar Tool Registration

If cuda-oxide is installed locally:
- `cargo oxide run <example>` — run examples
- `cargo oxide doctor` — validate toolchain
- `cargo oxide pipeline <example>` — show compilation pipeline
- `cargo oxide debug <example> --tui` — cuda-gdb debugging

Register in crabjar tool_registry if user wants `crabjar exec` access.
Requires CUDA 12+, LLVM 21+, and sandbox with GPU passthrough.

## Knowledge Bridge

The cuda-oxide knowledge entry (id: 2) in the knowledge store contains the
tagged summary. Query with `crabjar knowledge query --tags=cuda-oxide`.

## Output Contract

All analysis output must include a doubt block:

```json
{
  "doubt": {
    "assumptions": ["what was assumed"],
    "blind_spots": ["what couldn't be verified"],
    "last_validation": "YYYY-MM-DD",
    "stale_after": "condition that invalidates this review"
  }
}
```
