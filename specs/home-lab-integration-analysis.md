# Home-Lab Integration Analysis

**Date:** 2026-10-14
**Scope:** Analyze `~/projects/active/home-lab/` for crabjar integration vs private repo separation

## Executive Summary

The home-lab repo contains a mix of **infrastructure-as-code** (public-safe) and **environment-specific secrets** (private). With crabjar's SSH bastion now working, ~70% of the infrastructure definitions can migrate to crabjar as public code. The remaining 30% should stay private with parameterized references.

## What Can Integrate into Crabjar (Public-Safe)

### 1. Machine Topology Definitions
**Files:** `lab.toml` (machines section), `firewall/*.nft`

The machine definitions, service ports, and firewall rules are infrastructure-as-code that can be public. They describe *what* services exist and *how* they're connected, not *where* specifically or with *which credentials*.

**Integration approach:** Create a `crabjar-lab-topology` crate that defines the lab's machine/service topology as Rust data structures. This becomes the authoritative source for:
- SSH bastion target registry
- Service discovery for remote execution
- Firewall rule generation templates

### 2. Deployment Scripts (Parameterized)
**Files:** `scripts/*.sh`, `dotfiles.sh`

Most deployment scripts are reusable infrastructure patterns. They need parameterization to work across environments but the logic itself is public-safe.

**Integration approach:** Migrate to crabjar's execution pipeline with target-specific parameters injected via the secrets bridge at runtime.

### 3. Configuration Templates
**Files:** `configs/`, `ha-config/configuration.yaml` (template form)

Configuration files that use variables or are templates can be public. The actual values come from the environment/secrets at deploy time.

## What Must Stay Private (Environment-Specific)

### 1. Network Identifiers
- **LAN IPs:** `192.168.50.x` — reveals internal network topology
- **Tailscale IPs:** `100.x.x.x` — your Tailscale network identity
- **WOL MAC addresses:** Identifies specific hardware

**Solution:** Use named references (`jambo`, `ftw3`) resolved at runtime via crabjar's target registry. Store actual IPs in private config or environment variables.

### 2. SSH Usernames and Keys
- **SSH usernames:** `crombo`, `hermes`, `root` — reveals user accounts
- **Key paths:** `~/.ssh/id_lab` — reveals key management strategy

**Solution:** Parameterize via crabjar's secrets bridge. The public code says "use the lab SSH key" and the private config provides the actual path and identity.

### 3. Service-Specific Secrets
- **Tailscale API keys:** `TS_API_KEY` in scripts
- **Syncthing device IDs:** Unique identifiers for your devices
- **Home Assistant tokens/passwords**
- **Private PEM keys** (rustdesk_priv.pem)

**Solution:** These must stay in private config or environment variables, never committed to public repos.

### 4. Machine-Specific Notes
- Hardware details (RTX 3070 Ti, i5-12600K)
- Install dates and version-specific notes
- Personal annotations about machines

**Solution:** Keep in private state-docs or local notes, not public code.

## Recommended Integration Architecture

```
crabjar/ (public repo)
├── crates/lab-topology/          # Machine/service definitions as Rust structs
├── crates/lab-deploy/            # Parameterized deployment logic
├── configs/templates/            # Config templates with variable placeholders
└── scripts/                      # Infrastructure scripts (parameterized)

home-lab-private/ (private repo or local only)
├── lab.env                       # Actual IPs, usernames, key paths
├── secrets/                      # API keys, passwords, PEM files
└── overrides/                    # Machine-specific customizations
```

## Migration Path

### Phase 1: Topology Abstraction (Week 1-2)
- Create `crabjar-lab-topology` crate with machine/service definitions
- Replace hardcoded IPs in SSH bastion with topology lookups
- Parameterize firewall rules as templates

### Phase 2: Secrets Bridge Integration (Week 3-4)
- Integrate crabjar's secrets bridge for credential injection
- Migrate deployment scripts to use parameterized execution
- Establish private config layer for environment-specific values

### Phase 3: Full Migration (Week 5-6)
- Move all reusable infrastructure code to crabjar
- Keep only actual secrets and machine-specific notes private
- Document the separation clearly in both repos

## Risk Assessment

| Item | Public-Safe? | Risk if Exposed | Mitigation |
|------|-------------|-----------------|------------|
| Machine names/labels | Yes | Low (generic) | None needed |
| Service ports | Yes | Low (standard) | None needed |
| Firewall rules | Yes | Medium (network topology) | Parameterize IPs |
| LAN/Tailscale IPs | No | High (network exposure) | Private config only |
| SSH usernames | No | Medium (account enumeration) | Parameterize |
| API keys | No | Critical | Secrets bridge |
| PEM/private keys | No | Critical | Never commit |
| Deployment scripts | Yes | Low | Parameterize env-specific values |

## Conclusion

The home-lab repo is a good candidate for crabjar integration with proper separation of concerns. The key insight: **infrastructure-as-code should be public, environment-specific configuration should be private**. Crabjar's execution pipeline and secrets bridge provide the natural boundary between these two layers.

Start with Phase 1 (topology abstraction) to establish the pattern, then iterate on secrets integration as needed.