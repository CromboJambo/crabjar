# Crabjar Deployment Guide

This guide covers deploying crabjar to different environments: local development, home-lab nodes, and production-like setups.

## Prerequisites

- Rust toolchain (stable or nightly)
- `just` command runner (optional but recommended)
- SSH access to target machines (for remote deployment)
- Podman or Docker (for sandboxed execution)

## Quick Start (Local)

```bash
# Build release binary
cargo build --release -p crabjar

# Run directly
./target/release/crabjar --help

# Or use just
just build
just run state list
```

## Remote Deployment via SSH

The `scripts/deploy.sh` script handles cross-machine deployment:

```bash
# Deploy to a home-lab node
./scripts/deploy.sh lappy
./scripts/deploy.sh artix-box
./scripts/deploy.sh ftw3

# Deploy locally
./scripts/deploy.sh local
```

### What the Script Does

1. Builds release binary (`cargo build --release -p crabjar`)
2. Creates installation directory structure (`/opt/crabjar`)
3. Copies binary to target machine
4. Generates machine-specific `.crabjar_config.toml` with:
   - Machine ID (from hostname)
   - Version string
   - Guard settings (trust layer 0, no auto-grant)
   - Execution settings (podman runtime, sandbox user, timeout)
   - Telemetry configuration

### Supported Targets

| Target | SSH Host | SSH Key | Notes |
|--------|----------|---------|-------|
| `lappy` | crombo@lappy | ~/.ssh/id_lab | Home-lab node |
| `artix-box` | crombo@artix-box | ~/.ssh/id_lab | Artix Linux node |
| `ftw3` | crombo@ftw3 | ~/.ssh/id_lab | GPU training node |
| `pixel9a` | crombo@100.84.247.163:8022 | ~/.ssh/id_pixel9a | Pixel 9a phone (Termux) |
| `local` | — | — | Current machine |

## Manual Deployment

If you prefer to deploy manually:

```bash
# Build release binary
cargo build --release -p crabjar

# Create installation directory
sudo mkdir -p /opt/crabjar/data/logs
sudo cp target/release/crabjar /opt/crabjar/
sudo chmod +x /opt/crabjar/crabjar

# Generate config file
cat > /opt/crabjar/.crabjar_config.toml <<EOF
[identity]
machine_id = "$(hostname)"
version = "dev"

[guard]
trust_layer = 0
auto_grant = false

[execution]
container_runtime = "podman"
sandbox_user = "crabjar-agent"
timeout_seconds = 300

[telemetry]
enabled = true
log_level = "info"
output_dir = "/opt/crabjar/data/logs"
EOF

# Run crabjar
/opt/crabjar/crabjar --help
```

## Configuration Reference

The `.crabjar_config.toml` file controls crabjar behavior:

### Identity Section
```toml
[identity]
machine_id = "unique-machine-identifier"  # Used for fleet coordination
version = "0.12.5"                        # Version string
```

### Guard Section
```toml
[guard]
trust_layer = 0           # Initial trust layer (0-3)
auto_grant = false        # Auto-approve low-risk actions?
```

### Execution Section
```toml
[execution]
container_runtime = "podman"  # podman, docker, or none
sandbox_user = "crabjar-agent"  # User for sandboxed execution
timeout_seconds = 300     # Default command timeout
```

### Telemetry Section
```toml
[telemetry]
enabled = true            # Enable flight recorder?
log_level = "info"        # debug, info, warn, error
output_dir = "/opt/crabjar/data/logs"  # Log directory
```

## Fleet Deployment

For deploying across multiple home-lab nodes:

```bash
# Deploy to all nodes
for target in lappy artix-box ftw3; do
    ./scripts/deploy.sh $target
done
```

Or use the conductor service for coordinated fleet management (see `crabjar-conductor` crate).

## Verification

After deployment, verify the installation:

```bash
# Check binary is installed
/opt/crabjar/crabjar --version

# Run basic commands
/opt/crabjar/crabjar state list
/opt/crabjar/crabjar doctor check

# Check telemetry is working
ls -la /opt/crabjar/data/logs/
```

## Troubleshooting

### Binary not found
Ensure `/opt/crabjar` is in your PATH or use the full path.

### Config file errors
Check that `.crabjar_config.toml` is valid TOML and has all required sections.

### Podman/Docker issues
Verify the container runtime is installed and running:
```bash
podman --version  # or docker --version
```

### SSH deployment fails
Check SSH key permissions and host connectivity:
```bash
ssh -i ~/.ssh/id_lab crombo@target-host echo "connected"
```

## Production Considerations

For production-like deployments:

1. **Version pinning**: Use tagged releases instead of `dev` version strings
2. **Backup strategy**: Regularly back up `/opt/crabjar/data/` directory
3. **Monitoring**: Enable telemetry and monitor log output
4. **Security**: Set appropriate file permissions on config and data directories
5. **Updates**: Test updates in staging before deploying to production nodes

```bash
# Backup data directory
tar czf crabjar-data-backup-$(date +%Y%m%d).tar.gz /opt/crabjar/data/

# Restore from backup
tar xzf crabjar-data-backup-YYYYMMDD.tar.gz -C /
```

## Uninstallation

```bash
# Remove binary and config
sudo rm -rf /opt/crabjar

# Optionally remove data directory
sudo rm -rf /opt/crabjar/data/
```
