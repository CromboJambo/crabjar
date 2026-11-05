# vm-bridge — Roadmap

## Architecture

Per-VM websocket-to-display-socket bridge. One OS process per VM,
supervised by a parent process for isolation.

## Roadmap

### Phase 1: Hardening (current)
- [ ] Add per-VM TLS/auth differentiation
- [ ] Add target health-check (socket exists? port open?)
- [ ] Add graceful shutdown for supervisor
- [ ] Add per-VM config overrides

### Phase 2: Shared Terminal
- [ ] Terminal multiplexer integration (wezterm/zellij)
- [ ] Shared terminal protocol over websocket
- [ ] Terminal state sync across multiple clients

### Phase 3: Display Protocol Support
- [ ] Raw byte relay (dumb pipe)
- [x] SPICE protocol decode (server-side)
- [ ] VNC protocol decode (server-side)
- [ ] Protocol selection via manifest

### Phase 4: Screen Sharing
- [ ] PipeWire integration for screen share sources
- [ ] X11/XDG-Portal integration for Wayland screen capture
- [ ] Preview thumbnail generation
- [ ] Audio capture (microphone + system audio)

### Phase 5: crabjar-host Integration
- [ ] Add crabjar-host dependency (or be added as dependency)
- [ ] Provide screen sharing API for Teams plugin
- [ ] Add shared terminal API for Teams plugin
- [ ] Add display protocol routing for Teams preview window

## Dependencies

- `axum` (websocket support)
- `tokio` (async runtime)
- `futures-util` (streaming)
- `toml` (manifest parsing)
- `anyhow` (error handling)
- `tracing` (logging)

## Not Planned

- VM lifecycle management (by design - this is just the bridge)
- SPICE/VNC protocol implementation (too much for a bridge)
- Multi-user management (deferred to auth layer)
