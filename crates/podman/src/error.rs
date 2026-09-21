//! Podman integration errors.

use std::io;

#[derive(Debug, thiserror::Error)]
pub enum PodmanError {
    #[error("podman binary not found: {0}")]
    BinaryNotFound(String),

    #[error("failed to execute podman command: {source}\ncommand: {command}")]
    ExecutionFailed { source: io::Error, command: String },

    #[error("podman command failed with exit code {code}: {output}")]
    CommandFailed { code: i32, output: String },

    #[error("container creation failed: {0}")]
    ContainerCreation(String),

    #[error("container start failed: {0}")]
    ContainerStart(String),

    #[error("container exec failed: {0}")]
    ContainerExec(String),

    #[error("container inspect failed: {0}")]
    ContainerInspect(String),

    #[error("container removal failed: {0}")]
    ContainerRemove(String),

    #[error("failed to parse podman JSON output: {source}\nraw: {raw}")]
    JsonParse { source: serde_json::Error, raw: String },

    #[error("container not found: {0}")]
    ContainerNotFound(String),

    #[error("timeout waiting for container: {0}")]
    Timeout(String),
}
