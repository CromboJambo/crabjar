# crabjar-analyze MCP Server

## Goal
Expose the crabjar-analyze toolkit as an MCP (Model Context Protocol) server, enabling any MCP-compatible client (Claude Desktop, Cursor, OpenAI-compatible APIs, etc.) to use our reverse engineering analysis tools.

## Architecture
- **Transport**: stdio JSON-RPC (same as pesti-mcp-server)
- **Library**: `rmcp` crate v3.5+ (proven in pesti-mcp-server)
- **Pattern**: One MCP tool per analyzer subcommand, matching CLI interface

## Tools to Expose

### Binary Analysis (`analyze-binary`)
```json
{
  "name": "analyze_binary",
  "description": "Analyze ELF binary: header, sections, symbols, strings, dependencies",
  "parameters": {
    "path": {"type": "string", "description": "Path to ELF binary"},
    "options": {"type": "object", "properties": {
      "sections": {"type": "boolean"},
      "symbols": {"type": "boolean"},
      "dynamic_symbols": {"type": "boolean"},
      "strings": {"type": "boolean"},
      "dependencies": {"type": "boolean"}
    }}
  }
}
```

### HAR Analysis (`analyze-har`)
```json
{
  "name": "analyze_har",
  "description": "Analyze HTTP Archive (HAR) network capture files",
  "parameters": {
    "path": {"type": "string", "description": "Path to HAR file"},
    "options": {"type": "object", "properties": {
      "summary": {"type": "boolean"},
      "domains": {"type": "boolean"},
      "requests": {"type": "boolean"}
    }}
  }
}
```

### JavaScript Analysis (`analyze-js`)
```json
{
  "name": "analyze_js",
  "description": "Analyze JavaScript source for imports, exports, globals, functions",
  "parameters": {
    "path": {"type": "string", "description": "Path to JavaScript file"},
    "options": {"type": "object", "properties": {
      "imports": {"type": "boolean"},
      "exports": {"type": "boolean"},
      "functions": {"type": "boolean"},
      "globals": {"type": "boolean"}
    }}
  }
}
```

### APK Analysis (`analyze-apk`)
```json
{
  "name": "analyze_apk",
  "description": "Analyze Android APK package structure and manifest",
  "parameters": {
    "path": {"type": "string", "description": "Path to APK file"}
  }
}
```

### Firmware Analysis (`analyze-firmware`)
```json
{
  "name": "analyze_firmware",
  "description": "Analyze firmware binary: magic bytes, compression, sections",
  "parameters": {
    "path": {"type": "string", "description": "Path to firmware image"}
  }
}
```

### .NET Analysis (`analyze-dotnet`)
```json
{
  "name": "analyze_dotnet",
  "description": "Analyze .NET assembly: metadata, types, methods, dependencies",
  "parameters": {
    "path": {"type": "string", "description": "Path to .NET DLL/EXE"}
  }
}
```

## Implementation Plan

### Phase 1: Crate Scaffolding (Task 1)
- Create `analyze-mcp-server/` crate
- Add `rmcp` dependency with server feature
- Implement basic MCP server structure following pesti pattern
- Wire as workspace member

### Phase 2: Binary Analysis Tool (Task 2)
- Implement `analyze_binary` MCP tool
- Reuse analyze-native crate logic
- Return structured JSON matching crabjar CLI output format

### Phase 3: JavaScript Analysis Tool (Task 3)
- Implement `analyze_js` MCP tool
- Requires analyze-js crate (implement first if not done)

### Phase 4: HAR Analysis Tool (Task 4)
- Implement `analyze_har` MCP tool
- Requires analyze-har crate

### Phase 5: Remaining Tools (Tasks 5-7)
- APK, firmware, .NET analysis tools
- Each reuses corresponding analyze-* crate

## Design Decisions

1. **One crate per concern**: MCP server is separate from analyzer crates
2. **Reuse existing logic**: Don't reimplement parsing; call analyze-* crates directly
3. **Consistent output format**: Match crabjar CLI's JSON envelope (success/error, provenance, doubt)
4. **Stdio transport**: Universal compatibility with all MCP clients

## Comparison to REA
- REA: TypeScript MCP server wrapping external tools
- crabjar-analyze-mcp: Rust MCP server with pure-Rust implementations
- Both expose same conceptual interface to agents
- crabjar version has zero external dependencies (no Hopper/Ghidra/JADX required)

## Testing Strategy
- Unit tests for each tool function
- Integration test: spawn server, send MCP requests, verify responses
- Use real binaries/fixtures from analyze-* crate test suites
