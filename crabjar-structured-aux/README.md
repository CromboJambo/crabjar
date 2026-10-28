# crabjar-structured-aux

Structured auxiliary decision calls via OpenRouter's JEV endpoint.

This plugin replaces the `hermes-structured-aux-models` Python plugin with a native Rust implementation that integrates directly into crabjar's plugin system. It routes selected auxiliary tasks through bounded Jev decision calls instead of free-form chat prompts for improved reliability and cost efficiency.

## Tools

### structured-aux.approval
Bounded approval decision via JEV (approve/deny/needs_review).

```bash
crabjar exec structured-aux.approval --args '{"request":"Deploy to prod","policy":"Require approval for prod deployments"}'
```

### structured-aux.mcp
Bounded MCP tool selection via JEV.

```bash
crabjar exec structured-aux.mcp --args '{"request":"Get weather","tools":[{"name":"get_weather"},{"name":"search_web"}]}'
```

### structured-aux.compression
Bounded context compression via JEV.

```bash
crabjar exec structured-aux.compression --args '{"text":"Long context to compress..."}'
```

## Configuration

### OpenRouter (default)
Requires `OPENROUTER_API_KEY` environment variable. Default JEV model: `~typesafe/jev-latest`.

### Local llama-server
Configure the plugin with explicit base URL and model name to use a local llama-server instance instead of OpenRouter:

```rust
let plugin = StructuredAuxPlugin::with_config(
    Some("http://ftw3:8081".to_string()),  // Base URL (no /v1 suffix)
    Some("Qwen2.5-Coder-1.5B-Instruct-Q4_K_M".to_string()),  // Model name
    None,  // No API key needed for local servers
);
```

Supported llama-server endpoints: `/v1/chat/completions` (OpenAI-compatible format).

**Note:** The model must be capable of structured decision-making. Qwen2.5-Coder-1.5B works well for bounded approval/MCP tool selection tasks.

## Architecture

The plugin implements crabjar's `Plugin` trait and provides three tools that make bounded decision calls to OpenRouter's JEV endpoint. Each tool constructs a structured decision request with predefined options, reducing hallucination risk and enabling cost-effective auxiliary task routing.