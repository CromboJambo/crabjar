# AGENTS.md — orchestrator

> Purpose: ACP-compliant HTTP orchestrator — Axum server with SSE streaming, unified LLM inference backend, and prompt envelope (instruction-hijack defense).

## Layer

Layer 2: core — may depend on substrate (guard, telemetry, memory) and host-core.

## Public API

- `InferenceBackend` trait — unified interface for LLM backends (LM Studio, mistral.rs, future PESTI runner)
- `LmStudioClient` — multi-endpoint client (Native, OpenAI-compatible, Anthropic-compatible, Mistral.rs serve)
- `PromptEnvelope` + `PromptValidator` — instruction-hijack defense for all outbound prompts
- `SessionStore` — SQLite-backed session persistence for stateful chat
- Axum router: `/acp/run`, `/acp/prompt`, `/acp/chat`

## Key Files

- `src/main.rs` — Axum router: `/acp/run`, `/acp/prompt`, `/acp/chat`
- `src/lib.rs` — crate entry point (empty; binary-only crate)
- `src/backend/mod.rs` — `InferenceBackend` trait + `BackendKind` enum + LmStudioClient impl
- `src/lm_studio_client/mod.rs` — LM Studio client module root
- `src/lm_studio_client/types.rs` — unified message/request/response types, `LmStudioEndpoint` enum
- `src/lm_studio_client/client.rs` — `LmStudioClient` with `chat()` and `chat_with_system()` (both envelope-gated)
- `src/lm_studio_client/session.rs` — `SessionState` + `SessionStore` (SQLite persistence)
- `src/lm_studio_client/error.rs` — `LmStudioError` + `ToolCallInfo`
- `src/lm_studio_client/endpoints.rs` — endpoint converters (native, OpenAI, Anthropic)
- `src/lm_studio_client/prompt_envelope.rs` — `SourceLabel`, `LabeledContent`, `PromptEnvelope`, `PromptValidator`, `PromptError`
- `prompts/default_system.md` — default system prompt template

## Dependencies

- axum, tower-http, reqwest, tokio, tokio-stream, futures-util
- serde, serde_json
- thiserror, anyhow
- uuid, chrono, rusqlite, tracing, tracing-subscriber, sha2
- crabjar-guard, crabjar-telemetry, agent-context

## Pitfalls

- **Prompt envelope is mandatory** — every outbound prompt must pass through `PromptEnvelope` + `PromptValidator`. Never bypass it.
- **No lib.rs** — this is a binary-only crate. All public types are re-exported from `lm_studio_client/mod.rs` for external consumers.
- **Backend abstraction is thin** — `InferenceBackend` currently only wraps `LmStudioClient`. Adding new backends means implementing the trait, not modifying existing code.
- **SessionStore is optional** — if `session_store` is `None`, session IDs are generated in-memory. Persistence only works when configured.
- **Endpoint selection** — `LmStudioEndpoint` is controlled by `LM_STUDIO_ENDPOINT` env var. Default is `Openai`. Mistral.rs serve uses `MISTRALRS_SERVE_URL`.
- **Tool call result serialization** — `execute_tool_call` in `main.rs` uses `serde_json::from_str` on `tc.arguments.to_string()` — fragile for complex JSON. Prefer typed deserialization.
- **Guard DB path** — `execute_tool_call` reads `MIRROR_GUARD_ROOT` from env with a hardcoded fallback. This couples the orchestrator to mirror-lab paths. Consider injecting it via `AppState`.
- **SSE stream lifecycle** — `handle_run` creates a channel + spawns a task. The channel is unbounded (capacity 100). Monitor for memory pressure with long-running commands.
