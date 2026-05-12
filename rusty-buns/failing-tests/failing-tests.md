# State Doc: Failing Tests

Source: `github.com/oven-sh/bun` branch `claude/phase-a-port`

## Remaining 0.2% — Regression Tests

Two files in `test/regression/` that track the remaining non-passing cases.

### brotli-reset-leak.test.ts

Verifies Brotli compressor/decompressor `reset()` calls do not leak memory.

- **100,000 iterations** of `compressor.reset()` / `decompressor.reset()`
- Baseline RSS → final RSS after GC
- Threshold: memory growth < 50 MB
- **Pre-fix symptom**: each reset allocated ~400 KB without freeing previous encoder/decoder state → ~40 GB leakage over 100k iterations
- **Root cause**: Zig missing-deinit on reset path; Rust `Drop` semantics eliminate this class

### issue23966.test.ts

`Buffer.isEncoding()` Node.js compatibility test.

- `Buffer.isEncoding("")` should return `false` (Node vs Bun divergence)
- Valid encodings: `utf8`, `utf-8`, `hex`, `base64`, `ascii`, `latin1`, `binary`, `ucs2`, `ucs-2`, `utf16le`, `utf-16le`
- Invalid encodings: `invalid`, `utf32`, `something`
- Non-string values: `number`, `null`, `undefined`, `object`, `array` — all should return `false`
- **Root cause**: `node:buffer` binding in C++ `JSBuffer.cpp` — stays C/C++ (see migration spine Layer 2)

## Crash/Leak Audit Summary

Generated from 653 open GitHub issues matching crash/leak/segfault/panic keywords.

| Metric | Value |
|---|---|
| Total issues classified | 653 |
| Likely fixed by Rust port | 238 (36%) |
| Memory-adjacent | 480 |
| Not memory bugs | 140 |
| Duplicates / needs-repro | 11 / 22 |

### By confidence × category

| Category | High | Medium | Low | Total | Likely fixed |
|---|---|---|---|---|---|
| crash | 232 | 104 | 30 | **366** | 164 |
| uaf | 1 | 1 | 1 | **3** | 3 |
| double-free | 1 | 2 | 0 | **3** | 0 |
| leak | 15 | 22 | 3 | **40** | 19 |
| oom | 5 | 3 | 0 | **8** | 2 |
| assert | 48 | 4 | 2 | **54** | 45 |
| hang | 3 | 3 | 0 | **6** | 1 |
| not-memory | 128 | 11 | 1 | **140** | 2 |
| duplicate | 8 | 3 | 0 | **11** | 2 |
| needs-repro | 0 | 0 | 22 | **22** | 0 |

### Needs verification (216 issues)

Root cause in C++/JSC/NAPI/third-party code, CPU baseline, algorithmic recursion — not addressed by the Zig→Rust port. Requires explicit retest or upstream fixes.

Examples:
- Buffer.prototype.write TOCTOU (#30417) — entirely in C++ `JSBuffer.cpp`
- SIGILL on pre-SSE4.2 AMD CPUs (#7179) — CPU target issue
- bytecode cache segfault in JSC (#18416, #24144) — entirely in JSC C++
- JSC GC sweep crashes (#14738, #14800, #15775, #21072, #24194) — JavaScriptCore C++
- N-API third-party addon crashes (#5672, #10047, #10690, #15551, #15972) — native `.node` addon
- BoringSSL segfault (#23043) — C crypto library

### Low-confidence memory issues (30)

Unsymbolicated/garbled traces: #14997, #16504, #17176, #17231, #17262, #18273, #18355, #18714, #19125, #19132, #19954, #20967, #21399, #22046, #22349, #23005, #24401, #25550, #25790, #26984, #26985, #27000, #27692, #27929, #28175, #28274, #29336, #29488, #29497, #30418

### Needs repro (22)

#7054, #12507, #15291, #16649, #18940, #19087, #19685, #20341, #20641, #21367, #21560, #21609, #21683, #21798, #22051, #22567, #22738, #24202, #24390, #26528, #27998, #29479

### Top areas by likely-fixed count

| Area | Total | Likely fixed | Crash | Leak | Assert | Other |
|---|---|---|---|---|---|---|
| install | 85 | 49 | 48 | 1 | 17 | 19 |
| napi | 65 | 6 | 53 | 1 | 0 | 11 |
| runtime | 48 | 6 | 31 | 2 | 3 | 12 |
| jsc | 42 | 0 | 32 | 2 | 0 | 8 |
| bundler | 36 | 21 | 20 | 0 | 6 | 10 |
| http-server | 33 | 17 | 21 | 6 | 2 | 4 |
| fs | 24 | 15 | 14 | 2 | 0 | 8 |
| fetch | 21 | 8 | 8 | 6 | 1 | 6 |
| worker | 18 | 7 | 13 | 1 | 0 | 4 |

### Gate criteria per flip

tests pass `BUN_RS=0 ∧ =1` · shadow-diff equal · mitata p50 ≤2% · layout asserts × 6 platforms · cargo-tree L1 ⊬ bun_jsc · clippy -D · sha256(ZigGeneratedClasses.cpp) unchanged
