//! CrabJar Zed ACP Server Library
//! Provides the server-side implementation of the Zed Agent Communication Protocol (ACP).
//! Includes session management, request handling, and execution gate integration.

pub mod types;
pub use types::{AcpRequest, AcpResponse, AcpSession, AcpServerError};

pub mod server;
pub use server::{AcpAgentServer, ACP_VERSION};

#[cfg(test)]
mod tests;
