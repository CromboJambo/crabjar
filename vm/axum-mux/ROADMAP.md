# vm-bridge — Roadmap

## Architecture

Per-VM websocket relay for display protocols and terminal sessions.
One OS process per VM (supervisor/worker re-exec model), plus dedicated
modes for terminal relay and live review.

### Modes

| Mode | Invocation | Purpose |
|------|-----------|---------|
| Supervisor | (default) | Spawns one worker per VM from manifest.toml, restarts on crash |
| Worker | `--worker <name>` | Single-VM display proxy (byte-transparent websocket relay) |
| Terminal Relay | `--terminal-relay [port]` | Multi-client terminal session sync (typed event stream) |
| Review Bridge | `--review <port> <session> <cmd...>` | Run commands in herdr session, publish events to relay |

### APIs

- **Display proxy**: WebSocket on per-VM `listen_port`, raw byte relay to TCP/Unix target
- **Lifecycle API**: HTTP on 127.0.0.1:8090 — VM create/start/stop/destroy/snapshot via vm-core
- **Terminal relay**: WebSocket with JSON control frames + typed TerminalEvent fan-out

## Roadmap

### Phase 1: Hardening ✅ (complete)
- [x] Per-VM auth token (`token` field in manifest.toml; query param or Bearer header)
- [x] Target health-check (probes TCP/Unix socket before accepting websocket, returns 503 if down)
- [x] Graceful shutdown for supervisor (SIGTERM/SIGINT → kill workers → exit)
- [ ] Per-VM config overrides beyond manifest (env vars, CLI flags) — deferred, low priority

### Phase 2: Shared Terminal ✅ (complete)
- [x] Terminal event model in crabjar-terminal crate (inside the glass)
- [x] Shared terminal protocol over websocket (control + event frames)
- [x] Terminal state sync across multiple clients (broadcast fan-out)
- [x] Live review bridge for herdr sessions

### Phase 3: Display Protocol Support ✅ (complete as designed)
- [x] Raw byte relay (dumb pipe) — browser-side client does protocol decode
- [x] SPICE support (via spice-html5 over websocket relay)
- [x] VNC support (via noVNC over websocket relay)
- [x] Protocol selection via manifest (`protocol = "spice" | "vnc"`)

> Note: This proxy is intentionally byte-transparent (ADR-005). No protocol
> parsing happens server-side — the browser client handles all decoding.

### Phase 4: Screen Sharing
- [x] VNC frame capture module (`src/screen/vnc_client.rs` — minimal RFB client, raw encoding)
- [x] Preview thumbnail generation (`src/screen/thumbnail.rs` — JPEG encode from captured frames)
- [x] Thumbnail HTTP endpoint (`GET /screen/thumbnail?max_dim=320&quality=75`, auth-gated)
- [ ] PipeWire integration for screen share sources
- [ ] X11/XDG-Portal integration for Wayland screen capture
- [ ] Audio capture (microphone + system audio)

### Phase 5: crabjar-host Integration
- [ ] Expose lifecycle API through crabjar-host service discovery
- [ ] Provide screen sharing API for Teams plugin
- [ ] Add shared terminal API for Teams plugin
- [ ] Add display protocol routing for Teams preview window

## Dependencies

- `axum` (websocket + HTTP support)
- `tokio` (async runtime)
- `futures-util` (streaming)
- `serde` / `toml` (manifest parsing, JSON frames)
- `anyhow` (error handling)
- `tracing` / `tracing-subscriber` (logging)
- `image` (JPEG encoding for screen thumbnails)
- `vm-core` (VM lifecycle management via Libvirt)
- `crabjar-terminal` (TerminalEvent type for relay protocol)

## Not Planned

- VM lifecycle management *inside the proxy* (handled by lifecycle API + vm-core)
- SPICE/VNC protocol implementation server-side (browser clients do this)
- Multi-user management (deferred to auth layer / Phase 1 hardening)
