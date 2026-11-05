//! Zed Agent Client Protocol server implementation.
//!
//! Implements the ACP protocol for communication between Zed IDE and crabjar agent.
//! Handles session management, tool calls with execution gating, and prompt processing.

pub mod handlers;
pub mod protocol;
pub mod session;

#[cfg(test)]
mod tests;

// Re-export protocol types at crate root for compatibility
use crate::handlers::{AcpServerError, handle_request};
pub use crate::protocol::{AcpRequest, AcpResponse};
pub use crate::session::AcpSession;
use crabjar_guard::GuardDb;
use crabjar_lib::knowledge_store::KnowledgeBridge;

/// The CrabJar version string reported to Zed.
pub const ACP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The ACP agent server that processes requests from Zed.
pub struct AcpAgentServer {
    pub sessions: Vec<AcpSession>,
    pub guard_db: Option<GuardDb>,
    pub knowledge_bridge: Option<KnowledgeBridge>,
    pub initialized: bool,
}

impl AcpAgentServer {
    /// Create a new ACP agent server instance.
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            guard_db: None,
            knowledge_bridge: None,
            initialized: false,
        }
    }

    /// Configure the server to use a specific GuardDb for execution gating.
    pub fn with_guard_db(mut self, db: GuardDb) -> Self {
        self.guard_db = Some(db);
        self
    }

    /// Handle an incoming request and produce a response.
    pub async fn handle_request(
        &mut self,
        request: AcpRequest,
    ) -> Result<AcpResponse, AcpServerError> {
        handle_request(self, request).await
    }
}

impl Default for AcpAgentServer {
    fn default() -> Self {
        Self::new()
    }
}
