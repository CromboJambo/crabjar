# Fleet Coordination Deployment Guide

This guide covers deploying crabjar's fleet coordination layer across multiple machines for distributed task execution and load balancing.

## Architecture Overview

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Worker Node 1 │     │   Worker Node 2 │     │   Worker Node N │
│   (crabjar)     │     │   (crabjar)     │     │   (crabjar)     │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                        │                        │
         └────────────────────────┼────────────────────────┘
                                  │
                    ┌─────────────▼─────────────┐
                    │   Conductor Service        │
                    │   (crabjar-conductor)      │
                    │   :8090/api/v1             │
                    └───────────────────────────┘
```

## Prerequisites

- Rust toolchain installed on all nodes
- Network connectivity between conductor and worker nodes
- SSH access to all nodes for deployment
- (Optional) Tailscale or similar mesh networking for secure inter-node communication

## Step 1: Deploy Conductor Service

The conductor service coordinates task distribution across the fleet.

### On the conductor host:

```bash
# Clone crabjar repository
git clone https://github.com/crombojambo/crabjar.git
cd crabjar

# Build the conductor binary
cargo build --release -p crabjar-conductor

# Create service directory
mkdir -p /opt/crabjar/conductor
cp target/release/crabjar-conductor /opt/crabjar/conductor/

# Create configuration file
cat > /opt/crabjar/conductor/config.toml << 'EOF'
[server]
host = "0.0.0.0"
port = 8090

[database]
path = "/var/lib/crabjar/conductor.db"

[logging]
level = "info"
file = "/var/log/crabjar/conductor.log"
EOF

# Create directories
mkdir -p /var/lib/crabjar /var/log/crabjar
chown -R crabjar:crabjar /var/lib/crabjar /var/log/crabjar

# Create systemd service file
cat > /etc/systemd/system/crabjar-conductor.service << 'EOF'
[Unit]
Description=Crabjar Fleet Conductor
After=network.target

[Service]
Type=simple
User=crabjar
WorkingDirectory=/opt/crabjar/conductor
ExecStart=/opt/crabjar/conductor/crabjar-conductor --config /opt/crabjar/conductor/config.toml
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
systemctl enable crabjar-conductor
systemctl start crabjar-conductor

# Verify it's running
systemctl status crabjar-conductor
curl http://localhost:8090/api/v1/fleet/status
```

## Step 2: Deploy Worker Nodes

Each worker node runs the crabjar binary and registers with the conductor.

### On each worker node:

```bash
# Clone crabjar repository
git clone https://github.com/crombojambo/crabjar.git
cd crabjar

# Build the crabjar binary
cargo build --release -p crabjar

# Create service directory
mkdir -p /opt/crabjar/worker
cp target/release/crabjar /opt/crabjar/worker/

# Create configuration file pointing to conductor
cat > /opt/crabjar/worker/config.toml << 'EOF'
[conductor]
url = "http://<CONDUCTOR_HOST>:8090/api/v1"
hostname = "<THIS_NODE_HOSTNAME>"
capabilities = ["shell", "browser"]
max_concurrent_tasks = 4

[database]
path = "/var/lib/crabjar/worker.db"

[logging]
level = "info"
file = "/var/log/crabjar/worker.log"
EOF

# Create directories
mkdir -p /var/lib/crabjar /var/log/crabjar
chown -R crabjar:crabjar /var/lib/crabjar /var/log/crabjar

# Create systemd service file
cat > /etc/systemd/system/crabjar-worker.service << 'EOF'
[Unit]
Description=Crabjar Fleet Worker
After=network.target crabjar-conductor.service

[Service]
Type=simple
User=crabjar
WorkingDirectory=/opt/crabjar/worker
ExecStart=/opt/crabjar/worker/crabjar --config /opt/crabjar/worker/config.toml worker
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
systemctl enable crabjar-worker
systemctl start crabjar-worker

# Verify it's running
systemctl status crabjar-worker
```

## Step 3: Verify Fleet Status

From the conductor host, verify all workers are registered:

```bash
curl http://localhost:8090/api/v1/fleet/status
```

Expected response:
```json
{
  "success": true,
  "workers_online": 3,
  "tasks_in_progress": 0,
  "avg_completion_time_ms": 0,
  "last_updated": "2026-11-04T12:00:00Z"
}
```

## Step 4: Submit Test Task

Test the fleet by submitting a simple task through the conductor API:

```bash
curl -X POST http://localhost:8090/api/v1/goals \
  -H "Content-Type: application/json" \
  -d '{
    "description": "Test task: echo hello from fleet",
    "priority": "low",
    "tags": ["test"]
  }'
```

Monitor execution through the conductor dashboard or API:

```bash
# Check goal status
curl http://localhost:8090/api/v1/goals/<goal_id>

# List active workers
curl http://localhost:8090/api/v1/workers
```

## Configuration Reference

### Conductor Configuration (`config.toml`)

```toml
[server]
host = "0.0.0.0"          # Bind address
port = 8090                # HTTP port

[database]
path = "/var/lib/crabjar/conductor.db"  # SQLite database path

[logging]
level = "info"             # Log level: debug, info, warn, error
file = "/var/log/crabjar/conductor.log" # Log file path
```

### Worker Configuration (`config.toml`)

```toml
[conductor]
url = "http://<CONDUCTOR_HOST>:8090/api/v1"  # Conductor API URL
hostname = "<THIS_NODE_HOSTNAME>"             # Unique hostname for this worker
capabilities = ["shell", "browser"]           # Task capabilities
max_concurrent_tasks = 4                      # Maximum parallel tasks

[database]
path = "/var/lib/crabjar/worker.db"          # SQLite database path

[logging]
level = "info"                                # Log level
file = "/var/log/crabjar/worker.log"          # Log file path
```

## Scaling Considerations

### Adding More Workers

Simply deploy additional worker nodes with unique hostnames. The conductor automatically discovers and load-balances across all registered workers.

### High Availability

For production deployments requiring high availability:

1. **Database replication**: Use a replicated database backend instead of SQLite
2. **Conductor clustering**: Deploy multiple conductor instances behind a load balancer
3. **Worker health checks**: Configure heartbeat intervals and timeout thresholds

## Monitoring and Maintenance

### Log Rotation

Configure log rotation for both conductor and worker logs:

```bash
# Example logrotate configuration
/var/log/crabjar/*.log {
    daily
    rotate 7
    compress
    missingok
}
```

### Database Backup

Regularly backup the conductor database:

```bash
crontab -e
# Add: 0 2 * * * sqlite3 /var/lib/crabjar/conductor.db ".backup /var/backups/crabjar/conductor-$(date +\%Y-\%m-\%d).db"
```

### Worker Health Monitoring

Monitor worker health through the conductor API:

```bash
# Check for workers that haven't sent heartbeats recently
curl http://localhost:8090/api/v1/workers | jq '.workers[] | select(.last_heartbeat < now - 300)'
```

## Troubleshooting

### Worker Not Registering

Check network connectivity between worker and conductor:

```bash
# From worker node
curl http://<CONDUCTOR_HOST>:8090/api/v1/fleet/status
```

### Tasks Not Completing

Check worker logs for errors:

```bash
tail -f /var/log/crabjar/worker.log
```

### High Task Queue Times

Monitor fleet utilization:

```bash
curl http://localhost:8090/api/v1/fleet/status | jq '.tasks_in_progress'
```

Consider adding more worker nodes if consistently at capacity.