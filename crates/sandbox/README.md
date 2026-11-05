# Hermes Agent Sandbox Image

This Dockerfile defines the base image for running crabjar agent workloads in isolated Podman/Docker containers with terminal support.

## Building the Image

```bash
./build-sandbox-image.sh hermes-sandbox:latest
```

Or manually:

```bash
podman build -t hermes-sandbox:latest -f Dockerfile .
# or
docker build -t hermes-sandbox:latest -f Dockerfile .
```

## Using with Crabjar Sandbox

The image is referenced as the default base image in `sandbox/src/podman.rs`. To use it:

```rust
use crabjar_sandbox::podman::{PodmanSandbox, PodmanConfig};

let config = PodmanConfig::default();  // Uses hermes-sandbox:latest
let sandbox = PodmanSandbox::new(config);

// Create and run agent container
let container_id = sandbox.create_container("my-agent")?;
sandbox.start_container(&container_id)?;

// Execute commands in the container
let result = sandbox.exec_in_container(&container_id, ["echo", "hello"])?;
println!("{}", result.stdout);
```

## Image Features

- **Base**: Alpine Linux 3.21 (minimal footprint)
- **Shell**: Bash with proper terminal support
- **User**: Dedicated `agent` user (uid 1000) for isolation
- **Working Directory**: `/home/agent/workspace`
- **Network**: Standard ports exposed for agent services
- **Dependencies**: curl, wget, jq, git, openssh-client, coreutils, procps

## Customization

To use a different base image or customize the sandbox configuration:

```rust
let config = PodmanConfig {
    base_image: "custom-image:v1".to_string(),
    // ... other options
};
```

## Security Considerations

- Runs as non-root `agent` user by default
- Minimal attack surface with Alpine base
- No persistent data in container (use mounts for persistence)
- Network isolation via Podman network modes