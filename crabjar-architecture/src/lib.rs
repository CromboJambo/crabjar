/// crabjar-architecture: Mechanical dependency boundary enforcement.
///
/// Defines the dependency layering model for the crabjar workspace and
/// verifies that crates only depend on crates at their own level or below.
///
/// ## Layering Model
///
/// ```text
/// Layer 0: common (shared types, no workspace deps)
/// Layer 1: substrate (memory, guard, telemetry, sandbox — low-level storage/isolation)
/// Layer 2: authority (tool_registry — capability/registry layer)
/// Layer 3: runtime (orchestrator, vm-bridge — execution runtime)
/// Layer 4: host (host-core, host-system, host-observe, host-agent, etc.)
/// Layer 5: product (apps, host-binary — product-facing)
/// Layer 6: bridge (zed-acp-bridge, zed-acp-server — external protocol bridges)
/// Layer 7: skills (skill-script-runner, skill-reference-store — agent skills)
/// ```
///
/// Boundary rule: a crate in layer N may only depend on crates in layers 0..=N.
/// This prevents high-level crates from leaking into low-level ones.

pub mod layer;
pub mod boundary;

pub use layer::*;
pub use boundary::*;
