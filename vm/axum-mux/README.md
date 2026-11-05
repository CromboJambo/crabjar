# vm-bridge — sketch

A per-VM websocket-to-display-socket bridge. One OS process per VM,
supervised by a parent process, so a crash/hang/panic in one VM's
proxy can't touch the others or take down the supervisor.

## Shape

- `manifest.toml` — the declarative VM-to-socket mapping. Same spirit
  as `lfc.toml`: a flat, explicit, hand-editable list rather than
  anything that tries to manage VM lifecycle. This binary never
  starts or stops a VM — it only proxies to a display socket that's
  already there (started by hand with `qemu-system-x86_64 ... -spice
  ...` or a throwaway script, for now).

- `src/manifest.rs` — manifest struct + loader. `bind_addr` is
  required with no default on purpose: forgetting to set it to your
  tailscale IP should be a config error, not a silent bind to
  `0.0.0.0`. `vms` list is optional (defaults to empty) via
  `#[serde(default)]`.

- `src/main.rs` — entrypoint with two modes, selected by re-exec:
  - no args → **supervisor**: loads the manifest, spawns one child
    process per VM (`exe --worker <name>`), restarts a child with
    exponential backoff if it dies unexpectedly, leaves it alone if
    it exits cleanly (status 0 = "intentionally taken down for
    maintenance", not a crash to recover from).
  - `--worker <name>` → **worker**: runs the bridge for exactly one
    VM. This is the unit of isolation — it's a real, separate OS
    process, so a segfault in this process's networking code (or in
    a future native SPICE/VNC decode path, if you ever go there)
    can't touch siblings.

- `src/proxy.rs` — the actual bridge. Binds a websocket listener,
  and on each incoming connection dials the VM's target socket (TCP
  or unix, sketch supports both) and relays raw bytes in both
  directions. Deliberately a dumb pipe — it does not parse SPICE or
  VNC at all. The browser-side client (spice-html5 / noVNC) does the
  protocol decode; this binary's whole job is moving bytes.

## What's deliberately not here yet

- **Auth.** Right now anything that can reach `bind_addr:listen_port`
  on the tailnet gets a raw pipe straight to the VM's display socket.
  Fine for a single-user tailnet, a real gap the moment a second
  device or person joins it. Tailscale `whois`-by-source-IP via
  tailscaled's LocalAPI is the natural next step if/when that
  matters — deferred, not forgotten.
- **`protocol` field is unused.** It's there in the manifest for your
  own bookkeeping / for the frontend to pick spice-html5 vs noVNC,
  but the proxy itself doesn't care — bytes are bytes either way.
- **VM lifecycle.** No start/stop/health-check of the VM itself. The
  manifest assumes the target socket already exists when a worker
  starts; if it doesn't, that one worker's connections fail and the
  supervisor will keep restarting it on the same backoff as any other
  crash, which is harmless but not informative. Worth a clearer
  "target unreachable" distinction from "actually crashed" later.
- **Per-VM TLS/auth differentiation.** All VMs currently share the
  same `bind_addr`; that's fine since isolation already happens at
  the process level, but if you ever want different access policies
  per VM, that's a manifest field away.

## Tests

6 unit tests covering manifest parsing:
- Valid manifest with multiple VMs
- Find existing/missing VM by name
- Missing `bind_addr` correctly rejected
- Empty `vms` list parses (optional via `#[serde(default)]`)
- Protocol enum deserialization (spice/vnc)

## Toolchain note

This was compiled and verified against axum 0.7 / tokio 1.x. The
`toml = "0.7"` pin in Cargo.toml is for compatibility; on a current
rustc, `toml = "0.8"` and unpinned deps should resolve fine.
