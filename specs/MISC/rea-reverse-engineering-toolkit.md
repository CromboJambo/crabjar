# REA-Style Reverse Engineering Toolkit for Crabjar

## Context

REA (Reverse Engineer Anything) is a TypeScript MCP server that provides agents with reverse engineering tools across multiple artifact types. We want to build the crabjar equivalent in Rust, leveraging our existing infrastructure while matching REA's capability surface.

**What REA does:**
- Native binary analysis via Hopper/Ghidra/IDA backends (pseudocode, assembly, strings, symbols)
- JavaScript/Electron app inspection
- .NET assembly decompilation
- Website/network traffic analysis with HAR captures
- Android APK teardown (manifest, classes, decompiled methods)
- Firmware extraction and analysis
- Process behavior tracing

**Architecture comparison:**

| Aspect | REA | Crabjar approach |
|--------|-----|------------------|
| Language | TypeScript (Node 22+) | Rust |
| Interface | MCP server + CLI | CLI binary + lib crates |
| Backends | Multiple optional tools | Self-contained; no external deps for core |
| Agent integration | MCP protocol | Direct crate imports or CLI JSON |
| Output model | Evidence-based with provenance | Structured JSON + `doubt` blocks |

## Design Principles

1. **Compiled first**: No runtime dependency on Node ecosystem or optional tools being installed
2. **Guard system**: Execution policy for analyzers (REA assumes you can run them)
3. **Crate boundaries**: Each analyzer is a separate crate with explicit dependencies
4. **Evidence model**: Every analysis result includes provenance and limitations

## Crate Structure

```
crabjar/
├── crabjar-analyze/              # Main CLI binary, command dispatch
│   ├── src/main.rs
│   └── Cargo.toml
├── analyze-common/               # Shared types, evidence model, output formatting
│   ├── src/lib.rs
│   └── Cargo.toml
├── analyze-native/               # Binary analysis (ELF, Mach-O, PE)
│   ├── src/lib.rs
│   └── Cargo.toml
├── analyze-js/                   # JavaScript/Electron inspection
│   ├── src/lib.rs
│   └── Cargo.toml
├── analyze-net/                  # Network capture analysis (HAR, pcap)
│   ├── src/lib.rs
│   └── Cargo.toml
├── analyze-apk/                  # Android APK teardown
│   ├── src/lib.rs
│   └── Cargo.toml
├── analyze-firmware/             # Firmware extraction and analysis
│   ├── src/lib.rs
│   └── Cargo.toml
├── analyze-dotnet/               # .NET assembly inspection
│   ├── src/lib.rs
│   └── Cargo.toml
└── analyze-process/              # Process behavior tracing
    ├── src/lib.rs
    └── Cargo.toml
```

## Phase 1: Foundation

### Task 1.1: Crate scaffolding and CLI framework
- Create `crabjar-analyze` binary crate with clap-based CLI
- Implement command dispatch pattern matching crabjar's existing style
- Set up workspace member in root Cargo.toml
- Output format: structured JSON with success/error envelope

### Task 1.2: Evidence model (analyze-common)
- Define shared types for analysis results
- Implement provenance tracking (what tool was used, version, timestamp)
- Add `doubt` block support matching crabjar's existing pattern
- Error type hierarchy for all analyzer crates

## Phase 2: Core Analyzers

### Task 2.1: Native binary analysis (analyze-native)
**Priority: High — most requested REA feature**
- ELF header parsing and section enumeration
- Symbol table extraction (dynamic and static)
- String extraction with encoding detection
- Basic disassembly via libelf/libdwarf or inline parsing
- Backend abstraction for future Hopper/Ghidra integration

### Task 2.2: Network capture analysis (analyze-net)
**Priority: High — quick win, no external deps**
- HAR file parsing and request/response inspection
- Payload extraction and content-type sniffing
- Request flow reconstruction
- Basic pcap header parsing for protocol identification

### Task 2.3: JavaScript/Electron inspection (analyze-js)
**Priority: Medium**
- Bundle structure analysis
- Dependency graph extraction from package.json
- Electron main/renderer process identification
- IPC pattern detection in source

## Phase 3: Extended Analyzers

### Task 3.1: Android APK teardown (analyze-apk)
- ZIP extraction and manifest parsing
- Class enumeration from dex files
- Basic method signature extraction
- Resource file inventory

### Task 3.2: Firmware analysis (analyze-firmware)
**Priority: Medium — depends on external tools**
- File magic detection for firmware images
- Integration with binwalk/unblob via process spawning
- Region identification and extraction handoffs
- Output formatting for downstream native analysis

### Task 3.3: .NET assembly inspection (analyze-dotnet)
- PE header parsing for .NET assemblies
- Metadata table enumeration
- Type and method signature extraction
- Assembly reference graph

## Phase 4: Advanced Features

### Task 4.1: Process behavior tracing (analyze-process)
**Priority: Low — complex, platform-specific**
- Child process spawning with PTY
- Command execution with timeout and signal handling
- Output capture and structured logging
- Exit code and resource usage reporting

### Task 4.2: MCP server integration
- Implement MCP protocol handler in crabjar-analyze
- Expose all analyzer tools via MCP
- Enable use from any MCP-compatible agent (Claude Desktop, Cursor)
- This would be a differentiator vs REA's current approach

## Integration with Existing Crabjar Infrastructure

### Guard system
Each analyzer that spawns external processes or accesses the filesystem should go through the guard system:
```rust
let result = analyze_binary(&path).await?;
guard.report_action("analyze", "binary_analysis", &result.provenance)?;
```

### Knowledge base integration
Analysis results can be stored in crabjar's knowledge base for future reference:
```bash
crabjar analyze binary /usr/bin/sshd --store
# Creates a state-doc with analysis results
```

### Conductor service integration
Long-running analyses (firmware extraction, large binary disassembly) can be submitted as conductor goals:
```bash
crabjar analyze firmware image.bin --async
# Returns goal ID for later retrieval
```

## CLI Design

```bash
# Binary analysis
crabjar analyze binary /usr/bin/ls --sections --symbols --strings
crabjar analyze binary malware.exe --disassemble --entry-point

# Network analysis
crabjar analyze har capture.har --requests --payloads
crabjar analyze pcap traffic.pcap --protocols --flows

# JavaScript analysis
crabjar analyze js app.js --dependencies --electron-check
crabjar analyze bundle webpack-output.js --modules

# APK analysis
crabjar analyze apk app.apk --manifest --classes --resources

# Firmware analysis
crabjar analyze firmware image.bin --extract --regions

# .NET analysis
crabjar analyze dotnet app.dll --types --methods --references
```

## Success Criteria

1. **Binary analysis**: Can identify ELF sections, symbols, and strings from a compiled binary
2. **Network analysis**: Can parse HAR files and extract request/response patterns
3. **Evidence model**: Every result includes provenance (tool used, timestamp) and limitations
4. **Guard integration**: External tool invocations go through crabjar's guard system
5. **JSON output**: All results are structured JSON matching crabjar's CLI contract

## Risks and Mitigations

1. **External tool dependencies** (firmware analysis): Abstract behind trait, provide mock implementation for testing
2. **Large binary handling**: Stream processing where possible; document memory requirements
3. **Platform-specific code**: Use cfg attributes extensively; test on Linux primarily, macOS secondarily
4. **Security of external tools**: Guard system validates commands before execution

## Future Extensions

- **IDA Pro integration**: Backend for commercial disassembler
- **Ghidra integration**: Free alternative backend
- **Dynamic analysis**: Process tracing and system call interception
- **Symbolic execution**: For path exploration in binaries
- **Cross-reference database**: Link analyses across multiple artifacts
