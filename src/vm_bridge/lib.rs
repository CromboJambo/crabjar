/// vm_bridge: Integration layer for vm-bridge (per-VM websocket relay)
///
/// This module provides integration with vm-bridge for:
/// - Screen sharing (via WebSocket relay)
/// - Shared terminal (via WebSocket relay)
/// - Display protocol routing (for Teams preview window)

pub mod relay;
pub mod terminal;
pub mod screen;

pub use relay::*;
pub use terminal::*;
pub use screen::*;
