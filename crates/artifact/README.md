# crabjar-artifact

Content-addressed artifact packaging for Rust projects — Guix/Nix-style reproducible builds.

## What it does

Builds cargo projects into self-contained, versioned artifacts that are:
- **Reproducible**: Same inputs always produce same output (content-addressed hash)
- **Portable**: Target any platform via cross-compilation flags
- **Verifiable**: Hash-based integrity checking
- **Self-contained**: No external runtime dependencies

## Usage

```bash
# Build an artifact from a cargo project
cargo-artifact build . --target x86_64-unknown-linux-musl -o myapp.ca

# Inspect artifact metadata
cargo-artifact inspect myapp.ca

# Verify artifact integrity
cargo-artifact verify myapp.ca

# Install artifact to target location
cargo-artifact install myapp.ca --prefix /opt
```

## Artifact Format (.ca file)

JSON metadata describing the build:

```json
{
  "version": 1,
  "hash": "sha256:<content-addressed-hash>",
  "name": "package-name",
  "target": "x86_64-unknown-linux-musl",
  "inputs": {
    "source_hash": "sha256:<source-tree-hash>",
    "lockfile_hash": "sha256:<cargo-lock-hash>",
    "compiler_version": "rustc 1.80.0",
    "features": ["std"]
  },
  "outputs": {
    "binary": "package-name",
    "lib": null,
    "data_files": []
  }
}
```

## Design Philosophy

Inspired by Guix and Nix's functional packaging approach:
- Build = f(source, dependencies, environment) → artifact
- Content-addressed store (hash identifies the artifact)
- Isolation: each build has its own dependency graph
- Reproducibility: deterministic builds from declarations

## For Fleet Deployment

```bash
# On build machine (jambo)
cargo-artifact build . --target x86_64-unknown-linux-musl -o crabjar-worker.ca

# Distribute to fleet
scp crabjar-worker.ca ftw3:~
ssh ftw3 "cargo-artifact install crabjar-worker.ca"

# Verify all workers running identical artifact
fleet exec "cargo-artifact inspect /opt/crabjar/current.ca | jq .hash"
```

If two machines have the same .ca hash, they're running byte-identical software with identical dependencies. No drift.